use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tempfile::TempDir;

use resticdashboard_lib::models::{Profile, RepoConfig, RepoType, ResticSnapshot, ResticStats, ScheduleSpec, SessionCredentials};
use resticdashboard_lib::restic_cli::ResticEngine;
use resticdashboard_lib::scheduler::linux::LinuxSystemdAdapter;
use resticdashboard_lib::scheduler::macos::MacOsLaunchdAdapter;
use resticdashboard_lib::security::{redact_secrets, validate_restore_path};

#[test]
fn test_restic_json_parsing_snapshots_and_stats() {
    let snapshot_json = r#"[
        {
            "time": "2026-11-09T17:03:39.0503-06:00",
            "tree": "b23ba564acb268432c0d218e8e8e895c45fe7274c01ba155cf73c7d6407aa069",
            "paths": ["/tmp/restic-test-source with spaces/ünicøde"],
            "hostname": "macbook-declan",
            "username": "declan",
            "uid": 502,
            "gid": 20,
            "program_version": "restic 0.19.1",
            "summary": {
                "backup_start": "2026-11-09T17:03:39.0503-06:00",
                "backup_end": "2026-11-09T17:03:39.788356-06:00",
                "files_new": 2,
                "files_changed": 0,
                "files_unmodified": 0,
                "data_added": 1984,
                "total_files_processed": 2,
                "total_bytes_processed": 25
            },
            "id": "c1190655f48aaf95d7936565bb0843b85585e5fc389efdd6658c24a34f28eaae",
            "short_id": "c1190655"
        }
    ]"#;

    let snapshots: Vec<ResticSnapshot> = serde_json::from_str(snapshot_json).expect("Failed to parse snapshots JSON");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].short_id, "c1190655");
    assert_eq!(snapshots[0].paths[0], "/tmp/restic-test-source with spaces/ünicøde");
    assert_eq!(snapshots[0].summary.as_ref().unwrap().files_new, Some(2));
    assert_eq!(snapshots[0].summary.as_ref().unwrap().data_added, Some(1984));

    let stats_raw_json = r#"{
        "total_size": 1303,
        "total_uncompressed_size": 2176,
        "compression_ratio": 1.669992,
        "total_blob_count": 6,
        "snapshots_count": 1
    }"#;

    let stats: ResticStats = serde_json::from_str(stats_raw_json).expect("Failed to parse raw-data stats JSON");
    assert_eq!(stats.total_size, 1303);
    assert_eq!(stats.snapshots_count, 1);
    assert_eq!(stats.total_blob_count, Some(6));

    let stats_restore_json = r#"{
        "total_size": 25,
        "total_file_count": 5,
        "snapshots_count": 1
    }"#;
    let restore_stats: ResticStats = serde_json::from_str(stats_restore_json).expect("Failed to parse restore stats");
    assert_eq!(restore_stats.total_size, 25);
    assert_eq!(restore_stats.total_file_count, Some(5));
}

#[test]
fn test_missing_or_older_restic_fields_compatibility() {
    // Snapshot without summary and with older schema
    let older_snapshot = r#"[
        {
            "time": "2024-01-01T00:00:00Z",
            "paths": ["/home/user"],
            "hostname": "legacy-box",
            "id": "1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef",
            "short_id": "12345678"
        }
    ]"#;

    let parsed: Vec<ResticSnapshot> = serde_json::from_str(older_snapshot).expect("Should handle missing optional fields");
    assert_eq!(parsed[0].short_id, "12345678");
    assert!(parsed[0].summary.is_none());
    assert!(parsed[0].tags.is_none());
}

#[test]
fn test_secret_redaction_comprehensive() {
    let raw = "Executing: restic backup --repo s3:s3.us-west-004.backblazeb2.com/bkt with AWS_SECRET_ACCESS_KEY=K3yWith$ymbols! and RESTIC_PASSWORD=UltraSecretPass456";
    let redacted = redact_secrets(raw, &["K3yWith$ymbols!", "UltraSecretPass456"]);

    assert!(!redacted.contains("K3yWith$ymbols!"));
    assert!(!redacted.contains("UltraSecretPass456"));
    assert!(redacted.contains("[REDACTED"));
}

#[test]
fn test_path_traversal_protection() {
    let target_base = Path::new("/var/backups/restore_dst");

    // Rejects directory traversal
    assert!(validate_restore_path(target_base, "../../etc/shadow").is_err());
    assert!(validate_restore_path(target_base, "foo/../../bar/../../../secret").is_err());

    // Allows normal and non-ASCII relative paths
    let safe = validate_restore_path(target_base, "instances/Prism 1.20/saves/Mörld/level.dat");
    assert!(safe.is_ok());
    assert!(safe.unwrap().starts_with(target_base));
}

#[test]
fn test_macos_launchd_generation_uses_persistent_paths() {
    let runner_exe = PathBuf::from("/Users/declan/Library/Application Support/resticdashboard/bin/resticdashboard-runner");
    let base_dir = PathBuf::from("/Users/declan/Library/Application Support/resticdashboard");
    let schedule = ScheduleSpec {
        frequency: "daily".to_string(),
        hour: 3,
        minute: 15,
        day_of_week: None,
        active: true,
    };

    let xml = MacOsLaunchdAdapter::generate_plist_xml("test-profile", &runner_exe, &base_dir, &schedule);

    // Verify it uses the persistent runner path and NOT a USB path
    assert!(xml.contains("/Library/Application Support/resticdashboard/bin/resticdashboard-runner"));
    assert!(xml.contains("<string>runner</string>"));
    assert!(xml.contains("<string>--profile-id</string>"));
    assert!(xml.contains("<string>test-profile</string>"));
    assert!(xml.contains("<key>Hour</key>\n        <integer>3</integer>"));
    assert!(xml.contains("<key>Minute</key>\n        <integer>15</integer>"));
    assert!(!xml.contains("/Volumes/USB"));
}

#[test]
fn test_linux_systemd_generation_uses_persistent_paths() {
    let runner_exe = PathBuf::from("/home/user/.local/share/resticdashboard/bin/resticdashboard-runner");
    let base_dir = PathBuf::from("/home/user/.local/share/resticdashboard");
    let schedule = ScheduleSpec {
        frequency: "weekly".to_string(),
        hour: 2,
        minute: 30,
        day_of_week: Some(0), // Sunday
        active: true,
    };

    let service = LinuxSystemdAdapter::generate_service_unit(&runner_exe, "linux-prof", &base_dir);
    assert!(service.contains("/home/user/.local/share/resticdashboard/bin/resticdashboard-runner runner --profile-id linux-prof"));
    assert!(!service.contains("/media/usb"));

    let on_calendar = LinuxSystemdAdapter::format_on_calendar(&schedule);
    let timer = LinuxSystemdAdapter::generate_timer_unit("linux-prof", &on_calendar);
    assert!(timer.contains("Persistent=true"));
    assert!(timer.contains("Sun *-*-* 02:30:00"));
}

#[test]
fn test_subprocess_timeout_prevention() {
    // Test that commands which hang or take longer than the timeout are cleanly killed
    // and return a timeout error rather than blocking the calling thread.
    let mut cmd = std::process::Command::new("sleep");
    cmd.arg("10");

    let timeout = std::time::Duration::from_millis(400);
    let start = std::time::Instant::now();
    let res = resticdashboard_lib::restic_cli::run_command_with_timeout(cmd, timeout);

    assert!(res.is_err(), "Sleep 10 with 400ms timeout must return Err");
    let err_str = res.unwrap_err();
    assert!(err_str.contains("timed out"));
    assert!(start.elapsed() < std::time::Duration::from_millis(1500), "Must terminate quickly around timeout");
}

#[test]
fn test_real_local_repository_integration_slice() {
    let temp_dir = TempDir::new().expect("Failed to create tempdir");
    let repo_dir = temp_dir.path().join("repo");
    let src_dir = temp_dir.path().join("source_files with spaces");
    let restore_dir = temp_dir.path().join("restore_destination");

    fs::create_dir_all(&src_dir).unwrap();
    let test_file = src_dir.join("minecraft_options.txt");
    fs::write(&test_file, "gamma:1.0\nfov:70\nversion:1.20\n").unwrap();

    let sub_dir = src_dir.join("subfolder_ünicøde");
    fs::create_dir_all(&sub_dir).unwrap();
    let sub_file = sub_dir.join("world_meta.json");
    fs::write(&sub_file, "{\"name\":\"Super World\",\"hardcore\":false}").unwrap();

    // 1. Initialize repository with --insecure-no-password
    let engine = ResticEngine::new();
    let mut init_cmd = std::process::Command::new(engine.binary_path());
    init_cmd.args(["init", "-r", repo_dir.to_str().unwrap(), "--insecure-no-password"]);
    let init_out = init_cmd.output().expect("Failed to run restic init");
    assert!(init_out.status.success(), "Init should succeed: {}", String::from_utf8_lossy(&init_out.stderr));

    let repo_config = RepoConfig {
        id: "local-temp-test-repo".to_string(),
        name: "Temporary Integration Repo".to_string(),
        repo_type: RepoType::Local,
        target: repo_dir.to_str().unwrap().to_string(),
        insecure_no_password: true,
        has_password: false,
        b2_key_id: None,
        b2_endpoint: None,
        b2_bucket: None,
        b2_prefix: None,
        b2_management_key_id: None,
    };
    let creds = SessionCredentials::default();

    // 2. Safe connection test
    let conn_res = engine.test_connection(&repo_config, &creds);
    assert!(conn_res.is_ok(), "Test connection on fresh repo should succeed");

    // 3. Backup operation
    let profile = Profile {
        id: "temp-test-profile".to_string(),
        name: "Temp Profile".to_string(),
        repo_id: repo_config.id.clone(),
        machine_id: "test-machine".to_string(),
        machine_name: "Test Host".to_string(),
        source_paths: vec![src_dir.to_str().unwrap().to_string()],
        exclusions: vec!["*.tmp".to_string()],
        tags: vec!["integration-test".to_string()],
        schedule: None,
        minecraft_preset: None,
        pre_hook: None,
        post_hook: None,
        keep_last: None,
        tag_timestamp: Some(false),
        distinction_label: None,
        is_pinned: None,
    };

    let cancel_flag = Arc::new(AtomicBool::new(false));
    let backup_res = engine.run_backup(&repo_config, &profile, &creds, cancel_flag, |_| {});
    assert!(backup_res.is_ok(), "Backup should succeed");
    let run = backup_res.unwrap();
    assert_eq!(run.exit_code, 0);
    assert_eq!(run.status, "success");
    assert_eq!(run.total_files_processed, 2);
    let snapshot_id = run.snapshot_id.expect("Snapshot ID must be present");

    // 4. List snapshots
    let snapshots = engine.list_snapshots(&repo_config, &creds, None, None).expect("Should list snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].short_id, &snapshot_id[..8]);

    // 5. Query stats (raw-data vs restore-size)
    let raw_stats = engine.get_stats(&repo_config, &creds, "raw-data", None).expect("Stats raw-data");
    let restore_stats = engine.get_stats(&repo_config, &creds, "restore-size", None).expect("Stats restore-size");
    assert!(raw_stats.total_size > 0);
    assert!(restore_stats.total_size > 0);

    // 6. List nodes in snapshot (recursive=false tests single level, then recursive=true tests deep traversal)
    let nodes = engine.list_nodes(&repo_config, &creds, &snapshot_id, None, false).expect("List nodes");
    assert!(nodes.iter().any(|n| n.name == "minecraft_options.txt"));
    assert!(nodes.iter().any(|n| n.name == "world_meta.json"));

    let recursive_nodes = engine.list_nodes(&repo_config, &creds, &snapshot_id, None, true).expect("List nodes recursive");
    assert!(recursive_nodes.len() >= 2);

    // 7. Restore snapshot to new destination
    let restore_res = engine.run_restore(
        &repo_config,
        &creds,
        &snapshot_id,
        restore_dir.to_str().unwrap(),
        &[],
        true,
    );
    assert!(restore_res.is_ok(), "Restore should succeed");

    // 8. Verify restored content fidelity
    // Restic restores the full directory tree
    let found_options = walkdir::WalkDir::new(&restore_dir)
        .into_iter()
        .filter_map(|e| e.ok())
        .find(|e| e.file_name() == "minecraft_options.txt");
    assert!(found_options.is_some(), "Restored file must exist");
    let restored_content = fs::read_to_string(found_options.unwrap().path()).unwrap();
    assert_eq!(restored_content, "gamma:1.0\nfov:70\nversion:1.20\n");
}

#[test]
fn test_retention_policy_and_pinned_tag_protection() {
    let temp = TempDir::new().expect("Failed to create tempdir");
    let repo_dir = temp.path().join("repo");
    let src_dir = temp.path().join("src");
    fs::create_dir_all(&repo_dir).unwrap();
    fs::create_dir_all(&src_dir).unwrap();
    fs::write(src_dir.join("test_save.dat"), "world data").unwrap();

    let engine = ResticEngine::new();
    let repo_config = RepoConfig {
        id: "retention-test-repo".to_string(),
        name: "Retention Test Repo".to_string(),
        repo_type: RepoType::Local,
        target: repo_dir.to_str().unwrap().to_string(),
        insecure_no_password: true,
        has_password: false,
        b2_key_id: None,
        b2_endpoint: None,
        b2_bucket: None,
        b2_prefix: None,
        b2_management_key_id: None,
    };
    let creds = SessionCredentials::default();

    // Init repo with --insecure-no-password
    let mut init_cmd = std::process::Command::new(engine.binary_path());
    init_cmd.args(["init", "-r", repo_dir.to_str().unwrap(), "--insecure-no-password"]);
    let init_out = init_cmd.output().expect("Failed to run restic init");
    assert!(init_out.status.success(), "Init should succeed: {}", String::from_utf8_lossy(&init_out.stderr));

    // 1. Snapshot 1: Pinned / Important distinction backup
    let profile_pinned = Profile {
        id: "p1".to_string(),
        name: "Pinned Backup".to_string(),
        repo_id: repo_config.id.clone(),
        machine_id: "test-machine".to_string(),
        machine_name: "Test Host".to_string(),
        source_paths: vec![src_dir.to_str().unwrap().to_string()],
        exclusions: vec![],
        tags: vec!["distinction:Before Dragon".to_string()],
        schedule: None,
        minecraft_preset: None,
        pre_hook: None,
        post_hook: None,
        keep_last: None,
        tag_timestamp: Some(false),
        distinction_label: Some("Before Dragon".to_string()),
        is_pinned: Some(true),
    };
    let cancel = Arc::new(AtomicBool::new(false));
    let r1 = engine.run_backup(&repo_config, &profile_pinned, &creds, cancel.clone(), |_| {}).expect("Backup 1");
    let snap1_id = r1.snapshot_id.expect("Snap 1 ID");

    // Small delay to ensure separate timestamps if needed
    fs::write(src_dir.join("test_save.dat"), "world data v2").unwrap();

    // 2. Snapshot 2: Regular backup
    let mut profile_regular = profile_pinned.clone();
    profile_regular.id = "p2".to_string();
    profile_regular.tags = vec!["regular-1".to_string()];
    profile_regular.distinction_label = None;
    profile_regular.is_pinned = Some(false);
    let r2 = engine.run_backup(&repo_config, &profile_regular, &creds, cancel.clone(), |_| {}).expect("Backup 2");
    assert!(r2.snapshot_id.is_some());

    fs::write(src_dir.join("test_save.dat"), "world data v3").unwrap();

    // 3. Snapshot 3: Regular backup
    profile_regular.id = "p3".to_string();
    profile_regular.tags = vec!["regular-2".to_string()];
    let r3 = engine.run_backup(&repo_config, &profile_regular, &creds, cancel.clone(), |_| {}).expect("Backup 3");
    assert!(r3.snapshot_id.is_some());

    // 4. Verify 3 snapshots exist initially
    let all_snaps = engine.list_snapshots(&repo_config, &creds, None, None).expect("List 3 snapshots");
    assert_eq!(all_snaps.len(), 3);

    // 5. Execute retention policy: Keep last 1, but preserve pinned/important
    let forget_res = engine.run_forget_policy(&repo_config, &creds, 1, &["important", "pinned"]);
    assert!(forget_res.is_ok(), "Forget policy should execute successfully");

    // 6. Verify: Pinned snapshot (snap1) MUST still exist despite being the oldest!
    let remaining_snaps = engine.list_snapshots(&repo_config, &creds, None, None).expect("List remaining");
    assert!(
        remaining_snaps.iter().any(|s| s.id == snap1_id),
        "Pinned snapshot must NOT be deleted by retention policy"
    );
    assert_eq!(remaining_snaps.len(), 2, "Should keep 1 pinned + 1 latest regular snapshot");

    // 7. Verify tag modification
    let tag_res = engine.tag_snapshot(&repo_config, &creds, &snap1_id, &["custom-tag"], &[]);
    assert!(tag_res.is_ok(), "Tag operation should succeed");

    // 8. Test deleting a single snapshot manually one at a time
    let current_snaps = engine.list_snapshots(&repo_config, &creds, None, None).expect("List current");
    assert_eq!(current_snaps.len(), 2);
    let snap_to_delete = current_snaps[0].id.clone();
    let del_res = engine.delete_single_snapshot(&repo_config, &creds, &snap_to_delete, false);
    assert!(del_res.is_ok(), "Delete single snapshot should succeed: {:?}", del_res);
    let after_del = engine.list_snapshots(&repo_config, &creds, None, None).expect("List after delete");
    assert_eq!(after_del.len(), 1, "Only 1 snapshot should remain after manual single deletion");
    assert!(!after_del.iter().any(|s| s.id == snap_to_delete), "Deleted snapshot must not be in list");
}
