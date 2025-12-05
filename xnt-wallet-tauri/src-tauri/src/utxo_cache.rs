use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
use neptune_privacy::util_types::mutator_set::addition_record::AdditionRecord;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use hex;
use bincode;

/// In-memory cache for UTXOs owned by the user
/// Keyed by addition record canonical commitment for fast lookup
type UtxoCache = Arc<RwLock<HashMap<String, IncomingUtxo>>>;

static UTXO_CACHE: std::sync::OnceLock<UtxoCache> = std::sync::OnceLock::new();

fn get_cache() -> &'static UtxoCache {
    UTXO_CACHE.get_or_init(|| Arc::new(RwLock::new(HashMap::new())))
}

/// Store UTXOs in the cache
pub async fn store_utxos(utxos: Vec<IncomingUtxo>) {
    let cache = get_cache();
    let mut cache_guard = cache.write().await;
    for utxo in utxos {
        let ar = utxo.addition_record();
        let key = format!("0x{}", hex::encode(bincode::serialize(&ar.canonical_commitment).unwrap()));
        cache_guard.insert(key, utxo);
    }
}

/// Get a UTXO from cache by addition record hex
pub async fn get_utxo_by_addition_record(addition_record_hex: &str) -> Option<IncomingUtxo> {
    let cache = get_cache();
    let cache_guard = cache.read().await;
    
    // Try direct lookup first
    if let Some(utxo) = cache_guard.get(addition_record_hex) {
        return Some(utxo.clone());
    }
    
    // If not found, try to parse the addition record and search by canonical commitment
    if let Ok(ar_bytes) = hex::decode(addition_record_hex.trim_start_matches("0x")) {
        if let Ok(ar) = bincode::deserialize::<AdditionRecord>(&ar_bytes) {
            let key = format!("0x{}", hex::encode(bincode::serialize(&ar.canonical_commitment).unwrap()));
            return cache_guard.get(&key).cloned();
        }
    }
    
    None
}

/// Get all UTXOs from cache
pub async fn get_all_utxos() -> Vec<IncomingUtxo> {
    let cache = get_cache();
    let cache_guard = cache.read().await;
    cache_guard.values().cloned().collect()
}

/// Clear the UTXO cache
pub async fn clear_cache() {
    let cache = get_cache();
    let mut cache_guard = cache.write().await;
    cache_guard.clear();
}

/// Get UTXOs matching the given addition records
pub async fn get_utxos_by_addition_records(addition_record_hexes: &[String]) -> Vec<IncomingUtxo> {
    let mut result = Vec::new();
    for ar_hex in addition_record_hexes {
        if let Some(utxo) = get_utxo_by_addition_record(ar_hex).await {
            result.push(utxo);
        }
    }
    result
}

