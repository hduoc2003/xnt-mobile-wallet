use std::path::{Path, PathBuf};

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use base64::{engine::general_purpose, Engine as _};
use pbkdf2::pbkdf2_hmac_array;
use rand::{rng, RngCore};
use serde_json::json;
use tauri::Manager;
use sha2::Sha256;

const SALT_LEN: usize = 16;
const NONCE_LEN: usize = 12;
const PBKDF2_ITERATIONS: u32 = 100_000;

pub fn get_wallets_dir(app: &tauri::AppHandle) -> PathBuf {
    app.path()
        .app_data_dir()
        .expect("Failed to get app data directory")
        .join("wallets")
}

pub async fn get_keystore_path(app: tauri::AppHandle) -> Result<String, String> {
    let wallets_dir = get_wallets_dir(&app);
    Ok(json!({
        "success": true,
        "path": wallets_dir.to_string_lossy().to_string(),
    })
    .to_string())
}

pub async fn ensure_wallets_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    let wallets_dir = get_wallets_dir(app);
    tokio::fs::create_dir_all(&wallets_dir)
        .await
        .map_err(|e| format!("Failed to create wallets directory: {}", e))?;
    Ok(wallets_dir)
}

pub async fn create_keystore(
    app: tauri::AppHandle,
    seed: String,
    password: String,
) -> Result<String, String> {
    let timestamp = chrono::Utc::now().timestamp_millis();
    let file_name = format!("xnt-wallet-{}.json", timestamp);
    
    let wallets_dir = ensure_wallets_dir(&app).await?;
    let file_path = wallets_dir.join(&file_name);
    
    write_keystore_file(&file_path, &seed, &password).await?;
    
    Ok(json!({
        "success": true,
        "fileName": file_name,
    })
    .to_string())
}

/// Synced to disk: the registry may point at this file right after.
pub async fn write_keystore_file(
    path: &Path,
    mnemonic: &str,
    password: &str,
) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    let (mnemonic, password) = (mnemonic.to_string(), password.to_string());
    let encrypted = tokio::task::spawn_blocking(move || encrypt_secure(&mnemonic, &password))
        .await
        .map_err(|e| format!("Keystore encryption task failed: {e}"))??;
    let keystore = json!({
        "type": "xnt-wallet",
        "version": 1,
        "wallet": {
            "mnemonic": encrypted,
        },
    });
    let text = serde_json::to_string_pretty(&keystore).map_err(|e| e.to_string())?;
    let write_err = |e: std::io::Error| format!("Failed to write keystore: {}", e);
    let mut file = tokio::fs::File::create(path).await.map_err(write_err)?;
    file.write_all(text.as_bytes()).await.map_err(write_err)?;
    file.sync_all().await.map_err(write_err)
}

pub fn mnemonic_from_keystore(content: &str, password: &str) -> Result<String, String> {
    let keystore: serde_json::Value =
        serde_json::from_str(content).map_err(|e| format!("Invalid keystore format: {}", e))?;

    let encrypted = keystore["wallet"]["mnemonic"]
        .as_str()
        .ok_or_else(|| "Invalid keystore format".to_string())?;

    decrypt_secure(encrypted, password).map_err(|_| "Invalid password".to_string())
}

pub async fn save_keystore_file_as(
    app: &tauri::AppHandle,
    file_path: &Path,
    file_name: &str,
) -> Result<String, String> {
    if !file_path.exists() {
        return Ok(json!({
            "success": false,
            "error": "Keystore not found",
        })
        .to_string());
    }
    
    let content = tokio::fs::read_to_string(file_path)
        .await
        .map_err(|e| format!("Failed to read keystore: {}", e))?;
    
    use tauri_plugin_dialog::DialogExt;
    use tauri_plugin_fs::FilePath;
    
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    
    let file_path = window
        .dialog()
        .file()
        .set_file_name(file_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file()
        .ok_or("User cancelled")?;
    
    let path_buf = match file_path {
        FilePath::Path(p) => p,
        FilePath::Url(_url) => {
            return Err("File dialog not supported on this platform".to_string());
        }
    };
    
    tokio::fs::write(&path_buf, content)
        .await
        .map_err(|e| format!("Failed to save keystore: {}", e))?;
    
    Ok(json!({
        "success": true,
        "filePath": path_buf.to_string_lossy().to_string(),
    })
    .to_string())
}

pub async fn check_keystore(
    app: tauri::AppHandle,
    file_name: Option<String>,
) -> Result<String, String> {
    let wallets_dir = ensure_wallets_dir(&app).await?;
    
    let file_name = if let Some(name) = file_name {
        name
    } else {
        newest_keystore_file(&wallets_dir).await?
    };
    
    let file_path = wallets_dir.join(&file_name);
    let exists = file_path.exists();
    
    let min_block_height = if exists {
        if let Ok(content) = tokio::fs::read_to_string(&file_path).await {
            if let Ok(keystore) = serde_json::from_str::<serde_json::Value>(&content) {
                keystore["minBlockHeight"]
                    .as_i64()
                    .unwrap_or(0)
            } else {
                0
            }
        } else {
            0
        }
    } else {
        0
    };
    
    Ok(json!({
        "success": true,
        "exists": exists,
        "fileName": file_name,
        "minBlockHeight": min_block_height,
    })
    .to_string())
}

/// The keystore the app opens when none is named: the most recently modified `.json` in `wallets/`.
pub async fn newest_keystore_file(wallets_dir: &Path) -> Result<String, String> {
    let mut entries = tokio::fs::read_dir(wallets_dir)
        .await
        .map_err(|e| format!("Failed to read wallets directory: {}", e))?;

    let mut newest_file: Option<String> = None;
    let mut newest_time = 0u64;

    while let Some(entry) = entries
        .next_entry()
        .await
        .map_err(|e| format!("Failed to read directory entry: {}", e))?
    {
        let path = entry.path();
        if path.extension().and_then(|s| s.to_str()) == Some("json") {
            if let Ok(metadata) = entry.metadata().await {
                if let Ok(modified) = metadata.modified() {
                    let time = modified
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_err(|e| format!("{} is dated before 1970: {e}", path.display()))?
                        .as_secs();
                    if time > newest_time {
                        newest_time = time;
                        newest_file = path
                            .file_name()
                            .and_then(|n| n.to_str())
                            .map(|s| s.to_string());
                    }
                }
            }
        }
    }

    newest_file.ok_or_else(|| "No keystore found".to_string())
}

pub async fn get_min_block_height(
    app: tauri::AppHandle,
    file_name: String,
) -> Result<String, String> {
    let wallets_dir = ensure_wallets_dir(&app).await?;
    let file_path = wallets_dir.join(&file_name);
    
    if !file_path.exists() {
        return Ok(json!({
            "success": false,
            "error": "Keystore not found",
            "minBlockHeight": 0,
        })
        .to_string());
    }
    
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read keystore: {}", e))?;
    
    let keystore: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid keystore format: {}", e))?;
    
    let min_block_height = keystore["minBlockHeight"]
        .as_i64()
        .unwrap_or(0);
    
    Ok(json!({
        "success": true,
        "minBlockHeight": min_block_height,
    })
    .to_string())
}

pub async fn save_min_block_height(
    app: tauri::AppHandle,
    file_name: String,
    block_height: i64,
) -> Result<String, String> {
    update_min_block_height(app, file_name, Some(block_height)).await
}

pub async fn update_min_block_height(
    app: tauri::AppHandle,
    file_name: String,
    block_height: Option<i64>,
) -> Result<String, String> {
    let wallets_dir = ensure_wallets_dir(&app).await?;
    let file_path = wallets_dir.join(&file_name);
    
    if !file_path.exists() {
        return Ok(json!({
            "success": false,
            "error": "Keystore not found",
        })
        .to_string());
    }
    
    let content = tokio::fs::read_to_string(&file_path)
        .await
        .map_err(|e| format!("Failed to read keystore: {}", e))?;
    
    let mut keystore: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid keystore format: {}", e))?;
    
    if let Some(height) = block_height {
        keystore["minBlockHeight"] = json!(height);
    } else {
        keystore["minBlockHeight"] = json!(null);
    }
    
    tokio::fs::write(&file_path, serde_json::to_string_pretty(&keystore).unwrap())
        .await
        .map_err(|e| format!("Failed to update keystore: {}", e))?;
    
    Ok(json!({
        "success": true,
    })
    .to_string())
}

pub async fn keystore_exists(
    app: tauri::AppHandle,
    file_name: String,
) -> Result<String, String> {
    let wallets_dir = ensure_wallets_dir(&app).await?;
    let file_path = wallets_dir.join(&file_name);
    let exists = file_path.exists();
    
    Ok(json!({
        "success": true,
        "exists": exists,
    })
    .to_string())
}

pub async fn save_keystore_dialog(
    app: tauri::AppHandle,
    keystore_data: String,
    default_file_name: String,
) -> Result<String, String> {
    let wallets_dir = ensure_wallets_dir(&app).await?;
    let file_path = wallets_dir.join(&default_file_name);
    
    tokio::fs::write(&file_path, keystore_data)
        .await
        .map_err(|e| format!("Failed to save keystore: {}", e))?;
    
    Ok(json!({
        "success": true,
        "filePath": file_path.to_string_lossy().to_string(),
    })
    .to_string())
}

/// Derive a 256-bit key from the password using PBKDF2-HMAC-SHA256.
fn derive_key(password: &str, salt: &[u8]) -> [u8; 32] {
    pbkdf2_hmac_array::<Sha256, 32>(password.as_bytes(), salt, PBKDF2_ITERATIONS)
}

/// Encrypt data with AES-256-GCM using a password-derived key.
/// Output is base64(salt || nonce || ciphertext).
fn encrypt_secure(data: &str, password: &str) -> Result<String, String> {
    let mut salt = [0u8; SALT_LEN];
    let mut nonce_bytes = [0u8; NONCE_LEN];
    let mut rng = rng();
    rng.fill_bytes(&mut salt);
    rng.fill_bytes(&mut nonce_bytes);

    let key = derive_key(password, &salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(&nonce_bytes);

    let ciphertext = cipher
        .encrypt(nonce, data.as_bytes())
        .map_err(|e| format!("Encryption failed: {}", e))?;

    let mut payload = Vec::with_capacity(SALT_LEN + NONCE_LEN + ciphertext.len());
    payload.extend_from_slice(&salt);
    payload.extend_from_slice(&nonce_bytes);
    payload.extend_from_slice(&ciphertext);

    Ok(general_purpose::STANDARD.encode(payload))
}

fn decrypt_secure(encrypted: &str, password: &str) -> Result<String, String> {
    let decoded = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|_| "Invalid encrypted data".to_string())?;

    if decoded.len() < SALT_LEN + NONCE_LEN {
        return Err("Invalid encrypted data".to_string());
    }

    let (salt, rest) = decoded.split_at(SALT_LEN);
    let (nonce_bytes, ciphertext) = rest.split_at(NONCE_LEN);

    let key = derive_key(password, salt);
    let cipher = Aes256Gcm::new_from_slice(&key).map_err(|e| e.to_string())?;
    let nonce = Nonce::from_slice(nonce_bytes);

    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| "Invalid password or corrupted data".to_string())?;

    String::from_utf8(plaintext).map_err(|_| "Invalid UTF-8".to_string())
}

