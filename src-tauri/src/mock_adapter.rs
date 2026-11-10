use crate::models::{
    FolderSizeBreakdown, GrowthPoint, MinecraftPresetConfig, Profile, RepoConfig, RepoType,
    ResticNode, ResticSnapshot, ResticSnapshotSummary, RunResult, ScheduleSpec,
    StorageOverview,
};

pub struct MockResticAdapter;

impl MockResticAdapter {
    pub fn get_demo_repo() -> RepoConfig {
        RepoConfig {
            id: "demo-repo".to_string(),
            name: "Backblaze B2 (Demo Repo - Insecure No Password)".to_string(),
            repo_type: RepoType::S3B2,
            target: "s3:s3.us-west-004.backblazeb2.com/demo-minecraft-bucket/restic".to_string(),
            insecure_no_password: true,
            has_password: false,
            b2_key_id: Some("004demoKeyIdExample".to_string()),
            b2_endpoint: Some("s3.us-west-004.backblazeb2.com".to_string()),
            b2_bucket: Some("demo-minecraft-bucket".to_string()),
            b2_prefix: Some("restic".to_string()),
            b2_management_key_id: None,
        }
    }

    pub fn get_demo_profile() -> Profile {
        Profile {
            id: "prism-demo-profile".to_string(),
            name: "Prism Launcher Backups".to_string(),
            repo_id: "demo-repo".to_string(),
            machine_id: "macbook-declan".to_string(),
            machine_name: "MacBook Pro".to_string(),
            source_paths: vec!["/Users/declan/Library/Application Support/PrismLauncher".to_string()],
            exclusions: vec!["*.lock".to_string(), "logs/*".to_string(), "cache/*".to_string()],
            tags: vec!["minecraft".to_string(), "daily".to_string()],
            schedule: Some(ScheduleSpec {
                frequency: "daily".to_string(),
                hour: 4,
                minute: 0,
                day_of_week: None,
                active: true,
            }),
            minecraft_preset: Some(MinecraftPresetConfig {
                enabled: true,
                launcher_path: "/Users/declan/Library/Application Support/PrismLauncher".to_string(),
                selected_instances: vec!["Survival 1.20".to_string(), "Modded Factory".to_string()],
                warn_live_world: true,
                close_world_advised: true,
            }),
            pre_hook: None,
            post_hook: None,
            keep_last: Some(10),
            tag_timestamp: Some(true),
            distinction_label: Some("Demo Backup".to_string()),
            is_pinned: Some(false),
        }
    }

    pub fn get_demo_snapshots() -> Vec<ResticSnapshot> {
        vec![
            ResticSnapshot {
                id: "e1a9b4c8d3214567e890123456789abcdef0123456789abcdef0123456789abc".to_string(),
                short_id: "e1a9b4c8".to_string(),
                time: "2026-11-09T04:00:15Z".to_string(),
                paths: vec!["/Users/declan/Library/Application Support/PrismLauncher".to_string()],
                hostname: "macbook-declan".to_string(),
                username: Some("declan".to_string()),
                tags: Some(vec!["minecraft".to_string(), "daily".to_string()]),
                summary: Some(ResticSnapshotSummary {
                    files_new: Some(14),
                    files_changed: Some(82),
                    files_unmodified: Some(1420),
                    dirs_new: Some(2),
                    dirs_changed: Some(8),
                    dirs_unmodified: Some(190),
                    data_blobs: Some(96),
                    tree_blobs: Some(10),
                    data_added: Some(184_549_376), // ~176 MiB
                    data_added_packed: Some(112_400_000),
                    total_files_processed: Some(1516),
                    total_bytes_processed: Some(4_829_102_080), // ~4.5 GiB
                    total_duration: Some(18.4),
                    backup_start: Some("2026-11-09T04:00:15Z".to_string()),
                    backup_end: Some("2026-11-09T04:00:33Z".to_string()),
                }),
            },
            ResticSnapshot {
                id: "b2c3d4e5f67890123456789abcdef0123456789abcdef0123456789abcdef012".to_string(),
                short_id: "b2c3d4e5".to_string(),
                time: "2026-11-08T04:00:12Z".to_string(),
                paths: vec!["/Users/declan/Library/Application Support/PrismLauncher".to_string()],
                hostname: "macbook-declan".to_string(),
                username: Some("declan".to_string()),
                tags: Some(vec!["minecraft".to_string(), "daily".to_string()]),
                summary: Some(ResticSnapshotSummary {
                    files_new: Some(28),
                    files_changed: Some(64),
                    files_unmodified: Some(1390),
                    dirs_new: Some(4),
                    dirs_changed: Some(6),
                    dirs_unmodified: Some(188),
                    data_blobs: Some(92),
                    tree_blobs: Some(10),
                    data_added: Some(241_172_480),
                    data_added_packed: Some(152_000_000),
                    total_files_processed: Some(1482),
                    total_bytes_processed: Some(4_644_552_704),
                    total_duration: Some(22.1),
                    backup_start: Some("2026-11-08T04:00:12Z".to_string()),
                    backup_end: Some("2026-11-08T04:00:34Z".to_string()),
                }),
            },
            ResticSnapshot {
                id: "a3f8c1d2e4567890123456789abcdef0123456789abcdef0123456789abcdef012".to_string(),
                short_id: "a3f8c1d2".to_string(),
                time: "2026-11-01T04:00:09Z".to_string(),
                paths: vec!["/Users/declan/Library/Application Support/PrismLauncher".to_string()],
                hostname: "macbook-declan".to_string(),
                username: Some("declan".to_string()),
                tags: Some(vec!["minecraft".to_string(), "weekly".to_string()]),
                summary: Some(ResticSnapshotSummary {
                    files_new: Some(1450),
                    files_changed: Some(0),
                    files_unmodified: Some(0),
                    dirs_new: Some(192),
                    dirs_changed: Some(0),
                    dirs_unmodified: Some(0),
                    data_blobs: Some(1450),
                    tree_blobs: Some(192),
                    data_added: Some(4_500_000_000),
                    data_added_packed: Some(2_800_000_000),
                    total_files_processed: Some(1450),
                    total_bytes_processed: Some(4_500_000_000),
                    total_duration: Some(95.6),
                    backup_start: Some("2026-11-01T04:00:09Z".to_string()),
                    backup_end: Some("2026-11-01T04:01:44Z".to_string()),
                }),
            },
        ]
    }

    pub fn get_demo_storage_overview() -> StorageOverview {
        StorageOverview {
            raw_data_size: 3_245_120_000,   // ~3.02 GiB deduplicated raw repo data blobs
            restore_size: 13_973_654_784,   // ~13.01 GiB total logical restore size
            provider_b2_usage: None,        // Clearly labelled as "Unavailable" (not fabricated)
            b2_provider_details: None,
            snapshots_count: 3,
            growth_history: vec![
                GrowthPoint {
                    timestamp: "2026-11-01T04:01:44Z".to_string(),
                    raw_bytes: 2_800_000_000,
                    restore_bytes: 4_500_000_000,
                    snapshots: 1,
                    is_demo: true,
                },
                GrowthPoint {
                    timestamp: "2026-11-08T04:00:34Z".to_string(),
                    raw_bytes: 3_040_000_000,
                    restore_bytes: 9_144_552_704,
                    snapshots: 2,
                    is_demo: true,
                },
                GrowthPoint {
                    timestamp: "2026-11-09T04:00:33Z".to_string(),
                    raw_bytes: 3_245_120_000,
                    restore_bytes: 13_973_654_784,
                    snapshots: 3,
                    is_demo: true,
                },
            ],
            calculated_at: "2026-11-09T17:00:00Z".to_string(),
            measurement_method: "restic stats --mode raw-data / restore-size (Demo Simulation)".to_string(),
            is_demo: true,
        }
    }

    pub fn get_demo_folder_breakdown() -> Vec<FolderSizeBreakdown> {
        vec![
            FolderSizeBreakdown {
                folder_path: "instances/Survival 1.20".to_string(),
                size_bytes: 2_576_980_377, // ~2.4 GiB
                file_count: 720,
                percentage: 53.3,
            },
            FolderSizeBreakdown {
                folder_path: "instances/Modded Factory".to_string(),
                size_bytes: 1_621_350_144, // ~1.5 GiB
                file_count: 510,
                percentage: 33.6,
            },
            FolderSizeBreakdown {
                folder_path: "libraries".to_string(),
                size_bytes: 419_430_400, // ~400 MiB
                file_count: 180,
                percentage: 8.7,
            },
            FolderSizeBreakdown {
                folder_path: "meta & icons".to_string(),
                size_bytes: 212_341_159, // ~202 MiB
                file_count: 106,
                percentage: 4.4,
            },
        ]
    }

    pub fn get_demo_nodes() -> Vec<ResticNode> {
        vec![
            ResticNode {
                name: "instances".to_string(),
                node_type: "dir".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances".to_string(),
                size: Some(0),
                permissions: Some("drwxr-xr-x".to_string()),
                mtime: Some("2026-11-09T03:55:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "Survival 1.20".to_string(),
                node_type: "dir".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20".to_string(),
                size: Some(0),
                permissions: Some("drwxr-xr-x".to_string()),
                mtime: Some("2026-11-09T03:52:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "instance.cfg".to_string(),
                node_type: "file".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20/instance.cfg".to_string(),
                size: Some(2385),
                permissions: Some("-rw-r--r--".to_string()),
                mtime: Some("2026-11-07T00:47:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "saves".to_string(),
                node_type: "dir".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20/.minecraft/saves".to_string(),
                size: Some(0),
                permissions: Some("drwxr-xr-x".to_string()),
                mtime: Some("2026-11-09T03:51:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "World_Alpha".to_string(),
                node_type: "dir".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20/.minecraft/saves/World_Alpha".to_string(),
                size: Some(0),
                permissions: Some("drwxr-xr-x".to_string()),
                mtime: Some("2026-11-09T03:50:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "level.dat".to_string(),
                node_type: "file".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20/.minecraft/saves/World_Alpha/level.dat".to_string(),
                size: Some(3421),
                permissions: Some("-rw-r--r--".to_string()),
                mtime: Some("2026-11-09T03:50:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "region".to_string(),
                node_type: "dir".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20/.minecraft/saves/World_Alpha/region".to_string(),
                size: Some(0),
                permissions: Some("drwxr-xr-x".to_string()),
                mtime: Some("2026-11-09T03:45:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "r.0.0.mca".to_string(),
                node_type: "file".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/instances/Survival 1.20/.minecraft/saves/World_Alpha/region/r.0.0.mca".to_string(),
                size: Some(5_242_880),
                permissions: Some("-rw-r--r--".to_string()),
                mtime: Some("2026-11-09T03:45:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
            ResticNode {
                name: "prismlauncher.cfg".to_string(),
                node_type: "file".to_string(),
                path: "/Users/declan/Library/Application Support/PrismLauncher/prismlauncher.cfg".to_string(),
                size: Some(2385),
                permissions: Some("-rw-r--r--".to_string()),
                mtime: Some("2026-11-07T00:47:00Z".to_string()),
                message_type: Some("node".to_string()),
                struct_type: Some("node".to_string()),
            },
        ]
    }

    pub fn get_demo_run_results() -> Vec<RunResult> {
        vec![
            RunResult {
                run_id: "demo-run-1".to_string(),
                profile_id: "prism-demo-profile".to_string(),
                profile_name: "Prism Launcher Backups".to_string(),
                machine_id: "macbook-declan".to_string(),
                start_time: "2026-11-09T04:00:15Z".to_string(),
                end_time: "2026-11-09T04:00:33Z".to_string(),
                duration_seconds: 18.4,
                status: "success".to_string(),
                exit_code: 0,
                files_new: 14,
                files_changed: 82,
                files_unmodified: 1420,
                data_added_bytes: 184_549_376,
                total_files_processed: 1516,
                total_bytes_processed: 4_829_102_080,
                snapshot_id: Some("e1a9b4c8".to_string()),
                sanitized_logs: "[2026-11-09 04:00:15] restic backup started...\n[2026-11-09 04:00:33] Snapshot e1a9b4c8 saved successfully.".to_string(),
                error_message: None,
                is_verified_local_run: true,
            },
            RunResult {
                run_id: "demo-run-2".to_string(),
                profile_id: "prism-demo-profile".to_string(),
                profile_name: "Prism Launcher Backups".to_string(),
                machine_id: "macbook-declan".to_string(),
                start_time: "2026-11-08T04:00:12Z".to_string(),
                end_time: "2026-11-08T04:00:34Z".to_string(),
                duration_seconds: 22.1,
                status: "success".to_string(),
                exit_code: 0,
                files_new: 28,
                files_changed: 64,
                files_unmodified: 1390,
                data_added_bytes: 241_172_480,
                total_files_processed: 1482,
                total_bytes_processed: 4_644_552_704,
                snapshot_id: Some("b2c3d4e5".to_string()),
                sanitized_logs: "[2026-11-08 04:00:12] restic backup started...\n[2026-11-08 04:00:34] Snapshot b2c3d4e5 saved successfully.".to_string(),
                error_message: None,
                is_verified_local_run: true,
            },
        ]
    }
}
