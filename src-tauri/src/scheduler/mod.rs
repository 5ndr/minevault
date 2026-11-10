pub mod linux;
pub mod macos;
pub mod windows;

use std::fs;
use std::path::{Path, PathBuf};

use crate::models::{Profile, RepoConfig, SchedulerStatus, SessionCredentials};
use crate::security::set_restrictive_permissions;
use crate::system::get_persistent_app_dir;

pub trait SchedulerAdapter {
    fn platform_name(&self) -> &'static str;
    fn install(
        &self,
        profile: &Profile,
        repo: &RepoConfig,
        creds: &SessionCredentials,
        restic_bin: &Path,
    ) -> Result<SchedulerStatus, String>;
    fn uninstall(&self, profile_id: &str) -> Result<(), String>;
    fn disable(&self, profile_id: &str) -> Result<(), String>;
    fn enable(&self, profile_id: &str) -> Result<(), String>;
    fn status(&self, profile_id: &str) -> Result<SchedulerStatus, String>;
}

/// Helper to copy the current running binary and restic binary to the persistent local directory,
/// and save the persistent profile configuration and restrictive secrets.
pub fn setup_persistent_local_files(
    profile: &Profile,
    repo: &RepoConfig,
    creds: &SessionCredentials,
    restic_bin: &Path,
) -> Result<(PathBuf, PathBuf, PathBuf), String> {
    let base_dir = get_persistent_app_dir();
    let bin_dir = base_dir.join("bin");
    let configs_dir = base_dir.join("configs");
    let secrets_dir = base_dir.join("secrets");
    let logs_dir = base_dir.join("logs");
    let status_dir = base_dir.join("status");
    let locks_dir = base_dir.join("locks");

    fs::create_dir_all(&bin_dir)
        .map_err(|e| format!("Failed to create persistent bin dir: {}", e))?;
    fs::create_dir_all(&configs_dir)
        .map_err(|e| format!("Failed to create persistent configs dir: {}", e))?;
    fs::create_dir_all(&secrets_dir)
        .map_err(|e| format!("Failed to create persistent secrets dir: {}", e))?;
    fs::create_dir_all(&logs_dir)
        .map_err(|e| format!("Failed to create persistent logs dir: {}", e))?;
    fs::create_dir_all(&status_dir)
        .map_err(|e| format!("Failed to create persistent status dir: {}", e))?;
    fs::create_dir_all(&locks_dir)
        .map_err(|e| format!("Failed to create persistent locks dir: {}", e))?;

    // 1. Copy runner executable
    let current_exe =
        std::env::current_exe().map_err(|e| format!("Failed to get current executable: {}", e))?;
    let target_runner_name = if cfg!(windows) {
        "resticdashboard-runner.exe"
    } else {
        "resticdashboard-runner"
    };
    let persistent_runner = bin_dir.join(target_runner_name);

    if current_exe.is_file() {
        let _ = fs::copy(&current_exe, &persistent_runner);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&persistent_runner, fs::Permissions::from_mode(0o755));
        }
    }

    // 2. Copy restic engine binary if it's not already in bin_dir
    let target_restic_name = if cfg!(windows) {
        "restic.exe"
    } else {
        "restic"
    };
    let persistent_restic = bin_dir.join(target_restic_name);
    if restic_bin.is_file() && restic_bin != persistent_restic {
        let _ = fs::copy(restic_bin, &persistent_restic);
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            let _ = fs::set_permissions(&persistent_restic, fs::Permissions::from_mode(0o755));
        }
    }

    // 3. Save profile and repository configuration (NO plain secrets inside)
    let config_path = configs_dir.join(format!("{}.json", profile.id));
    let combined_config = serde_json::json!({
        "profile": profile,
        "repo": repo,
    });
    fs::write(
        &config_path,
        serde_json::to_string_pretty(&combined_config).unwrap(),
    )
    .map_err(|e| format!("Failed to write persistent config: {}", e))?;

    // 4. Save secret file with restrictive 0600 permissions
    let secret_path = secrets_dir.join(format!("{}.env", profile.id));
    let mut secret_lines = Vec::new();
    if let Some(ref pass) = creds.password {
        secret_lines.push(format!("RESTIC_PASSWORD={}", pass));
    }
    if let Some(ref app_key) = creds.b2_application_key {
        secret_lines.push(format!("AWS_SECRET_ACCESS_KEY={}", app_key));
    }
    fs::write(&secret_path, secret_lines.join("\n"))
        .map_err(|e| format!("Failed to write secrets file: {}", e))?;

    set_restrictive_permissions(&secret_path).map_err(|e| {
        format!(
            "Failed to set restrictive permissions on secret file: {}",
            e
        )
    })?;

    Ok((persistent_runner, config_path, secret_path))
}

pub fn get_active_scheduler() -> Box<dyn SchedulerAdapter> {
    #[cfg(target_os = "macos")]
    {
        Box::new(macos::MacOsLaunchdAdapter::new())
    }

    #[cfg(target_os = "linux")]
    {
        Box::new(linux::LinuxSystemdAdapter::new())
    }

    #[cfg(target_os = "windows")]
    {
        Box::new(windows::WindowsTaskSchedulerAdapter::new())
    }

    #[cfg(not(any(target_os = "macos", target_os = "linux", target_os = "windows")))]
    {
        Box::new(macos::MacOsLaunchdAdapter::new())
    }
}
