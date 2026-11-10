pub mod b2_api;
pub mod commands;
pub mod mock_adapter;
pub mod models;
pub mod process_guard;
pub mod restic_cli;
pub mod runner;
pub mod scheduler;
pub mod security;
pub mod system;
pub mod usb_vault;

use commands::AppState;
use std::sync::Arc;
use std::sync::atomic::AtomicBool;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let is_backup_running = Arc::new(AtomicBool::new(false));
    let state = AppState {
        cancel_flag,
        is_backup_running,
    };

    tauri::Builder::default()
        .manage(state)
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            commands::get_system_info,
            commands::test_repo_connection,
            commands::list_snapshots,
            commands::get_storage_overview,
            commands::get_snapshot_folder_breakdown,
            commands::list_snapshot_nodes,
            commands::start_manual_backup,
            commands::cancel_manual_backup,
            commands::restore_files,
            commands::get_free_disk_space_cmd,
            commands::run_integrity_check_cmd,
            commands::install_scheduler,
            commands::uninstall_scheduler,
            commands::get_scheduler_status_cmd,
            commands::record_restore_test,
            commands::get_dashboard_summary,
            commands::get_demo_data,
            commands::save_profile_cmd,
            commands::load_profiles_cmd,
            commands::save_repo_config_cmd,
            commands::load_repo_configs_cmd,
            commands::pick_folder,
            commands::get_supported_launchers,
            commands::scan_launcher_instances_cmd,
            commands::check_minecraft_processes_cmd,
            commands::close_minecraft_processes_cmd,
            commands::run_forget_now,
            commands::toggle_snapshot_pinned,
            commands::delete_snapshot,
            commands::fetch_b2_provider_usage,
            commands::get_cached_b2_provider_usage,
            commands::apply_b2_lifecycle_rules_cmd,
            commands::purge_b2_hidden_versions_cmd,
            commands::save_usb_vault_cmd,
            commands::unlock_usb_vault_cmd,
            commands::check_usb_vault_status_cmd,
            commands::clear_usb_vault_cmd,
            commands::download_restic_engine_cmd,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
