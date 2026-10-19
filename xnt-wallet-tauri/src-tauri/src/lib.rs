#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(non_snake_case)]
use log::Level;

#[cfg(target_os = "android")]
use android_logger;
#[cfg(target_os = "android")]
use libc;

mod account_import;
mod accounts;
mod bridge_deposit;
mod bridge_payout;
mod bridge_status;
mod chain_fingerprint;
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

/// Off the main thread: dCTIDH derivation is slow on phones.
#[tauri::command]
async fn derive_account_keys(
    seed_phrase: Vec<String>,
    index: u64,
    network: String,
) -> Result<keys::AccountKeys, String> {
    let network = keys::parse_network(&network)?;
    tokio::task::spawn_blocking(move || keys::derive_account_keys(&seed_phrase, index, network))
        .await
        .map_err(|e| format!("Key derivation task failed: {e}"))?
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
async fn get_cached_utxos_local(
    accounts: Option<Vec<utxo_scanning::AccountKeysRef>>,
) -> Result<String, String> {
    utxo_scanning::get_cached_utxos_local(accounts).await
}

#[tauri::command]
async fn get_pending_balance(app: tauri::AppHandle, request: serde_json::Value) -> Result<String, String> {
    let pending_request: utxo_scanning::GetPendingBalanceRequest = serde_json::from_value(request)
        .map_err(|e| format!("Invalid request: {}", e))?;

    utxo_scanning::get_pending_balance_from_mempool(&app, pending_request).await
}

#[tauri::command]
async fn clear_utxo_cache() -> Result<(), String> {
    utxo_scanning::clear_cache_locked().await;
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
async fn check_chain(app: tauri::AppHandle) -> Result<chain_fingerprint::ChainCheck, String> {
    let rpc_url = rpc_config::resolve_rpc_url(&app).await?;
    let (check, _) = chain_fingerprint::check(&transaction::http_client()?, &rpc_url).await?;
    Ok(check)
}

#[tauri::command]
async fn accounts_load(app: tauri::AppHandle) -> Result<accounts::Registry, String> {
    accounts::load_at(&accounts::data_dir(&app)?).await
}

#[tauri::command]
async fn accounts_add(
    app: tauri::AppHandle,
    seed_id: String,
    index: u64,
) -> Result<accounts::AccountEntry, String> {
    accounts::update_at(&accounts::data_dir(&app)?, |r| r.add(&seed_id, index)).await
}

#[tauri::command]
async fn accounts_rename(
    app: tauri::AppHandle,
    id: String,
    name: Option<String>,
) -> Result<accounts::Registry, String> {
    accounts::update_at(&accounts::data_dir(&app)?, |r| {
        r.rename(&id, name)?;
        Ok(r.clone())
    })
    .await
}

#[tauri::command]
async fn accounts_set_hidden(
    app: tauri::AppHandle,
    id: String,
    hidden: bool,
) -> Result<accounts::Registry, String> {
    accounts::update_at(&accounts::data_dir(&app)?, |r| {
        r.set_hidden(&id, hidden)?;
        Ok(r.clone())
    })
    .await
}

#[tauri::command]
async fn accounts_set_active(
    app: tauri::AppHandle,
    id: String,
) -> Result<accounts::Registry, String> {
    accounts::update_at(&accounts::data_dir(&app)?, |r| {
        r.set_active(&id)?;
        Ok(r.clone())
    })
    .await
}

#[tauri::command]
async fn accounts_mark_backed_up(
    app: tauri::AppHandle,
    seed_id: String,
) -> Result<accounts::Registry, String> {
    accounts::update_at(&accounts::data_dir(&app)?, |r| {
        r.mark_backed_up(&seed_id)?;
        Ok(r.clone())
    })
    .await
}

#[tauri::command]
async fn accounts_remove_seed(
    app: tauri::AppHandle,
    seed_id: String,
) -> Result<accounts::Registry, String> {
    accounts::remove_seed_at(&accounts::data_dir(&app)?, &seed_id).await
}

#[tauri::command]
async fn accounts_import(
    app: tauri::AppHandle,
    source: account_import::ImportSource,
    wallet_password: String,
) -> Result<accounts::SeedEntry, String> {
    account_import::import_at(&accounts::data_dir(&app)?, source, wallet_password).await
}

#[tauri::command]
async fn accounts_discover(
    app: tauri::AppHandle,
    seed_id: String,
    wallet_password: String,
) -> Result<Vec<accounts::AccountEntry>, String> {
    let rpc_url = rpc_config::resolve_rpc_url(&app).await?;
    accounts::discover_at(
        &accounts::data_dir(&app)?,
        &transaction::http_client()?,
        &rpc_url,
        &seed_id,
        wallet_password,
    )
    .await
}

#[tauri::command]
async fn accounts_seed_phrase(
    app: tauri::AppHandle,
    seed_id: String,
    wallet_password: String,
) -> Result<Vec<String>, String> {
    accounts::seed_phrase_at(&accounts::data_dir(&app)?, &seed_id, wallet_password).await
}

#[tauri::command]
async fn accounts_save_keystore_as(app: tauri::AppHandle, seed_id: String) -> Result<String, String> {
    let data_dir = accounts::data_dir(&app)?;
    let registry = accounts::load_at(&data_dir).await?;
    let seed = registry
        .seeds
        .iter()
        .find(|s| s.id == seed_id)
        .ok_or_else(|| format!("Unknown seed {seed_id}"))?;
    let path = accounts::keystore_path(&data_dir, seed)?;
    keystore::save_keystore_file_as(&app, &path, &seed.keystore_file).await
}

#[tauri::command]
async fn forget_accounts(
    app: tauri::AppHandle,
    accounts: Vec<utxo_scanning::AccountKeysRef>,
) -> Result<(), String> {
    let ids: Vec<String> = accounts.iter().map(|a| a.account_id.clone()).collect();
    utxo_scanning::forget_accounts_utxos(accounts).await?;
    bridge_status::remove_for_accounts(&app, &ids).await
}

#[tauri::command]
async fn accounts_reset_main(
    app: tauri::AppHandle,
    keystore_file: String,
) -> Result<accounts::Registry, String> {
    accounts::reset_for_main_at(&accounts::data_dir(&app)?, keystore_file).await
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
            derive_account_keys,
            create_keystore,
            check_keystore,
            get_min_block_height,
            save_min_block_height,
            update_min_block_height,
            keystore_exists,
            save_keystore_dialog,
            get_keystore_path,
            build_transaction,
            scan_utxos_client_side,
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
            check_chain,
            accounts_load,
            accounts_add,
            accounts_rename,
            accounts_set_hidden,
            accounts_set_active,
            accounts_mark_backed_up,
            accounts_remove_seed,
            accounts_reset_main,
            forget_accounts,
            accounts_import,
            accounts_discover,
            accounts_seed_phrase,
            accounts_save_keystore_as,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
