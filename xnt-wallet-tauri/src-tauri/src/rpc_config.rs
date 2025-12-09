use std::fs;
use std::path::PathBuf;
use tauri::{AppHandle, Manager};
use serde::{Deserialize, Serialize};

const CONFIG_FILE: &str = "config.json";
pub const DEFAULT_RPC_URL: &str = "http://zsmartex.com:9897";

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

