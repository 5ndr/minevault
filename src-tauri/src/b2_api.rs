use chrono::Utc;
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};
use std::process::Command;

use crate::models::{RepoConfig, SessionCredentials};
use crate::system::get_persistent_app_dir;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2ProviderUsage {
    pub total_bytes: u64,
    pub file_count: u64,
    pub bucket_name: String,
    pub prefix: Option<String>,
    pub measured_at: String,
    pub source: String,
    pub is_partial: bool,
    #[serde(default)]
    pub active_bytes: u64,
    #[serde(default)]
    pub active_file_count: u64,
    #[serde(default)]
    pub hidden_version_bytes: u64,
    #[serde(default)]
    pub hidden_version_count: u64,
    #[serde(default)]
    pub has_lifecycle_rules: bool,
    #[serde(default)]
    pub lifecycle_warning: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct B2PurgeResult {
    pub deleted_files_count: u64,
    pub freed_bytes: u64,
    pub updated_usage: B2ProviderUsage,
}

#[derive(Deserialize)]
struct B2StorageApiInfo {
    #[serde(rename = "apiUrl", default)]
    api_url: Option<String>,
    #[serde(rename = "downloadUrl", default)]
    #[allow(dead_code)]
    download_url: Option<String>,
}

#[derive(Deserialize)]
struct B2ApiInfo {
    #[serde(rename = "storageApi", default)]
    storage_api: Option<B2StorageApiInfo>,
}

#[derive(Deserialize)]
struct B2AuthResponse {
    #[serde(rename = "accountId")]
    account_id: String,
    #[serde(rename = "apiUrl", default)]
    api_url: Option<String>,
    #[serde(rename = "authorizationToken")]
    authorization_token: String,
    #[serde(rename = "apiInfo", default)]
    api_info: Option<B2ApiInfo>,
}

impl B2AuthResponse {
    pub fn get_api_url(&self) -> Result<String, String> {
        if let Some(url) = &self.api_url
            && !url.trim().is_empty()
        {
            return Ok(url.trim().to_string());
        }
        if let Some(info) = &self.api_info
            && let Some(storage) = &info.storage_api
            && let Some(url) = &storage.api_url
            && !url.trim().is_empty()
        {
            return Ok(url.trim().to_string());
        }
        Err("Missing 'apiUrl' in B2 authorization response".to_string())
    }
}

#[derive(Deserialize)]
struct B2BucketItem {
    #[serde(rename = "bucketId")]
    bucket_id: String,
    #[serde(rename = "bucketName")]
    bucket_name: String,
    #[serde(rename = "lifecycleRules", default)]
    lifecycle_rules: Vec<serde_json::Value>,
}

#[derive(Deserialize)]
struct B2ListBucketsResponse {
    #[serde(default)]
    buckets: Vec<B2BucketItem>,
}

#[derive(Deserialize)]
struct B2FileItem {
    #[serde(rename = "contentLength", default)]
    content_length: u64,
    #[serde(rename = "fileName", default)]
    file_name: String,
    #[serde(rename = "fileId", default)]
    file_id: Option<String>,
}

#[derive(Deserialize)]
struct B2ListFileNamesResponse {
    #[serde(default)]
    files: Vec<B2FileItem>,
    #[serde(rename = "nextFileName")]
    next_file_name: Option<String>,
}

#[derive(Deserialize)]
struct B2FileVersionItem {
    #[serde(rename = "fileId")]
    file_id: String,
    #[serde(rename = "fileName")]
    file_name: String,
    #[serde(rename = "action")]
    action: String,
    #[serde(rename = "contentLength", default)]
    content_length: u64,
}

#[derive(Deserialize)]
struct B2ListFileVersionsResponse {
    #[serde(default)]
    files: Vec<B2FileVersionItem>,
    #[serde(rename = "nextFileName")]
    next_file_name: Option<String>,
    #[serde(rename = "nextFileId")]
    next_file_id: Option<String>,
}

/// Parses a Backblaze B2 target string into (host, bucket_name, prefix)
pub fn parse_b2_target(target: &str) -> Option<(String, String, Option<String>)> {
    let mut clean = target.trim();
    if clean.starts_with("s3:") {
        clean = &clean[3..];
    }
    if clean.starts_with("//") {
        clean = &clean[2..];
    }
    if clean.starts_with("https://") {
        clean = &clean[8..];
    } else if clean.starts_with("http://") {
        clean = &clean[7..];
    }

    let parts: Vec<&str> = clean.split('/').filter(|s| !s.is_empty()).collect();
    if parts.is_empty() {
        return None;
    }

    let host = parts[0].to_string();
    if parts.len() < 2 {
        return None;
    }

    let bucket = parts[1].to_string();
    let prefix = if parts.len() > 2 {
        Some(parts[2..].join("/"))
    } else {
        None
    };

    Some((host, bucket, prefix))
}

fn resolve_b2_credentials(
    repo: &RepoConfig,
    creds: &SessionCredentials,
) -> Result<(String, String), String> {
    let mut key_id = repo
        .b2_management_key_id
        .clone()
        .or_else(|| repo.b2_key_id.clone());

    let mut app_key = creds
        .b2_management_app_key
        .clone()
        .or_else(|| creds.b2_application_key.clone());

    if (key_id.is_none() || app_key.is_none())
        && let Ok(vault) = crate::usb_vault::load_usb_vault(None)
    {
        if key_id.is_none() {
            key_id = vault.b2_management_key_id.or(vault.b2_key_id);
        }
        if app_key.is_none() {
            app_key = vault.b2_management_app_key.or(vault.b2_application_key);
        }
    }

    let k = key_id.ok_or_else(|| {
        "B2 Application Key ID is unconfigured. Please provide your B2 Key ID or Application Key.".to_string()
    })?;
    let a = app_key.ok_or_else(|| {
        "B2 Secret Application Key is unconfigured in session credentials or USB vault.".to_string()
    })?;

    Ok((k, a))
}

fn authorize_b2(key_id: &str, app_key: &str) -> Result<B2AuthResponse, String> {
    let auth_arg = format!("{}:{}", key_id, app_key);
    let mut auth_cmd = Command::new("curl");
    auth_cmd
        .arg("-s")
        .arg("-m")
        .arg("15")
        .arg("-u")
        .arg(&auth_arg)
        .arg("https://api.backblazeb2.com/b2api/v3/b2_authorize_account");

    let auth_out = auth_cmd
        .output()
        .map_err(|e| format!("Failed to execute curl: {}", e))?;
    if !auth_out.status.success() {
        let err_text = String::from_utf8_lossy(&auth_out.stderr);
        return Err(format!(
            "B2 Authorize Account network error: {}",
            err_text.trim()
        ));
    }

    let auth_str = String::from_utf8_lossy(&auth_out.stdout);
    if auth_str.contains("\"code\":") && !auth_str.contains("\"authorizationToken\":") {
        return Err(format!("B2 Authentication failed: {}", auth_str.trim()));
    }

    let auth_resp: B2AuthResponse = serde_json::from_str(&auth_str).map_err(|e| {
        format!(
            "Failed to parse B2 authorization response: {} (response: {})",
            e, auth_str
        )
    })?;

    Ok(auth_resp)
}

fn get_b2_bucket(
    api_url: &str,
    auth_token: &str,
    account_id: &str,
    bucket_name: &str,
) -> Result<B2BucketItem, String> {
    let list_buckets_body = serde_json::json!({
        "accountId": account_id,
        "bucketName": bucket_name
    });

    let list_buckets_url = format!("{}/b2api/v3/b2_list_buckets", api_url);
    let mut list_cmd = Command::new("curl");
    list_cmd
        .arg("-s")
        .arg("-m")
        .arg("20")
        .arg("-H")
        .arg(format!("Authorization: {}", auth_token))
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-d")
        .arg(list_buckets_body.to_string())
        .arg(&list_buckets_url);

    let list_out = list_cmd
        .output()
        .map_err(|e| format!("Failed to list B2 buckets: {}", e))?;
    let mut list_str = String::from_utf8_lossy(&list_out.stdout).to_string();

    if list_str.contains("\"status\": 404") || list_str.contains("\"code\": \"not_found\"") {
        let v2_url = format!("{}/b2api/v2/b2_list_buckets", api_url);
        let mut v2_cmd = Command::new("curl");
        v2_cmd
            .arg("-s")
            .arg("-m")
            .arg("20")
            .arg("-H")
            .arg(format!("Authorization: {}", auth_token))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(list_buckets_body.to_string())
            .arg(&v2_url);
        if let Ok(v2_out) = v2_cmd.output() {
            list_str = String::from_utf8_lossy(&v2_out.stdout).to_string();
        }
    }

    let list_resp: B2ListBucketsResponse = serde_json::from_str(&list_str).map_err(|e| {
        format!(
            "Failed to parse B2 bucket list: {} (response: {})",
            e, list_str
        )
    })?;

    let bucket_entry = list_resp
        .buckets
        .into_iter()
        .find(|b| b.bucket_name.eq_ignore_ascii_case(bucket_name))
        .ok_or_else(|| {
            format!(
                "Bucket '{}' was not found or is inaccessible with this B2 key.",
                bucket_name
            )
        })?;

    Ok(bucket_entry)
}

/// Queries Backblaze B2 Native API directly to measure physical bucket storage, active blobs, and version breakdown
pub fn query_b2_bucket_usage(
    repo: &RepoConfig,
    creds: &SessionCredentials,
) -> Result<B2ProviderUsage, String> {
    let (key_id, app_key) = resolve_b2_credentials(repo, creds)?;

    let (bucket_name, prefix) = if let Some(b) = &repo.b2_bucket {
        (b.clone(), repo.b2_prefix.clone())
    } else if let Some((_, b, p)) = parse_b2_target(&repo.target) {
        (b, p)
    } else {
        return Err(format!(
            "Could not parse B2 bucket name from target URL '{}'. Format expected: s3:https://s3.<region>.backblazeb2.com/<bucket>/<prefix>",
            repo.target
        ));
    };

    let auth_resp = authorize_b2(&key_id, &app_key)?;
    let api_url = auth_resp.get_api_url()?;
    let bucket_entry = get_b2_bucket(
        &api_url,
        &auth_resp.authorization_token,
        &auth_resp.account_id,
        &bucket_name,
    )?;
    let has_lifecycle_rules = !bucket_entry.lifecycle_rules.is_empty();

    // 1. Measure Active (Visible) Files via b2_list_file_names
    let mut list_files_url = format!("{}/b2api/v3/b2_list_file_names", api_url);
    let mut active_bytes: u64 = 0;
    let mut active_file_count: u64 = 0;
    let mut start_file_name: Option<String> = None;
    let prefix_filter = prefix.clone().unwrap_or_default();
    let mut seen_active_file_names: HashSet<String> = HashSet::new();

    for iterations in 1..=500 {
        let mut req_body = serde_json::json!({
            "bucketId": bucket_entry.bucket_id,
            "maxFileCount": 1000
        });

        if !prefix_filter.is_empty() {
            req_body["prefix"] = serde_json::json!(prefix_filter);
        }
        if let Some(s) = &start_file_name {
            req_body["startFileName"] = serde_json::json!(s);
        }

        let mut files_cmd = Command::new("curl");
        files_cmd
            .arg("-s")
            .arg("-m")
            .arg("25")
            .arg("-H")
            .arg(format!("Authorization: {}", auth_resp.authorization_token))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(req_body.to_string())
            .arg(&list_files_url);

        let files_out = files_cmd
            .output()
            .map_err(|e| format!("Failed to list files: {}", e))?;
        let mut files_str = String::from_utf8_lossy(&files_out.stdout).to_string();

        if iterations == 1
            && (files_str.contains("\"status\": 404")
                || files_str.contains("\"code\": \"not_found\""))
        {
            list_files_url = format!("{}/b2api/v2/b2_list_file_names", api_url);
            let mut v2_cmd = Command::new("curl");
            v2_cmd
                .arg("-s")
                .arg("-m")
                .arg("25")
                .arg("-H")
                .arg(format!("Authorization: {}", auth_resp.authorization_token))
                .arg("-H")
                .arg("Content-Type: application/json")
                .arg("-d")
                .arg(req_body.to_string())
                .arg(&list_files_url);
            if let Ok(v2_out) = v2_cmd.output() {
                files_str = String::from_utf8_lossy(&v2_out.stdout).to_string();
            }
        }

        if files_str.contains("\"code\":") && !files_str.contains("\"files\":") {
            return Err(format!("B2 list file names error: {}", files_str.trim()));
        }

        let files_resp: B2ListFileNamesResponse = serde_json::from_str(&files_str)
            .map_err(|e| format!("Failed to parse B2 files: {} (response: {})", e, files_str))?;

        for file in files_resp.files {
            if seen_active_file_names.insert(file.file_name) {
                active_bytes += file.content_length;
                active_file_count += 1;
            }
        }

        if let Some(next) = files_resp.next_file_name {
            start_file_name = Some(next);
        } else {
            break;
        }
    }

    // 2. Measure Total Versions (including hidden/obsolete versions) via b2_list_file_versions
    let mut total_version_bytes: u64 = 0;
    let mut total_version_count: u64 = 0;
    let mut hidden_count: u64 = 0;
    let mut start_version_name: Option<String> = None;
    let mut start_version_id: Option<String> = None;
    let mut seen_version_ids: HashSet<String> = HashSet::new();

    let mut list_versions_url = format!("{}/b2api/v3/b2_list_file_versions", api_url);

    for iterations in 1..=500 {
        let mut body = serde_json::json!({
            "bucketId": bucket_entry.bucket_id,
            "maxFileCount": 1000
        });

        if !prefix_filter.is_empty() {
            body["prefix"] = serde_json::json!(prefix_filter);
        }
        if let Some(s) = &start_version_name {
            body["startFileName"] = serde_json::json!(s);
        }
        if let Some(id) = &start_version_id {
            body["startFileId"] = serde_json::json!(id);
        }

        let mut v_cmd = Command::new("curl");
        v_cmd
            .arg("-s")
            .arg("-m")
            .arg("25")
            .arg("-H")
            .arg(format!("Authorization: {}", auth_resp.authorization_token))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(body.to_string())
            .arg(&list_versions_url);

        let v_out = v_cmd
            .output()
            .map_err(|e| format!("Failed to list versions: {}", e))?;
        let mut v_str = String::from_utf8_lossy(&v_out.stdout).to_string();

        if iterations == 1
            && (v_str.contains("\"status\": 404") || v_str.contains("\"code\": \"not_found\""))
        {
            list_versions_url = format!("{}/b2api/v2/b2_list_file_versions", api_url);
            let mut v2_cmd = Command::new("curl");
            v2_cmd
                .arg("-s")
                .arg("-m")
                .arg("25")
                .arg("-H")
                .arg(format!("Authorization: {}", auth_resp.authorization_token))
                .arg("-H")
                .arg("Content-Type: application/json")
                .arg("-d")
                .arg(body.to_string())
                .arg(&list_versions_url);
            if let Ok(v2_out) = v2_cmd.output() {
                v_str = String::from_utf8_lossy(&v2_out.stdout).to_string();
            }
        }

        let v_resp: B2ListFileVersionsResponse = match serde_json::from_str(&v_str) {
            Ok(r) => r,
            Err(_) => break, // Fallback gracefully if versions endpoint restricted
        };

        for f in v_resp.files {
            if seen_version_ids.insert(f.file_id) {
                total_version_bytes += f.content_length;
                total_version_count += 1;
                if f.action == "hide" {
                    hidden_count += 1;
                }
            }
        }

        if let Some(next_name) = v_resp.next_file_name {
            start_version_name = Some(next_name);
            start_version_id = v_resp.next_file_id;
        } else {
            break;
        }
    }

    let total_bytes = if total_version_bytes > 0 {
        total_version_bytes
    } else {
        active_bytes
    };

    let hidden_version_bytes = total_bytes.saturating_sub(active_bytes);
    let hidden_version_count = if total_version_count > active_file_count {
        total_version_count - active_file_count
    } else {
        hidden_count
    };

    let lifecycle_warning = if hidden_version_bytes > 0 && !has_lifecycle_rules {
        Some(format!(
            "Backblaze B2 is retaining {:.2} GB ({} files) of obsolete/deleted backup versions because no lifecycle rule is configured. Backblaze default 'Keep all versions' duplicates deleted/pruned restic blobs forever.",
            hidden_version_bytes as f64 / 1_000_000_000.0,
            hidden_version_count
        ))
    } else {
        None
    };

    let usage = B2ProviderUsage {
        total_bytes,
        file_count: if total_version_count > 0 {
            total_version_count
        } else {
            active_file_count
        },
        bucket_name,
        prefix,
        measured_at: Utc::now().to_rfc3339(),
        source: "Backblaze B2 Native API (Direct & Version Breakdown)".to_string(),
        is_partial: false,
        active_bytes,
        active_file_count,
        hidden_version_bytes,
        hidden_version_count,
        has_lifecycle_rules,
        lifecycle_warning,
    };

    save_cached_b2_usage(&usage);
    Ok(usage)
}

/// Sets lifecycle rules on the Backblaze B2 bucket to automatically delete hidden versions after 1 day
pub fn apply_b2_lifecycle_rules(
    repo: &RepoConfig,
    creds: &SessionCredentials,
) -> Result<String, String> {
    let (key_id, app_key) = resolve_b2_credentials(repo, creds)?;
    let auth_resp = authorize_b2(&key_id, &app_key)?;
    let api_url = auth_resp.get_api_url()?;

    let (bucket_name, _prefix) = if let Some(b) = &repo.b2_bucket {
        (b.clone(), repo.b2_prefix.clone())
    } else if let Some((_, b, p)) = parse_b2_target(&repo.target) {
        (b, p)
    } else {
        return Err(format!(
            "Could not parse B2 bucket name from target URL '{}'",
            repo.target
        ));
    };

    let bucket_entry = get_b2_bucket(
        &api_url,
        &auth_resp.authorization_token,
        &auth_resp.account_id,
        &bucket_name,
    )?;

    let mut update_url = format!("{}/b2api/v3/b2_update_bucket", api_url);
    let body = serde_json::json!({
        "accountId": auth_resp.account_id,
        "bucketId": bucket_entry.bucket_id,
        "lifecycleRules": [
            {
                "fileNamePrefix": "",
                "daysFromHidingToDeleting": 1
            }
        ]
    });

    let mut update_cmd = Command::new("curl");
    update_cmd
        .arg("-s")
        .arg("-m")
        .arg("20")
        .arg("-H")
        .arg(format!("Authorization: {}", auth_resp.authorization_token))
        .arg("-H")
        .arg("Content-Type: application/json")
        .arg("-d")
        .arg(body.to_string())
        .arg(&update_url);

    let out = update_cmd
        .output()
        .map_err(|e| format!("Failed to update bucket: {}", e))?;
    let mut out_str = String::from_utf8_lossy(&out.stdout).to_string();

    if out_str.contains("\"status\": 404") || out_str.contains("\"code\": \"not_found\"") {
        update_url = format!("{}/b2api/v2/b2_update_bucket", api_url);
        let mut v2_cmd = Command::new("curl");
        v2_cmd
            .arg("-s")
            .arg("-m")
            .arg("20")
            .arg("-H")
            .arg(format!("Authorization: {}", auth_resp.authorization_token))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(body.to_string())
            .arg(&update_url);
        if let Ok(v2_out) = v2_cmd.output() {
            out_str = String::from_utf8_lossy(&v2_out.stdout).to_string();
        }
    }

    if !out_str.contains("\"lifecycleRules\"") {
        return Err(format!("Failed to update B2 lifecycle rules: {}", out_str));
    }

    Ok("Successfully enabled B2 lifecycle rule: Backblaze will automatically delete old/hidden versions after 1 day.".to_string())
}

/// Purges all obsolete hidden versions and older duplicate uploads from the B2 bucket.
///
/// SAFETY GUARANTEES:
/// 1. Prior to deleting any version, it queries `b2_list_file_names` to create a strict whitelist
///    of all active, existing files and their active `fileId`s.
/// 2. If an active file is encountered in versions, it is explicitly bypassed and NEVER deleted.
/// 3. If the active file listing is empty or fails, the purge aborts immediately with an error.
/// 4. Verifies the restic anchor file (`config`) is present among active files before proceeding.
/// 5. Deletions are verified against the Backblaze response before being counted as freed.
pub fn purge_b2_hidden_versions(
    repo: &RepoConfig,
    creds: &SessionCredentials,
) -> Result<B2PurgeResult, String> {
    // 1. Ensure lifecycle rules are enabled
    let _ = apply_b2_lifecycle_rules(repo, creds);

    let (key_id, app_key) = resolve_b2_credentials(repo, creds)?;
    let auth_resp = authorize_b2(&key_id, &app_key)?;
    let api_url = auth_resp.get_api_url()?;

    let (bucket_name, prefix) = if let Some(b) = &repo.b2_bucket {
        (b.clone(), repo.b2_prefix.clone())
    } else if let Some((_, b, p)) = parse_b2_target(&repo.target) {
        (b, p)
    } else {
        return Err(format!(
            "Could not parse B2 bucket name from target URL '{}'",
            repo.target
        ));
    };

    let bucket_entry = get_b2_bucket(
        &api_url,
        &auth_resp.authorization_token,
        &auth_resp.account_id,
        &bucket_name,
    )?;
    let prefix_filter = prefix.clone().unwrap_or_default();

    // 2. BUILD ACTIVE FILE WHITELIST
    // Query b2_list_file_names: this endpoint returns ONLY active, visible files.
    let mut active_file_ids: HashSet<String> = HashSet::new();
    let mut active_file_names: HashSet<String> = HashSet::new();
    let mut start_file_name: Option<String> = None;
    let mut list_files_url = format!("{}/b2api/v3/b2_list_file_names", api_url);

    for iterations in 1..=500 {
        let mut req_body = serde_json::json!({
            "bucketId": bucket_entry.bucket_id,
            "maxFileCount": 1000
        });

        if !prefix_filter.is_empty() {
            req_body["prefix"] = serde_json::json!(prefix_filter);
        }
        if let Some(s) = &start_file_name {
            req_body["startFileName"] = serde_json::json!(s);
        }

        let mut files_cmd = Command::new("curl");
        files_cmd
            .arg("-s")
            .arg("-m")
            .arg("25")
            .arg("-H")
            .arg(format!("Authorization: {}", auth_resp.authorization_token))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(req_body.to_string())
            .arg(&list_files_url);

        let files_out = files_cmd
            .output()
            .map_err(|e| format!("Failed to list active files: {}", e))?;
        let mut files_str = String::from_utf8_lossy(&files_out.stdout).to_string();

        if iterations == 1
            && (files_str.contains("\"status\": 404")
                || files_str.contains("\"code\": \"not_found\""))
        {
            list_files_url = format!("{}/b2api/v2/b2_list_file_names", api_url);
            let mut v2_cmd = Command::new("curl");
            v2_cmd
                .arg("-s")
                .arg("-m")
                .arg("25")
                .arg("-H")
                .arg(format!("Authorization: {}", auth_resp.authorization_token))
                .arg("-H")
                .arg("Content-Type: application/json")
                .arg("-d")
                .arg(req_body.to_string())
                .arg(&list_files_url);
            if let Ok(v2_out) = v2_cmd.output() {
                files_str = String::from_utf8_lossy(&v2_out.stdout).to_string();
            }
        }

        if files_str.contains("\"code\":") && !files_str.contains("\"files\":") {
            return Err(format!("B2 list file names error: {}", files_str.trim()));
        }

        let files_resp: B2ListFileNamesResponse =
            serde_json::from_str(&files_str).map_err(|e| {
                format!(
                    "Failed to parse B2 active files: {} (response: {})",
                    e, files_str
                )
            })?;

        for file in files_resp.files {
            if let Some(fid) = file.file_id {
                active_file_ids.insert(fid);
            }
            active_file_names.insert(file.file_name);
        }

        if let Some(next) = files_resp.next_file_name {
            if iterations == 500 {
                return Err("Safety abort: Repository has over 500,000 files; active file inventory truncated. Purge stopped.".to_string());
            }
            start_file_name = Some(next);
        } else {
            break;
        }
    }

    // Safety check 1: Active files must not be empty
    if active_file_ids.is_empty() {
        return Err("Safety abort: Active file scan returned 0 files. Aborting purge to protect repository.".to_string());
    }

    // Safety check 2: Anchor check - verify restic config file is present among active files
    let has_restic_anchor = active_file_names
        .iter()
        .any(|n| n.ends_with("/config") || n == "config");
    if !has_restic_anchor {
        return Err("Safety abort: Restic repository 'config' file not found among active files. Check bucket/prefix configuration.".to_string());
    }

    // 3. SCAN VERSIONS FOR OBSOLETE BLOBS
    // Map of fileId -> (fileName, contentLength)
    let mut obsolete_versions_map: HashMap<String, (String, u64)> = HashMap::new();
    let mut start_version_name: Option<String> = None;
    let mut start_version_id: Option<String> = None;
    let mut list_versions_url = format!("{}/b2api/v3/b2_list_file_versions", api_url);

    for iterations in 1..=500 {
        let mut body = serde_json::json!({
            "bucketId": bucket_entry.bucket_id,
            "maxFileCount": 1000
        });
        if !prefix_filter.is_empty() {
            body["prefix"] = serde_json::json!(prefix_filter);
        }
        if let Some(s) = &start_version_name {
            body["startFileName"] = serde_json::json!(s);
        }
        if let Some(id) = &start_version_id {
            body["startFileId"] = serde_json::json!(id);
        }

        let mut cmd = Command::new("curl");
        cmd.arg("-s")
            .arg("-m")
            .arg("25")
            .arg("-H")
            .arg(format!("Authorization: {}", auth_resp.authorization_token))
            .arg("-H")
            .arg("Content-Type: application/json")
            .arg("-d")
            .arg(body.to_string())
            .arg(&list_versions_url);

        let out = cmd
            .output()
            .map_err(|e| format!("Failed to list file versions: {}", e))?;
        let mut out_str = String::from_utf8_lossy(&out.stdout).to_string();

        if iterations == 1
            && (out_str.contains("\"status\": 404") || out_str.contains("\"code\": \"not_found\""))
        {
            list_versions_url = format!("{}/b2api/v2/b2_list_file_versions", api_url);
            let mut v2_cmd = Command::new("curl");
            v2_cmd
                .arg("-s")
                .arg("-m")
                .arg("25")
                .arg("-H")
                .arg(format!("Authorization: {}", auth_resp.authorization_token))
                .arg("-H")
                .arg("Content-Type: application/json")
                .arg("-d")
                .arg(body.to_string())
                .arg(&list_versions_url);
            if let Ok(v2_out) = v2_cmd.output() {
                out_str = String::from_utf8_lossy(&v2_out.stdout).to_string();
            }
        }

        if out_str.contains("\"code\":") && !out_str.contains("\"files\":") {
            return Err(format!("B2 list file versions error: {}", out_str.trim()));
        }

        let resp: B2ListFileVersionsResponse = serde_json::from_str(&out_str).map_err(|e| {
            format!(
                "Failed to parse file versions: {} (response: {})",
                e, out_str
            )
        })?;

        for f in resp.files {
            // HARD WHITELIST CHECK:
            // If this fileId is in our active files set, it is the CURRENT active file. NEVER DELETE!
            if active_file_ids.contains(&f.file_id) {
                continue;
            }

            // If it is not in active_file_ids, it is either:
            // a) A hide marker (action: "hide", contentLength: 0)
            // b) An obsolete superseded prior version of an active file
            // c) A dead version of an already deleted restic pack file
            obsolete_versions_map.insert(f.file_id.clone(), (f.file_name, f.content_length));
        }

        if let Some(next_name) = resp.next_file_name {
            if iterations == 500 {
                break;
            }
            start_version_name = Some(next_name);
            start_version_id = resp.next_file_id;
        } else {
            break;
        }
    }

    let mut deleted_count: u64 = 0;
    let mut freed_bytes: u64 = 0;

    let obsolete_list: Vec<(String, String, u64)> = obsolete_versions_map
        .into_iter()
        .map(|(fid, (fname, clen))| (fname, fid, clen))
        .collect();

    // 4. Delete obsolete versions in parallel batches of 16 threads
    let chunk_size = 16;
    for chunk in obsolete_list.chunks(chunk_size) {
        let auth_token = auth_resp.authorization_token.clone();
        let api_url_clone = api_url.clone();
        let chunk_vec: Vec<(String, String, u64)> = chunk.to_vec();

        std::thread::scope(|s| {
            let mut handles = Vec::new();
            for (fname, fid, clen) in chunk_vec {
                let token = auth_token.clone();
                let url = format!("{}/b2api/v3/b2_delete_file_version", api_url_clone);
                handles.push(s.spawn(move || {
                    let req = serde_json::json!({
                        "fileName": fname,
                        "fileId": fid
                    });
                    let out = Command::new("curl")
                        .arg("-s")
                        .arg("-m")
                        .arg("15")
                        .arg("-H")
                        .arg(format!("Authorization: {}", token))
                        .arg("-H")
                        .arg("Content-Type: application/json")
                        .arg("-d")
                        .arg(req.to_string())
                        .arg(&url)
                        .output();

                    if let Ok(o) = out {
                        let stdout_str = String::from_utf8_lossy(&o.stdout);
                        // Verified deletion check: must succeed and contain fileId
                        if o.status.success()
                            && stdout_str.contains("\"fileId\"")
                            && !stdout_str.contains("\"code\":")
                        {
                            return Some(clen);
                        }
                    }
                    None
                }));
            }

            for handle in handles {
                if let Ok(Some(clen)) = handle.join() {
                    deleted_count += 1;
                    freed_bytes += clen;
                }
            }
        });
    }

    // Refresh usage after purge
    let updated_usage = query_b2_bucket_usage(repo, creds)?;

    Ok(B2PurgeResult {
        deleted_files_count: deleted_count,
        freed_bytes,
        updated_usage,
    })
}

pub fn save_cached_b2_usage(usage: &B2ProviderUsage) {
    let base_dir = get_persistent_app_dir();
    let file = base_dir.join("b2_provider_usage.json");
    let _ = std::fs::create_dir_all(&base_dir);
    let _ = std::fs::write(
        &file,
        serde_json::to_string_pretty(usage).unwrap_or_default(),
    );
}

pub fn load_cached_b2_usage() -> Option<B2ProviderUsage> {
    let base_dir = get_persistent_app_dir();
    let file = base_dir.join("b2_provider_usage.json");
    if file.is_file()
        && let Ok(content) = std::fs::read_to_string(&file)
        && let Ok(usage) = serde_json::from_str::<B2ProviderUsage>(&content)
    {
        return Some(usage);
    }
    None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_b2_target() {
        let (host, bucket, prefix) =
            parse_b2_target("s3:https://s3.us-east-005.backblazeb2.com/PrismBackups/minecraft")
                .unwrap();
        assert_eq!(host, "s3.us-east-005.backblazeb2.com");
        assert_eq!(bucket, "PrismBackups");
        assert_eq!(prefix, Some("minecraft".to_string()));

        let (host2, bucket2, prefix2) =
            parse_b2_target("s3:https://s3.us-west-004.backblazeb2.com/my-bucket").unwrap();
        assert_eq!(host2, "s3.us-west-004.backblazeb2.com");
        assert_eq!(bucket2, "my-bucket");
        assert_eq!(prefix2, None);

        let (host3, bucket3, prefix3) = parse_b2_target(
            "https://s3.us-east-005.backblazeb2.com/PrismBackups/nested/subfolder/",
        )
        .unwrap();
        assert_eq!(host3, "s3.us-east-005.backblazeb2.com");
        assert_eq!(bucket3, "PrismBackups");
        assert_eq!(prefix3, Some("nested/subfolder".to_string()));
    }

    #[test]
    fn test_parse_b2_v3_auth_response() {
        let json_str = r#"{
            "accountId": "673c7b2a023e",
            "apiInfo": {
                "storageApi": {
                    "absoluteMinimumPartSize": 5000000,
                    "apiUrl": "https://api005.backblazeb2.com",
                    "bucketId": null,
                    "bucketName": null,
                    "downloadUrl": "https://f005.backblazeb2.com",
                    "infoType": "storageApi",
                    "namePrefix": null,
                    "recommendedPartSize": 100000000,
                    "s3ApiUrl": "https://s3.us-east-005.backblazeb2.com"
                }
            },
            "applicationKeyExpirationTimestamp": null,
            "authorizationToken": "4_005673c7b2a023e0000000000_01c8543c_7028a2_acct_7XufW3NqSp1WR3AmRc-l-Ol3g54="
        }"#;

        let auth_resp: B2AuthResponse = serde_json::from_str(json_str).unwrap();
        assert_eq!(auth_resp.account_id, "673c7b2a023e");
        assert_eq!(
            auth_resp.authorization_token,
            "4_005673c7b2a023e0000000000_01c8543c_7028a2_acct_7XufW3NqSp1WR3AmRc-l-Ol3g54="
        );
        assert_eq!(
            auth_resp.get_api_url().unwrap(),
            "https://api005.backblazeb2.com"
        );
    }
}
