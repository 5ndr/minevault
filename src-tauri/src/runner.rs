use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::sync::atomic::AtomicBool;
use std::thread;
use std::time::Duration;

use crate::models::{Profile, RepoConfig, RunResult, SessionCredentials};
use crate::restic_cli::ResticEngine;

/// Unattended runner main execution logic.
pub fn execute_unattended_runner(profile_id: &str, base_dir: &Path) -> Result<i32, String> {
    let configs_dir = base_dir.join("configs");
    let secrets_dir = base_dir.join("secrets");
    let logs_dir = base_dir.join("logs");
    let status_dir = base_dir.join("status");
    let locks_dir = base_dir.join("locks");

    let config_file = configs_dir.join(format!("{}.json", profile_id));
    if !config_file.is_file() {
        return Err(format!(
            "Profile configuration not found at {}",
            config_file.display()
        ));
    }

    let config_raw = fs::read_to_string(&config_file)
        .map_err(|e| format!("Failed to read config file: {}", e))?;
    let config_json: serde_json::Value =
        serde_json::from_str(&config_raw).map_err(|e| format!("Invalid JSON in config: {}", e))?;

    let profile: Profile =
        serde_json::from_value(config_json.get("profile").cloned().unwrap_or_default())
            .map_err(|e| format!("Failed to deserialize profile: {}", e))?;
    let repo: RepoConfig =
        serde_json::from_value(config_json.get("repo").cloned().unwrap_or_default())
            .map_err(|e| format!("Failed to deserialize repo: {}", e))?;

    // Read secrets file (0600)
    let secret_file = secrets_dir.join(format!("{}.env", profile_id));
    let mut creds = SessionCredentials::default();
    if secret_file.is_file()
        && let Ok(content) = fs::read_to_string(&secret_file)
    {
        for line in content.lines() {
            let trimmed = line.trim();
            if let Some((k, v)) = trimmed.split_once('=') {
                match k.trim() {
                    "RESTIC_PASSWORD" => creds.password = Some(v.trim().to_string()),
                    "AWS_SECRET_ACCESS_KEY" => {
                        creds.b2_application_key = Some(v.trim().to_string())
                    }
                    _ => {}
                }
            }
        }
    }

    // 1. Single-instance lock to prevent overlapping runs
    let lock_file = locks_dir.join(format!("{}.lock", profile_id));
    if lock_file.is_file()
        && let Ok(pid_str) = fs::read_to_string(&lock_file)
        && let Ok(pid) = pid_str.trim().parse::<i32>()
        && is_pid_alive(pid)
    {
        let msg = format!(
            "Skipping backup: another instance (PID {}) is already running.",
            pid
        );
        append_log(&logs_dir, profile_id, &msg);
        return Ok(0);
    }

    let current_pid = std::process::id();
    let _ = fs::write(&lock_file, current_pid.to_string());

    // Ensure lock is removed on exit
    struct LockGuard<'a>(&'a PathBuf);
    impl<'a> Drop for LockGuard<'a> {
        fn drop(&mut self) {
            let _ = fs::remove_file(self.0);
        }
    }
    let _guard = LockGuard(&lock_file);

    // 2. Process check: ensure Minecraft launcher and game instances are closed
    let mut game_running = false;
    for attempt in 0..6 {
        let proc_check = crate::process_guard::detect_minecraft_processes();
        if !proc_check.is_running {
            game_running = false;
            break;
        }
        game_running = true;
        if attempt < 5 {
            thread::sleep(Duration::from_secs(5));
        }
    }

    if game_running {
        let warn_msg = "Scheduled backup postponed: Minecraft launcher or active game instance is running. Postponed to protect world saves from corruption.";
        append_log(&logs_dir, profile_id, warn_msg);
        let postponed_result = RunResult {
            run_id: uuid::Uuid::new_v4().to_string(),
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            machine_id: profile.machine_id.clone(),
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: chrono::Utc::now().to_rfc3339(),
            duration_seconds: 0.0,
            status: "postponed".to_string(),
            exit_code: 0,
            files_new: 0,
            files_changed: 0,
            files_unmodified: 0,
            data_added_bytes: 0,
            total_files_processed: 0,
            total_bytes_processed: 0,
            snapshot_id: None,
            sanitized_logs: warn_msg.to_string(),
            error_message: Some(warn_msg.to_string()),
            is_verified_local_run: true,
        };
        save_status(&status_dir, profile_id, &postponed_result);
        return Ok(0);
    }

    // 3. Prepare restic engine
    let engine = ResticEngine::new();

    // 3. Network reachability test with bounded retries (3 attempts, 10s sleep)
    let mut connected = false;
    let mut last_conn_err = String::new();
    for attempt in 1..=3 {
        match engine.test_connection(&repo, &creds) {
            Ok(_) => {
                connected = true;
                break;
            }
            Err(e) => {
                last_conn_err = e;
                if attempt < 3 {
                    thread::sleep(Duration::from_secs(10));
                }
            }
        }
    }

    if !connected {
        let err_msg = format!(
            "Unattended run aborted: network/repo unreachable after 3 attempts: {}",
            last_conn_err
        );
        append_log(&logs_dir, profile_id, &err_msg);

        let failed_result = RunResult {
            run_id: uuid::Uuid::new_v4().to_string(),
            profile_id: profile.id.clone(),
            profile_name: profile.name.clone(),
            machine_id: profile.machine_id.clone(),
            start_time: chrono::Utc::now().to_rfc3339(),
            end_time: chrono::Utc::now().to_rfc3339(),
            duration_seconds: 0.0,
            status: "failed".to_string(),
            exit_code: 1,
            files_new: 0,
            files_changed: 0,
            files_unmodified: 0,
            data_added_bytes: 0,
            total_files_processed: 0,
            total_bytes_processed: 0,
            snapshot_id: None,
            sanitized_logs: err_msg.clone(),
            error_message: Some(err_msg),
            is_verified_local_run: true,
        };

        save_status(&status_dir, profile_id, &failed_result);
        return Ok(1);
    }

    // 4. Run backup
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let backup_res = engine.run_backup(&repo, &profile, &creds, cancel_flag, |_| {});

    match backup_res {
        Ok(result) => {
            append_log(
                &logs_dir,
                profile_id,
                &format!(
                    "Backup finished with status '{}' (exit code {}). Duration: {:.2}s. Files: {}. Added: {} bytes.",
                    result.status,
                    result.exit_code,
                    result.duration_seconds,
                    result.total_files_processed,
                    result.data_added_bytes
                ),
            );
            save_status(&status_dir, profile_id, &result);
            Ok(result.exit_code)
        }
        Err(err) => {
            append_log(
                &logs_dir,
                profile_id,
                &format!("Backup failed with error: {}", err),
            );
            let failed_result = RunResult {
                run_id: uuid::Uuid::new_v4().to_string(),
                profile_id: profile.id.clone(),
                profile_name: profile.name.clone(),
                machine_id: profile.machine_id.clone(),
                start_time: chrono::Utc::now().to_rfc3339(),
                end_time: chrono::Utc::now().to_rfc3339(),
                duration_seconds: 0.0,
                status: "failed".to_string(),
                exit_code: 1,
                files_new: 0,
                files_changed: 0,
                files_unmodified: 0,
                data_added_bytes: 0,
                total_files_processed: 0,
                total_bytes_processed: 0,
                snapshot_id: None,
                sanitized_logs: err.clone(),
                error_message: Some(err),
                is_verified_local_run: true,
            };
            save_status(&status_dir, profile_id, &failed_result);
            Ok(1)
        }
    }
}

fn append_log(logs_dir: &Path, profile_id: &str, message: &str) {
    let log_file = logs_dir.join(format!("{}.log", profile_id));
    rotate_if_needed(&log_file);

    let timestamp = chrono::Local::now().format("%Y-%m-%d %H:%M:%S");
    let line = format!("[{}] {}\n", timestamp, message);

    if let Ok(mut f) = OpenOptions::new().create(true).append(true).open(&log_file) {
        let _ = f.write_all(line.as_bytes());
    }
}

fn rotate_if_needed(log_path: &Path) {
    if let Ok(meta) = fs::metadata(log_path) {
        // Rotate if greater than 10 MB
        if meta.len() > 10 * 1024 * 1024 {
            let backup_path = log_path.with_extension("log.old");
            let _ = fs::rename(log_path, backup_path);
        }
    }
}

fn save_status(status_dir: &Path, profile_id: &str, result: &RunResult) {
    let status_file = status_dir.join(format!("{}.json", profile_id));
    if let Ok(json) = serde_json::to_string_pretty(result) {
        let _ = fs::write(&status_file, json);
    }
}

pub fn is_pid_alive(pid: i32) -> bool {
    #[cfg(unix)]
    {
        unsafe { libc::kill(pid, 0) == 0 }
    }
    #[cfg(windows)]
    {
        let out = std::process::Command::new("tasklist")
            .args(["/FI", &format!("PID eq {}", pid), "/NH"])
            .output();
        if let Ok(out) = out {
            let stdout = String::from_utf8_lossy(&out.stdout);
            return stdout.contains(&pid.to_string());
        }
        false
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = pid;
        false
    }
}
