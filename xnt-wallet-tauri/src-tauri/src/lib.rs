#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)]
use log::Level;

#[cfg(target_os = "android")]
use android_logger;
#[cfg(target_os = "android")]
use libc;

mod bridge_deposit;
mod bridge_payout;
mod bridge_status;
mod keys;
mod keystore;
mod proxy;
mod rpc_client;
mod rpc_config;
mod transaction;
mod tx_status;
mod utxo_cache;
mod utxo_scanning;

#[tauri::command]
fn generate_seed() -> Result<String, String> {
    keys::generate_seed()
}

#[tauri::command]
fn generate_keys_from_seed(
    seed_phrase: Vec<String>,
    network: Option<String>,
    key_type: Option<String>,
) -> Result<String, String> {
    keys::generate_keys_from_seed(seed_phrase, network, key_type)
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
    proxy::proxy_json_rpc(app, method, params).await
}

#[tauri::command]
async fn proxy_http_get(url: String) -> Result<String, String> {
    proxy::proxy_http_get(url).await
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
async fn build_bridge_deposit(app: tauri::AppHandle, payload: serde_json::Value) -> Result<String, String> {
    bridge_deposit::build_bridge_deposit(app, payload).await
}

#[tauri::command]
async fn list_bridge_deposits(app: tauri::AppHandle) -> Result<Vec<bridge_status::DepositRecord>, String> {
    bridge_status::list(&app).await
}

#[tauri::command]
async fn discard_bridge_deposit(
    app: tauri::AppHandle,
    commitment: String,
) -> Result<Vec<bridge_status::DepositRecord>, String> {
    bridge_status::discard(&app, &commitment).await
}

#[tauri::command]
async fn refresh_bridge_deposits(app: tauri::AppHandle) -> Result<Vec<bridge_status::DepositRecord>, String> {
    bridge_status::refresh(&app).await
}

#[tauri::command]
async fn tx_statuses(
    app: tauri::AppHandle,
    commitments: Vec<String>,
) -> Result<Vec<tx_status::TxStatus>, String> {
    tx_status::statuses(&app, &commitments).await
}

#[tauri::command]
async fn bridge_burn_serial(app: tauri::AppHandle, commitment: String) -> Result<Option<String>, String> {
    bridge_payout::burn_serial(&app, &commitment).await
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
            build_bridge_deposit,
            list_bridge_deposits,
            discard_bridge_deposit,
            refresh_bridge_deposits,
            tx_statuses,
            bridge_burn_serial,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
