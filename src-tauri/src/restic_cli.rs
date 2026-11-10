use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

use crate::models::{
    BackupProgressEvent, IntegrityCheckResult, Profile, RepoConfig, ResticNode, ResticSnapshot,
    ResticStats, RunResult, SessionCredentials,
};
use crate::security::{redact_secrets, validate_restore_path};
use crate::system::get_persistent_app_dir;

/// Executes a Command with a hard timeout, capturing output without blocking indefinitely.
/// Pipes are drained concurrently in dedicated background threads to prevent OS pipe buffer deadlocks.
pub fn run_command_with_timeout(
    mut cmd: Command,
    timeout: Duration,
) -> Result<std::process::Output, String> {
    cmd.stdin(Stdio::null());
    cmd.stdout(Stdio::piped());
    cmd.stderr(Stdio::piped());

    let mut child = cmd
        .spawn()
        .map_err(|e| format!("Failed to spawn process: {}", e))?;

    let mut stdout_handle = child
        .stdout
        .take()
        .ok_or_else(|| "Failed to capture stdout".to_string())?;
    let mut stderr_handle = child
        .stderr
        .take()
        .ok_or_else(|| "Failed to capture stderr".to_string())?;

    let stdout_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stdout_handle.read_to_end(&mut buf);
        buf
    });

    let stderr_thread = thread::spawn(move || {
        let mut buf = Vec::new();
        let _ = stderr_handle.read_to_end(&mut buf);
        buf
    });

    let start = Instant::now();
    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                let stdout = stdout_thread.join().unwrap_or_default();
                let stderr = stderr_thread.join().unwrap_or_default();
                return Ok(std::process::Output {
                    status,
                    stdout,
                    stderr,
                });
            }
            Ok(None) => {
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    let _ = child.wait();
                    let _ = stdout_thread.join();
                    let _ = stderr_thread.join();
                    return Err(format!(
                        "Operation timed out after {:.1}s: The remote repository or network is not responding.",
                        timeout.as_secs_f32()
                    ));
                }
                thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                let _ = child.kill();
                let _ = stdout_thread.join();
                let _ = stderr_thread.join();
                return Err(format!("Process wait error: {}", e));
            }
        }
    }
}

#[derive(Clone)]
pub struct ResticEngine {
    binary_path: PathBuf,
}

impl Default for ResticEngine {
    fn default() -> Self {
        Self::new()
    }
}

impl ResticEngine {
    /// Detects or sets the restic binary location.
    pub fn new() -> Self {
        let binary_path = Self::find_binary();
        Self { binary_path }
    }

    pub fn with_path(path: PathBuf) -> Self {
        Self { binary_path: path }
    }

    pub fn binary_path(&self) -> &Path {
        &self.binary_path
    }

    /// Locate restic binary:
    /// 1. USB directory layout (relative to executable and USB root)
    /// 2. Persistent local app bin dir (if installed on host)
    /// 3. Standard OS search paths / which restic
    pub fn find_binary() -> PathBuf {
        let target_bin = if cfg!(windows) {
            "restic.exe"
        } else {
            "restic"
        };

        // 1. Search relative to current executable (USB controller layout & macOS bundle)
        if let Ok(current_exe) = std::env::current_exe()
            && let Some(parent) = current_exe.parent()
        {
            let mut search_dirs = Vec::new();
            search_dirs.push(parent.to_path_buf());
            search_dirs.push(parent.join("bin"));
            // macOS app bundle: ResticDashboard.app/Contents/MacOS -> Resources/bin
            search_dirs.push(parent.join("../Resources/bin"));
            search_dirs.push(parent.join("../Resources"));
            // USB root or subfolder relative:
            search_dirs.push(parent.join("../bin"));
            search_dirs.push(parent.join("../../bin"));
            search_dirs.push(parent.join("../../../bin"));
            search_dirs.push(parent.join("../../../../bin"));

            for dir in search_dirs {
                let candidate = dir.join(target_bin);
                if candidate.is_file() {
                    if let Ok(canon) = candidate.canonicalize() {
                        return canon;
                    }
                    return candidate;
                }
            }
        }

        // 2. Check current working directory candidates (e.g. during dev or launch script)
        let cwd_candidates = [
            format!("./bin/{}", target_bin),
            format!("./{}", target_bin),
            format!("usb_dist/macos/bin/{}", target_bin),
            format!("usb_dist/linux/bin/{}", target_bin),
            format!("usb_dist/windows/bin/{}", target_bin),
            format!("../usb_dist/macos/bin/{}", target_bin),
        ];

        for cand in &cwd_candidates {
            let p = PathBuf::from(cand);
            if p.is_file() {
                if let Ok(canon) = p.canonicalize() {
                    return canon;
                }
                return p;
            }
        }

        // 3. Check persistent host directory (e.g. installed on this machine)
        let persistent_bin = get_persistent_app_dir().join("bin").join(target_bin);
        if persistent_bin.is_file() {
            return persistent_bin;
        }

        // 4. Host OS candidates / Homebrew fallback
        let candidates = [
            "/opt/homebrew/bin/restic",
            "/usr/local/bin/restic",
            "/usr/bin/restic",
            "C:\\ProgramData\\chocolatey\\bin\\restic.exe",
            "C:\\Windows\\System32\\restic.exe",
        ];

        for cand in &candidates {
            let p = PathBuf::from(cand);
            if p.is_file() {
                return p;
            }
        }

        // Fallback to name in PATH
        PathBuf::from(target_bin)
    }

    /// Builds a Command with structured arguments, repo flags, and credential environment variables.
    fn prepare_command(
        &self,
        subcommand: &str,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        extra_args: &[&str],
    ) -> (Command, Vec<String>) {
        let mut cmd = Command::new(&self.binary_path);
        cmd.stdin(Stdio::null()); // Prevent hanging on interactive stdin password prompts
        let mut sensitive_strings = Vec::new();

        // Subcommand
        cmd.arg(subcommand);

        // Target repository
        cmd.arg("-r");
        cmd.arg(&repo.target);

        // Insecure passwordless mode support
        if repo.insecure_no_password {
            cmd.arg("--insecure-no-password");
        }

        // Extra arguments
        for arg in extra_args {
            cmd.arg(arg);
        }

        // Credentials via environment variables
        if let Some(ref pass) = creds.password
            && !pass.is_empty()
        {
            cmd.env("RESTIC_PASSWORD", pass);
            sensitive_strings.push(pass.clone());
        }

        if let Some(ref key_id) = repo.b2_key_id
            && !key_id.is_empty()
        {
            cmd.env("AWS_ACCESS_KEY_ID", key_id);
        }

        if let Some(ref app_key) = creds.b2_application_key
            && !app_key.is_empty()
        {
            cmd.env("AWS_SECRET_ACCESS_KEY", app_key);
            sensitive_strings.push(app_key.clone());
        }

        (cmd, sensitive_strings)
    }

    /// Read-only connectivity test: runs `restic snapshots --no-lock --json --latest 1`
    /// Bounded by an 8-second timeout so invalid endpoints or retries never hang the app.
    pub fn test_connection(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
    ) -> Result<String, String> {
        let (cmd, sensitive) = self.prepare_command(
            "snapshots",
            repo,
            creds,
            &[
                "--no-lock",
                "--json",
                "--latest",
                "1",
                "--stuck-request-timeout=8s",
            ],
        );

        let output = run_command_with_timeout(cmd, Duration::from_secs(8))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        match output.status.code() {
            Some(0) => Ok("Successfully connected to repository.".to_string()),
            Some(10) => Err("Repository does not exist at specified path/bucket.".to_string()),
            Some(11) => Err("Repository is currently locked by another process.".to_string()),
            Some(12) => {
                Err("Repository password incorrect or empty password rejected.".to_string())
            }
            Some(code) => Err(format!(
                "Connection failed (exit code {}): {}",
                code,
                stderr.trim()
            )),
            None => Err("Process terminated by signal".to_string()),
        }
    }

    /// Initializes a new restic repository at the target destination.
    pub fn init_repository(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
    ) -> Result<String, String> {
        let (cmd, sensitive) = self.prepare_command("init", repo, creds, &[]);

        let output = run_command_with_timeout(cmd, Duration::from_secs(45))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);

        if output.status.success() {
            let msg = if !stdout.trim().is_empty() {
                stdout.trim().to_string()
            } else {
                "Repository initialized successfully.".to_string()
            };
            Ok(msg)
        } else {
            Err(format!("Initialization failed: {}", stderr.trim()))
        }
    }

    /// List snapshots with optional machine hostname and path filtering.
    pub fn list_snapshots(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        host_filter: Option<&str>,
        path_filter: Option<&str>,
    ) -> Result<Vec<ResticSnapshot>, String> {
        let mut extra_args = vec!["--no-lock", "--json"];

        let host_arg;
        if let Some(h) = host_filter
            && !h.trim().is_empty()
        {
            host_arg = format!("--host={}", h);
            extra_args.push(&host_arg);
        }

        let path_arg;
        if let Some(p) = path_filter
            && !p.trim().is_empty()
        {
            path_arg = format!("--path={}", p);
            extra_args.push(&path_arg);
        }

        let (cmd, sensitive) = self.prepare_command("snapshots", repo, creds, &extra_args);
        let output = run_command_with_timeout(cmd, Duration::from_secs(45))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!(
                "Failed to list snapshots (exit code {:?}): {}",
                output.status.code(),
                stderr.trim()
            ));
        }

        let snapshots: Vec<ResticSnapshot> = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse snapshots JSON: {}", e))?;

        Ok(snapshots)
    }

    /// Retrieve repository statistics (raw-data or restore-size)
    pub fn get_stats(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        mode: &str, // "raw-data" or "restore-size"
        snapshot_id: Option<&str>,
    ) -> Result<ResticStats, String> {
        let mut extra_args = vec!["--no-lock", "--json", "--mode", mode];
        if let Some(id) = snapshot_id {
            extra_args.push(id);
        }

        let (cmd, sensitive) = self.prepare_command("stats", repo, creds, &extra_args);
        let output = run_command_with_timeout(cmd, Duration::from_secs(90))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!(
                "Failed to calculate stats in mode '{}': {}",
                mode,
                stderr.trim()
            ));
        }

        let stats: ResticStats = serde_json::from_str(&stdout)
            .map_err(|e| format!("Failed to parse stats JSON: {}", e))?;

        Ok(stats)
    }

    /// List files and folders inside a snapshot.
    /// If recursive is true, traverses all subdirectories (--recursive).
    pub fn list_nodes(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        snapshot_id: &str,
        dir_filter: Option<&str>,
        recursive: bool,
    ) -> Result<Vec<ResticNode>, String> {
        let mut extra_args = vec!["--no-lock", "--json"];
        if recursive {
            extra_args.push("--recursive");
        }
        extra_args.push(snapshot_id);
        if let Some(dir) = dir_filter
            && !dir.trim().is_empty()
        {
            extra_args.push(dir);
        }

        let (cmd, sensitive) = self.prepare_command("ls", repo, creds, &extra_args);
        let timeout_secs = if recursive { 300 } else { 60 };
        let output = run_command_with_timeout(cmd, Duration::from_secs(timeout_secs))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!(
                "Failed to list files in snapshot: {}",
                stderr.trim()
            ));
        }

        let mut nodes = Vec::new();
        for line in stdout.lines() {
            let line = line.trim();
            if line.is_empty() {
                continue;
            }
            if let Ok(node) = serde_json::from_str::<ResticNode>(line) {
                // Filter out the initial header "message_type: snapshot"
                if node.node_type == "file"
                    || node.node_type == "dir"
                    || node.node_type == "symlink"
                {
                    nodes.push(node);
                }
            }
        }

        Ok(nodes)
    }

    /// Runs a manual or scheduled backup with structured arguments and streaming output.
    pub fn run_backup<F>(
        &self,
        repo: &RepoConfig,
        profile: &Profile,
        creds: &SessionCredentials,
        cancel_signal: Arc<AtomicBool>,
        mut on_progress: F,
    ) -> Result<RunResult, String>
    where
        F: FnMut(BackupProgressEvent),
    {
        // Preflight check: verify source paths exist
        let mut missing_sources = Vec::new();
        for src in &profile.source_paths {
            let p = Path::new(src);
            if !p.exists() {
                missing_sources.push(src.clone());
            }
        }

        if !missing_sources.is_empty() {
            return Err(format!(
                "Backup aborted: configured source path(s) do not exist: {}",
                missing_sources.join(", ")
            ));
        }

        let start_time_chrono = chrono::Utc::now();
        let run_id = uuid::Uuid::new_v4().to_string();

        let mut extra_args = vec!["--json"];

        // Exclusions
        let mut exclusion_args = Vec::new();
        for exc in &profile.exclusions {
            exclusion_args.push(format!("--exclude={}", exc));
        }
        for exc_arg in &exclusion_args {
            extra_args.push(exc_arg);
        }

        // Tags
        let mut tag_args = Vec::new();
        for tag in &profile.tags {
            tag_args.push(format!("--tag={}", tag));
        }

        // Distinction / Custom label tag
        if let Some(ref label) = profile.distinction_label {
            let trimmed = label.trim();
            if !trimmed.is_empty() {
                tag_args.push(format!("--tag={}", trimmed));
            }
        }

        // Timestamp tag in user-friendly local date format: MM-DD-YY_HH-MM (defaults to true)
        if profile.tag_timestamp.unwrap_or(true) {
            let time_tag = chrono::Local::now().format("%m-%d-%y_%H-%M").to_string();
            tag_args.push(format!("--tag={}", time_tag));
        }

        // Pinned / Important tag
        if profile.is_pinned == Some(true) {
            tag_args.push("--tag=important".to_string());
            tag_args.push("--tag=pinned".to_string());
        }

        for tag_arg in &tag_args {
            extra_args.push(tag_arg);
        }

        // Source paths
        let source_slices: Vec<&str> = profile.source_paths.iter().map(|s| s.as_str()).collect();
        for s in &source_slices {
            extra_args.push(s);
        }

        let (mut cmd, sensitive) = self.prepare_command("backup", repo, creds, &extra_args);
        cmd.stdout(Stdio::piped());
        cmd.stderr(Stdio::piped());

        let mut child = cmd
            .spawn()
            .map_err(|e| format!("Failed to spawn backup process: {}", e))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let mut sanitized_logs = String::new();
        let mut snapshot_id = None;
        let mut files_new = 0;
        let mut files_changed = 0;
        let mut files_unmodified = 0;
        let mut data_added = 0;
        let mut total_files = 0;
        let mut total_bytes = 0;

        let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
        let reader = BufReader::new(stdout);

        for line_res in reader.lines() {
            if cancel_signal.load(Ordering::Relaxed) {
                let _ = child.kill();
                return Err("Backup was cancelled by user.".to_string());
            }

            if let Ok(line) = line_res {
                let clean_line = redact_secrets(&line, &sensitive_slices);
                sanitized_logs.push_str(&clean_line);
                sanitized_logs.push('\n');

                if let Ok(v) = serde_json::from_str::<serde_json::Value>(&clean_line) {
                    let msg_type = v.get("message_type").and_then(|s| s.as_str()).unwrap_or("");
                    if msg_type == "status" {
                        let percent = v
                            .get("percent_done")
                            .and_then(|p| p.as_f64())
                            .unwrap_or(0.0)
                            * 100.0;
                        let t_files = v.get("total_files").and_then(|f| f.as_u64()).unwrap_or(0);
                        let f_done = v.get("files_done").and_then(|f| f.as_u64()).unwrap_or(0);
                        let t_bytes = v.get("total_bytes").and_then(|b| b.as_u64()).unwrap_or(0);
                        let b_done = v.get("bytes_done").and_then(|b| b.as_u64()).unwrap_or(0);
                        let cur_file = v
                            .get("current_files")
                            .and_then(|cf| cf.as_array())
                            .and_then(|arr| arr.first())
                            .and_then(|val| val.as_str())
                            .map(|s| s.to_string());
                        let elapsed = v
                            .get("seconds_elapsed")
                            .and_then(|s| s.as_u64())
                            .unwrap_or(0);
                        let remaining = v.get("seconds_remaining").and_then(|s| s.as_u64());

                        on_progress(BackupProgressEvent {
                            percent_done: percent,
                            total_files: t_files,
                            files_done: f_done,
                            total_bytes: t_bytes,
                            bytes_done: b_done,
                            current_file: cur_file,
                            seconds_elapsed: elapsed,
                            seconds_remaining: remaining,
                        });
                    } else if msg_type == "summary" {
                        files_new = v.get("files_new").and_then(|f| f.as_u64()).unwrap_or(0);
                        files_changed =
                            v.get("files_changed").and_then(|f| f.as_u64()).unwrap_or(0);
                        files_unmodified = v
                            .get("files_unmodified")
                            .and_then(|f| f.as_u64())
                            .unwrap_or(0);
                        data_added = v.get("data_added").and_then(|d| d.as_u64()).unwrap_or(0);
                        total_files = v
                            .get("total_files_processed")
                            .and_then(|tf| tf.as_u64())
                            .unwrap_or(0);
                        total_bytes = v
                            .get("total_bytes_processed")
                            .and_then(|tb| tb.as_u64())
                            .unwrap_or(0);
                        snapshot_id = v
                            .get("snapshot_id")
                            .and_then(|s| s.as_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }

        let status = child
            .wait()
            .map_err(|e| format!("Failed to wait for child: {}", e))?;
        let end_time_chrono = chrono::Utc::now();
        let duration = (end_time_chrono - start_time_chrono).num_milliseconds() as f64 / 1000.0;

        let exit_code = status.code().unwrap_or(-1);
        let run_status = match exit_code {
            0 => "success".to_string(),
            3 => "partial_success".to_string(), // Some source data unreadable, snapshot created
            10 => "failed".to_string(),
            11 => "failed".to_string(),
            12 => "failed".to_string(),
            _ => "failed".to_string(),
        };

        let error_message = match exit_code {
            0 => None,
            3 => Some(
                "Completed with warnings: some source data could not be read (snapshot created)."
                    .to_string(),
            ),
            10 => Some("Repository does not exist.".to_string()),
            11 => Some("Repository is already locked by another process.".to_string()),
            12 => Some("Invalid repository password.".to_string()),
            c => Some(format!("Backup failed with exit code {}.", c)),
        };

        // Enforce retention policy if configured, protecting pinned / important snapshots
        if (exit_code == 0 || exit_code == 3)
            && let Some(keep_last) = profile.keep_last
            && keep_last > 0
            && let Ok(forget_out) =
                self.run_forget_policy(repo, creds, keep_last, &["important", "pinned"])
        {
            sanitized_logs.push_str("\n--- Retention Policy Pruned Old Backups ---\n");
            sanitized_logs.push_str(&forget_out);
        }

        Ok(RunResult {
            run_id,
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            machine_id: profile.machine_id.clone(),
            start_time: start_time_chrono.to_rfc3339(),
            end_time: end_time_chrono.to_rfc3339(),
            duration_seconds: duration,
            status: run_status,
            exit_code,
            files_new,
            files_changed,
            files_unmodified,
            data_added_bytes: data_added,
            total_files_processed: total_files,
            total_bytes_processed: total_bytes,
            snapshot_id,
            sanitized_logs,
            error_message,
            is_verified_local_run: true,
        })
    }

    /// Prunes snapshots keeping the last N, strictly preserving pinned/important snapshots.
    pub fn run_forget_policy(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        keep_last: u32,
        keep_tags: &[&str],
    ) -> Result<String, String> {
        let keep_last_str = keep_last.to_string();
        let mut extra_args = vec!["--keep-last", &keep_last_str, "--prune"];
        let mut tag_args = Vec::new();
        for t in keep_tags {
            tag_args.push(format!("--keep-tag={}", t));
        }
        for ta in &tag_args {
            extra_args.push(ta);
        }

        let (cmd, sensitive) = self.prepare_command("forget", repo, creds, &extra_args);
        let timeout = Duration::from_secs(1800); // 30 mins for forget & prune
        let output = run_command_with_timeout(cmd, timeout)?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!(
                "Forget policy execution failed (code {:?}): {}",
                output.status.code(),
                stderr.trim()
            ));
        }

        Ok(stdout)
    }

    /// Adds or removes tags on a snapshot
    pub fn tag_snapshot(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        snapshot_id: &str,
        add_tags: &[&str],
        remove_tags: &[&str],
    ) -> Result<String, String> {
        let mut extra_args = Vec::new();
        let mut add_args = Vec::new();
        for t in add_tags {
            add_args.push(format!("--add={}", t));
        }
        for a in &add_args {
            extra_args.push(a.as_str());
        }

        let mut rem_args = Vec::new();
        for t in remove_tags {
            rem_args.push(format!("--remove={}", t));
        }
        for r in &rem_args {
            extra_args.push(r.as_str());
        }

        extra_args.push(snapshot_id);

        let (cmd, sensitive) = self.prepare_command("tag", repo, creds, &extra_args);
        let output = run_command_with_timeout(cmd, Duration::from_secs(15))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!("Tag operation failed: {}", stderr.trim()));
        }

        Ok(format!(
            "Snapshot {} tags updated.",
            &snapshot_id[..8.min(snapshot_id.len())]
        ))
    }

    /// Permanently deletes a single specific snapshot by ID, with optional prune.
    pub fn delete_single_snapshot(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        snapshot_id: &str,
        prune: bool,
    ) -> Result<String, String> {
        let mut extra_args = vec![snapshot_id];
        if prune {
            extra_args.push("--prune");
        }

        let (cmd, sensitive) = self.prepare_command("forget", repo, creds, &extra_args);
        let timeout = if prune {
            Duration::from_secs(1800)
        } else {
            Duration::from_secs(30)
        };
        let output = run_command_with_timeout(cmd, timeout)?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!(
                "Failed to delete snapshot {} (code {:?}): {}",
                &snapshot_id[..8.min(snapshot_id.len())],
                output.status.code(),
                stderr.trim()
            ));
        }

        Ok(stdout)
    }

    /// Restore files from a snapshot to a target directory.
    pub fn run_restore(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        snapshot_id: &str,
        target_dir: &str,
        include_paths: &[String],
        overwrite_confirmed: bool,
    ) -> Result<String, String> {
        let target_path = Path::new(target_dir);
        if !target_path.exists() {
            std::fs::create_dir_all(target_path)
                .map_err(|e| format!("Failed to create restore target directory: {}", e))?;
        }

        // Validate each include path to prevent directory traversal
        for inc in include_paths {
            validate_restore_path(target_path, inc)?;
        }

        let mut extra_args = vec![snapshot_id, "--target", target_dir];

        // Explicit confirmation required for overwriting
        let overwrite_flag = if overwrite_confirmed {
            "--overwrite=always"
        } else {
            "--overwrite=never"
        };
        extra_args.push(overwrite_flag);

        let mut inc_args = Vec::new();
        for inc in include_paths {
            inc_args.push(format!("--include={}", inc));
        }
        for inc_arg in &inc_args {
            extra_args.push(inc_arg);
        }

        let (cmd, sensitive) = self.prepare_command("restore", repo, creds, &extra_args);
        let output = run_command_with_timeout(cmd, Duration::from_secs(3600))?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        if !output.status.success() {
            return Err(format!(
                "Restore failed (exit code {:?}): {}",
                output.status.code(),
                stderr.trim()
            ));
        }

        Ok("Restore completed successfully.".to_string())
    }

    /// Repository structural and integrity check.
    pub fn run_check(
        &self,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        read_data: bool,
    ) -> Result<IntegrityCheckResult, String> {
        let mut extra_args = vec!["--no-lock"];
        if read_data {
            extra_args.push("--read-data-subset=5%"); // Safe 5% random subset to verify data blocks without locking hours
        }

        let (cmd, sensitive) = self.prepare_command("check", repo, creds, &extra_args);
        let timeout = if read_data {
            Duration::from_secs(600)
        } else {
            Duration::from_secs(120)
        };
        let output = run_command_with_timeout(cmd, timeout)?;

        let sensitive_slices: Vec<&str> = sensitive.iter().map(|s| s.as_str()).collect();
        let stdout = redact_secrets(&String::from_utf8_lossy(&output.stdout), &sensitive_slices);
        let stderr = redact_secrets(&String::from_utf8_lossy(&output.stderr), &sensitive_slices);

        let success = output.status.success();
        let details = if success {
            stdout.trim().to_string()
        } else {
            stderr.trim().to_string()
        };

        Ok(IntegrityCheckResult {
            timestamp: chrono::Utc::now().to_rfc3339(),
            success,
            details,
            read_data,
        })
    }
}
