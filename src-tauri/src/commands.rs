use std::fs;
use std::path::Path;
use std::process::Command;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use tauri::{AppHandle, Emitter, State};

use crate::mock_adapter::MockResticAdapter;
use crate::models::{
    DashboardAlert, DashboardSummary, FolderSizeBreakdown, GrowthPoint, IntegrityCheckResult,
    Profile, RepoConfig, ResticNode, ResticSnapshot, RestoreOptions, RestoreTestRecord, RunResult,
    SchedulerStatus, SessionCredentials, StorageOverview,
};
use crate::restic_cli::ResticEngine;
use crate::scheduler::get_active_scheduler;
use crate::system::{
    detect_prism_launcher_path, get_disk_free_space, get_machine_id, get_machine_name,
    get_persistent_app_dir, list_prism_launcher_instances,
};

pub struct AppState {
    pub cancel_flag: Arc<AtomicBool>,
    pub is_backup_running: Arc<AtomicBool>,
}

#[derive(serde::Serialize)]
pub struct SystemInfo {
    pub machine_id: String,
    pub machine_name: String,
    pub platform: String,
    pub prism_launcher_path: Option<String>,
    pub prism_instances: Vec<String>,
    pub persistent_app_dir: String,
    pub restic_found: bool,
    pub restic_path: String,
    pub is_usb_bundled: bool,
}

#[tauri::command]
pub fn get_system_info() -> SystemInfo {
    let engine = ResticEngine::new();
    let restic_path = engine.binary_path().to_string_lossy().to_string();
    let restic_found = engine.binary_path().is_file()
        || Command::new(engine.binary_path())
            .arg("version")
            .output()
            .map(|o| o.status.success())
            .unwrap_or(false);

    let is_usb_bundled = restic_path.contains("usb_dist")
        || restic_path.contains("Resources/bin")
        || restic_path.contains("/Volumes/")
        || restic_path.contains("D:\\")
        || restic_path.contains("E:\\");

    let prism_path = detect_prism_launcher_path();
    let prism_instances = if let Some(ref p) = prism_path {
        list_prism_launcher_instances(p)
    } else {
        Vec::new()
    };

    SystemInfo {
        machine_id: get_machine_id(),
        machine_name: get_machine_name(),
        platform: std::env::consts::OS.to_string(),
        prism_launcher_path: prism_path.map(|p| p.to_string_lossy().to_string()),
        prism_instances,
        persistent_app_dir: get_persistent_app_dir().to_string_lossy().to_string(),
        restic_found,
        restic_path,
        is_usb_bundled,
    }
}

#[tauri::command]
pub async fn test_repo_connection(
    repo: RepoConfig,
    creds: SessionCredentials,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        engine.test_connection(&repo, &creds)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn list_snapshots(
    repo: RepoConfig,
    creds: SessionCredentials,
    host_filter: Option<String>,
    path_filter: Option<String>,
) -> Result<Vec<ResticSnapshot>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        engine.list_snapshots(
            &repo,
            &creds,
            host_filter.as_deref(),
            path_filter.as_deref(),
        )
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn get_storage_overview(
    repo: RepoConfig,
    creds: SessionCredentials,
) -> Result<StorageOverview, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();

        // 1. Raw deduplicated data size in repository
        let raw_stats = engine.get_stats(&repo, &creds, "raw-data", None)?;
        // 2. Logical uncompressed restore size across all snapshots
        let restore_stats = engine.get_stats(&repo, &creds, "restore-size", None)?;

        let now_ts = chrono::Utc::now().to_rfc3339();

        // 3. Accumulate real growth history in persistent local history file
        let base_dir = get_persistent_app_dir();
        let history_file = base_dir.join("storage_history.json");
        let mut history: Vec<GrowthPoint> = Vec::new();

        if history_file.is_file()
            && let Ok(content) = fs::read_to_string(&history_file)
            && let Ok(existing) = serde_json::from_str::<Vec<GrowthPoint>>(&content)
        {
            history = existing;
        }

        // Add current measurement to real history
        history.push(GrowthPoint {
            timestamp: now_ts.clone(),
            raw_bytes: raw_stats.total_size,
            restore_bytes: restore_stats.total_size,
            snapshots: raw_stats.snapshots_count,
            is_demo: false,
        });

        // Save updated history (keep up to 50 historical measurement points)
        if history.len() > 50 {
            history.remove(0);
        }
        let _ = fs::create_dir_all(&base_dir);
        let _ = fs::write(&history_file, serde_json::to_string(&history).unwrap());

        // Check if cached B2 provider usage is available
        let b2_provider_details = crate::b2_api::load_cached_b2_usage();
        let provider_b2_usage = b2_provider_details.as_ref().map(|u| u.total_bytes);

        Ok(StorageOverview {
            raw_data_size: raw_stats.total_size,
            restore_size: restore_stats.total_size,
            provider_b2_usage,
            b2_provider_details,
            snapshots_count: raw_stats.snapshots_count,
            growth_history: history,
            calculated_at: now_ts,
            measurement_method:
                "restic stats --mode raw-data (repo blobs) & --mode restore-size (logical)"
                    .to_string(),
            is_demo: false,
        })
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

fn categorize_minecraft_path(path: &str) -> String {
    let lower = path.to_lowercase();
    let parts: Vec<&str> = path.split(['/', '\\']).filter(|s| !s.is_empty()).collect();

    // Check for instances/<instance_name>
    let mut instance_name: Option<String> = None;
    for (i, part) in parts.iter().enumerate() {
        if (part.eq_ignore_ascii_case("instances") || part.eq_ignore_ascii_case("profiles"))
            && let Some(next) = parts.get(i + 1)
        {
            instance_name = Some(next.to_string());
            break;
        }
    }

    // 1. Worlds & Saves
    if let Some(saves_idx) = parts.iter().position(|p| p.eq_ignore_ascii_case("saves")) {
        if let Some(world) = parts.get(saves_idx + 1) {
            return if let Some(inst) = &instance_name {
                format!("Instance: {} / World: {}", inst, world)
            } else {
                format!("World: {}", world)
            };
        }
        return if let Some(inst) = &instance_name {
            format!("Instance: {} / Saves", inst)
        } else {
            "Worlds & Saves".to_string()
        };
    }

    // 2. Mods & Plugins
    if parts
        .iter()
        .any(|p| p.eq_ignore_ascii_case("mods") || p.eq_ignore_ascii_case("plugins"))
        || lower.ends_with(".jar")
    {
        return if let Some(inst) = &instance_name {
            format!("Instance: {} (Mods)", inst)
        } else {
            "Mods & Plugins".to_string()
        };
    }

    // 3. Resource Packs
    if parts
        .iter()
        .any(|p| p.eq_ignore_ascii_case("resourcepacks") || p.eq_ignore_ascii_case("texturepacks"))
    {
        return if let Some(inst) = &instance_name {
            format!("Instance: {} (Resource Packs)", inst)
        } else {
            "Resource Packs".to_string()
        };
    }

    // 4. Shader Packs
    if parts.iter().any(|p| p.eq_ignore_ascii_case("shaderpacks")) {
        return if let Some(inst) = &instance_name {
            format!("Instance: {} (Shader Packs)", inst)
        } else {
            "Shader Packs".to_string()
        };
    }

    // 5. Configs
    if parts
        .iter()
        .any(|p| p.eq_ignore_ascii_case("config") || p.eq_ignore_ascii_case("configs"))
    {
        return if let Some(inst) = &instance_name {
            format!("Instance: {} (Configs)", inst)
        } else {
            "Configs & Settings".to_string()
        };
    }

    // 6. Game Libraries & Assets
    if parts.iter().any(|p| p.eq_ignore_ascii_case("libraries")) {
        return "Game Libraries".to_string();
    }
    if parts.iter().any(|p| p.eq_ignore_ascii_case("assets")) {
        return "Game Assets".to_string();
    }

    // 7. Logs & Crash Reports
    if parts
        .iter()
        .any(|p| p.eq_ignore_ascii_case("logs") || p.eq_ignore_ascii_case("crash-reports"))
    {
        return "Logs & Crash Reports".to_string();
    }

    if let Some(inst) = instance_name {
        return format!("Instance: {}", inst);
    }

    if lower.ends_with(".cfg") || lower.ends_with(".json") || lower.ends_with(".txt") {
        return "Launcher Configurations".to_string();
    }

    // Default to the last 2 directory components or top level
    if parts.len() >= 3 {
        format!("{}/{}", parts[parts.len() - 3], parts[parts.len() - 2])
    } else if let Some(first) = parts.first() {
        first.to_string()
    } else {
        "Miscellaneous Files".to_string()
    }
}

#[tauri::command]
pub async fn get_snapshot_folder_breakdown(
    repo: RepoConfig,
    creds: SessionCredentials,
    snapshot_id: String,
) -> Result<Vec<FolderSizeBreakdown>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_dir = get_persistent_app_dir();
        let cache_file = base_dir
            .join("cache")
            .join("breakdown")
            .join(format!("{}.json", snapshot_id));

        // Return immediately if cached
        if cache_file.is_file()
            && let Ok(content) = fs::read_to_string(&cache_file)
            && let Ok(cached) = serde_json::from_str::<Vec<FolderSizeBreakdown>>(&content)
            && !cached.is_empty()
        {
            return Ok(cached);
        }

        let nodes_cache_file = base_dir
            .join("cache")
            .join("nodes")
            .join(format!("{}.json", snapshot_id));

        // Use cached full nodes if available to avoid duplicate network queries
        let nodes: Vec<ResticNode> = if nodes_cache_file.is_file() {
            if let Ok(content) = fs::read_to_string(&nodes_cache_file) {
                serde_json::from_str(&content).unwrap_or_default()
            } else {
                Vec::new()
            }
        } else {
            Vec::new()
        };

        let nodes = if !nodes.is_empty() {
            nodes
        } else {
            let engine = ResticEngine::new();
            // Pass recursive=true to obtain all nested files in the snapshot
            let fetched = engine.list_nodes(&repo, &creds, &snapshot_id, None, true)?;
            if let Some(parent) = nodes_cache_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(
                &nodes_cache_file,
                serde_json::to_string(&fetched).unwrap_or_default(),
            );
            fetched
        };

        let mut dir_sizes: std::collections::HashMap<String, (u64, u64)> =
            std::collections::HashMap::new();
        let mut total_size: u64 = 0;

        for node in nodes {
            if node.node_type != "file" {
                continue;
            }
            let size = node.size.unwrap_or(0);
            total_size += size;

            let group_key = categorize_minecraft_path(&node.path);
            let entry = dir_sizes.entry(group_key).or_insert((0, 0));
            entry.0 += size;
            entry.1 += 1;
        }

        let total_categories = dir_sizes.len().max(1) as f64;
        let mut breakdown: Vec<FolderSizeBreakdown> = dir_sizes
            .into_iter()
            .map(|(folder_path, (size_bytes, file_count))| {
                let percentage = if total_size > 0 {
                    (size_bytes as f64 / total_size as f64) * 100.0
                } else {
                    100.0 / total_categories
                };
                FolderSizeBreakdown {
                    folder_path,
                    size_bytes,
                    file_count,
                    percentage,
                }
            })
            .filter(|b| b.size_bytes > 0 || b.file_count > 0)
            .collect();

        breakdown.sort_by(|a, b| {
            b.size_bytes
                .cmp(&a.size_bytes)
                .then_with(|| b.file_count.cmp(&a.file_count))
        });

        // Persist to local disk cache for instant future loads
        if !breakdown.is_empty() {
            if let Some(parent) = cache_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(
                &cache_file,
                serde_json::to_string(&breakdown).unwrap_or_default(),
            );
        }

        Ok(breakdown)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn list_snapshot_nodes(
    repo: RepoConfig,
    creds: SessionCredentials,
    snapshot_id: String,
    dir_filter: Option<String>,
) -> Result<Vec<ResticNode>, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_dir = get_persistent_app_dir();
        let nodes_cache_file = base_dir
            .join("cache")
            .join("nodes")
            .join(format!("{}.json", snapshot_id));

        // Return immediately if full nodes are cached and no dir_filter
        if dir_filter.is_none()
            && nodes_cache_file.is_file()
            && let Ok(content) = fs::read_to_string(&nodes_cache_file)
            && let Ok(cached) = serde_json::from_str::<Vec<ResticNode>>(&content)
            && !cached.is_empty()
        {
            return Ok(cached);
        }

        let engine = ResticEngine::new();
        let recursive = dir_filter.is_none();
        let nodes = engine.list_nodes(
            &repo,
            &creds,
            &snapshot_id,
            dir_filter.as_deref(),
            recursive,
        )?;

        if dir_filter.is_none() && !nodes.is_empty() {
            if let Some(parent) = nodes_cache_file.parent() {
                let _ = fs::create_dir_all(parent);
            }
            let _ = fs::write(
                &nodes_cache_file,
                serde_json::to_string(&nodes).unwrap_or_default(),
            );
        }

        Ok(nodes)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn pick_folder() -> Result<Option<String>, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let folder = rfd::FileDialog::new().pick_folder();
        Ok(folder.map(|p| p.to_string_lossy().to_string()))
    })
    .await
    .map_err(|e| format!("Folder picker error: {}", e))?
}

#[tauri::command]
pub fn get_supported_launchers() -> Vec<crate::models::LauncherDescriptor> {
    crate::system::detect_all_launchers()
}

#[tauri::command]
pub fn scan_launcher_instances_cmd(launcher_path: String) -> Vec<String> {
    let p = Path::new(&launcher_path);
    crate::system::scan_instances_in_path(p)
}

#[tauri::command]
pub fn check_minecraft_processes_cmd() -> crate::process_guard::ProcessCheckResult {
    crate::process_guard::detect_minecraft_processes()
}

#[tauri::command]
pub fn close_minecraft_processes_cmd(pids: Vec<u32>, force: Option<bool>) -> Result<usize, String> {
    crate::process_guard::close_processes(&pids, force.unwrap_or(false))
}

#[tauri::command]
pub async fn start_manual_backup(
    app: AppHandle,
    state: State<'_, AppState>,
    repo: RepoConfig,
    profile: Profile,
    creds: SessionCredentials,
    force: Option<bool>,
) -> Result<RunResult, String> {
    if !force.unwrap_or(false) {
        let check = crate::process_guard::detect_minecraft_processes();
        if check.is_running {
            let names: Vec<String> = check.processes.iter().map(|p| p.name.clone()).collect();
            return Err(format!(
                "Cannot start backup: Minecraft is currently running ({})! Please close the launcher and all game instances before backing up to avoid world corruption.",
                names.join(", ")
            ));
        }
    }

    if state.is_backup_running.swap(true, Ordering::SeqCst) {
        return Err(
            "A backup is already currently in progress. Please wait for it to finish.".to_string(),
        );
    }
    struct BackupRunningGuard(Arc<AtomicBool>);
    impl Drop for BackupRunningGuard {
        fn drop(&mut self) {
            self.0.store(false, Ordering::SeqCst);
        }
    }
    let _running_guard = BackupRunningGuard(state.is_backup_running.clone());

    state.cancel_flag.store(false, Ordering::Relaxed);
    let cancel_flag = state.cancel_flag.clone();

    let app_handle = app.clone();
    let engine = ResticEngine::new();

    let result = engine.run_backup(&repo, &profile, &creds, cancel_flag, move |progress| {
        let _ = app_handle.emit("backup-progress", progress);
    })?;

    // Record verified local execution result
    let base_dir = get_persistent_app_dir();
    let status_dir = base_dir.join("status");
    let _ = fs::create_dir_all(&status_dir);
    let status_file = status_dir.join(format!("{}.json", profile.id));
    let _ = fs::write(&status_file, serde_json::to_string_pretty(&result).unwrap());

    // Also append to run history list
    let history_file = base_dir.join(format!("{}_history.json", profile.id));
    let mut history = Vec::new();
    if history_file.is_file()
        && let Ok(c) = fs::read_to_string(&history_file)
        && let Ok(arr) = serde_json::from_str::<Vec<RunResult>>(&c)
    {
        history = arr;
    }
    history.insert(0, result.clone());
    if history.len() > 30 {
        history.truncate(30);
    }
    let _ = fs::write(
        &history_file,
        serde_json::to_string_pretty(&history).unwrap(),
    );

    Ok(result)
}

#[tauri::command]
pub fn cancel_manual_backup(state: State<'_, AppState>) -> Result<(), String> {
    state.cancel_flag.store(true, Ordering::Relaxed);
    Ok(())
}

#[tauri::command]
pub async fn restore_files(
    repo: RepoConfig,
    creds: SessionCredentials,
    options: RestoreOptions,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        engine.run_restore(
            &repo,
            &creds,
            &options.snapshot_id,
            &options.target_dir,
            &options.include_paths,
            options.overwrite_confirmed,
        )
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub fn get_free_disk_space_cmd(target_dir: String) -> Result<u64, String> {
    get_disk_free_space(Path::new(&target_dir))
}

#[tauri::command]
pub async fn run_integrity_check_cmd(
    repo: RepoConfig,
    creds: SessionCredentials,
    read_data: bool,
) -> Result<IntegrityCheckResult, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        let result = engine.run_check(&repo, &creds, read_data)?;

        let base_dir = get_persistent_app_dir();
        let check_file = base_dir.join("last_integrity_check.json");
        let _ = fs::write(&check_file, serde_json::to_string_pretty(&result).unwrap());

        Ok(result)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub fn install_scheduler(
    profile: Profile,
    repo: RepoConfig,
    creds: SessionCredentials,
) -> Result<SchedulerStatus, String> {
    let mut engine = ResticEngine::new();
    // If restic is not found on host or USB, automatically download native engine to host bin dir
    if !engine.binary_path().is_file() {
        let _ = crate::system::download_native_restic_engine();
        engine = ResticEngine::new();
    }
    let scheduler = get_active_scheduler();
    scheduler.install(&profile, &repo, &creds, engine.binary_path())
}

#[tauri::command]
pub async fn download_restic_engine_cmd() -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(|| {
        let path = crate::system::download_native_restic_engine()?;
        Ok(path.to_string_lossy().to_string())
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub fn uninstall_scheduler(profile_id: String) -> Result<(), String> {
    let scheduler = get_active_scheduler();
    scheduler.uninstall(&profile_id)
}

#[tauri::command]
pub fn get_scheduler_status_cmd(profile_id: String) -> Result<SchedulerStatus, String> {
    let scheduler = get_active_scheduler();
    scheduler.status(&profile_id)
}

#[tauri::command]
pub fn record_restore_test(record: RestoreTestRecord) -> Result<(), String> {
    let base_dir = get_persistent_app_dir();
    let _ = fs::create_dir_all(&base_dir);
    let test_file = base_dir.join("last_restore_test.json");
    fs::write(&test_file, serde_json::to_string_pretty(&record).unwrap())
        .map_err(|e| format!("Failed to record restore test: {}", e))
}

#[tauri::command]
pub async fn get_dashboard_summary(
    repo: Option<RepoConfig>,
    creds: Option<SessionCredentials>,
    profile: Option<Profile>,
) -> Result<DashboardSummary, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let base_dir = get_persistent_app_dir();
        let scheduler = get_active_scheduler();

        let mut connection_status = "offline".to_string();
        let mut connection_error = None;
        let mut total_retained_snapshots = 0;
        let mut oldest_restore_point = None;
        let mut alerts = Vec::new();

        // Check repository connectivity if configured and not a placeholder
        if let (Some(r), Some(c)) = (&repo, &creds) {
            let is_placeholder = r.target.contains("my-backup-bucket")
                || r.target.contains("YOUR_BUCKET_NAME")
                || r.target.trim().is_empty();

            if r.insecure_no_password {
                alerts.push(DashboardAlert {
                    level: "info".to_string(),
                    title: "Passwordless Repository".to_string(),
                    message: "Repository uses --insecure-no-password. Access requires network/storage permissions but no decryption key.".to_string(),
                    code: "INSECURE_PASSWORDLESS".to_string(),
                });
            }

            if is_placeholder {
                connection_status = "unconfigured".to_string();
            } else {
                let engine = ResticEngine::new();
                match engine.test_connection(r, c) {
                    Ok(_) => {
                        connection_status = "connected".to_string();
                        if let Ok(snapshots) = engine.list_snapshots(r, c, None, None) {
                            total_retained_snapshots = snapshots.len() as u64;
                            if let Some(first) = snapshots.first() {
                                oldest_restore_point = Some(first.time.clone());
                            }
                        }
                    }
                    Err(e) => {
                        connection_status = "error".to_string();
                        connection_error = Some(e.clone());
                        alerts.push(DashboardAlert {
                            level: "error".to_string(),
                            title: "Repository Connection Error".to_string(),
                            message: e,
                            code: "CONNECTION_FAILED".to_string(),
                        });
                    }
                }
            }
        }

        // Check last run and scheduler status if profile is configured
        let mut scheduler_status = None;
        let mut last_successful_backup = None;
        let mut last_attempted_backup = None;
        let mut next_scheduled_run = None;
        let mut automatic_backups_installed = false;
        let mut missing_sources = Vec::new();

        if let Some(ref prof) = profile {
            // Verify source paths
            for p in &prof.source_paths {
                if !Path::new(p).exists() {
                    missing_sources.push(p.clone());
                }
            }
            if !missing_sources.is_empty() {
                alerts.push(DashboardAlert {
                    level: "warning".to_string(),
                    title: "Missing Source Directory".to_string(),
                    message: format!("Configured source path(s) not found on this machine: {}", missing_sources.join(", ")),
                    code: "MISSING_SOURCES".to_string(),
                });
            }

            // Check scheduler
            if let Ok(st) = scheduler.status(&prof.id) {
                automatic_backups_installed = st.installed;
                scheduler_status = Some(st);
            }

            // Next scheduled run
            if let Some(ref s) = prof.schedule
                && s.active {
                    next_scheduled_run = Some(format!("{:02}:{:02} ({})", s.hour, s.minute, s.frequency));
                }

            // Check last run from local status file
            let status_file = base_dir.join("status").join(format!("{}.json", prof.id));
            if status_file.is_file()
                && let Ok(content) = fs::read_to_string(&status_file)
                    && let Ok(run) = serde_json::from_str::<RunResult>(&content) {
                        last_attempted_backup = Some(run.clone());
                        if run.status == "success" || run.status == "partial_success" {
                            last_successful_backup = Some(run);
                        }
                    }
        }

        // Last integrity check
        let mut last_integrity_check = None;
        let check_file = base_dir.join("last_integrity_check.json");
        if check_file.is_file()
            && let Ok(content) = fs::read_to_string(&check_file)
                && let Ok(chk) = serde_json::from_str::<IntegrityCheckResult>(&content) {
                    last_integrity_check = Some(chk);
                }

        // Last confirmed restore test
        let mut last_confirmed_restore_test = None;
        let test_file = base_dir.join("last_restore_test.json");
        if test_file.is_file()
            && let Ok(content) = fs::read_to_string(&test_file)
                && let Ok(tst) = serde_json::from_str::<RestoreTestRecord>(&content) {
                    last_confirmed_restore_test = Some(tst);
                }

        Ok(DashboardSummary {
            connection_status,
            connection_error,
            last_successful_backup,
            last_attempted_backup,
            next_scheduled_run,
            automatic_backups_installed,
            scheduler_status,
            total_retained_snapshots,
            oldest_restore_point,
            last_integrity_check,
            last_confirmed_restore_test,
            missing_sources,
            alerts,
            current_machine_id: get_machine_id(),
            current_machine_name: get_machine_name(),
        })
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[derive(serde::Serialize)]
pub struct DemoDataSet {
    pub repo: RepoConfig,
    pub profile: Profile,
    pub snapshots: Vec<ResticSnapshot>,
    pub storage_overview: StorageOverview,
    pub folder_breakdown: Vec<FolderSizeBreakdown>,
    pub nodes: Vec<ResticNode>,
    pub runs: Vec<RunResult>,
}

#[tauri::command]
pub fn get_demo_data() -> DemoDataSet {
    DemoDataSet {
        repo: MockResticAdapter::get_demo_repo(),
        profile: MockResticAdapter::get_demo_profile(),
        snapshots: MockResticAdapter::get_demo_snapshots(),
        storage_overview: MockResticAdapter::get_demo_storage_overview(),
        folder_breakdown: MockResticAdapter::get_demo_folder_breakdown(),
        nodes: MockResticAdapter::get_demo_nodes(),
        runs: MockResticAdapter::get_demo_run_results(),
    }
}

#[tauri::command]
pub fn save_profile_cmd(profile: Profile) -> Result<(), String> {
    let base_dir = get_persistent_app_dir();
    let profiles_dir = base_dir.join("saved_profiles");
    fs::create_dir_all(&profiles_dir).map_err(|e| e.to_string())?;

    let file = profiles_dir.join(format!("{}.json", profile.id));
    fs::write(&file, serde_json::to_string_pretty(&profile).unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_profiles_cmd() -> Vec<Profile> {
    let base_dir = get_persistent_app_dir();
    let profiles_dir = base_dir.join("saved_profiles");
    let mut profiles = Vec::new();

    if let Ok(entries) = fs::read_dir(profiles_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(c) = fs::read_to_string(entry.path())
                && let Ok(p) = serde_json::from_str::<Profile>(&c)
            {
                profiles.push(p);
            }
        }
    }

    if profiles.is_empty() {
        let prism_path = detect_prism_launcher_path();
        let sources = if let Some(p) = prism_path {
            vec![p.to_string_lossy().to_string()]
        } else {
            vec![]
        };

        profiles.push(Profile {
            id: "default-local-profile".to_string(),
            name: "Local Prism Launcher Backup".to_string(),
            repo_id: "default-repo".to_string(),
            machine_id: get_machine_id(),
            machine_name: get_machine_name(),
            source_paths: sources,
            exclusions: vec![
                "*.lock".to_string(),
                "*.log".to_string(),
                "cache/".to_string(),
                "metacache".to_string(),
            ],
            tags: vec!["prism".to_string(), "minecraft".to_string()],
            schedule: None,
            minecraft_preset: None,
            pre_hook: None,
            post_hook: None,
            keep_last: Some(10),
            tag_timestamp: Some(true),
            distinction_label: None,
            is_pinned: Some(false),
        });
    }

    profiles
}

#[tauri::command]
pub async fn run_forget_now(
    repo: RepoConfig,
    creds: SessionCredentials,
    keep_last: u32,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        engine.run_forget_policy(&repo, &creds, keep_last, &["important", "pinned"])
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn toggle_snapshot_pinned(
    repo: RepoConfig,
    creds: SessionCredentials,
    snapshot_id: String,
    pinned: bool,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        if pinned {
            engine.tag_snapshot(&repo, &creds, &snapshot_id, &["important", "pinned"], &[])
        } else {
            engine.tag_snapshot(&repo, &creds, &snapshot_id, &[], &["important", "pinned"])
        }
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn delete_snapshot(
    repo: RepoConfig,
    creds: SessionCredentials,
    snapshot_id: String,
    prune: bool,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let engine = ResticEngine::new();
        engine.delete_single_snapshot(&repo, &creds, &snapshot_id, prune)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub fn save_repo_config_cmd(repo: RepoConfig) -> Result<(), String> {
    let base_dir = get_persistent_app_dir();
    let repos_dir = base_dir.join("saved_repos");
    fs::create_dir_all(&repos_dir).map_err(|e| e.to_string())?;

    let file = repos_dir.join(format!("{}.json", repo.id));
    fs::write(&file, serde_json::to_string_pretty(&repo).unwrap()).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn load_repo_configs_cmd() -> Vec<RepoConfig> {
    let base_dir = get_persistent_app_dir();
    let repos_dir = base_dir.join("saved_repos");
    let mut repos = Vec::new();

    if let Ok(entries) = fs::read_dir(repos_dir) {
        for entry in entries.flatten() {
            if entry.path().extension().and_then(|e| e.to_str()) == Some("json")
                && let Ok(c) = fs::read_to_string(entry.path())
                && let Ok(r) = serde_json::from_str::<RepoConfig>(&c)
            {
                repos.push(r);
            }
        }
    }

    repos
}

#[tauri::command]
pub async fn fetch_b2_provider_usage(
    repo: RepoConfig,
    creds: SessionCredentials,
) -> Result<crate::b2_api::B2ProviderUsage, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut effective_creds = creds;
        if effective_creds.b2_management_app_key.is_none()
            && effective_creds.b2_application_key.is_none()
            && let Ok(vault) = crate::usb_vault::load_usb_vault(None)
        {
            effective_creds.b2_management_app_key = vault.b2_management_app_key;
            effective_creds.b2_application_key = vault.b2_application_key;
        }
        crate::b2_api::query_b2_bucket_usage(&repo, &effective_creds)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn apply_b2_lifecycle_rules_cmd(
    repo: RepoConfig,
    creds: SessionCredentials,
) -> Result<String, String> {
    tauri::async_runtime::spawn_blocking(move || {
        let mut effective_creds = creds;
        if effective_creds.b2_management_app_key.is_none()
            && effective_creds.b2_application_key.is_none()
            && let Ok(vault) = crate::usb_vault::load_usb_vault(None)
        {
            effective_creds.b2_management_app_key = vault.b2_management_app_key;
            effective_creds.b2_application_key = vault.b2_application_key;
        }
        crate::b2_api::apply_b2_lifecycle_rules(&repo, &effective_creds)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub async fn purge_b2_hidden_versions_cmd(
    state: State<'_, AppState>,
    repo: RepoConfig,
    creds: SessionCredentials,
) -> Result<crate::b2_api::B2PurgeResult, String> {
    if state.is_backup_running.load(Ordering::SeqCst) {
        return Err("Cannot purge obsolete versions while a backup is currently in progress. Please wait for it to complete.".to_string());
    }

    let base_dir = get_persistent_app_dir();
    let locks_dir = base_dir.join("locks");
    if locks_dir.is_dir()
        && let Ok(entries) = fs::read_dir(&locks_dir)
    {
        for entry in entries.flatten() {
            if let Ok(pid_str) = fs::read_to_string(entry.path())
                && let Ok(pid) = pid_str.trim().parse::<i32>()
                && crate::runner::is_pid_alive(pid)
            {
                return Err(format!(
                    "Cannot purge obsolete versions: background scheduled backup (PID {}) is currently running.",
                    pid
                ));
            }
        }
    }

    tauri::async_runtime::spawn_blocking(move || {
        let mut effective_creds = creds;
        if effective_creds.b2_management_app_key.is_none()
            && effective_creds.b2_application_key.is_none()
            && let Ok(vault) = crate::usb_vault::load_usb_vault(None)
        {
            effective_creds.b2_management_app_key = vault.b2_management_app_key;
            effective_creds.b2_application_key = vault.b2_application_key;
        }
        crate::b2_api::purge_b2_hidden_versions(&repo, &effective_creds)
    })
    .await
    .map_err(|e| format!("Task execution error: {}", e))?
}

#[tauri::command]
pub fn get_cached_b2_provider_usage() -> Option<crate::b2_api::B2ProviderUsage> {
    crate::b2_api::load_cached_b2_usage()
}

#[tauri::command]
pub fn save_usb_vault_cmd(
    repo: RepoConfig,
    creds: SessionCredentials,
    passphrase: Option<String>,
) -> Result<String, String> {
    let data = crate::usb_vault::EncryptedVaultData {
        b2_key_id: repo.b2_key_id,
        b2_application_key: creds.b2_application_key,
        b2_management_key_id: repo.b2_management_key_id,
        b2_management_app_key: creds.b2_management_app_key,
        repo_password: creds.password,
        updated_at: chrono::Utc::now().to_rfc3339(),
    };
    crate::usb_vault::save_usb_vault(&data, passphrase.as_deref())
}

#[tauri::command]
pub fn unlock_usb_vault_cmd(
    passphrase: Option<String>,
) -> Result<crate::usb_vault::EncryptedVaultData, String> {
    crate::usb_vault::load_usb_vault(passphrase.as_deref())
}

#[tauri::command]
pub fn check_usb_vault_status_cmd() -> crate::usb_vault::UsbVaultStatus {
    crate::usb_vault::check_usb_vault_status()
}

#[tauri::command]
pub fn clear_usb_vault_cmd() -> Result<(), String> {
    crate::usb_vault::clear_usb_vault()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_categorize_minecraft_paths() {
        // Prism launcher instance world
        let p1 = "/Users/player/Library/Application Support/PrismLauncher/instances/1.20.4/saves/MyWorld/level.dat";
        assert_eq!(
            categorize_minecraft_path(p1),
            "Instance: 1.20.4 / World: MyWorld"
        );

        // Windows path instance mods
        let p2 = r"C:\Users\Player\AppData\Roaming\PrismLauncher\instances\Fabric\mods\sodium.jar";
        assert_eq!(categorize_minecraft_path(p2), "Instance: Fabric (Mods)");

        // Relative path instance world
        let p3 = "instances/Vanilla/saves/Survival/region/r.0.0.mca";
        assert_eq!(
            categorize_minecraft_path(p3),
            "Instance: Vanilla / World: Survival"
        );

        // Standalone saves
        let p4 = "saves/Singleplayer/level.dat";
        assert_eq!(categorize_minecraft_path(p4), "World: Singleplayer");

        // Resource packs
        let p5 = "/Users/player/Library/Application Support/PrismLauncher/instances/1.20.4/resourcepacks/Faithful.zip";
        assert_eq!(
            categorize_minecraft_path(p5),
            "Instance: 1.20.4 (Resource Packs)"
        );

        // Shader packs
        let p6 = "shaderpacks/ComplementaryShaders.zip";
        assert_eq!(categorize_minecraft_path(p6), "Shader Packs");

        // Launcher root config
        let p7 = "/Users/player/Library/Application Support/PrismLauncher/prismlauncher.cfg";
        assert_eq!(categorize_minecraft_path(p7), "Launcher Configurations");
    }
}
