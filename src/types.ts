export type RepoType = 'local' | 's3_b2';

export interface RepoConfig {
  id: string;
  name: string;
  repo_type: RepoType;
  target: string;
  insecure_no_password: boolean;
  has_password: boolean;
  b2_key_id?: string;
  b2_endpoint?: string;
  b2_bucket?: string;
  b2_prefix?: string;
  b2_management_key_id?: string;
}

export interface SessionCredentials {
  password?: string;
  b2_application_key?: string;
  b2_management_app_key?: string;
  save_to_local_os?: boolean;
}

export interface UsbVaultStatus {
  exists: boolean;
  is_passphrase_protected: boolean;
  vault_path: string;
}

export interface EncryptedVaultData {
  b2_key_id?: string;
  b2_application_key?: string;
  b2_management_key_id?: string;
  b2_management_app_key?: string;
  repo_password?: string;
  updated_at: string;
}

export interface ScheduleSpec {
  frequency: 'hourly' | 'daily' | 'weekly';
  hour: number;
  minute: number;
  day_of_week?: number;
  active: boolean;
}

export interface MinecraftPresetConfig {
  enabled: boolean;
  launcher_path: string;
  selected_instances: string[];
  warn_live_world: boolean;
  close_world_advised: boolean;
}

export interface RunningMinecraftProcess {
  pid: number;
  name: string;
  kind: 'launcher' | 'game_instance';
  command_line: string;
}

export interface ProcessCheckResult {
  is_running: boolean;
  processes: RunningMinecraftProcess[];
}

export interface Profile {
  id: string;
  name: string;
  repo_id: string;
  machine_id: string;
  machine_name: string;
  source_paths: string[];
  exclusions: string[];
  tags: string[];
  schedule?: ScheduleSpec;
  minecraft_preset?: MinecraftPresetConfig;
  pre_hook?: string;
  post_hook?: string;
  keep_last?: number;
  tag_timestamp?: boolean;
  distinction_label?: string;
  is_pinned?: boolean;
}

export interface ResticSnapshotSummary {
  files_new?: number;
  files_changed?: number;
  files_unmodified?: number;
  dirs_new?: number;
  dirs_changed?: number;
  dirs_unmodified?: number;
  data_blobs?: number;
  tree_blobs?: number;
  data_added?: number;
  data_added_packed?: number;
  total_files_processed?: number;
  total_bytes_processed?: number;
  total_duration?: number;
  backup_start?: string;
  backup_end?: string;
}

export interface ResticSnapshot {
  id: string;
  short_id: string;
  time: string;
  paths: string[];
  hostname: string;
  username?: string;
  tags?: string[];
  summary?: ResticSnapshotSummary;
}

export interface ResticNode {
  name: string;
  type: 'file' | 'dir' | 'symlink';
  path: string;
  size?: number;
  permissions?: string;
  mtime?: string;
}

export interface FolderSizeBreakdown {
  folder_path: string;
  size_bytes: number;
  file_count: number;
  percentage: number;
}

export interface GrowthPoint {
  timestamp: string;
  raw_bytes: number;
  restore_bytes: number;
  snapshots: number;
  is_demo: boolean;
}

export interface B2ProviderUsage {
  total_bytes: number;
  file_count: number;
  bucket_name: string;
  prefix?: string;
  measured_at: string;
  source: string;
  is_partial: boolean;
  active_bytes?: number;
  active_file_count?: number;
  hidden_version_bytes?: number;
  hidden_version_count?: number;
  has_lifecycle_rules?: boolean;
  lifecycle_warning?: string;
}

export interface B2PurgeResult {
  deleted_files_count: number;
  freed_bytes: number;
  updated_usage: B2ProviderUsage;
}

export interface StorageOverview {
  raw_data_size: number;
  restore_size: number;
  provider_b2_usage?: number | null;
  b2_provider_details?: B2ProviderUsage | null;
  snapshots_count: number;
  growth_history: GrowthPoint[];
  calculated_at: string;
  measurement_method: string;
  is_demo: boolean;
}

export interface RunResult {
  run_id: string;
  profile_id: string;
  profile_name: string;
  machine_id: string;
  start_time: string;
  end_time: string;
  duration_seconds: number;
  status: 'success' | 'partial_success' | 'failed' | 'cancelled' | 'unknown';
  exit_code: number;
  files_new: number;
  files_changed: number;
  files_unmodified: number;
  data_added_bytes: number;
  total_files_processed: number;
  total_bytes_processed: number;
  snapshot_id?: string;
  sanitized_logs: string;
  error_message?: string;
  is_verified_local_run: boolean;
}

export interface IntegrityCheckResult {
  timestamp: string;
  success: boolean;
  details: string;
  read_data: boolean;
}

export interface RestoreTestRecord {
  timestamp: string;
  snapshot_id: string;
  destination: string;
  success: boolean;
  notes: string;
}

export interface DashboardAlert {
  level: 'warning' | 'error' | 'info';
  title: string;
  message: string;
  code: string;
}

export interface SchedulerStatus {
  platform: string;
  installed: boolean;
  enabled: boolean;
  target_binary_path: string;
  config_path: string;
  schedule_file_path: string;
  last_run?: RunResult;
  notes: string;
}

export interface DashboardSummary {
  connection_status: 'connected' | 'offline' | 'unauthenticated' | 'error';
  connection_error?: string;
  last_successful_backup?: RunResult;
  last_attempted_backup?: RunResult;
  next_scheduled_run?: string;
  automatic_backups_installed: boolean;
  scheduler_status?: SchedulerStatus;
  total_retained_snapshots: number;
  oldest_restore_point?: string;
  last_integrity_check?: IntegrityCheckResult;
  last_confirmed_restore_test?: RestoreTestRecord;
  missing_sources: string[];
  alerts: DashboardAlert[];
  current_machine_id: string;
  current_machine_name: string;
}

export interface SystemInfo {
  machine_id: string;
  machine_name: string;
  platform: string;
  prism_launcher_path?: string;
  prism_instances: string[];
  persistent_app_dir: string;
  restic_found: boolean;
  restic_path: string;
  is_usb_bundled: boolean;
}

export interface BackupProgressEvent {
  percent_done: number;
  total_files: number;
  files_done: number;
  total_bytes: number;
  bytes_done: number;
  current_file?: string;
  seconds_elapsed: number;
  seconds_remaining?: number;
}

export interface RestoreOptions {
  snapshot_id: string;
  target_dir: string;
  include_paths: string[];
  overwrite_confirmed: boolean;
}

export interface LauncherDescriptor {
  id: string;
  name: string;
  default_path: string;
  detected: boolean;
  instances: string[];
}
