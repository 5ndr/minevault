use std::fs;
use std::path::PathBuf;
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::{Aes256Gcm, Nonce};
use pbkdf2::pbkdf2_hmac;
use serde::{Deserialize, Serialize};
use sha2::Sha256;
use crate::system::get_persistent_app_dir;

const VAULT_FILE_NAME: &str = "minevault_keys.enc";
const PBKDF2_ROUNDS_PASSPHRASE: u32 = 100_000;
const PBKDF2_ROUNDS_DEVICE: u32 = 50_000;
const DEFAULT_SALT_SEED: &[u8] = b"MineVault_USB_Storage_Seed_2026_Secure";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EncryptedVaultData {
    pub b2_key_id: Option<String>,
    pub b2_application_key: Option<String>,
    pub b2_management_key_id: Option<String>,
    pub b2_management_app_key: Option<String>,
    pub repo_password: Option<String>,
    pub updated_at: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VaultEnvelope {
    pub version: u32,
    pub kdf: String,
    pub cipher: String,
    pub salt: String,
    pub nonce: String,
    pub ciphertext: String,
    pub has_passphrase: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct UsbVaultStatus {
    pub exists: bool,
    pub is_passphrase_protected: bool,
    pub vault_path: String,
}

/// Locates the USB root directory or current executable directory for storing USB vault
pub fn get_usb_vault_path() -> PathBuf {
    // 1. Check parent directories of current executable
    if let Ok(exe) = std::env::current_exe() {
        if let Some(parent) = exe.parent() {
            let mut check_dir = parent.to_path_buf();
            for _ in 0..4 {
                let cand = check_dir.join(VAULT_FILE_NAME);
                if cand.is_file() {
                    return cand;
                }
                let usb_cand = check_dir.join("usb_dist").join(VAULT_FILE_NAME);
                if usb_cand.is_file() {
                    return usb_cand;
                }
                if let Some(p) = check_dir.parent() {
                    check_dir = p.to_path_buf();
                } else {
                    break;
                }
            }
        }
    }

    // 2. Check current working directory and its parent
    if let Ok(cwd) = std::env::current_dir() {
        let cand = cwd.join(VAULT_FILE_NAME);
        if cand.is_file() {
            return cand;
        }
        let usb_cand = cwd.join("usb_dist").join(VAULT_FILE_NAME);
        if usb_cand.is_file() {
            return usb_cand;
        }
        if let Some(parent) = cwd.parent() {
            let p_cand = parent.join(VAULT_FILE_NAME);
            if p_cand.is_file() {
                return p_cand;
            }
            let p_usb_cand = parent.join("usb_dist").join(VAULT_FILE_NAME);
            if p_usb_cand.is_file() {
                return p_usb_cand;
            }
        }
    }

    // 3. Check persistent app dir
    let persistent = get_persistent_app_dir().join(VAULT_FILE_NAME);
    if persistent.is_file() {
        return persistent;
    }

    // Default target for saving: CWD or persistent dir
    if let Ok(cwd) = std::env::current_dir() {
        return cwd.join(VAULT_FILE_NAME);
    }
    persistent
}

fn derive_key(passphrase: Option<&str>, salt: &[u8]) -> [u8; 32] {
    let mut key = [0u8; 32];
    if let Some(pass) = passphrase {
        let pass_trimmed = pass.trim();
        if !pass_trimmed.is_empty() {
            pbkdf2_hmac::<Sha256>(
                pass_trimmed.as_bytes(),
                salt,
                PBKDF2_ROUNDS_PASSPHRASE,
                &mut key,
            );
            return key;
        }
    }

    // Device / Default encryption seed
    let seed = [DEFAULT_SALT_SEED, b":MineVaultDefaultKey"].concat();
    pbkdf2_hmac::<Sha256>(&seed, salt, PBKDF2_ROUNDS_DEVICE, &mut key);
    key
}

/// Encrypts and saves application keys and credentials to the USB drive
pub fn save_usb_vault(
    data: &EncryptedVaultData,
    passphrase: Option<&str>,
) -> Result<String, String> {
    let vault_path = get_usb_vault_path();
    if let Some(parent) = vault_path.parent() {
        let _ = fs::create_dir_all(parent);
    }

    // Generate random 16-byte salt and 12-byte nonce
    let mut salt = [0u8; 16];
    let mut nonce_bytes = [0u8; 12];
    getrandom::getrandom(&mut salt).map_err(|e| format!("Random generator error: {}", e))?;
    getrandom::getrandom(&mut nonce_bytes).map_err(|e| format!("Random generator error: {}", e))?;

    let has_passphrase = passphrase.map(|p| !p.trim().is_empty()).unwrap_or(false);
    let key = derive_key(passphrase, &salt);

    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Cipher initialization error: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let plaintext = serde_json::to_vec(data)
        .map_err(|e| format!("Serialization error: {}", e))?;

    let ciphertext = cipher
        .encrypt(nonce, plaintext.as_ref())
        .map_err(|e| format!("Encryption error: {}", e))?;

    let envelope = VaultEnvelope {
        version: 1,
        kdf: "PBKDF2-HMAC-SHA256".to_string(),
        cipher: "AES-256-GCM".to_string(),
        salt: hex::encode(salt),
        nonce: hex::encode(nonce_bytes),
        ciphertext: hex::encode(ciphertext),
        has_passphrase,
    };

    let envelope_json = serde_json::to_string_pretty(&envelope)
        .map_err(|e| format!("Envelope formatting error: {}", e))?;

    fs::write(&vault_path, envelope_json)
        .map_err(|e| format!("Failed to write encrypted vault to '{}': {}", vault_path.display(), e))?;

    Ok(vault_path.to_string_lossy().to_string())
}

/// Unlocks and decrypts the application keys from the USB drive
pub fn load_usb_vault(passphrase: Option<&str>) -> Result<EncryptedVaultData, String> {
    let vault_path = get_usb_vault_path();
    if !vault_path.is_file() {
        return Err(format!("USB vault file not found at '{}'", vault_path.display()));
    }

    let content = fs::read_to_string(&vault_path)
        .map_err(|e| format!("Failed to read vault file: {}", e))?;

    let envelope: VaultEnvelope = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse vault envelope: {}", e))?;

    let salt = hex::decode(&envelope.salt)
        .map_err(|e| format!("Invalid salt encoding: {}", e))?;
    let nonce_bytes = hex::decode(&envelope.nonce)
        .map_err(|e| format!("Invalid nonce encoding: {}", e))?;
    let ciphertext = hex::decode(&envelope.ciphertext)
        .map_err(|e| format!("Invalid ciphertext encoding: {}", e))?;

    let key = derive_key(passphrase, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key)
        .map_err(|e| format!("Cipher initialization error: {}", e))?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let decrypted_bytes = cipher
        .decrypt(nonce, ciphertext.as_ref())
        .map_err(|_| "Failed to decrypt USB vault. Invalid passphrase or corrupted file.".to_string())?;

    let data: EncryptedVaultData = serde_json::from_slice(&decrypted_bytes)
        .map_err(|e| format!("Failed to deserialize decrypted vault credentials: {}", e))?;

    Ok(data)
}

/// Returns the status of the USB vault
pub fn check_usb_vault_status() -> UsbVaultStatus {
    let vault_path = get_usb_vault_path();
    let exists = vault_path.is_file();
    let mut is_passphrase_protected = false;

    if exists {
        if let Ok(content) = fs::read_to_string(&vault_path) {
            if let Ok(envelope) = serde_json::from_str::<VaultEnvelope>(&content) {
                is_passphrase_protected = envelope.has_passphrase;
            }
        }
    }

    UsbVaultStatus {
        exists,
        is_passphrase_protected,
        vault_path: vault_path.to_string_lossy().to_string(),
    }
}

/// Removes the encrypted USB vault file
pub fn clear_usb_vault() -> Result<(), String> {
    let vault_path = get_usb_vault_path();
    if vault_path.is_file() {
        fs::remove_file(&vault_path).map_err(|e| e.to_string())?;
    }
    Ok(())
}

// Simple hex encoding/decoding helper
mod hex {
    pub fn encode(data: impl AsRef<[u8]>) -> String {
        data.as_ref().iter().map(|b| format!("{:02x}", b)).collect()
    }

    pub fn decode(s: &str) -> Result<Vec<u8>, String> {
        if s.len() % 2 != 0 {
            return Err("Invalid hex length".to_string());
        }
        (0..s.len())
            .step_by(2)
            .map(|i| u8::from_str_radix(&s[i..i + 2], 16).map_err(|e| e.to_string()))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vault_encryption_decryption_with_passphrase() {
        let temp = tempfile::tempdir().unwrap();
        let file = temp.path().join("test_vault.enc");

        let original = EncryptedVaultData {
            b2_key_id: Some("005testKeyId".to_string()),
            b2_application_key: Some("K005secretAppKey".to_string()),
            b2_management_key_id: None,
            b2_management_app_key: None,
            repo_password: Some("myRepoPass".to_string()),
            updated_at: chrono::Utc::now().to_rfc3339(),
        };

        // Encrypt with passphrase "SuperSecret123"
        let mut salt = [0u8; 16];
        let mut nonce = [0u8; 12];
        getrandom::getrandom(&mut salt).unwrap();
        getrandom::getrandom(&mut nonce).unwrap();

        let key = derive_key(Some("SuperSecret123"), &salt);
        let cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let plaintext = serde_json::to_vec(&original).unwrap();
        let ct = cipher.encrypt(Nonce::from_slice(&nonce), plaintext.as_ref()).unwrap();

        let env = VaultEnvelope {
            version: 1,
            kdf: "PBKDF2".to_string(),
            cipher: "AES-256-GCM".to_string(),
            salt: hex::encode(salt),
            nonce: hex::encode(nonce),
            ciphertext: hex::encode(ct),
            has_passphrase: true,
        };
        fs::write(&file, serde_json::to_string(&env).unwrap()).unwrap();

        // Wrong passphrase must fail
        let wrong_key = derive_key(Some("WrongPass"), &salt);
        let wrong_cipher = Aes256Gcm::new_from_slice(&wrong_key).unwrap();
        assert!(wrong_cipher.decrypt(Nonce::from_slice(&nonce), hex::decode(&env.ciphertext).unwrap().as_ref()).is_err());

        // Correct passphrase must succeed and match original
        let correct_cipher = Aes256Gcm::new_from_slice(&key).unwrap();
        let dec = correct_cipher.decrypt(Nonce::from_slice(&nonce), hex::decode(&env.ciphertext).unwrap().as_ref()).unwrap();
        let loaded: EncryptedVaultData = serde_json::from_slice(&dec).unwrap();
        assert_eq!(loaded.b2_application_key, original.b2_application_key);
        assert_eq!(loaded.b2_key_id, original.b2_key_id);
    }
}
