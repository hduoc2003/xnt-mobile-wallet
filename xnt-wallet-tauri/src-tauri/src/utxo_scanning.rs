use neptune_privacy::prelude::triton_vm::prelude::Tip5;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::prelude::twenty_first::tip5::digest::Digest;
use neptune_privacy::application::json_rpc::core::model::common::RpcNativeCurrencyAmount;
use neptune_privacy::application::json_rpc::core::model::message::{RpcAoclLeaf, RpcIndexedUtxo};
use neptune_privacy::protocol::consensus::transaction::utxo::Utxo;
use neptune_privacy::application::config::network::Network;
use std::str::FromStr;
use num_traits::Zero;
use reqwest::Client;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter};
use crate::rpc_config;
use serde::{Deserialize, Serialize};

use crate::chain_fingerprint;
use crate::utxo_cache;
use crate::rpc_client;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanUtxosRequest {
    /// Primary spending key (hex). For multi-key support use spending_keys_hex.
    #[serde(default)]
    pub spending_key_hex: String,
    /// All spending keys to scan (hex). If non-empty, takes precedence over spending_key_hex.
    #[serde(default)]
    pub spending_keys_hex: Vec<String>,
    pub start_block: u64,
    pub end_block: u64,
    /// Replaces the key fields above when non-empty: UTXOs are tagged with their account.
    #[serde(default)]
    pub accounts: Vec<AccountKeysRef>,
    /// Merges into the cache; leaves `last_scanned_block` and the chain record alone.
    #[serde(default)]
    pub isolated: bool,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct AccountKeysRef {
    pub account_id: String,
    pub spending_keys_hex: Vec<String>,
}

/// Leaves out the keys.
impl std::fmt::Debug for AccountKeysRef {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "AccountKeysRef({})", self.account_id)
    }
}

/// Per-key context used during multi-key scanning.
struct KeyContext {
    spending_key: SpendingKey,
    label: String,
    receiver_id_hash_hex: String,
    receiver_preimage: Digest,
    account_id: Option<String>,
    /// A pre-v0.3 dCTIDH key: finds funds that are moved, not spent, see [`crate::legacy_dctidh`].
    legacy_dctidh: bool,
}

impl KeyContext {
    /// Blocking: derives a key.
    fn legacy_counterpart(&self) -> Option<KeyContext> {
        let key = crate::legacy_dctidh::legacy_key(&self.spending_key)?;
        Some(KeyContext {
            label: "dCTIDH legacy".to_string(),
            receiver_id_hash_hex: receiver_id_hash_hex(&key),
            receiver_preimage: key.privacy_preimage(),
            spending_key: key,
            account_id: self.account_id.clone(),
            legacy_dctidh: true,
        })
    }
}

pub fn receiver_id_hash_hex(key: &SpendingKey) -> String {
    Tip5::hash_varlen(&[key.receiver_identifier()]).to_hex()
}

/// `Ok(None)` for a symmetric key. Blocking: parsing a dCTIDH key derives its public key.
fn key_context(key_hex: &str, account_id: Option<&str>) -> Result<Option<KeyContext>, String> {
    let bytes = hex::decode(key_hex.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid spending key hex: {e}"))?;
    let sk: SpendingKey = bincode::deserialize(&bytes)
        .map_err(|e| format!("Undeserializable spending key: {e}"))?;
    let label = match &sk {
        SpendingKey::Generation(_) => "Generation".to_string(),
        SpendingKey::dCTIDH(_)    => "dCTIDH".to_string(),
        SpendingKey::Symmetric(_) => return Ok(None),
    };
    Ok(Some(KeyContext {
        receiver_id_hash_hex: receiver_id_hash_hex(&sk),
        receiver_preimage: sk.privacy_preimage(),
        spending_key: sk,
        label,
        account_id: account_id.map(str::to_string),
        legacy_dctidh: false,
    }))
}

/// Every key must parse and be able to receive, so no account is silently left out.
/// `with_legacy_dctidh` adds each dCTIDH key's legacy-format counterpart.
async fn contexts_for(accounts: Vec<AccountKeysRef>, with_legacy_dctidh: bool) -> Result<Vec<KeyContext>, String> {
    tokio::task::spawn_blocking(move || {
        let mut contexts = Vec::new();
        for account in &accounts {
            for key in &account.spending_keys_hex {
                let context = key_context(key, Some(&account.account_id))
                    .map_err(|e| format!("Account {}: {e}", account.account_id))?
                    .ok_or_else(|| format!("Account {}: a key cannot receive", account.account_id))?;
                let legacy = if with_legacy_dctidh { context.legacy_counterpart() } else { None };
                contexts.push(context);
                contexts.extend(legacy);
            }
        }
        Ok(contexts)
    })
    .await
    .map_err(|e| format!("Key parsing task failed: {e}"))?
}

/// The legacy keys-only request: keys that do not parse are skipped with a warning.
fn legacy_contexts(keys: &[String]) -> Vec<KeyContext> {
    keys.iter()
        .filter_map(|k| match key_context(k, None) {
            Ok(context) => context,
            Err(e) => {
                log::warn!("[Scan] Skipping spending key: {e}");
                None
            }
        })
        .collect()
}

/// With `tagged`, only owned UTXOs; others (e.g. a locked seed's) stay cached and count nowhere.
fn cached_infos(
    tagged: bool,
    contexts: &[KeyContext],
    cached: &[utxo_cache::CachedUtxo],
    legacy_key_type: &str,
) -> Vec<UtxoInfo> {
    cached
        .iter()
        .filter_map(|utxo| {
            match contexts.iter().find(|c| c.receiver_preimage == utxo.utxo.receiver_preimage) {
                Some(c) => Some(cached_utxo_to_info(utxo, c.label.clone(), c.account_id.clone(), c.legacy_dctidh)),
                None if tagged => None,
                None => Some(cached_utxo_to_info(utxo, legacy_key_type.to_string(), None, false)),
            }
        })
        .collect()
}

/// An isolated scan adds to the prior cache; a normal one replaces it and moves `last_scanned`.
fn cache_after_scan(
    isolated: bool,
    cache: Vec<utxo_cache::CachedUtxo>,
    validated: Vec<utxo_cache::CachedUtxo>,
    found: Vec<utxo_cache::CachedUtxo>,
    persisted_last_scanned: u64,
    scanned_to: u64,
) -> (Vec<utxo_cache::CachedUtxo>, u64) {
    let (mut utxos, last_scanned) = if isolated {
        (cache, persisted_last_scanned)
    } else {
        (validated, scanned_to)
    };
    utxos.extend(found);
    (utxos, last_scanned)
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanUtxosResponse {
    pub success: bool,
    pub utxos: Vec<UtxoInfo>,
    pub last_scanned_block: u64,
    pub chain_reset: bool,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetCachedUtxosResponse {
    pub success: bool,
    pub utxos: Vec<UtxoInfo>,
    pub last_scanned_block: u64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
#[serde(rename_all = "camelCase")]
pub struct ScanProgress {
    pub current_block: u64,
    pub end_block: u64,
    pub start_block: u64,
    pub utxos_found: usize,
    pub percentage: f64,
    pub status: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UtxoInfo {
    pub block_height: u64,
    pub amount: String,
    pub addition_record_hex: String,
    pub utxo_id: String,
    pub utxo_digest: String,
    pub utxo_hash: String,
    pub is_timelocked: bool,
    pub release_date_ms: Option<u64>,
    pub key_type: String,
    pub is_pending: bool,
    pub account_id: Option<String>,
    /// At a pre-v0.3 dCTIDH lock: not spendable, only moved by a migration.
    pub is_legacy_dctidh: bool,
}

fn settlement_payout_utxo(lock_script_hash: Digest, amount_nau: RpcNativeCurrencyAmount) -> Utxo {
    Utxo::new_native_currency(lock_script_hash, NativeCurrencyAmount::from(amount_nau))
}

/// Builds a [`UtxoInfo`] view from a cached UTXO; the cache keeps no key type or account.
fn cached_utxo_to_info(
    cached: &utxo_cache::CachedUtxo,
    key_type: String,
    account_id: Option<String>,
    is_legacy_dctidh: bool,
) -> UtxoInfo {
    let utxo_ref = &cached.utxo.utxo;
    let amount = utxo_ref.get_native_currency_amount();
    let addition_record = cached.utxo.addition_record();
    let utxo_hash = Tip5::hash(utxo_ref);
    let (is_timelocked, release_date_ms) = match utxo_ref.release_date() {
        Some(ts) => (true, Some(ts.to_millis() as u64)),
        None => (false, None),
    };
    UtxoInfo {
        block_height: cached.block_height,
        amount: format!("{}", amount),
        addition_record_hex: format!(
            "0x{}",
            hex::encode(bincode::serialize(&addition_record).unwrap_or_default())
        ),
        utxo_id: cached.utxo_id(),
        utxo_digest: addition_record.canonical_commitment.to_hex(),
        utxo_hash: format!("0x{}", utxo_hash.to_hex()),
        is_timelocked,
        release_date_ms,
        key_type,
        is_pending: false,
        account_id,
        is_legacy_dctidh,
    }
}

/// Filters out UTXOs whose AbsoluteIndexSet hash matches a removal record in any
/// pending mempool transaction.  This prevents the wallet from building a new
/// transaction that double-spends a UTXO that is already referenced by an
/// unconfirmed transaction in the mempool.
///
/// Returns only the subset of `cached_utxos` that are NOT currently being spent
/// in the mempool.  If the mempool cannot be queried the error is propagated so
/// the caller can decide whether to proceed or abort.
pub async fn filter_utxos_spent_in_mempool(
    client: &Client,
    rpc_url: &str,
    cached_utxos: &[utxo_cache::CachedUtxo],
) -> Result<Vec<utxo_cache::CachedUtxo>, String> {
    use neptune_privacy::util_types::mutator_set::removal_record::absolute_index_set::AbsoluteIndexSet;

    if cached_utxos.is_empty() {
        return Ok(Vec::new());
    }

    // Fetch AbsoluteIndexSet hashes for every input across all pending mempool txs.
    let mempool_hashes = rpc_client::get_mempool_removal_hashes(client, rpc_url).await?;

    if mempool_hashes.is_empty() {
        // No pending transactions – nothing to filter.
        return Ok(cached_utxos.to_vec());
    }

    // Identify which UTXOs have a pending mempool spend.
    let mut mempool_spent_indices: HashSet<usize> = HashSet::new();
    for (idx, cached_utxo) in cached_utxos.iter().enumerate() {
        let abs_indices = AbsoluteIndexSet::compute(
            Tip5::hash(&cached_utxo.utxo.utxo),
            cached_utxo.utxo.sender_randomness,
            cached_utxo.utxo.receiver_preimage,
            cached_utxo.aocl_leaf_index,
        );
        if mempool_hashes.contains(&Tip5::hash(&abs_indices).to_hex()) {
            mempool_spent_indices.insert(idx);
        }
    }

    let filtered: Vec<utxo_cache::CachedUtxo> = cached_utxos
        .iter()
        .enumerate()
        .filter(|(idx, _)| !mempool_spent_indices.contains(idx))
        .map(|(_, utxo)| utxo.clone())
        .collect();

    Ok(filtered)
}

/// `drop_unindexed`: on a confirmed chain a cached (mined) UTXO with no AOCL index is stale.
pub async fn validate_utxos_with_rpc(
    client: &Client,
    rpc_url: &str,
    cached_utxos: &[utxo_cache::CachedUtxo],
    drop_unindexed: bool,
) -> Result<Vec<utxo_cache::CachedUtxo>, String> {
    use neptune_privacy::util_types::mutator_set::removal_record::absolute_index_set::AbsoluteIndexSet;
    use neptune_privacy::util_types::mutator_set::commit;
    
    if cached_utxos.is_empty() {
        return Ok(Vec::new());
    }
    
    let mut commitments: Vec<String> = Vec::new();
    let mut utxo_data: Vec<(usize, Digest, Digest, Digest)> = Vec::new();
    
    for (idx, cached_utxo) in cached_utxos.iter().enumerate() {
        let utxo_ref = &cached_utxo.utxo.utxo;
        let utxo_hash = Tip5::hash(utxo_ref);
        let sender_randomness = cached_utxo.utxo.sender_randomness;
        let receiver_preimage = cached_utxo.utxo.receiver_preimage;
        
        let addition_record = commit(
            utxo_hash,
            sender_randomness,
            receiver_preimage.hash(),
        );
        
        let commitment_hex = addition_record.canonical_commitment.to_hex();
        commitments.push(commitment_hex);
        utxo_data.push((idx, utxo_hash, sender_randomness, receiver_preimage));
    }
    let leaves = rpc_client::get_aocl_leaves(client, rpc_url, &commitments).await?;
    
    let mut absolute_index_set_hashes: Vec<Digest> = Vec::new();
    let mut utxo_to_hash_map: Vec<usize> = Vec::new();
    let mut no_aocl_indices: Vec<usize> = Vec::new();

    for (i, (utxo_idx, utxo_hash, sender_randomness, receiver_preimage)) in utxo_data.iter().enumerate() {
        let aocl_index = cached_utxos[*utxo_idx].aocl_leaf_index;
        if leaves[i]
            .iter()
            .any(|leaf| leaf.aocl_leaf_index == aocl_index)
        {
            let abs_indices = AbsoluteIndexSet::compute(
                *utxo_hash,
                *sender_randomness,
                *receiver_preimage,
                aocl_index,
            );
            let abs_hash = Tip5::hash(&abs_indices);
            absolute_index_set_hashes.push(abs_hash);
            utxo_to_hash_map.push(*utxo_idx);
        } else {
            no_aocl_indices.push(*utxo_idx);
        }
    }
    if drop_unindexed && !no_aocl_indices.is_empty() {
        log::warn!(
            "[Validate] Dropping {} cached UTXO(s) the node has no AOCL index for",
            no_aocl_indices.len()
        );
        no_aocl_indices.clear();
    }

    if absolute_index_set_hashes.is_empty() {
        let kept: Vec<utxo_cache::CachedUtxo> = no_aocl_indices
            .iter()
            .map(|&i| cached_utxos[i].clone())
            .collect();
        if !kept.is_empty() {
            log::warn!("[Validate] No AOCL indices found for any cached UTXO - keeping all {} as unvalidated", kept.len());
        }
        return Ok(kept);
    }

    let spent_statuses = rpc_client::get_spent_status(client, rpc_url, &absolute_index_set_hashes).await?;

    // Collect the indices of UTXOs we keep (unspent, or lacking an AOCL index),
    // then rebuild the result in the *original* input order. Preserving order is
    // required because callers map per-input spending keys to inputs positionally
    // (mixed-key transactions); appending some entries out of order would pair the
    // wrong key with the wrong UTXO.
    let mut kept_indices: HashSet<usize> = no_aocl_indices.into_iter().collect();
    for (hash_idx, &utxo_idx) in utxo_to_hash_map.iter().enumerate() {
        if spent_statuses.get(hash_idx).and_then(|s| *s).is_none() {
            kept_indices.insert(utxo_idx);
        }
    }

    let validated: Vec<utxo_cache::CachedUtxo> = cached_utxos
        .iter()
        .enumerate()
        .filter(|(idx, _)| kept_indices.contains(idx))
        .map(|(_, utxo)| utxo.clone())
        .collect();

    Ok(validated)
}

/// Fetches raw indexed UTXOs from the archival node for one receiver ID hash over a block range.
/// Retries up to 3 times on transient errors.
/// A reply without a `utxos` array is an error, never "nothing found".
fn utxos_of(result: &serde_json::Value) -> Result<Vec<serde_json::Value>, String> {
    match result.get("utxos") {
        Some(serde_json::Value::Array(utxos)) => Ok(utxos.clone()),
        _ => Err("archival_getUtxosByReceiver returned no utxos list".into()),
    }
}

async fn fetch_batch_for_receiver(
    client: &Client,
    rpc_url: &str,
    receiver_id_hash_hex: &str,
    from_block: u64,
    to_block: u64,
) -> Result<Vec<serde_json::Value>, String> {
    let request_body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": "archival_getUtxosByReceiver",
        "params": {
            "receiverIdHash": receiver_id_hash_hex,
            "fromBlockHeight": from_block,
            "toBlockHeight": to_block
        },
        "id": 1
    });

    let mut retries = 3u32;
    loop {
        match client.post(rpc_url).json(&request_body).send().await {
            Ok(resp) => {
                let status = resp.status();
                let text = match resp.text().await {
                    Ok(t) => t,
                    Err(e) => {
                        retries -= 1;
                        if retries == 0 { return Err(format!("archival_getUtxosByReceiver: {e}")); }
                        continue;
                    }
                };
                if !status.is_success() {
                    return Err(format!("archival_getUtxosByReceiver: HTTP {status}"));
                }
                let json: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("archival_getUtxosByReceiver: unreadable reply: {e}"))?;
                let result = rpc_client::extract_rpc_result("archival_getUtxosByReceiver", &json)?;
                return utxos_of(&result);
            }
            Err(e) => {
                retries -= 1;
                log::warn!("[Scan] RPC request failed (retries_left={}): {}", retries, e);
                if retries == 0 { return Err(format!("archival_getUtxosByReceiver: {e}")); }
                let delay_ms = 1000 * (3 - retries) as u64;
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

pub async fn has_activity(
    client: &Client,
    rpc_url: &str,
    receiver_id_hash_hex: &str,
    to_block: u64,
) -> Result<bool, String> {
    const NODE_RANGE_LIMIT: u64 = 10000;
    let mut from = 0;
    while from <= to_block {
        let to = std::cmp::min(from + NODE_RANGE_LIMIT - 1, to_block);
        let entries = fetch_batch_for_receiver(client, rpc_url, receiver_id_hash_hex, from, to).await?;
        if !entries.is_empty() {
            return Ok(true);
        }
        from = to + 1;
    }
    Ok(false)
}

/// Waits for a running scan, so it cannot write the old cache back afterwards.
pub async fn clear_cache_locked() {
    let _scan_guard = SCAN_LOCK.lock().await;
    utxo_cache::clear_cache().await;
}

pub async fn forget_accounts_utxos(accounts: Vec<AccountKeysRef>) -> Result<(), String> {
    let contexts = contexts_for(accounts, true).await?;
    let _scan_guard = SCAN_LOCK.lock().await;
    let kept: Vec<_> = utxo_cache::get_all_utxos()
        .await
        .into_iter()
        .filter(|u| !contexts.iter().any(|c| c.receiver_preimage == u.utxo.receiver_preimage))
        .collect();
    utxo_cache::replace_cache_and_persist(kept, utxo_cache::get_last_scanned_block().await).await
}

#[derive(Debug, PartialEq, Eq)]
enum LeafPick {
    Free(u64),
    /// Every leaf at that height is already known: the output was found before.
    AllTaken,
    NotCanonical,
}

/// Identical outputs share a commitment, so each takes the next free leaf at its height.
fn leaf_for_output(
    leaves: &[RpcAoclLeaf],
    block_height: u64,
    is_taken: impl Fn(u64) -> bool,
) -> LeafPick {
    let mut at_height: Vec<u64> = leaves
        .iter()
        .filter(|leaf| u64::from(leaf.block_height) == block_height)
        .map(|leaf| leaf.aocl_leaf_index)
        .collect();
    if at_height.is_empty() {
        return LeafPick::NotCanonical;
    }
    at_height.sort_unstable();
    at_height
        .into_iter()
        .find(|&leaf| !is_taken(leaf))
        .map_or(LeafPick::AllTaken, LeafPick::Free)
}

/// Blocks below the previous scan's end that each scan scans again, to pick up shallow reorgs.
const RESCAN_OVERLAP: u64 = 10;

/// Serializes scans so one from a just-replaced endpoint cannot interleave with the chain check.
static SCAN_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

pub async fn scan_utxos_client_side(
    app: &AppHandle,
    request: ScanUtxosRequest,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let _scan_guard = SCAN_LOCK.lock().await;
    // Resolved under the lock so a request queued before an endpoint change scans the new one.
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    let (chain_check, node_chain) = chain_fingerprint::check(&client, &rpc_url).await?;
    let chain_reset = chain_check.needs_full_rescan();
    let network = Network::from_str(&node_chain.network)
        .map_err(|e| format!("Unknown network '{}': {e}", node_chain.network))?;
    // Every cached UTXO goes: on another chain, or on the first check, whose cache chain is unknown.
    let chain_changed = matches!(chain_check, chain_fingerprint::ChainCheck::Reset { .. })
        || (chain_check == chain_fingerprint::ChainCheck::FirstRecord
            && !utxo_cache::get_all_utxos().await.is_empty());
    if request.isolated && (chain_reset || chain_changed) {
        return Err("The node is on a different chain than the cache: run a full scan first".into());
    }
    let mut request = request;
    // An old-format cache loads empty, and the keystore's min height may be past our UTXOs.
    let never_scanned = utxo_cache::get_last_scanned_block().await == 0;
    if chain_reset || chain_changed || (never_scanned && !request.isolated) {
        request.start_block = 0;
    }

    let tagged = !request.accounts.is_empty();
    let key_contexts: Vec<KeyContext> = if tagged {
        contexts_for(request.accounts.clone(), true).await?
    } else {
        let keys_to_scan: Vec<String> = if !request.spending_keys_hex.is_empty() {
            request.spending_keys_hex.clone()
        } else if !request.spending_key_hex.is_empty() {
            vec![request.spending_key_hex.clone()]
        } else {
            return Err("No spending key provided".to_string());
        };
        legacy_contexts(&keys_to_scan)
    };

    if key_contexts.is_empty() {
        return Err("No valid spending keys to scan".to_string());
    }

    let key_labels_str: String = key_contexts.iter().map(|k| k.label.as_str()).collect::<Vec<_>>().join(", ");
    let force_rescan = !request.isolated && request.start_block == 0;

    log::info!(
        "[Scan] Starting UTXO scan | keys=[{}] | blocks={}-{} | force_rescan={} | rpc={}",
        key_labels_str,
        request.start_block,
        request.end_block,
        force_rescan,
        rpc_url,
    );

    // On the same chain, UTXOs of accounts not in the request (e.g. a locked seed's) stay.
    let mut kept_unowned: Vec<utxo_cache::CachedUtxo> = Vec::new();
    if force_rescan {
        if tagged && !chain_changed {
            let unowned: Vec<_> = utxo_cache::get_all_utxos()
                .await
                .into_iter()
                .filter(|u| !key_contexts.iter().any(|c| c.receiver_preimage == u.utxo.receiver_preimage))
                .collect();
            kept_unowned = validate_utxos_with_rpc(&client, &rpc_url, &unowned, true).await?;
        }
        log::info!("[Scan] Force rescan - clearing UTXO cache, keeping {} of other accounts", kept_unowned.len());
        utxo_cache::clear_cache().await;
    }

    let cache_before = utxo_cache::get_all_utxos().await;
    let validated_cached_utxos: Vec<utxo_cache::CachedUtxo> = if request.isolated {
        cache_before.clone()
    } else if force_rescan {
        kept_unowned
    } else {
        log::debug!("[Scan] Loaded {} cached UTXOs - validating with RPC", cache_before.len());
        let validated = validate_utxos_with_rpc(&client, &rpc_url, &cache_before, true).await?;
        log::info!("[Scan] Cache validation: {}/{} UTXOs still unspent", validated.len(), cache_before.len());
        validated
    };
    
    // Older caches may hold UTXOs the scan now refuses: another lock, or legacy past the cutoff.
    let validated_cached_utxos: Vec<utxo_cache::CachedUtxo> = validated_cached_utxos
        .into_iter()
        .filter(|u| match key_contexts.iter().find(|c| c.receiver_preimage == u.utxo.receiver_preimage) {
            Some(c) => {
                u.utxo.utxo.lock_script_hash() == c.spending_key.lock_script_hash()
                    && (!c.legacy_dctidh || crate::legacy_dctidh::recognized_at(network, u.block_height))
            }
            None => true,
        })
        .collect();
    let mut utxo_infos: Vec<UtxoInfo> =
        cached_infos(tagged, &key_contexts, &validated_cached_utxos, "dCTIDH");
    for info in &utxo_infos {
        log::debug!(
            "[Scan] Cached UTXO | key_type={} | amount={} | block={} | timelocked={} | hash={}",
            info.key_type,
            info.amount,
            info.block_height,
            info.is_timelocked,
            &info.utxo_hash[..18],
        );
    }

    let mut found_utxos: Vec<utxo_cache::CachedUtxo> = Vec::new();
    let mut taken: HashSet<(String, u64)> = validated_cached_utxos
        .iter()
        .map(|u| {
            (
                u.utxo.addition_record().canonical_commitment.to_hex(),
                u.aocl_leaf_index,
            )
        })
        .collect();
    
    let persisted_last_scanned = if force_rescan { 0 } else { utxo_cache::get_last_scanned_block().await };
    if request.isolated {
        // Later normal scans start just below the persisted height, so cover up to it.
        request.end_block = std::cmp::max(request.end_block, persisted_last_scanned);
    }
    let mut last_scanned = if persisted_last_scanned > 0 && !request.isolated {
        persisted_last_scanned
    } else {
        request.start_block
    };
    
    let batch_size = 10000;
    let mut current_block = if force_rescan || request.isolated {
        request.start_block
    } else {
        std::cmp::max(request.start_block, persisted_last_scanned.saturating_sub(RESCAN_OVERLAP))
    };
    
    let total_blocks = request.end_block.saturating_sub(request.start_block) + 1;
    
    let initial_progress = ScanProgress {
        current_block: request.start_block,
        end_block: request.end_block,
        start_block: request.start_block,
        utxos_found: 0,
        percentage: 0.0,
        status: "Starting scan...".to_string(),
    };
    let _ = app.emit("utxo-scan-progress", &initial_progress);

    // Fetched before the batches so a reorg during the scan shows up as a mismatch next time.
    let anchor = if request.isolated {
        None
    } else {
        Some(
            chain_fingerprint::scanned_tip(
                &client,
                &rpc_url,
                request.end_block.saturating_sub(RESCAN_OVERLAP + 1),
            )
            .await?,
        )
    };

    while current_block <= request.end_block {
        let batch_start_time = std::time::Instant::now();
        let batch_end = std::cmp::min(current_block + batch_size - 1, request.end_block);

        let blocks_scanned = current_block.saturating_sub(request.start_block);
        let percentage = if total_blocks > 0 {
            (blocks_scanned as f64 / total_blocks as f64 * 100.0).min(100.0)
        } else {
            0.0
        };

        log::debug!(
            "[Scan] Batch blocks={}-{} | {:.1}% | utxos_found_so_far={}",
            current_block, batch_end, percentage, utxo_infos.len()
        );

        let progress = ScanProgress {
            current_block,
            end_block: request.end_block,
            start_block: request.start_block,
            utxos_found: utxo_infos.len(),
            percentage,
            status: format!("Scanning blocks {}-{}...", current_block, batch_end),
        };
        let _ = app.emit("utxo-scan-progress", &progress);
        
        let mut cached_utxos: Vec<utxo_cache::CachedUtxo> = Vec::new();
        let mut unspent_count = 0;

        for key_ctx in &key_contexts {
            let indexed_utxos = fetch_batch_for_receiver(
                &client, &rpc_url, &key_ctx.receiver_id_hash_hex, current_block, batch_end,
            ).await?;

            if !indexed_utxos.is_empty() {
                log::info!(
                    "[Scan] blocks={}-{} | key={} | RPC returned {} announcements to decrypt",
                    current_block, batch_end, key_ctx.label, indexed_utxos.len()
                );
            }

            use neptune_privacy::prelude::twenty_first::prelude::BFieldElement;
            use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;

            for indexed_utxo in &indexed_utxos {
                let indexed: RpcIndexedUtxo = serde_json::from_value(indexed_utxo.clone())
                    .map_err(|e| format!("Unreadable indexed UTXO from the node: {e}"))?;
                let (block_height, utxo, sender_randomness, is_public) = match indexed {
                    RpcIndexedUtxo::Encrypted { block_height, ciphertext, .. } => {
                        let Ok((utxo, sender_randomness, _payment_id)) = key_ctx.spending_key.decrypt(&ciphertext) else {
                            continue;
                        };
                        // Anyone can announce to a receiver id; only a UTXO this key unlocks is ours.
                        if utxo.lock_script_hash() != key_ctx.spending_key.lock_script_hash() {
                            log::warn!("[Scan] Announcement at block={block_height} is not locked to key_type={}; skipping", key_ctx.label);
                            continue;
                        }
                        if key_ctx.legacy_dctidh && !crate::legacy_dctidh::recognized_at(network, u64::from(block_height)) {
                            log::info!("[Scan] Skipping a legacy dCTIDH UTXO at block={block_height}: after the cutoff");
                            continue;
                        }
                        log::debug!(
                            "[Scan] Decrypted announcement at block={} | key_type={}",
                            block_height, key_ctx.label
                        );
                        (u64::from(block_height), utxo, sender_randomness, false)
                    }
                    // Bridge settlement payouts carry no ciphertext; the node indexes them in the clear.
                    RpcIndexedUtxo::Public { block_height, lock_script_hash, amount_nau, sender_randomness, .. } => {
                        // Payouts go to current addresses; a legacy key has none.
                        if key_ctx.legacy_dctidh {
                            continue;
                        }
                        if lock_script_hash != key_ctx.spending_key.clone().to_address().lock_script_hash() {
                            log::warn!(
                                "[Scan] Bridge payout at block={} is not locked to key_type={}; skipping",
                                block_height, key_ctx.label
                            );
                            continue;
                        }
                        (u64::from(block_height), settlement_payout_utxo(lock_script_hash, amount_nau), sender_randomness, true)
                    }
                };

                let incoming_utxo = IncomingUtxo {
                    utxo: utxo.clone(),
                    sender_randomness,
                    receiver_preimage: key_ctx.receiver_preimage,
                    is_guesser_fee: false,
                    payment_id: BFieldElement::new(0),
                };

                let commitment = incoming_utxo
                    .addition_record()
                    .canonical_commitment
                    .to_hex();
                let leaves = rpc_client::get_aocl_leaves(
                    &client,
                    &rpc_url,
                    std::slice::from_ref(&commitment),
                )
                .await?;
                let aocl_leaf_index = match leaf_for_output(&leaves[0], block_height, |leaf| {
                    taken.contains(&(commitment.clone(), leaf))
                }) {
                    LeafPick::Free(leaf) => leaf,
                    LeafPick::AllTaken => continue,
                    LeafPick::NotCanonical => {
                        log::warn!(
                            "[Scan] Output at block={} (public={}) has no canonical AOCL leaf; skipping",
                            block_height, is_public
                        );
                        continue;
                    }
                };
                taken.insert((commitment, aocl_leaf_index));

                let temp_cached_utxo = utxo_cache::CachedUtxo {
                    utxo: incoming_utxo.clone(),
                    block_height,
                    aocl_leaf_index,
                };

                let validated_utxos = validate_utxos_with_rpc(
                    &client, &rpc_url, &[temp_cached_utxo.clone()], false,
                ).await?;

                if validated_utxos.is_empty() {
                    log::debug!(
                        "[Scan] Decrypted UTXO at block={} is already spent — skipping",
                        block_height
                    );
                    continue;
                }

                unspent_count += 1;

                for utxo in &validated_utxos {
                    let utxo_ref = &utxo.utxo.utxo;
                    let amount = utxo_ref.get_native_currency_amount();
                    let addition_record = utxo.utxo.addition_record();
                    let utxo_hash = Tip5::hash(utxo_ref);
                    let utxo_digest = addition_record.canonical_commitment.to_hex();

                    let release_date_opt = utxo_ref.release_date();
                    let (is_timelocked, release_date_ms) = if let Some(ts) = release_date_opt {
                        (true, Some(ts.to_millis() as u64))
                    } else {
                        (false, None)
                    };

                    let amount_str = format!("{}", amount);
                    let hash_str = format!("0x{}", utxo_hash.to_hex());

                    log::info!(
                        "[Scan] ✓ UTXO found | key_type={} | amount={} XNT | block={} | timelocked={}{} | hash={}",
                        key_ctx.label,
                        amount_str,
                        block_height,
                        is_timelocked,
                        if let Some(ms) = release_date_ms {
                            format!(" (release={})", ms)
                        } else {
                            String::new()
                        },
                        &hash_str[..std::cmp::min(18, hash_str.len())],
                    );

                    utxo_infos.push(UtxoInfo {
                        block_height,
                        utxo_id: utxo.utxo_id(),
                        amount: amount_str,
                        addition_record_hex: format!(
                            "0x{}",
                            hex::encode(bincode::serialize(&addition_record)
                                .map_err(|e| format!("Failed to serialize addition record: {}", e))?)
                        ),
                        utxo_digest,
                        utxo_hash: hash_str,
                        is_timelocked,
                        release_date_ms,
                        key_type: key_ctx.label.clone(),
                        is_pending: false,
                        account_id: key_ctx.account_id.clone(),
                        is_legacy_dctidh: key_ctx.legacy_dctidh,
                    });

                    cached_utxos.push(utxo.clone());
                    found_utxos.push(utxo.clone());
                }
            }
        }

        if !cached_utxos.is_empty() {
            log::info!(
                "[Scan] Batch blocks={}-{} | {} new unspent UTXO(s) persisted to cache",
                current_block, batch_end, cached_utxos.len()
            );
            utxo_cache::store_utxos(cached_utxos).await;
        }

        let batch_elapsed = batch_start_time.elapsed();
        if unspent_count > 0 || batch_elapsed.as_secs() >= 2 {
            log::debug!(
                "[Scan] Batch done | blocks={}-{} | elapsed={:.2}s | new_utxos={}",
                current_block, batch_end, batch_elapsed.as_secs_f32(), unspent_count
            );
        }
            
            last_scanned = batch_end;
        
        current_block = batch_end + 1;
    }
    
    let mut seen_ids: HashSet<String> = HashSet::new();
    utxo_infos.retain(|u| seen_ids.insert(u.utxo_id.clone()));

    log::info!(
        "[Scan] ✓ Scan complete | keys=[{}] | blocks={}-{} | total_utxos={} | last_scanned_block={}",
        key_labels_str,
        request.start_block,
        request.end_block,
        utxo_infos.len(),
        last_scanned,
    );

    if utxo_infos.is_empty() {
        log::info!("[Scan] No UTXOs found for keys=[{}]", key_labels_str);
    } else {
        let total_amount: f64 = utxo_infos
            .iter()
            .filter_map(|u| u.amount.parse::<f64>().ok())
            .sum();
        log::info!(
            "[Scan] Total balance: {:.8} XNT across {} UTXO(s) | keys=[{}]",
            total_amount,
            utxo_infos.len(),
            key_labels_str,
        );
    }

    let completion_progress = ScanProgress {
        current_block: request.end_block,
        end_block: request.end_block,
        start_block: request.start_block,
        utxos_found: utxo_infos.len(),
        percentage: 100.0,
        status: format!("Scan completed! Found {} UTXO(s)", utxo_infos.len()),
    };
    let _ = app.emit("utxo-scan-progress", &completion_progress);
    
    let (to_persist, last_scanned) = cache_after_scan(
        request.isolated,
        cache_before,
        validated_cached_utxos,
        found_utxos,
        persisted_last_scanned,
        last_scanned,
    );
    utxo_cache::replace_cache_and_persist(to_persist.clone(), last_scanned).await?;

    // Mark UTXOs that are already referenced by a pending mempool transaction so
    // the frontend can exclude them from coin selection before proof generation.
    if let Ok(non_pending) = filter_utxos_spent_in_mempool(&client, &rpc_url, &to_persist).await {
        let non_pending_ids: std::collections::HashSet<String> =
            non_pending.iter().map(|u| u.utxo_id()).collect();
        for info in &mut utxo_infos {
            if !non_pending_ids.contains(&info.utxo_id) {
                info.is_pending = true;
            }
        }
    }

    if let Some(anchor) = anchor {
        chain_fingerprint::record(node_chain, anchor)?;
    }

    let response = ScanUtxosResponse {
        success: true,
        utxos: utxo_infos,
        last_scanned_block: last_scanned,
        chain_reset,
        error: None,
    };
    
    Ok(serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))?)
}

pub async fn get_cached_utxos_local(accounts: Option<Vec<AccountKeysRef>>) -> Result<String, String> {
    let cached_utxos = utxo_cache::get_all_utxos().await;
    let last_scanned = utxo_cache::get_last_scanned_block().await;

    let tagged = accounts.is_some();
    let contexts = match accounts {
        Some(accounts) => contexts_for(accounts, true).await?,
        None => Vec::new(),
    };
    let utxos = cached_infos(tagged, &contexts, &cached_utxos, "");

    let response = GetCachedUtxosResponse {
        success: true,
        utxos,
        last_scanned_block: last_scanned,
        error: None,
    };

    Ok(serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize cached UTXOs: {}", e))?)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPendingBalanceRequest {
    #[serde(default)]
    pub spending_key_hex: String,
    /// Replaces `spending_key_hex` when non-empty; the response then splits by account.
    #[serde(default)]
    pub accounts: Vec<AccountKeysRef>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AccountPending {
    pub account_id: String,
    pub pending: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPendingBalanceResponse {
    pub success: bool,
    pub pending_balance: String,
    pub by_account: Vec<AccountPending>,
    pub error: Option<String>,
}

fn pending_per_context(
    contexts: &[KeyContext],
    announcements: &[neptune_privacy::protocol::consensus::transaction::announcement::Announcement],
    totals: &mut [NativeCurrencyAmount],
) {
    use neptune_privacy::state::wallet::address::{
        ciphertext_from_announcement, receiver_identifier_from_announcement, KeyType,
    };

    for announcement in announcements {
        let Ok(kt) = KeyType::try_from(announcement) else {
            continue;
        };
        let Ok(receiver_identifier) = receiver_identifier_from_announcement(announcement) else {
            continue;
        };
        for (context, total) in contexts.iter().zip(totals.iter_mut()) {
            let (base_key_type, subaddr_key_type) = match &context.spending_key {
                SpendingKey::Generation(_) => (KeyType::Generation, KeyType::GenerationSubAddr),
                SpendingKey::dCTIDH(_) => (KeyType::dCTIDH, KeyType::dCTIDHSubAddr),
                SpendingKey::Symmetric(_) => continue,
            };
            if (kt != base_key_type && kt != subaddr_key_type)
                || receiver_identifier != context.spending_key.receiver_identifier()
            {
                continue;
            }
            let Ok(ciphertext_bfes) = ciphertext_from_announcement(announcement) else {
                continue;
            };
            if let Ok((utxo, _sender_randomness, _payment_id)) = context.spending_key.decrypt(&ciphertext_bfes) {
                *total += utxo.get_native_currency_amount();
            }
        }
    }
}

fn pending_by_account(contexts: &[KeyContext], totals: &[NativeCurrencyAmount]) -> Vec<AccountPending> {
    let mut by_account: Vec<(String, NativeCurrencyAmount)> = Vec::new();
    for (context, total) in contexts.iter().zip(totals) {
        let Some(account_id) = &context.account_id else {
            continue;
        };
        match by_account.iter_mut().find(|(id, _)| id == account_id) {
            Some((_, sum)) => *sum += *total,
            None => by_account.push((account_id.clone(), *total)),
        }
    }
    by_account
        .into_iter()
        .map(|(account_id, pending)| AccountPending { account_id, pending: format!("{}", pending) })
        .collect()
}

pub async fn get_pending_balance_from_mempool(
    app: &AppHandle,
    request: GetPendingBalanceRequest,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
        
    let rpc_url = rpc_config::get_custom_rpc_endpoint(app)
        .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string());

    let contexts: Vec<KeyContext> = if !request.accounts.is_empty() {
        contexts_for(request.accounts, false).await?
    } else {
        match key_context(&request.spending_key_hex, None)? {
            Some(context) => vec![context],
            None => {
                return serde_json::to_string(&GetPendingBalanceResponse {
                    success: true,
                    pending_balance: "0".to_string(),
                    by_account: Vec::new(),
                    error: Some("Symmetric spending keys not supported".to_string()),
                }).map_err(|e| format!("Failed to serialize response: {}", e));
            }
        }
    };
    let empty_by_account = || pending_by_account(&contexts, &vec![NativeCurrencyAmount::zero(); contexts.len()]);
    
    let tx_ids = rpc_client::get_mempool_transactions(&client, &rpc_url).await?;
    
    if tx_ids.is_empty() {
        return Ok(serde_json::to_string(&GetPendingBalanceResponse {
            success: true,
            pending_balance: "0".to_string(),
            by_account: empty_by_account(),
            error: None,
        }).map_err(|e| format!("Failed to serialize response: {}", e))?);
    }
    
    let mut totals = vec![NativeCurrencyAmount::zero(); contexts.len()];
    for tx_id_hex in tx_ids {
        let tx_kernel = match rpc_client::get_mempool_transaction_kernel(&client, &rpc_url, &tx_id_hex).await {
            Ok(Some(kernel)) => kernel,
            // Left the mempool since it was listed.
            Ok(None) => continue,
            Err(e) => return Err(e),
        };
        pending_per_context(&contexts, &tx_kernel.announcements, &mut totals);
    }
    let pending_total = totals.iter().fold(NativeCurrencyAmount::zero(), |sum, t| sum + *t);
    
    Ok(serde_json::to_string(&GetPendingBalanceResponse {
        success: true,
        pending_balance: format!("{}", pending_total),
        by_account: pending_by_account(&contexts, &totals),
        error: None,
    }).map_err(|e| format!("Failed to serialize response: {}", e))?)
}


#[cfg(test)]
mod tests {
    use super::*;
    use neptune_privacy::protocol::consensus::bridge::aggregated_announcement::AggregatedAnnouncementData;
    use neptune_privacy::util_types::mutator_set::commit;

    const PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent";

    fn account(id: &str, index: u64) -> AccountKeysRef {
        let words: Vec<String> = PHRASE.split(' ').map(String::from).collect();
        let keys = crate::keys::derive_account_keys(
            &words,
            index,
            crate::keys::parse_network("testnet").unwrap(),
        )
        .unwrap();
        AccountKeysRef {
            account_id: id.into(),
            spending_keys_hex: vec![keys.generation_key_hex, keys.dctidh_key_hex],
        }
    }

    async fn parse_accounts(accounts: &[AccountKeysRef]) -> Vec<KeyContext> {
        contexts_for(accounts.to_vec(), false).await.unwrap()
    }

    fn coins(amount: &str) -> NativeCurrencyAmount {
        NativeCurrencyAmount::coins_from_str(amount).unwrap()
    }

    fn cached_for(context: &KeyContext, amount: &str) -> utxo_cache::CachedUtxo {
        use neptune_privacy::prelude::twenty_first::prelude::BFieldElement;
        use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
        let lock_script_hash = context.spending_key.clone().to_address().lock_script_hash();
        utxo_cache::CachedUtxo {
            utxo: IncomingUtxo {
                utxo: Utxo::new_native_currency(lock_script_hash, coins(amount)),
                sender_randomness: Digest::default(),
                receiver_preimage: context.receiver_preimage,
                is_guesser_fee: false,
                payment_id: BFieldElement::new(0),
            },
            block_height: 7,
            aocl_leaf_index: 0,
        }
    }

    #[tokio::test]
    async fn tags_utxos_by_receiver_preimage() {
        let contexts = parse_accounts(&[account("main:0", 0), account("main:1", 1)]).await;
        let cached: Vec<_> = contexts.iter().map(|c| cached_for(c, "1")).collect();
        let infos = cached_infos(true, &contexts, &cached, "");
        let tags: Vec<_> = infos
            .iter()
            .map(|i| (i.account_id.as_deref().unwrap(), i.key_type.as_str()))
            .collect();
        assert_eq!(
            tags,
            [
                ("main:0", "Generation"),
                ("main:0", "dCTIDH"),
                ("main:1", "Generation"),
                ("main:1", "dCTIDH")
            ]
        );
    }

    #[tokio::test]
    async fn cached_utxos_get_key_type() {
        let contexts = parse_accounts(&[account("main:0", 0)]).await;
        let cached = [cached_for(&contexts[0], "1")];
        assert_eq!(cached_infos(true, &contexts, &cached, "")[0].key_type, "Generation");
    }

    #[tokio::test]
    async fn unowned_utxo_stays_cached_untagged() {
        let contexts = parse_accounts(&[account("main:0", 0)]).await;
        let locked_seed = parse_accounts(&[account("imp-1:0", 5)]).await;
        let cached = vec![cached_for(&contexts[1], "1"), cached_for(&locked_seed[0], "2")];
        let infos = cached_infos(true, &contexts, &cached, "");
        assert_eq!(infos.len(), 1);
        assert_eq!(infos[0].account_id.as_deref(), Some("main:0"));

        let (persisted, _) = cache_after_scan(false, vec![], cached.clone(), vec![], 0, 9);
        assert_eq!(persisted.len(), 2);

        let legacy: Vec<_> = contexts.into_iter().map(|c| KeyContext { account_id: None, ..c }).collect();
        let legacy_infos = cached_infos(false, &legacy, &cached, "dCTIDH");
        assert_eq!(legacy_infos.len(), 2);
        assert!(legacy_infos.iter().all(|i| i.account_id.is_none()));
        assert_eq!(legacy_infos[1].key_type, "dCTIDH");
    }

    #[tokio::test]
    async fn legacy_dctidh_utxos_are_tagged_and_marked() {
        let contexts = contexts_for(vec![account("main:0", 0)], true).await.unwrap();
        let labels: Vec<_> = contexts.iter().map(|c| (c.label.as_str(), c.legacy_dctidh)).collect();
        assert_eq!(labels, [("Generation", false), ("dCTIDH", false), ("dCTIDH legacy", true)]);
        let legacy = &contexts[2];
        assert_ne!(legacy.receiver_preimage, contexts[1].receiver_preimage);
        assert_ne!(legacy.receiver_id_hash_hex, contexts[1].receiver_id_hash_hex);

        let cached = utxo_cache::CachedUtxo {
            utxo: neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo { receiver_preimage: legacy.receiver_preimage, ..cached_for(&contexts[1], "3").utxo },
            block_height: 7,
            aocl_leaf_index: 0,
        };
        let infos = cached_infos(true, &contexts, &[cached], "");
        assert_eq!(infos.len(), 1);
        assert!(infos[0].is_legacy_dctidh);
        assert_eq!(infos[0].account_id.as_deref(), Some("main:0"));
    }

    #[tokio::test]
    async fn accounts_request_never_returns_unowned_utxos() {
        let contexts = parse_accounts(&[account("main:0", 0)]).await;
        let cached = [cached_for(&contexts[0], "1")];
        assert!(cached_infos(true, &[], &cached, "").is_empty());
    }

    #[tokio::test]
    async fn invalid_account_key_is_an_error() {
        let bad = AccountKeysRef {
            account_id: "main:3".into(),
            spending_keys_hex: vec!["0xzz".into()],
        };
        let error = contexts_for(vec![account("main:0", 0), bad], false).await.err().unwrap();
        assert!(error.starts_with("Account main:3"), "{error}");
    }

    #[test]
    fn missing_utxos_list_is_an_error() {
        assert!(utxos_of(&serde_json::json!({})).is_err());
        assert!(utxos_of(&serde_json::json!({"utxos": null})).is_err());
        assert_eq!(utxos_of(&serde_json::json!({"utxos": []})), Ok(vec![]));
    }

    #[tokio::test]
    async fn isolated_scan_merges_and_keeps_last_scanned() {
        let contexts = parse_accounts(&[account("main:0", 0)]).await;
        let [a, b, c] = ["1", "2", "3"].map(|amount| cached_for(&contexts[0], amount));
        let amounts = |utxos: Vec<utxo_cache::CachedUtxo>| -> Vec<String> {
            utxos.iter().map(|u| u.utxo.utxo.get_native_currency_amount().to_string()).collect()
        };
        let (persisted, last) =
            cache_after_scan(true, vec![a.clone(), b], vec![a.clone()], vec![c.clone()], 500, 20);
        assert_eq!(amounts(persisted), ["1", "2", "3"].map(|n| coins(n).to_string()));
        assert_eq!(last, 500);

        let (persisted, last) = cache_after_scan(false, vec![], vec![a], vec![c], 500, 520);
        assert_eq!(amounts(persisted), ["1", "3"].map(|n| coins(n).to_string()));
        assert_eq!(last, 520);
    }

    #[tokio::test]
    async fn pending_is_split_by_account() {
        use neptune_privacy::state::wallet::transaction_output::TxOutput;
        let contexts = parse_accounts(&[account("main:0", 0), account("main:1", 1)]).await;
        let announcements: Vec<_> = [(0, "1"), (1, "2"), (3, "4")]
            .into_iter()
            .map(|(context, amount)| {
                let address = contexts[context].spending_key.clone().to_address();
                TxOutput::onchain_native_currency(coins(amount), Digest::default(), address, false)
                    .announcement()
                    .unwrap()
            })
            .collect();
        let mut totals = vec![NativeCurrencyAmount::zero(); contexts.len()];
        pending_per_context(&contexts, &announcements, &mut totals);
        let by_account: Vec<_> = pending_by_account(&contexts, &totals)
            .into_iter()
            .map(|p| (p.account_id, p.pending))
            .collect();
        assert_eq!(
            by_account,
            [
                ("main:0".to_string(), format!("{}", coins("3"))),
                ("main:1".to_string(), format!("{}", coins("4")))
            ]
        );
    }

    #[test]
    #[ignore = "pre-v0.3 fixture: its UTXO hash predates v0.3's type scripts; regenerate from a v0.3-chain block"]
    fn settlement_payout_matches_its_onchain_output() {
        let indexed: RpcIndexedUtxo = serde_json::from_str(include_str!("../test_data/block_252_payout.json")).unwrap();
        let RpcIndexedUtxo::Public { lock_script_hash, amount_nau, sender_randomness, .. } = indexed else {
            panic!("expected a public entry");
        };
        let utxo = settlement_payout_utxo(lock_script_hash, amount_nau);
        let kernel: serde_json::Value =
            serde_json::from_str(include_str!("../test_data/block_252_tx_kernel.json")).unwrap();
        let kernel = rpc_client::kernel_from_result(&kernel).unwrap().unwrap();
        let burn = &AggregatedAnnouncementData::find_in_announcements(&kernel.announcements)
            .unwrap()
            .burns[0];
        let commitment =
            commit(Tip5::hash(&utxo), sender_randomness, burn.privacy_digest).canonical_commitment;
        assert!(kernel.outputs.iter().any(|o| o.canonical_commitment == commitment));
    }

    fn leaf(aocl_leaf_index: u64, block_height: u64) -> RpcAoclLeaf {
        RpcAoclLeaf {
            aocl_leaf_index,
            block_height: block_height.into(),
            block_digest: Digest::default(),
        }
    }

    #[test]
    fn identical_payments_take_separate_leaves() {
        let leaves = [leaf(9, 5), leaf(4, 5), leaf(20, 8)];
        assert_eq!(leaf_for_output(&leaves, 5, |_| false), LeafPick::Free(4));
        assert_eq!(leaf_for_output(&leaves, 5, |l| l == 4), LeafPick::Free(9));
        assert_eq!(
            leaf_for_output(&leaves, 5, |l| l == 4 || l == 9),
            LeafPick::AllTaken
        );
        assert_eq!(leaf_for_output(&leaves, 8, |_| false), LeafPick::Free(20));
        assert_eq!(
            leaf_for_output(&leaves, 6, |_| false),
            LeafPick::NotCanonical
        );
        assert_eq!(leaf_for_output(&[], 5, |_| false), LeafPick::NotCanonical);
    }
}
