use neptune_privacy::prelude::twenty_first::math::b_field_element::BFieldElement;
use neptune_privacy::prelude::triton_vm::prelude::Tip5;
use neptune_privacy::protocol::consensus::transaction::announcement::Announcement;
use neptune_privacy::protocol::consensus::transaction::transaction_kernel::{
    TransactionKernel, TransactionKernelProxy,
};
use neptune_privacy::application::json_rpc::core::model::block::transaction_kernel::RpcTransactionKernel;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::protocol::proof_abstractions::timestamp::Timestamp;
use neptune_privacy::prelude::twenty_first::tip5::digest::Digest;
use neptune_privacy::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use neptune_privacy::application::json_rpc::core::model::message::GetArchivalMutatorSetResponse;
use num_traits::Zero;
use reqwest::Client;
use std::collections::HashSet;
use tauri::{AppHandle, Emitter};
use crate::rpc_config;
use serde::{Deserialize, Serialize};

use crate::utxo_cache;
use crate::transaction;
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
        }
    }
    
    if absolute_index_set_hashes.is_empty() {
        return Ok(Vec::new());
    }
    
    let spent_statuses = rpc_client::get_spent_status(client, rpc_url, &absolute_index_set_hashes).await?;
    
    let validated: Vec<utxo_cache::CachedUtxo> = utxo_to_hash_map
        .iter()
        .enumerate()
        .filter_map(|(hash_idx, &utxo_idx)| {
            if spent_statuses.get(hash_idx).and_then(|s| *s).is_none() {
                Some(cached_utxos[utxo_idx].clone())
            } else {
                None
            }
        })
        .collect();
    
    Ok(validated)
}

fn parse_rpc_bfield_elements(hex_str: &str) -> Result<Vec<BFieldElement>, String> {
    let hex_clean = hex_str.strip_prefix("0x")
        .ok_or_else(|| "Missing 0x prefix".to_string())?;
    
    if hex_clean.len() % 16 != 0 {
        return Err(format!("Invalid hex length: must be multiple of 16, got {}", hex_clean.len()));
    }
    
    let bytes = hex::decode(hex_clean)
        .map_err(|e| format!("Invalid hex: {}", e))?;
    
    let elements: Vec<BFieldElement> = bytes
        .chunks_exact(8)
        .map(|chunk| {
            let array: [u8; 8] = chunk.try_into().unwrap();
            BFieldElement::new(u64::from_be_bytes(array))
        })
        .collect();
    
    Ok(elements)
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
                    .or_else(|| {
                        std::env::var("VITE_APP_API")
                            .ok()
                            .filter(|v| !v.trim().is_empty())
                    })
                    .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string())
            }
        } else {
            rpc_config::get_custom_rpc_endpoint(app)
                .or_else(|| {
                    std::env::var("VITE_APP_API")
                        .ok()
                        .filter(|v| !v.trim().is_empty())
                })
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
        key_contexts.push(KeyContext { spending_key: sk, label, receiver_id_hash_hex, receiver_preimage });
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
                log::warn!("[Scan] Cache validation failed (ignoring): {}", e);
            }
        }
    }
    
    let mut utxo_infos: Vec<UtxoInfo> = validated_cached_utxos
        .iter()
        .map(|utxo| {
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
            // Infer key type from receiver_preimage — match against known key contexts.
            let key_type = key_contexts.iter()
                .find(|kc| kc.receiver_preimage == utxo.utxo.receiver_preimage)
                .map(|kc| kc.label.clone())
                .unwrap_or_else(|| "dCTIDH".to_string());
            log::debug!(
                "[Scan] Cached UTXO | key_type={} | amount={} | block={} | timelocked={} | hash={}",
                key_type,
                amount,
                utxo.block_height,
                is_timelocked,
                &format!("0x{}", utxo_hash.to_hex())[..18],
            );
            UtxoInfo {
                block_height: utxo.block_height,
                amount: format!("{}", amount),
                addition_record_hex: format!(
                    "0x{}",
                    hex::encode(bincode::serialize(&addition_record).unwrap())
                ),
                utxo_digest,
                utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                is_timelocked,
                release_date_ms,
                key_type,
            }
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
                let block_height = indexed_utxo.get("blockHeight")
                    .and_then(|v| v.as_u64())
                    .unwrap_or(0);

                let ciphertext_bfes: Vec<u64> = match indexed_utxo.get("ciphertext") {
                    Some(serde_json::Value::Array(arr)) => {
                        arr.iter()
                            .filter_map(|v| v.as_u64().or_else(|| v.as_str().and_then(|s| s.parse().ok())))
                            .collect()
                    }
                    _ => Vec::new(),
                };

                let ciphertext_bytes: Vec<u8> = ciphertext_bfes
                    .iter()
                    .flat_map(|&v| v.to_le_bytes())
                    .collect();

                let ciphertext: Vec<BFieldElement> = ciphertext_bytes
                    .chunks(8)
                    .map(|chunk| {
                        let mut arr = [0u8; 8];
                        arr[..chunk.len()].copy_from_slice(chunk);
                        BFieldElement::new(u64::from_le_bytes(arr))
                    })
                    .collect();

                let (utxo, sender_randomness, _payment_id) = match key_ctx.spending_key.decrypt(&ciphertext) {
                    Ok(result) => {
                        log::debug!(
                            "[Scan] Decrypted announcement at block={} | key_type={}",
                            block_height, key_ctx.label
                        );
                        result
                    }
                    Err(_) => continue,
                };

                let incoming_utxo = IncomingUtxo {
                    utxo: utxo.clone(),
                    sender_randomness,
                    receiver_preimage: key_ctx.receiver_preimage,
                    is_guesser_fee: false,
                    payment_id: BFieldElement::new(0),
                };

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

    let mut final_seen: HashSet<String> = HashSet::new();
    utxo_infos.retain(|u| {
        let key = if !u.addition_record_hex.is_empty() {
            u.addition_record_hex.clone()
        } else {
            u.utxo_hash.clone()
        };
        if key.is_empty() {
            return false;
        }
        if final_seen.contains(&key) {
            false
        } else {
            final_seen.insert(key);
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
        .or_else(|| {
            std::env::var("VITE_APP_API")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
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
        .map(|utxo| {
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
            UtxoInfo {
                block_height: utxo.block_height,
                amount: format!("{}", amount),
                addition_record_hex: format!(
                    "0x{}",
                    hex::encode(bincode::serialize(&addition_record).unwrap_or_default())
                ),
                utxo_digest,
                utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                is_timelocked,
                release_date_ms,
                key_type: String::new(),
            }
        })
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
        .map(|utxo| {
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
            UtxoInfo {
                block_height: utxo.block_height,
                amount: format!("{}", amount),
                addition_record_hex: format!(
                    "0x{}",
                    hex::encode(bincode::serialize(&addition_record).unwrap_or_default())
                ),
                utxo_digest,
                utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                is_timelocked,
                release_date_ms,
                key_type: String::new(),
            }
        })
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

