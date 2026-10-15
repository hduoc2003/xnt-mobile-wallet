use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.json";
pub const DEFAULT_RPC_URL: &str = env!("XNT_DEFAULT_RPC_URL");

#[derive(Serialize, Deserialize, Debug, Default)]
struct Config {
    #[serde(default)]
    rpc_url: String,
}

fn get_config_path(app: &AppHandle) -> PathBuf {
    let app_data_dir = app
        .path()
        .app_data_dir()
        .expect("Failed to get app data directory");
    
    if !app_data_dir.exists() {
        fs::create_dir_all(&app_data_dir).expect("Failed to create app data directory");
    }
    
    app_data_dir.join(CONFIG_FILE)
}

fn load_config(app: &AppHandle) -> Config {
    let config_path = get_config_path(app);
    
    if config_path.exists() {
        match fs::read_to_string(&config_path) {
            Ok(content) => {
                match serde_json::from_str::<Config>(&content) {
                    Ok(config) => return config,
                    Err(e) => {
                        log::warn!("Failed to parse config file: {}", e);
                    }
                }
            }
            Err(e) => {
                log::warn!("Failed to read config file: {}", e);
            }
        }
    }
    
    Config::default()
}

fn save_config(app: &AppHandle, config: &Config) -> Result<(), String> {
    let config_path = get_config_path(app);
    let json = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;
    
    fs::write(&config_path, json)
        .map_err(|e| format!("Failed to write config file: {}", e))?;
    
    Ok(())
}

pub fn get_custom_rpc_endpoint(app: &AppHandle) -> Option<String> {
    let config = load_config(app);
    let rpc_url = config.rpc_url.trim().to_string();
    
    if !rpc_url.is_empty() {
        Some(rpc_url)
    } else {
        None
    }
}

pub fn set_rpc_endpoint(app: &AppHandle, endpoint: String) -> Result<(), String> {
    let endpoint = endpoint.trim().to_string();
    let is_empty = endpoint.is_empty();
    
    if !is_empty {
        if !endpoint.starts_with("http://") && !endpoint.starts_with("https://") {
            return Err("Invalid endpoint format. Must start with http:// or https://".to_string());
        }
    }
    
    let mut config = load_config(app);
    let endpoint_clone = endpoint.clone();
    config.rpc_url = endpoint_clone;
    
    save_config(app, &config)?;
    
    Ok(())
}

/// Custom endpoint from config, else the build's default.
pub async fn resolve_rpc_url(app: &AppHandle) -> Result<String, String> {
    let app = app.clone();
    let custom = tokio::task::spawn_blocking(move || get_custom_rpc_endpoint(&app))
        .await
        .map_err(|e| format!("Config read task failed: {e}"))?;
    Ok(custom.unwrap_or_else(|| DEFAULT_RPC_URL.to_string()))
}

/// Origin (scheme + host + port) of the RPC URL; the edge serves the relayer routes there.
pub fn edge_base_url(rpc_url: &str) -> Result<String, String> {
    let url = reqwest::Url::parse(rpc_url).map_err(|e| format!("Invalid RPC URL {rpc_url}: {e}"))?;
    Ok(url.origin().ascii_serialization())
}

#[cfg(test)]
mod tests {
    use super::edge_base_url;

    #[test]
    fn edge_base_url_strips_path() {
        assert_eq!(
            edge_base_url("https://leviathandev.neptune.io/xnt-rpc").unwrap(),
            "https://leviathandev.neptune.io"
        );
        assert_eq!(edge_base_url("http://127.0.0.1:9897").unwrap(), "http://127.0.0.1:9897");
        assert!(edge_base_url("not a url").is_err());
    }
}
