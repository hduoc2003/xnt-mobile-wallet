use tauri::Manager;
use std::path::PathBuf;
use serde_json::json;

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
    //TODO: try to research encryption like AES-256-GCM
    let encrypted = encrypt_simple(&seed, &password);
    let timestamp = chrono::Utc::now().timestamp_millis();
    let file_name = format!("xnt-wallet-{}.json", timestamp);
    
    let wallets_dir = ensure_wallets_dir(&app).await?;
    let file_path = wallets_dir.join(&file_name);
    
    let keystore = json!({
        "type": "xnt-wallet",
        "version": 1,
        "wallet": {
            "mnemonic": encrypted,
        },
    });
    
    tokio::fs::write(&file_path, serde_json::to_string_pretty(&keystore).unwrap())
        .await
        .map_err(|e| format!("Failed to write keystore: {}", e))?;
    
    Ok(json!({
        "success": true,
        "fileName": file_name,
    })
    .to_string())
}

pub async fn decrypt_keystore(
    app: tauri::AppHandle,
    file_name: String,
    password: String,
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
    
    let keystore: serde_json::Value = serde_json::from_str(&content)
        .map_err(|e| format!("Invalid keystore format: {}", e))?;
    
    let encrypted = keystore["wallet"]["mnemonic"]
        .as_str()
        .ok_or_else(|| "Invalid keystore format".to_string())?;
    
    let seed = decrypt_simple(encrypted, &password)
        .map_err(|_| "Invalid password".to_string())?;
    
    Ok(json!({
        "success": true,
        "phrase": seed,
    })
    .to_string())
}

pub async fn save_keystore_as(
    app: tauri::AppHandle,
    file_name: String,
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
    
    use tauri_plugin_dialog::DialogExt;
    use tauri_plugin_fs::FilePath;
    
    let window = app.get_webview_window("main").ok_or("Window not found")?;
    
    let file_path = window
        .dialog()
        .file()
        .set_file_name(&file_name)
        .add_filter("JSON", &["json"])
        .blocking_save_file()
        .ok_or("User cancelled")?;
    
    let path_buf = match file_path {
        FilePath::Path(p) => p,
        FilePath::Url(url) => {
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
        // Find newest keystore file
        let mut entries = tokio::fs::read_dir(&wallets_dir)
            .await
            .map_err(|e| format!("Failed to read wallets directory: {}", e))?;
        
        let mut newest_file: Option<String> = None;
        let mut newest_time = 0u64;
        
        while let Some(entry) = entries.next_entry().await.map_err(|e| format!("Failed to read directory entry: {}", e))? {
            let path = entry.path();
            if path.extension().and_then(|s| s.to_str()) == Some("json") {
                if let Ok(metadata) = entry.metadata().await {
                    if let Ok(modified) = metadata.modified() {
                        let time = modified.duration_since(std::time::UNIX_EPOCH)
                            .unwrap()
                            .as_secs();
                        if time > newest_time {
                            newest_time = time;
                            newest_file = path.file_name()
                                .and_then(|n| n.to_str())
                                .map(|s| s.to_string());
                        }
                    }
                }
            }
        }
        
        newest_file.ok_or_else(|| "No keystore found".to_string())?
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

// Simple encryption/decryption (NOT production-ready, use proper crypto in production)
fn encrypt_simple(data: &str, password: &str) -> String {
    use base64::{Engine as _, engine::general_purpose};
    let mut result = Vec::new();
    let password_bytes = password.as_bytes();
    for (i, byte) in data.as_bytes().iter().enumerate() {
        result.push(byte ^ password_bytes[i % password_bytes.len()]);
    }
    general_purpose::STANDARD.encode(&result)
}

fn decrypt_simple(encrypted: &str, password: &str) -> Result<String, String> {
    use base64::{Engine as _, engine::general_purpose};
    let decoded = general_purpose::STANDARD
        .decode(encrypted)
        .map_err(|_| "Invalid encrypted data")?;
    let mut result = Vec::new();
    let password_bytes = password.as_bytes();
    for (i, byte) in decoded.iter().enumerate() {
        result.push(byte ^ password_bytes[i % password_bytes.len()]);
    }
    String::from_utf8(result).map_err(|_| "Invalid UTF-8".to_string())
}

