use neptune_privacy::prelude::triton_vm::prelude::Tip5;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::prelude::twenty_first::tip5::digest::Digest;
use neptune_privacy::application::json_rpc::core::model::common::RpcNativeCurrencyAmount;
use neptune_privacy::application::json_rpc::core::model::message::RpcIndexedUtxo;
use neptune_privacy::protocol::consensus::transaction::utxo::Utxo;
use num_traits::Zero;
use reqwest::Client;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter};
use crate::rpc_config;
use serde::{Deserialize, Serialize};

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
    pub rpc_url: Option<String>,
}

/// Per-key context used during multi-key scanning.
struct KeyContext {
    spending_key: SpendingKey,
    label: String,
    receiver_id_hash_hex: String,
    receiver_preimage: Digest,
    lock_script_hash: Digest,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanUtxosResponse {
    pub success: bool,
    pub utxos: Vec<UtxoInfo>,
    pub last_scanned_block: u64,
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
    pub utxo_digest: String,
    pub utxo_hash: String,
    pub is_timelocked: bool,
    pub release_date_ms: Option<u64>,
    pub key_type: String,
    pub is_pending: bool,
}

fn settlement_payout_utxo(lock_script_hash: Digest, amount_nau: RpcNativeCurrencyAmount) -> Utxo {
    Utxo::new_native_currency(lock_script_hash, NativeCurrencyAmount::from(amount_nau))
}

/// Builds a [`UtxoInfo`] view from a cached UTXO.
///
/// `key_type` is supplied by the caller: during a scan it is inferred from the
/// active key context, but the cache does not retain it, so cache-read paths
/// pass an empty string.
fn cached_utxo_to_info(cached: &utxo_cache::CachedUtxo, key_type: String) -> UtxoInfo {
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
        utxo_digest: addition_record.canonical_commitment.to_hex(),
        utxo_hash: format!("0x{}", utxo_hash.to_hex()),
        is_timelocked,
        release_date_ms,
        key_type,
        is_pending: false,
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
    use neptune_privacy::util_types::mutator_set::commit;

    if cached_utxos.is_empty() {
        return Ok(Vec::new());
    }

    // Fetch AbsoluteIndexSet hashes for every input across all pending mempool txs.
    let mempool_hashes = rpc_client::get_mempool_removal_hashes(client, rpc_url).await?;

    if mempool_hashes.is_empty() {
        // No pending transactions – nothing to filter.
        return Ok(cached_utxos.to_vec());
    }

    // Compute the AOCL commitment for each candidate UTXO so we can look up its
    // AOCL leaf index, which is needed to reconstruct the AbsoluteIndexSet.
    let mut commitments: Vec<String> = Vec::new();
    let mut utxo_data: Vec<(usize, Digest, Digest, Digest)> = Vec::new();

    for (idx, cached_utxo) in cached_utxos.iter().enumerate() {
        let utxo_ref = &cached_utxo.utxo.utxo;
        let utxo_hash = Tip5::hash(utxo_ref);
        let sender_randomness = cached_utxo.utxo.sender_randomness;
        let receiver_preimage = cached_utxo.utxo.receiver_preimage;

        let addition_record = commit(utxo_hash, sender_randomness, receiver_preimage.hash());
        commitments.push(addition_record.canonical_commitment.to_hex());
        utxo_data.push((idx, utxo_hash, sender_randomness, receiver_preimage));
    }

    let indices = rpc_client::get_aocl_leaf_indices(client, rpc_url, &commitments).await?;

    // Identify which UTXOs have a pending mempool spend.
    let mut mempool_spent_indices: HashSet<usize> = HashSet::new();
    for (i, (utxo_idx, utxo_hash, sender_randomness, receiver_preimage)) in utxo_data.iter().enumerate() {
        if let Some(Some(aocl_index)) = indices.get(i).cloned() {
            let abs_indices = AbsoluteIndexSet::compute(
                *utxo_hash,
                *sender_randomness,
                *receiver_preimage,
                aocl_index,
            );
            let abs_hash = Tip5::hash(&abs_indices);
            if mempool_hashes.contains(&abs_hash.to_hex()) {
                mempool_spent_indices.insert(*utxo_idx);
            }
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

pub async fn validate_utxos_with_rpc(
    client: &Client,
    rpc_url: &str,
    cached_utxos: &[utxo_cache::CachedUtxo],
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
    let indices = rpc_client::get_aocl_leaf_indices(client, rpc_url, &commitments).await?;
    
    let mut absolute_index_set_hashes: Vec<Digest> = Vec::new();
    let mut utxo_to_hash_map: Vec<usize> = Vec::new();
    let mut no_aocl_indices: Vec<usize> = Vec::new();

    for (i, (utxo_idx, utxo_hash, sender_randomness, receiver_preimage)) in utxo_data.iter().enumerate() {
        if let Some(Some(aocl_index)) = indices.get(i).cloned() {
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
            log::debug!("[Validate] UTXO #{} has no AOCL index — keeping as unspent", utxo_idx);
            no_aocl_indices.push(*utxo_idx);
        }
    }

    if absolute_index_set_hashes.is_empty() {
        let kept: Vec<utxo_cache::CachedUtxo> = no_aocl_indices
            .iter()
            .map(|&i| cached_utxos[i].clone())
            .collect();
        log::warn!("[Validate] No AOCL indices found for any cached UTXO — keeping all {} as unvalidated", kept.len());
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
/// Retries up to 3 times on transient errors. Returns None on unrecoverable failure.
async fn fetch_batch_for_receiver(
    client: &Client,
    rpc_url: &str,
    receiver_id_hash_hex: &str,
    from_block: u64,
    to_block: u64,
) -> Option<Vec<serde_json::Value>> {
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
                    Err(_) => {
                        retries -= 1;
                        if retries == 0 { return None; }
                        continue;
                    }
                };
                if !status.is_success() || text.trim().is_empty() {
                    return None;
                }
                let json: serde_json::Value = match serde_json::from_str(&text) {
                    Ok(j) => j,
                    Err(_) => return None,
                };
                let result = match rpc_client::extract_rpc_result("archival_getUtxosByReceiver", &json) {
                    Ok(r) => r,
                    Err(_) => return None,
                };
                return match result.get("utxos") {
                    Some(serde_json::Value::Array(arr)) => Some(arr.clone()),
                    _ => Some(vec![]),
                };
            }
            Err(e) => {
                retries -= 1;
                log::warn!("[Scan] RPC request failed (retries_left={}): {}", retries, e);
                if retries == 0 { return None; }
                let delay_ms = 1000 * (3 - retries) as u64;
                tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
            }
        }
    }
}

pub async fn scan_utxos_client_side(
    app: &AppHandle,
    request: ScanUtxosRequest,
) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let rpc_url = {
        if let Some(ref v) = request.rpc_url {
            if !v.trim().is_empty() {
                v.clone()
            } else {
                rpc_config::get_custom_rpc_endpoint(app)
                    .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string())
            }
        } else {
            rpc_config::get_custom_rpc_endpoint(app)
                .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string())
        }
    };

    use neptune_privacy::prelude::twenty_first::prelude::Tip5;

    // Build the list of key hexes to scan: prefer spending_keys_hex if non-empty.
    let keys_to_scan: Vec<String> = if !request.spending_keys_hex.is_empty() {
        request.spending_keys_hex.clone()
    } else if !request.spending_key_hex.is_empty() {
        vec![request.spending_key_hex.clone()]
    } else {
        return Err("No spending key provided".to_string());
    };

    // Parse and validate all spending keys into KeyContext entries.
    let mut key_contexts: Vec<KeyContext> = Vec::new();
    for key_hex in &keys_to_scan {
        let bytes = match hex::decode(key_hex.trim_start_matches("0x")) {
            Ok(b) => b,
            Err(e) => { log::warn!("[Scan] Skipping invalid spending key hex: {}", e); continue; }
        };
        let sk: SpendingKey = match bincode::deserialize(&bytes) {
            Ok(k) => k,
            Err(e) => { log::warn!("[Scan] Skipping undeserializable spending key: {}", e); continue; }
        };
        if matches!(sk, SpendingKey::Symmetric(_)) { continue; }
        let label = match &sk {
            SpendingKey::Generation(_) => "Generation".to_string(),
            SpendingKey::dCTIDH(_)    => "dCTIDH".to_string(),
            SpendingKey::Symmetric(_) => unreachable!(),
        };
        let receiver_id = sk.receiver_identifier();
        let receiver_id_hash = Tip5::hash_varlen(&[receiver_id]);
        let receiver_id_hash_hex = receiver_id_hash.to_hex();
        let receiver_preimage = sk.privacy_preimage();
        let lock_script_hash = sk.clone().to_address().lock_script_hash();
        key_contexts.push(KeyContext {
            spending_key: sk,
            label,
            receiver_id_hash_hex,
            receiver_preimage,
            lock_script_hash,
        });
    }

    if key_contexts.is_empty() {
        return Err("No valid spending keys to scan".to_string());
    }

    let key_labels_str: String = key_contexts.iter().map(|k| k.label.as_str()).collect::<Vec<_>>().join(", ");
    let force_rescan = request.start_block == 0;

    log::info!(
        "[Scan] Starting UTXO scan | keys=[{}] | blocks={}-{} | force_rescan={} | rpc={}",
        key_labels_str,
        request.start_block,
        request.end_block,
        force_rescan,
        rpc_url,
    );

    if force_rescan {
        log::info!("[Scan] Force rescan — clearing UTXO cache");
        utxo_cache::clear_cache().await;
    }

    let mut validated_cached_utxos: Vec<utxo_cache::CachedUtxo> = Vec::new();
    if !force_rescan {
        let cached_utxos = utxo_cache::get_all_utxos().await;
        log::debug!("[Scan] Loaded {} cached UTXOs — validating with RPC", cached_utxos.len());
        match validate_utxos_with_rpc(&client, &rpc_url, &cached_utxos).await {
            Ok(validated) => {
                log::info!("[Scan] Cache validation: {}/{} UTXOs still unspent", validated.len(), cached_utxos.len());
                validated_cached_utxos = validated;
            }
            Err(e) => {
                // Fall back to all cached UTXOs so timelocked (and other)
                // UTXOs are not silently dropped when the RPC is unreachable.
                // A spent UTXO included here will simply fail when broadcast.
                log::warn!("[Scan] Cache validation failed — using unvalidated cache as fallback: {}", e);
                validated_cached_utxos = cached_utxos;
            }
        }
    }
    
    let mut utxo_infos: Vec<UtxoInfo> = validated_cached_utxos
        .iter()
        .map(|utxo| {
            // Infer key type from receiver_preimage — match against known key contexts.
            let key_type = key_contexts
                .iter()
                .find(|kc| kc.receiver_preimage == utxo.utxo.receiver_preimage)
                .map(|kc| kc.label.clone())
                .unwrap_or_else(|| "dCTIDH".to_string());
            let info = cached_utxo_to_info(utxo, key_type);
            log::debug!(
                "[Scan] Cached UTXO | key_type={} | amount={} | block={} | timelocked={} | hash={}",
                info.key_type,
                info.amount,
                info.block_height,
                info.is_timelocked,
                &info.utxo_hash[..18],
            );
            info
        })
        .collect();

    let mut incoming_utxos_for_persist = validated_cached_utxos.clone();
    
    let persisted_last_scanned = if force_rescan { 0 } else { utxo_cache::get_last_scanned_block().await };
    let mut last_scanned = if persisted_last_scanned > 0 {
        persisted_last_scanned
    } else {
        request.start_block
    };
    
    let batch_size = 10000;
    let mut current_block = if force_rescan {
        request.start_block
    } else {
        std::cmp::max(request.start_block, persisted_last_scanned.saturating_sub(10))
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
            let indexed_utxos = match fetch_batch_for_receiver(
                &client, &rpc_url, &key_ctx.receiver_id_hash_hex, current_block, batch_end,
            ).await {
                Some(arr) => arr,
                None => continue,
            };

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
                        log::debug!(
                            "[Scan] Decrypted announcement at block={} | key_type={}",
                            block_height, key_ctx.label
                        );
                        (u64::from(block_height), utxo, sender_randomness, false)
                    }
                    // Bridge settlement payouts carry no ciphertext; the node indexes them in the clear.
                    RpcIndexedUtxo::Public { block_height, lock_script_hash, amount_nau, sender_randomness, .. } => {
                        if lock_script_hash != key_ctx.lock_script_hash {
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

                // Built from node-supplied fields rather than decrypted, so it must exist on chain.
                if is_public {
                    let commitment = incoming_utxo.addition_record().canonical_commitment.to_hex();
                    let indices = rpc_client::get_aocl_leaf_indices(&client, &rpc_url, &[commitment]).await?;
                    if !matches!(indices.as_slice(), [Some(_)]) {
                        log::warn!(
                            "[Scan] Bridge payout at block={} has no matching output on chain; skipping",
                            block_height
                        );
                        continue;
                    }
                }

                let temp_cached_utxo = utxo_cache::CachedUtxo {
                    utxo: incoming_utxo.clone(),
                    block_height,
                };

                let validated_utxos = match validate_utxos_with_rpc(
                    &client, &rpc_url, &[temp_cached_utxo.clone()],
                ).await {
                    Ok(validated) => validated,
                    Err(_) => continue,
                };

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
                    });

                    cached_utxos.push(utxo.clone());
                    incoming_utxos_for_persist.push(utxo.clone());
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
    
    let mut seen_hashes: HashSet<String> = HashSet::new();
    utxo_infos.retain(|u| {
        let key = if !u.addition_record_hex.is_empty() {
            u.addition_record_hex.clone()
        } else {
            u.utxo_hash.clone()
        };
        if key.is_empty() {
            return false;
        }
        if seen_hashes.contains(&key) {
            false
        } else {
            seen_hashes.insert(key);
            true
        }
    });

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
    
    utxo_cache::replace_cache_and_persist(incoming_utxos_for_persist.clone(), last_scanned).await;

    // Mark UTXOs that are already referenced by a pending mempool transaction so
    // the frontend can exclude them from coin selection before proof generation.
    if let Ok(non_pending) = filter_utxos_spent_in_mempool(&client, &rpc_url, &incoming_utxos_for_persist).await {
        let non_pending_digests: std::collections::HashSet<String> = non_pending
            .iter()
            .map(|u| u.utxo.addition_record().canonical_commitment.to_hex())
            .collect();
        for info in &mut utxo_infos {
            if !non_pending_digests.contains(&info.utxo_digest) {
                info.is_pending = true;
            }
        }
    }

    let response = ScanUtxosResponse {
        success: true,
        utxos: utxo_infos,
        last_scanned_block: last_scanned,
        error: None,
    };
    
    Ok(serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))?)
}

pub async fn get_cached_utxos(app: &AppHandle) -> Result<String, String> {
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let rpc_url = rpc_config::get_custom_rpc_endpoint(app)
        .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string());

    let cached_utxos = utxo_cache::get_all_utxos().await;
    let last_scanned = utxo_cache::get_last_scanned_block().await;

    let valid_utxos = match validate_utxos_with_rpc(&client, &rpc_url, &cached_utxos).await {
        Ok(validated) => validated,
        Err(_) => cached_utxos
    };

    utxo_cache::replace_cache_and_persist(valid_utxos.clone(), last_scanned).await;

    let utxos: Vec<UtxoInfo> = valid_utxos
        .iter()
        .map(|utxo| cached_utxo_to_info(utxo, String::new()))
        .collect();

    let response = GetCachedUtxosResponse {
        success: true,
        utxos,
        last_scanned_block: last_scanned,
        error: None,
    };

    Ok(serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize cached UTXOs: {}", e))?)
}

pub async fn get_cached_utxos_local() -> Result<String, String> {
    let cached_utxos = utxo_cache::get_all_utxos().await;
    let last_scanned = utxo_cache::get_last_scanned_block().await;

    let utxos: Vec<UtxoInfo> = cached_utxos
        .iter()
        .map(|utxo| cached_utxo_to_info(utxo, String::new()))
        .collect();

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
    pub spending_key_hex: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPendingBalanceResponse {
    pub success: bool,
    pub pending_balance: String,
    pub error: Option<String>,
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
    
    
    let spending_key_bytes = hex::decode(
        request.spending_key_hex.trim_start_matches("0x")
    )
    .map_err(|e| format!("Invalid spending key hex: {}", e))?;
    
    let spending_key: SpendingKey = bincode::deserialize(&spending_key_bytes)
        .map_err(|e| format!("Failed to deserialize spending key: {}", e))?;
    
    if matches!(spending_key, SpendingKey::Symmetric(_)) {
        return Ok(serde_json::to_string(&GetPendingBalanceResponse {
            success: true,
            pending_balance: "0".to_string(),
            error: Some("Symmetric spending keys not supported".to_string()),
        }).map_err(|e| format!("Failed to serialize response: {}", e))?);
    }
    
    let mut pending_total = NativeCurrencyAmount::zero();
    
    let tx_ids = match rpc_client::get_mempool_transactions(&client, &rpc_url).await {
        Ok(ids) => ids,
        Err(_) => {
            return Ok(serde_json::to_string(&GetPendingBalanceResponse {
                success: true,
                pending_balance: "0".to_string(),
                error: None,
            }).map_err(|e| format!("Failed to serialize response: {}", e))?);
        }
    };
    
    if tx_ids.is_empty() {
        return Ok(serde_json::to_string(&GetPendingBalanceResponse {
            success: true,
            pending_balance: "0".to_string(),
            error: None,
        }).map_err(|e| format!("Failed to serialize response: {}", e))?);
    }
    
    for tx_id_hex in tx_ids {
        let tx_kernel = match rpc_client::get_mempool_transaction_kernel(&client, &rpc_url, &tx_id_hex).await {
            Ok(Some(kernel)) => kernel,
            Ok(None) => continue,
            Err(_) => continue,
        };
        
        {
            use neptune_privacy::state::wallet::address::{
                ciphertext_from_announcement, receiver_identifier_from_announcement, KeyType,
            };

            let receiver_identifier = spending_key.receiver_identifier();
            let (base_key_type, subaddr_key_type): (KeyType, KeyType) = match &spending_key {
                SpendingKey::Generation(_) => (KeyType::Generation, KeyType::GenerationSubAddr),
                SpendingKey::dCTIDH(_) => (KeyType::dCTIDH, KeyType::dCTIDHSubAddr),
                SpendingKey::Symmetric(_) => unreachable!(),
            };

            for announcement in &tx_kernel.announcements {
                let kt = match KeyType::try_from(announcement) {
                    Ok(kt) => kt,
                    Err(_) => continue,
                };
                if kt != base_key_type && kt != subaddr_key_type {
                    continue;
                }

                if !matches!(receiver_identifier_from_announcement(announcement), Ok(rid) if rid == receiver_identifier) {
                    continue;
                }

                let ciphertext_bfes = match ciphertext_from_announcement(announcement) {
                    Ok(ct) => ct,
                    Err(_) => continue,
                };

                if let Ok((utxo, _sender_randomness, _payment_id)) = spending_key.decrypt(&ciphertext_bfes) {
                    let amount = utxo.get_native_currency_amount();
                    pending_total = pending_total + amount;
                }
            }
        }
    }
    
    let pending_balance_str = format!("{}", pending_total);
    
    Ok(serde_json::to_string(&GetPendingBalanceResponse {
        success: true,
        pending_balance: pending_balance_str,
        error: None,
    }).map_err(|e| format!("Failed to serialize response: {}", e))?)
}


#[cfg(test)]
mod tests {
    use super::*;
    use neptune_privacy::protocol::consensus::bridge::aggregated_announcement::AggregatedAnnouncementData;
    use neptune_privacy::util_types::mutator_set::commit;

    #[test]
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
}
