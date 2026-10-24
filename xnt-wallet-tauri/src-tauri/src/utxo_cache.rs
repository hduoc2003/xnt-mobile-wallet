use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
use std::collections::HashMap;
use std::sync::Arc;
use tokio::sync::RwLock;
use std::path::PathBuf;
use std::fs;
use std::io::{self, Read, Write};
use serde::{Deserialize, Serialize};
use bincode;
use tauri::Manager;

#[derive(Clone)]
pub struct CachedUtxo {
    pub utxo: IncomingUtxo,
    pub block_height: u64,
    /// Identical payments share a commitment; each leaf is a separate UTXO.
    pub aocl_leaf_index: u64,
}

impl CachedUtxo {
    pub fn utxo_id(&self) -> String {
        format!(
            "{}:{}",
            self.utxo.addition_record().canonical_commitment.to_hex(),
            self.aocl_leaf_index
        )
    }
}

type UtxoCache = Arc<RwLock<HashMap<String, CachedUtxo>>>;

static UTXO_CACHE: std::sync::OnceLock<UtxoCache> = std::sync::OnceLock::new();
static LAST_SCANNED: std::sync::OnceLock<Arc<RwLock<u64>>> = std::sync::OnceLock::new();
static CACHE_PATH: std::sync::OnceLock<PathBuf> = std::sync::OnceLock::new();

/// Leads the cache file; older files (one entry per commitment) lack it and are not loaded.
const CACHE_FORMAT: u64 = u64::from_le_bytes(*b"XNTUTXO2");

#[derive(Serialize, Deserialize)]
struct PersistedUtxoCache {
    format: u64,
    utxos: Vec<PersistedUtxo>,
    last_scanned_block: u64,
}

#[derive(Serialize, Deserialize, Clone)]
struct PersistedUtxo {
    utxo: IncomingUtxo,
    block_height: u64,
    aocl_leaf_index: u64,
}

fn get_utxo_key(utxo: &CachedUtxo) -> String {
    utxo.utxo_id()
}

fn deduplicate_utxos(utxos: Vec<CachedUtxo>) -> Vec<CachedUtxo> {
    let mut seen = std::collections::HashSet::new();
    let mut unique = Vec::new();
    for utxo in utxos {
        let key = get_utxo_key(&utxo);
        if !seen.contains(&key) {
            seen.insert(key);
            unique.push(utxo);
        }
    }
    unique
}

fn get_cache() -> &'static UtxoCache {
    UTXO_CACHE.get_or_init(|| {
        let (initial_map, last_scanned) = if CACHE_PATH.get().is_some() {
            load_persisted_cache()
        } else {
            (HashMap::new(), 0)
        };
        init_last_scanned(last_scanned);
        Arc::new(RwLock::new(initial_map))
    })
}

pub async fn reload_cache_from_disk() {
    if CACHE_PATH.get().is_none() {
        log::warn!("[UTXO Cache] Cache path not initialized, skipping reload");
        return;
    }
    
    let (initial_map, last_scanned) = load_persisted_cache();
    let count = initial_map.len();
    {
        let cache = get_cache();
        let mut cache_guard = cache.write().await;
        cache_guard.clear();
        for (key, utxo) in initial_map {
            cache_guard.insert(key, utxo);
        }
    }
    {
        let last_lock = get_last_scanned_lock();
        let mut last = last_lock.write().await;
        *last = last_scanned;
    }
    log::info!("[UTXO Cache] Reloaded {} cached UTXOs from disk", count);
}

fn init_last_scanned(initial: u64) -> Arc<RwLock<u64>> {
    LAST_SCANNED.get_or_init(|| Arc::new(RwLock::new(initial))).clone()
}

fn get_last_scanned_lock() -> Arc<RwLock<u64>> {
    LAST_SCANNED
        .get_or_init(|| Arc::new(RwLock::new(0)))
        .clone()
}

pub async fn store_utxos(utxos: Vec<CachedUtxo>) {
    let cache = get_cache();
    let mut cache_guard = cache.write().await;
    let unique_utxos = deduplicate_utxos(utxos);
    for cu in unique_utxos {
        let key = get_utxo_key(&cu);
        cache_guard.insert(key, cu);
    }
}

pub async fn get_all_utxos() -> Vec<CachedUtxo> {
    let cache = get_cache();
    let cache_guard = cache.read().await;
    cache_guard.values().cloned().collect()
}

pub async fn replace_cache_and_persist(utxos: Vec<CachedUtxo>, last_scanned_block: u64) -> Result<(), String> {
    let unique_utxos = deduplicate_utxos(utxos);
    {
        let cache = get_cache();
        let mut cache_guard = cache.write().await;
        cache_guard.clear();
        for utxo in unique_utxos.iter() {
            let key = get_utxo_key(utxo);
            cache_guard.insert(key, utxo.clone());
        }
    }

    {
        let last_lock = get_last_scanned_lock();
        let mut last = last_lock.write().await;
        *last = last_scanned_block;
    }

    persist_cache_to_disk(unique_utxos, last_scanned_block)
        .map_err(|e| format!("Failed to persist the UTXO cache: {e}"))
}

pub async fn get_last_scanned_block() -> u64 {
    let last_lock = get_last_scanned_lock();
    let last = last_lock.read().await;
    *last
}

pub async fn clear_cache() {
    let cache = get_cache();
    let mut cache_guard = cache.write().await;
    cache_guard.clear();
    {
        let last_lock = get_last_scanned_lock();
        let mut last = last_lock.write().await;
        *last = 0;
    }
    if let Err(e) = persist_cache_to_disk(Vec::new(), 0) {
        log::warn!("[UTXO Cache] Failed to persist cleared cache: {}", e);
    }
}

/// Unknown ids are left out.
pub async fn get_utxos_by_ids(utxo_ids: &[String]) -> Vec<CachedUtxo> {
    let cache = get_cache();
    let cache_guard = cache.read().await;
    utxo_ids
        .iter()
        .filter_map(|id| cache_guard.get(id).cloned())
        .collect()
}

pub fn init_cache_path(app: &tauri::AppHandle) {
    let cache_path = app.path()
        .app_data_dir()
        .map(|dir| dir.join("utxo_cache.bin"))
        .unwrap_or_else(|_| {
            let base = std::env::var("HOME")
                .map(PathBuf::from)
                .unwrap_or_else(|_| PathBuf::from("."));
            base.join(".xnt-wallet").join("utxo_cache.bin")
        });
    let _ = CACHE_PATH.set(cache_path);
}

fn cache_file_path() -> PathBuf {
    CACHE_PATH.get().cloned().unwrap_or_else(|| {
        let base = std::env::var("HOME")
            .map(PathBuf::from)
            .unwrap_or_else(|_| PathBuf::from("."));
        base.join(".xnt-wallet").join("utxo_cache.bin")
    })
}

pub fn chain_file_path() -> PathBuf {
    cache_file_path().with_file_name("chain.json")
}

fn ensure_cache_dir(path: &PathBuf) -> io::Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn load_persisted_cache() -> (HashMap<String, CachedUtxo>, u64) {
    let mut map = HashMap::new();
    let mut last_scanned_block = 0u64;
    let path = cache_file_path();
    if let Ok(mut file) = fs::File::open(&path) {
        let mut buf = Vec::new();
        if let Err(e) = file.read_to_end(&mut buf) {
            log::warn!("[UTXO Cache] Failed to read cache file: {}", e);
            return (map, last_scanned_block);
        }
        match bincode::deserialize::<PersistedUtxoCache>(&buf) {
            Ok(persisted) if persisted.format == CACHE_FORMAT => {
                last_scanned_block = persisted.last_scanned_block;
                // Deduplicate UTXOs when loading from disk
                let cached_utxos: Vec<CachedUtxo> = persisted.utxos
                    .into_iter()
                    .map(|pu| CachedUtxo {
                        utxo: pu.utxo,
                        block_height: pu.block_height,
                        aocl_leaf_index: pu.aocl_leaf_index,
                    })
                    .collect();
                let unique_utxos = deduplicate_utxos(cached_utxos);
                for utxo in unique_utxos {
                    let key = get_utxo_key(&utxo);
                    map.insert(key, utxo);
                }
                log::info!("[UTXO Cache] Loaded {} cached UTXOs from disk", map.len());
            }
            Ok(_) => {
                log::warn!(
                    "[UTXO Cache] Cache file has an older format; the next scan starts at block 0"
                );
            }
            Err(e) => {
                log::warn!("[UTXO Cache] Failed to deserialize cache file: {}; the next scan starts at block 0", e);
            }
        }
    }
    (map, last_scanned_block)
}

fn persist_cache_to_disk(utxos: Vec<CachedUtxo>, last_scanned_block: u64) -> io::Result<()> {
    let path = cache_file_path();
    ensure_cache_dir(&path)?;
    // Deduplicate before persisting to ensure no duplicates on disk
    let unique_utxos = deduplicate_utxos(utxos);
    let payload = PersistedUtxoCache {
        format: CACHE_FORMAT,
        utxos: unique_utxos
            .into_iter()
            .map(|u| PersistedUtxo {
                utxo: u.utxo,
                block_height: u.block_height,
                aocl_leaf_index: u.aocl_leaf_index,
            })
            .collect(),
        last_scanned_block,
    };
    let bytes = bincode::serialize(&payload).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
    let mut file = fs::File::create(&path)?;
    file.write_all(&bytes)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use neptune_privacy::prelude::twenty_first::prelude::BFieldElement;
    use neptune_privacy::prelude::twenty_first::tip5::digest::Digest;
    use neptune_privacy::protocol::consensus::transaction::utxo::Utxo;
    use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;

    fn cached(aocl_leaf_index: u64) -> CachedUtxo {
        CachedUtxo {
            utxo: IncomingUtxo {
                utxo: Utxo::new_native_currency(
                    Digest::default(),
                    NativeCurrencyAmount::coins_from_str("1").unwrap(),
                ),
                sender_randomness: Digest::default(),
                receiver_preimage: Digest::default(),
                is_guesser_fee: false,
                payment_id: BFieldElement::new(0),
            },
            block_height: 7,
            aocl_leaf_index,
        }
    }

    #[test]
    fn identical_payments_stay_separate() {
        let unique = deduplicate_utxos(vec![cached(3), cached(4), cached(3)]);
        let leaves: Vec<u64> = unique.iter().map(|u| u.aocl_leaf_index).collect();
        assert_eq!(leaves, [3, 4]);
        assert_ne!(unique[0].utxo_id(), unique[1].utxo_id());
    }

    #[test]
    fn older_cache_format_is_not_loaded() {
        #[derive(Serialize)]
        struct OldUtxo {
            utxo: IncomingUtxo,
            block_height: u64,
        }
        #[derive(Serialize)]
        struct OldCache {
            utxos: Vec<OldUtxo>,
            last_scanned_block: u64,
        }
        for count in [0, 1, 2] {
            let old = OldCache {
                utxos: (0..count)
                    .map(|_| OldUtxo {
                        utxo: cached(0).utxo,
                        block_height: 7,
                    })
                    .collect(),
                last_scanned_block: 900,
            };
            let buf = bincode::serialize(&old).unwrap();
            let loads = matches!(
                bincode::deserialize::<PersistedUtxoCache>(&buf),
                Ok(persisted) if persisted.format == CACHE_FORMAT
            );
            assert!(!loads);
        }
    }
}
