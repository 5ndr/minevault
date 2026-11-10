use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum RepoType {
    Local,
    #[serde(rename = "s3_b2")]
    S3B2,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RepoConfig {
    pub id: String,
    pub name: String,
    pub repo_type: RepoType,
    pub target: String,
    pub insecure_no_password: bool,
    pub has_password: bool,
    #[serde(default)]
    pub b2_key_id: Option<String>,
    #[serde(default)]
    pub b2_endpoint: Option<String>,
    #[serde(default)]
    pub b2_bucket: Option<String>,
    #[serde(default)]
    pub b2_prefix: Option<String>,
    #[serde(default)]
    pub b2_management_key_id: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct SessionCredentials {
    #[serde(default)]
    pub password: Option<String>,
    #[serde(default)]
    pub b2_application_key: Option<String>,
    #[serde(default)]
    pub b2_management_app_key: Option<String>,
    #[serde(default)]
    pub save_to_local_os: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LauncherDescriptor {
    pub id: String,
    pub name: String,
    pub default_path: String,
    pub detected: bool,
    pub instances: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MinecraftPresetConfig {
    pub enabled: bool,
    pub launcher_path: String,
    pub selected_instances: Vec<String>,
    pub warn_live_world: bool,
    pub close_world_advised: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScheduleSpec {
    pub frequency: String, // "hourly", "daily", "weekly"
    pub hour: u32,
    pub minute: u32,
    #[serde(default)]
    pub day_of_week: Option<u32>,
    pub active: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub repo_id: String,
    pub machine_id: String,
    pub machine_name: String,
    pub source_paths: Vec<String>,
    #[serde(default)]
    pub exclusions: Vec<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    #[serde(default)]
    pub schedule: Option<ScheduleSpec>,
    #[serde(default)]
    pub minecraft_preset: Option<MinecraftPresetConfig>,
    #[serde(default)]
    pub pre_hook: Option<String>,
    #[serde(default)]
    pub post_hook: Option<String>,
    #[serde(default)]
    pub keep_last: Option<u32>,
    #[serde(default)]
    pub tag_timestamp: Option<bool>,
    #[serde(default)]
    pub distinction_label: Option<String>,
    #[serde(default)]
    pub is_pinned: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticSnapshotSummary {
    #[serde(default)]
    pub files_new: Option<u64>,
    #[serde(default)]
    pub files_changed: Option<u64>,
    #[serde(default)]
    pub files_unmodified: Option<u64>,
    #[serde(default)]
    pub dirs_new: Option<u64>,
    #[serde(default)]
    pub dirs_changed: Option<u64>,
    #[serde(default)]
    pub dirs_unmodified: Option<u64>,
    #[serde(default)]
    pub data_blobs: Option<u64>,
    #[serde(default)]
    pub tree_blobs: Option<u64>,
    #[serde(default)]
    pub data_added: Option<u64>,
    #[serde(default)]
    pub data_added_packed: Option<u64>,
    #[serde(default)]
    pub total_files_processed: Option<u64>,
    #[serde(default)]
    pub total_bytes_processed: Option<u64>,
    #[serde(default)]
    pub total_duration: Option<f64>,
    #[serde(default)]
    pub backup_start: Option<String>,
    #[serde(default)]
    pub backup_end: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticSnapshot {
    pub id: String,
    pub short_id: String,
    pub time: String,
    pub paths: Vec<String>,
    pub hostname: String,
    #[serde(default)]
    pub username: Option<String>,
    #[serde(default)]
    pub tags: Option<Vec<String>>,
    #[serde(default)]
    pub summary: Option<ResticSnapshotSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticNode {
    pub name: String,
    #[serde(rename = "type")]
    pub node_type: String,
    pub path: String,
    #[serde(default)]
    pub size: Option<u64>,
    #[serde(default)]
    pub permissions: Option<String>,
    #[serde(default)]
    pub mtime: Option<String>,
    #[serde(default)]
    pub message_type: Option<String>,
    #[serde(default)]
    pub struct_type: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResticStats {
    pub total_size: u64,
    #[serde(default)]
    pub total_file_count: Option<u64>,
    #[serde(default)]
    pub total_blob_count: Option<u64>,
    pub snapshots_count: u64,
    #[serde(default)]
    pub compression_ratio: Option<f64>,
    #[serde(default)]
    pub compression_space_saving: Option<f64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GrowthPoint {
    pub timestamp: String,
    pub raw_bytes: u64,
    pub restore_bytes: u64,
    pub snapshots: u64,
    pub is_demo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FolderSizeBreakdown {
    pub folder_path: String,
    pub size_bytes: u64,
    pub file_count: u64,
    pub percentage: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageOverview {
    pub raw_data_size: u64,
    pub restore_size: u64,
    pub provider_b2_usage: Option<u64>,
    #[serde(default)]
    pub b2_provider_details: Option<crate::b2_api::B2ProviderUsage>,
    pub snapshots_count: u64,
    pub growth_history: Vec<GrowthPoint>,
    pub calculated_at: String,
    pub measurement_method: String,
    pub is_demo: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RunResult {
    pub run_id: String,
    pub profile_id: String,
    pub profile_name: String,
    pub machine_id: String,
    pub start_time: String,
    pub end_time: String,
    pub duration_seconds: f64,
    pub status: String, // "success", "partial_success", "failed", "cancelled", "unknown"
    pub exit_code: i32,
    pub files_new: u64,
    pub files_changed: u64,
    pub files_unmodified: u64,
    pub data_added_bytes: u64,
    pub total_files_processed: u64,
    pub total_bytes_processed: u64,
    #[serde(default)]
    pub snapshot_id: Option<String>,
    pub sanitized_logs: String,
    #[serde(default)]
    pub error_message: Option<String>,
    pub is_verified_local_run: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrityCheckResult {
    pub timestamp: String,
    pub success: bool,
    pub details: String,
    pub read_data: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreTestRecord {
    pub timestamp: String,
    pub snapshot_id: String,
    pub destination: String,
    pub success: bool,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardAlert {
    pub level: String, // "warning", "error", "info"
    pub title: String,
    pub message: String,
    pub code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SchedulerStatus {
    pub platform: String,
    pub installed: bool,
    pub enabled: bool,
    pub target_binary_path: String,
    pub config_path: String,
    pub schedule_file_path: String,
    pub last_run: Option<RunResult>,
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DashboardSummary {
    pub connection_status: String, // "connected", "offline", "unauthenticated", "error"
    pub connection_error: Option<String>,
    pub last_successful_backup: Option<RunResult>,
    pub last_attempted_backup: Option<RunResult>,
    pub next_scheduled_run: Option<String>,
    pub automatic_backups_installed: bool,
    pub scheduler_status: Option<SchedulerStatus>,
    pub total_retained_snapshots: u64,
    pub oldest_restore_point: Option<String>,
    pub last_integrity_check: Option<IntegrityCheckResult>,
    pub last_confirmed_restore_test: Option<RestoreTestRecord>,
    pub missing_sources: Vec<String>,
    pub alerts: Vec<DashboardAlert>,
    pub current_machine_id: String,
    pub current_machine_name: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RestoreOptions {
    pub snapshot_id: String,
    pub target_dir: String,
    pub include_paths: Vec<String>,
    pub overwrite_confirmed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BackupProgressEvent {
    pub percent_done: f64,
    pub total_files: u64,
    pub files_done: u64,
    pub total_bytes: u64,
    pub bytes_done: u64,
    pub current_file: Option<String>,
    pub seconds_elapsed: u64,
    pub seconds_remaining: Option<u64>,
}
