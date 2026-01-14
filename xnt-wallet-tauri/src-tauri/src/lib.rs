#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)]
use neptune_privacy::application::config::network::Network;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::wallet_entropy::WalletEntropy;
use neptune_privacy::state::wallet::secret_key_material::SecretKeyMaterial;
use rand::Rng;
use std::str::FromStr;
use log::Level;
use reqwest::Client;
use reqwest::header::{HeaderMap, USER_AGENT};
use tauri::Manager;

#[cfg(target_os = "android")]
use android_logger;
#[cfg(target_os = "android")]
use libc;

mod keystore;
mod transaction;
mod utxo_scanning;
mod utxo_cache;
mod rpc_client;

mod rpc_config;

#[tauri::command]
fn generate_seed() -> Result<String, String> {
    let mut rng = rand::rng();
    let entropy: [u8; 24] = rng.random();
    
    let mnemonic = bip39::Mnemonic::from_entropy(&entropy, bip39::Language::English)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;
    
    let seed_phrase: Vec<String> = mnemonic
        .phrase()
        .split(' ')
        .map(|s| s.to_string())
        .collect();
    
    let secret_key_material = SecretKeyMaterial::from_phrase(&seed_phrase)
        .map_err(|e| format!("Failed to create secret key from phrase: {}", e))?;
    let wallet_entropy = WalletEntropy::from(secret_key_material);
    
    let spending_key = wallet_entropy.nth_generation_spending_key(0);
    let receiver_id = spending_key.receiver_identifier();
    
    Ok(serde_json::json!({
        "success": true,
        "seed_phrase": seed_phrase,
        "receiver_identifier": format!("0x{:016x}", receiver_id.value()),
    })
    .to_string())
}

#[tauri::command]
fn generate_keys_from_seed(
    seed_phrase: Vec<String>,
    network: Option<String>,
) -> Result<String, String> {
    let wallet_entropy = WalletEntropy::from_phrase(&seed_phrase)
        .map_err(|e| format!("Invalid seed phrase: {}", e))?;

    let spending_key = wallet_entropy.nth_generation_spending_key(0);
    let spending_key_enum = SpendingKey::Generation(spending_key.clone());
    let receiver_id = spending_key.receiver_identifier();
    let receiving_address = spending_key_enum.to_address();
    
    let network_str = network.as_deref().unwrap_or("main");
    let network_str_normalized = match network_str {
        "mainnet" => "main",
        "testnet" => "testnet",
        other => other,
    };
    let network_enum = Network::from_str(network_str_normalized)
        .map_err(|e| format!("Invalid network '{}': {}", network_str, e))?;
    
    let address_string = receiving_address
        .to_display_bech32m(network_enum)
        .map_err(|e| format!("Failed to encode address: {}", e))?;

    let spending_key_bytes = bincode::serialize(&spending_key_enum)
        .map_err(|e| format!("Failed to serialize spending key: {}", e))?;

    Ok(serde_json::json!({
        "success": true,
        "spending_key_hex": format!("0x{}", hex::encode(&spending_key_bytes)),
        "receiver_identifier": format!("0x{:016x}", receiver_id.value()),
        "address": address_string,
    })
    .to_string())
}

#[tauri::command]
async fn create_keystore(
    app: tauri::AppHandle,
    seed: String,
    password: String,
) -> Result<String, String> {
    keystore::create_keystore(app, seed, password).await
}

#[tauri::command]
async fn decrypt_keystore(
    app: tauri::AppHandle,
    fileName: String,
    password: String,
) -> Result<String, String> {
    keystore::decrypt_keystore(app, fileName, password).await
}

#[tauri::command]
async fn save_keystore_as(
    app: tauri::AppHandle,
    fileName: String,
) -> Result<String, String> {
    keystore::save_keystore_as(app, fileName).await
}

#[tauri::command]
async fn check_keystore(
    app: tauri::AppHandle,
    fileName: Option<String>,
) -> Result<String, String> {
    keystore::check_keystore(app, fileName).await
}

#[tauri::command]
async fn get_min_block_height(
    app: tauri::AppHandle,
    fileName: String,
) -> Result<String, String> {
    keystore::get_min_block_height(app, fileName).await
}

#[tauri::command]
async fn save_min_block_height(
    app: tauri::AppHandle,
    fileName: String,
    block_height: i64,
) -> Result<String, String> {
    keystore::save_min_block_height(app, fileName, block_height).await
}

#[tauri::command]
async fn update_min_block_height(
    app: tauri::AppHandle,
    fileName: String,
    block_height: Option<i64>,
) -> Result<String, String> {
    keystore::update_min_block_height(app, fileName, block_height).await
}

#[tauri::command]
async fn keystore_exists(
    app: tauri::AppHandle,
    fileName: String,
) -> Result<String, String> {
    keystore::keystore_exists(app, fileName).await
}

#[tauri::command]
async fn save_keystore_dialog(
    app: tauri::AppHandle,
    keystore_data: String,
    default_file_name: String,
) -> Result<String, String> {
    keystore::save_keystore_dialog(app, keystore_data, default_file_name).await
}

#[tauri::command]
async fn get_keystore_path(app: tauri::AppHandle) -> Result<String, String> {
    keystore::get_keystore_path(app).await
}

#[tauri::command]
async fn build_transaction(app: tauri::AppHandle, payload: serde_json::Value) -> Result<String, String> {
    transaction::build_transaction(app, payload).await
}

#[tauri::command]
async fn scan_utxos_client_side(app: tauri::AppHandle, request: serde_json::Value) -> Result<String, String> {
    let scan_request: utxo_scanning::ScanUtxosRequest = serde_json::from_value(request)
        .map_err(|e| format!("Invalid request: {}", e))?;
    
    utxo_scanning::scan_utxos_client_side(&app, scan_request).await
}

#[tauri::command]
async fn get_cached_utxos(app: tauri::AppHandle) -> Result<String, String> {
    utxo_scanning::get_cached_utxos(&app).await
}

#[tauri::command]
async fn get_cached_utxos_local() -> Result<String, String> {
    utxo_scanning::get_cached_utxos_local().await
}

#[tauri::command]
async fn get_pending_balance(app: tauri::AppHandle, request: serde_json::Value) -> Result<String, String> {
    let pending_request: utxo_scanning::GetPendingBalanceRequest = serde_json::from_value(request)
        .map_err(|e| format!("Invalid request: {}", e))?;

    utxo_scanning::get_pending_balance_from_mempool(&app, pending_request).await
}

#[tauri::command]
async fn clear_utxo_cache() -> Result<(), String> {
    utxo_cache::clear_cache().await;
    Ok(())
}

#[tauri::command]
async fn proxy_json_rpc(
    app: tauri::AppHandle,
    method: String,
    params: serde_json::Value,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    
    let rpc_url = rpc_config::get_custom_rpc_endpoint(&app)
        .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string());

    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });

    let response = client
        .post(&rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Request error: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if !status.is_success() {
        return Err(format!("Upstream error {status}: {text}"));
    }

    Ok(text)
}

#[tauri::command]
async fn proxy_http_get(url: String) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;

    let mut headers = HeaderMap::new();
    headers.insert(USER_AGENT, "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/126.0.0.0 Safari/537.36".parse().unwrap());

    let response = client
        .get(&url)
        .headers(headers)
        .send()
        .await
        .map_err(|e| format!("Request error: {e}"))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read response body: {e}"))?;

    if !status.is_success() {
        return Err(format!("Upstream error {status}: {text}"));
    }

    Ok(text)
}

#[tauri::command]
async fn set_rpc_endpoint(
    app: tauri::AppHandle,
    endpoint: String,
) -> Result<(), String> {
    rpc_config::set_rpc_endpoint(&app, endpoint)
}

#[tauri::command]
async fn get_custom_rpc_endpoint(app: tauri::AppHandle) -> Result<Option<String>, String> {
    Ok(rpc_config::get_custom_rpc_endpoint(&app))
}

#[tauri::command]
fn log_message(level: String, message: String) -> Result<(), String> {
    let lvl = match level.to_lowercase().as_str() {
        "trace" => Level::Trace,
        "debug" => Level::Debug,
        "info" => Level::Info,
        "warn" => Level::Warn,
        "error" => Level::Error,
        _ => Level::Info,
    };

    log::log!(lvl, "{message}");
    Ok(())
}

#[cfg(target_os = "android")]
#[no_mangle]
pub extern "C" fn __gxx_personality_v0(
    _version: libc::c_int,
    _actions: libc::c_int,
    _exception_class: u64,
    _exception_object: *mut libc::c_void,
    _context: *mut libc::c_void,
) -> libc::c_int {
    unsafe {
        libc::abort();
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    #[cfg(target_os = "android")]
    {
        android_logger::init_once(
            android_logger::Config::default()
                .with_max_level(log::LevelFilter::Debug)
                .with_tag("XNT Wallet"),
        );
        log::info!("[XNT Wallet] Logger initialized for Android");
    }
    
    #[cfg(not(target_os = "android"))]
    {
        env_logger::Builder::from_default_env()
            .filter_level(log::LevelFilter::Info)
            .filter_module("xnt_wallet_tauri", log::LevelFilter::Debug)
            .filter_module("neptune_privacy", log::LevelFilter::Warn)
            .init();
        log::info!("[XNT Wallet] Logger initialized for desktop");
    }

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            utxo_cache::init_cache_path(app.handle());
            tauri::async_runtime::spawn(async {
                utxo_cache::reload_cache_from_disk().await;
            });
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            generate_seed,
            generate_keys_from_seed,
            create_keystore,
            decrypt_keystore,
            save_keystore_as,
            check_keystore,
            get_min_block_height,
            save_min_block_height,
            update_min_block_height,
            keystore_exists,
            save_keystore_dialog,
            get_keystore_path,
            build_transaction,
            scan_utxos_client_side,
            get_cached_utxos,
            get_cached_utxos_local,
            clear_utxo_cache,
            get_pending_balance,
            proxy_json_rpc,
            proxy_http_get,
            set_rpc_endpoint,
            get_custom_rpc_endpoint,
            log_message,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
