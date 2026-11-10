use std::path::{Component, Path, PathBuf};
use std::sync::LazyLock;

static REDACT_PATTERNS: LazyLock<Vec<(regex::Regex, &'static str)>> = LazyLock::new(|| {
    vec![
        (
            regex::Regex::new(r"(?i)(RESTIC_PASSWORD\s*=\s*)([^\s\r\n]+)").unwrap(),
            "$1[REDACTED_PASSWORD]",
        ),
        (
            regex::Regex::new(r"(?i)(AWS_SECRET_ACCESS_KEY\s*=\s*)([^\s\r\n]+)").unwrap(),
            "$1[REDACTED_SECRET_KEY]",
        ),
        (
            regex::Regex::new(r"(?i)(--password-file\s+)(\S+)").unwrap(),
            "$1[PATH_REDACTED]",
        ),
        (
            regex::Regex::new(r"(?i)(--password-command\s+)(.+?)(\s+--|\s*$)").unwrap(),
            "$1[COMMAND_REDACTED]$3",
        ),
    ]
});

/// Redact known secrets (passwords, access keys) from arbitrary strings, logs, or command previews.
pub fn redact_secrets(input: &str, sensitive_strings: &[&str]) -> String {
    if input.is_empty() {
        return String::new();
    }

    let mut redacted = input.to_string();

    for secret in sensitive_strings {
        if !secret.trim().is_empty() && secret.len() >= 3 {
            redacted = redacted.replace(secret, "[REDACTED_SECRET]");
        }
    }

    // Only run regexes if candidate patterns might exist in the string
    let upper = input.to_ascii_uppercase();
    if upper.contains("RESTIC_PASSWORD")
        || upper.contains("AWS_SECRET")
        || upper.contains("--PASSWORD")
    {
        for (re, rep) in REDACT_PATTERNS.iter() {
            redacted = re.replace_all(&redacted, *rep).to_string();
        }
    }

    redacted
}

/// Sets restrictive OS permissions (0600 on Unix) on secret files.
pub fn set_restrictive_permissions(path: &Path) -> std::io::Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let perms = std::fs::Permissions::from_mode(0o600);
        std::fs::set_permissions(path, perms)?;
    }

    #[cfg(not(unix))]
    {
        // On Windows, the file created in %LOCALAPPDATA% is by default user-owned
        // and restricted to the current user's profile ACL.
        let _ = path;
    }

    Ok(())
}

/// Validates that a restore target path and included paths cannot escape the destination directory.
pub fn validate_restore_path(
    target_base: &Path,
    relative_candidate: &str,
) -> Result<PathBuf, String> {
    let rel_path = Path::new(relative_candidate);

    // Prevent path traversal components
    for comp in rel_path.components() {
        match comp {
            Component::ParentDir => {
                return Err(format!(
                    "Path traversal attempt detected with parent directory '..' in '{}'",
                    relative_candidate
                ));
            }
            Component::Prefix(_) | Component::RootDir => {
                // If the user specifies an absolute path, we must ensure it gets safely relativized under target_base
                // restic snapshots often store absolute paths like /Users/... or C:\...
            }
            Component::Normal(_) | Component::CurDir => {}
        }
    }

    // Safely strip prefix or root if present
    let mut safe_components = Vec::new();
    for comp in rel_path.components() {
        if let Component::Normal(c) = comp {
            safe_components.push(c);
        }
    }

    let mut dest = target_base.to_path_buf();
    for comp in safe_components {
        dest.push(comp);
    }

    Ok(dest)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_secret_redaction() {
        let text = "Connecting to s3 with AWS_SECRET_ACCESS_KEY=SuperSecretKey123 and RESTIC_PASSWORD=myRepoPasswordNow";
        let redacted = redact_secrets(text, &["SuperSecretKey123", "myRepoPasswordNow"]);
        assert!(!redacted.contains("SuperSecretKey123"));
        assert!(!redacted.contains("myRepoPasswordNow"));
        assert!(redacted.contains("[REDACTED_SECRET]") || redacted.contains("[REDACTED"));
    }

    #[test]
    fn test_path_traversal_detection() {
        let base = Path::new("/tmp/restore_target");
        let malicious = "../../../etc/passwd";
        let res = validate_restore_path(base, malicious);
        assert!(res.is_err(), "Should detect parent dir traversal");

        let safe = "Users/demo/Library/Application Support/PrismLauncher";
        let res_safe = validate_restore_path(base, safe);
        assert!(res_safe.is_ok());
        let final_path = res_safe.unwrap();
        assert!(final_path.starts_with(base));
    }
}
