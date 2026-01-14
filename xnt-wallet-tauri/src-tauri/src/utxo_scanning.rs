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

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanUtxosRequest {
    pub spending_key_hex: String,
    pub start_block: u64,
    pub end_block: u64,
    pub rpc_url: Option<String>,
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
    pub utxo_hash: String,
    pub is_timelocked: bool,
    pub release_date_ms: Option<u64>,
}

fn extract_rpc_result(method: &str, response: &serde_json::Value) -> Result<serde_json::Value, String> {
    if let Some(error) = response.get("error") {
        return Err(format!("RPC error for {}: {}", method, error));
    }
    
    response
        .get("result")
        .ok_or_else(|| format!("Missing result in RPC response for {}", method))
        .map(|v| v.clone())
}

async fn get_current_mutator_set_accumulator(
    client: &Client,
    rpc_url: &str,
) -> Result<MutatorSetAccumulator, String> {
    let response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getArchivalMutatorSet",
            "params": [],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get mutator set accumulator: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read mutator set accumulator response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Non-success status {}: {}", status, response_text));
    }

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse mutator set accumulator response: {}", e))?;

    let result = extract_rpc_result("archival_getArchivalMutatorSet", &response_json)?;
    
    let msa_response: GetArchivalMutatorSetResponse = serde_json::from_value(result)
        .map_err(|e| format!("Failed to deserialize mutator set accumulator: {}", e))?;

    Ok(msa_response.archival_mutator_set.into())
}

async fn get_spent_status_from_rpc(
    client: &Client,
    rpc_url: &str,
    absolute_index_set_hashes: &[Digest],
) -> Result<Vec<Option<u64>>, String> {
    let hashes_vec: Vec<String> = absolute_index_set_hashes
        .iter()
        .map(|d| d.to_hex())
        .collect();
    
    let response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getSpentStatus",
            "params": {
                "absoluteIndexSetHashes": hashes_vec
            },
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get spent status: {}", e))?;

    let status = response.status();
    let response_text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read spent status response: {}", e))?;

    if !status.is_success() {
        return Err(format!("Non-success status {}: {}", status, response_text));
    }

    let response_json: serde_json::Value = serde_json::from_str(&response_text)
        .map_err(|e| format!("Failed to parse spent status response: {}", e))?;

    let result = extract_rpc_result("archival_getSpentStatus", &response_json)?;
    
    let spent_at_heights: Vec<Option<u64>> = result
        .get("spentAtHeights")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing spentAtHeights".to_string())?
        .iter()
        .map(|v| {
            if v.is_null() {
                None
            } else {
                v.as_u64()
            }
        })
        .collect();

    Ok(spent_at_heights)
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
    let aocl_response = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getAoclLeafIndices",
            "params": {
                "commitments": commitments
            },
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get AOCL indices: {}", e))?;
    
    let aocl_status = aocl_response.status();
    let aocl_text = aocl_response
        .text()
        .await
        .map_err(|e| format!("Failed to read AOCL response: {}", e))?;
    
    if !aocl_status.is_success() {
        return Err(format!("Failed to get AOCL indices: status {}", aocl_status));
    }
    
    let aocl_json: serde_json::Value = serde_json::from_str(&aocl_text)
        .map_err(|e| format!("Failed to parse AOCL response: {}", e))?;
    
    let aocl_result = extract_rpc_result("archival_getAoclLeafIndices", &aocl_json)?;
    let indices = aocl_result
        .get("indices")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing indices".to_string())?;
    
    let mut absolute_index_set_hashes: Vec<Digest> = Vec::new();
    let mut utxo_to_hash_map: Vec<usize> = Vec::new();
    
    for (i, (utxo_idx, utxo_hash, sender_randomness, receiver_preimage)) in utxo_data.iter().enumerate() {
        if let Some(aocl_index) = indices.get(i).and_then(|v| v.as_u64()) {
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
    
    let spent_statuses = get_spent_status_from_rpc(client, rpc_url, &absolute_index_set_hashes).await?;
    
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

pub async fn scan_utxos_client_side(
    app: &AppHandle,
    request: ScanUtxosRequest,
) -> Result<String, String> {
    log::debug!(
        "[UTXO Scan] start_block={} end_block={} rpc_url={}",
        request.start_block,
        request.end_block,
        request.rpc_url.as_deref().unwrap_or("<default>")
    );

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

    let spending_key_bytes = hex::decode(
        request.spending_key_hex.trim_start_matches("0x")
    )
    .map_err(|e| format!("Invalid spending key hex: {}", e))?;
    
    let spending_key: SpendingKey = bincode::deserialize(&spending_key_bytes)
        .map_err(|e| format!("Failed to deserialize spending key: {}", e))?;
    
    let generation_spending_key = match spending_key {
        SpendingKey::Generation(key) => key,
        SpendingKey::Symmetric(_) => {
            return Err("Symmetric spending keys not supported".to_string());
        }
    };
    
    use neptune_privacy::prelude::twenty_first::prelude::Tip5;
    let receiver_identifier = generation_spending_key.receiver_identifier();
    let receiver_id_hash = Tip5::hash_varlen(&[receiver_identifier]);
    let receiver_id_hash_hex = receiver_id_hash.to_hex();
    let receiver_preimage = generation_spending_key.receiver_preimage();
    
    let force_rescan = request.start_block == 0;

    if force_rescan {
        utxo_cache::clear_cache().await;
    }

    let mut validated_cached_utxos: Vec<utxo_cache::CachedUtxo> = Vec::new();
    if !force_rescan {
        let cached_utxos = utxo_cache::get_all_utxos().await;
        match validate_utxos_with_rpc(&client, &rpc_url, &cached_utxos).await {
            Ok(validated) => {
                validated_cached_utxos = validated;
                log::info!("[UTXO Scan] Validated {} cached UTXOs using archival_getSpentStatus", validated_cached_utxos.len());
            }
            Err(e) => {
                log::warn!("[UTXO Scan] Failed to validate cached UTXOs using archival_getSpentStatus: {}. Skipping cached UTXOs.", e);
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
                    hex::encode(bincode::serialize(&addition_record).unwrap())
                ),
                utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                is_timelocked,
                release_date_ms,
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
        
        let progress = ScanProgress {
            current_block,
            end_block: request.end_block,
            start_block: request.start_block,
            utxos_found: utxo_infos.len(),
            percentage,
            status: format!("Scanning blocks {}-{}...", current_block, batch_end),
        };
        let _ = app.emit("utxo-scan-progress", &progress);
        
        let mut retries = 3;
        let mut response_result = None;
        
        while retries > 0 {
            let request_body = serde_json::json!({
                "jsonrpc": "2.0",
                "method": "archival_getUtxosByReceiver",
                "params": {
                    "receiverIdHash": receiver_id_hash_hex,
                    "fromBlockHeight": current_block,
                    "toBlockHeight": batch_end
                },
                "id": 1
            });
            
            let request_start_time = std::time::Instant::now();
            let request_body_str = serde_json::to_string(&request_body)
                .unwrap_or_else(|_| "Failed to serialize".to_string());
            
            log::info!(
                "[UTXO Scan] [REQUEST] Blocks {}-{} | Retry {}/3 | URL: {}",
                current_block,
                batch_end,
                4 - retries,
                rpc_url
            );
            log::info!(
                "[UTXO Scan] [REQUEST BODY] {}",
                request_body_str
            );

            match client
                .post(&rpc_url)
                .json(&request_body)
                .send()
                .await
            {
                Ok(resp) => {
                    let request_duration = request_start_time.elapsed();
                    let status = resp.status();
                    let response_start_time = std::time::Instant::now();
                    
                    let response_text = match resp.text().await {
                        Ok(text) => {
                            let response_duration = response_start_time.elapsed();
                            let total_request_time = request_start_time.elapsed();
                            
                            log::info!(
                                "[UTXO Scan] [RESPONSE] Blocks {}-{} | Status: {} | Request time: {:.2}ms | Response read time: {:.2}ms | Total time: {:.2}ms | Body size: {} bytes",
                                current_block,
                                batch_end,
                                status,
                                request_duration.as_secs_f64() * 1000.0,
                                response_duration.as_secs_f64() * 1000.0,
                                total_request_time.as_secs_f64() * 1000.0,
                                text.len()
                            );
                            
                            if text.len() < 10000 {
                                log::info!(
                                    "[UTXO Scan] [RESPONSE BODY] {}",
                                    text
                                );
                            } else {
                                log::info!(
                                    "[UTXO Scan] [RESPONSE BODY] (truncated, {} bytes) First 500 chars: {}",
                                    text.len(),
                                    text.chars().take(500).collect::<String>()
                                );
                            }
                            
                            text
                        }
                        Err(e) => {
                            log::warn!(
                                "[UTXO Scan] [ERROR] Failed to read response body: {}",
                                e
                            );
                            retries -= 1;
                            if retries == 0 {
                                response_result = None;
                                break;
                            }
                            continue;
                        }
                    };
                    
                    response_result = Some((status, response_text));
                    break;
                }
                Err(e) => {
                    let request_duration = request_start_time.elapsed();
                    retries -= 1;
                    log::warn!(
                        "[UTXO Scan] [ERROR] Blocks {}-{} | Request failed after {:.2}ms | Error: {} | Remaining retries: {}",
                        current_block,
                        batch_end,
                        request_duration.as_secs_f64() * 1000.0,
                        e,
                        retries
                    );
                    if retries == 0 {
                        log::warn!(
                            "[UTXO Scan] [ERROR] Giving up on batch {}-{} after all retries",
                            current_block,
                            batch_end
                        );
                        response_result = None;
                        break;
                    }
                    let delay_ms = 1000 * (3 - retries);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
        
        let (status, response_text) = match response_result {
            Some((s, t)) => (s, t),
            None => {
                log::warn!(
                    "[UTXO Scan] [ERROR] No response for batch {}-{} (all retries failed)",
                    current_block,
                    batch_end
                );
                current_block = batch_end + 1;
                continue;
            }
        };
        
        if !status.is_success() {
            log::warn!(
                "[UTXO Scan] Non-success status {} for batch {}-{} body_prefix={}",
                status,
                current_block,
                batch_end,
                response_text.chars().take(200).collect::<String>()
            );
            current_block = batch_end + 1;
            continue;
        }
        
        if response_text.trim().is_empty() {
            log::warn!(
                "[UTXO Scan] Empty response for batch {}-{}",
                current_block,
                batch_end
            );
            current_block = batch_end + 1;
            continue;
        }
        
        let response_json: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(json) => json,
            Err(e) => {
                log::warn!("[UTXO Scan] Failed to parse batch response (response: {}): {}. Skipping batch.", 
                    response_text.chars().take(200).collect::<String>(), 
                    e
                );
                current_block = batch_end + 1;
                continue;
            }
        };
        let result = match extract_rpc_result("archival_getUtxosByReceiver", &response_json) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("[UTXO Scan] Error getting indexed UTXOs for blocks {} to {}: {}. Skipping batch.", 
                    current_block, batch_end, e);
                current_block = batch_end + 1;
                continue;
            }
        };
        
        let indexed_utxos = match result.get("utxos") {
            Some(serde_json::Value::Array(arr)) => arr,
            _ => {
                log::warn!("[UTXO Scan] Invalid utxos array in response for batch {} to {}", current_block, batch_end);
                current_block = batch_end + 1;
                continue;
            }
        };
        
        let mut cached_utxos: Vec<utxo_cache::CachedUtxo> = Vec::new();
        let mut unspent_count = 0;
        let mut last_block_height_in_batch = 0u64;
        
        for indexed_utxo in indexed_utxos {
            let block_height = indexed_utxo.get("blockHeight")
                .and_then(|v| v.as_u64())
                .unwrap_or(0);
            last_block_height_in_batch = block_height;
            
            let block_digest_hex = indexed_utxo.get("blockDigest")
                .and_then(|v| v.as_str())
                .map(|s| s.to_string())
                .unwrap_or_default();
            
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
            
            use neptune_privacy::prelude::twenty_first::prelude::BFieldElement;
            let ciphertext: Vec<BFieldElement> = ciphertext_bytes
                .chunks(8)
                .map(|chunk| {
                    let mut arr = [0u8; 8];
                    arr[..chunk.len()].copy_from_slice(chunk);
                    BFieldElement::new(u64::from_le_bytes(arr))
                })
                .collect();
            
            let (utxo, sender_randomness, _payment_id) = match generation_spending_key.decrypt(&ciphertext) {
                Ok(result) => result,
                Err(_) => {
                        continue;
                    }
                };
                
            use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
            let incoming_utxo = IncomingUtxo {
                utxo: utxo.clone(),
                sender_randomness,
                receiver_preimage,
                is_guesser_fee: false,
                payment_id: BFieldElement::new(0),
            };
            
            let temp_cached_utxo = utxo_cache::CachedUtxo {
                utxo: incoming_utxo.clone(),
                block_height,
            };
            
            let validated_utxos = match validate_utxos_with_rpc(&client, &rpc_url, &[temp_cached_utxo.clone()]).await {
                Ok(validated) => validated,
                Err(e) => {
                    log::warn!("[UTXO Scan] Failed to validate UTXO using archival_getSpentStatus: {}. Skipping.", e);
                                continue;
                            }
                        };
                        
            if validated_utxos.is_empty() {
                            continue;
                        }
                        
                        unspent_count += 1;
            
            for utxo in &validated_utxos {
                let utxo_ref = &utxo.utxo.utxo;
                        let amount = utxo_ref.get_native_currency_amount();
                let addition_record = utxo.utxo.addition_record();
                        let utxo_hash = Tip5::hash(utxo_ref);

                        let release_date_opt = utxo_ref.release_date();
                        let (is_timelocked, release_date_ms) = if let Some(ts) = release_date_opt {
                    (true, Some(ts.to_millis() as u64))
                        } else {
                            (false, None)
                        };

                        let amount_str = format!("{}", amount);

                        utxo_infos.push(UtxoInfo {
                    block_height: block_height,
                            amount: amount_str,
                            addition_record_hex: format!(
                                "0x{}",
                                hex::encode(bincode::serialize(&addition_record)
                                    .map_err(|e| format!("Failed to serialize addition record: {}", e))?)
                            ),
                            utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                            is_timelocked,
                            release_date_ms,
                        });
                        
                cached_utxos.push(utxo.clone());
                incoming_utxos_for_persist.push(utxo.clone());
            }
                    }
                    
                    if unspent_count > 0 {
                        log::info!("[UTXO Scan] Batch {} to {}: Found {} unspent UTXO(s)", current_block, batch_end, unspent_count);
                    }
                    
                    let cached_count = cached_utxos.len();
                    if !cached_utxos.is_empty() {
                        utxo_cache::store_utxos(cached_utxos).await;
                    }
            
            if last_block_height_in_batch > 0 {
                last_scanned = last_block_height_in_batch;
            }
            
            let batch_total_time = batch_start_time.elapsed();
            log::info!(
                "[UTXO Scan] [BATCH COMPLETE] Blocks {}-{} | Total batch time: {:.2}ms | UTXOs found: {} | Cached: {}",
                current_block,
                batch_end,
                batch_total_time.as_secs_f64() * 1000.0,
                unspent_count,
                cached_count
            );
        
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
        Ok(validated) => {
            log::info!("[Get Cached UTXOs] Validated {} UTXOs using archival_getSpentStatus", validated.len());
            validated
        }
        Err(e) => {
            log::warn!("[Get Cached UTXOs] Failed to validate using archival_getSpentStatus: {}. Returning cached UTXOs without validation.", e);
            cached_utxos
        }
    };

    utxo_cache::replace_cache_and_persist(valid_utxos.clone(), last_scanned).await;

    let utxos: Vec<UtxoInfo> = valid_utxos
        .iter()
        .map(|utxo| {
            let utxo_ref = &utxo.utxo.utxo;
            let amount = utxo_ref.get_native_currency_amount();
            let addition_record = utxo.utxo.addition_record();
            let utxo_hash = Tip5::hash(utxo_ref);
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
                utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                is_timelocked,
                release_date_ms,
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
                utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                is_timelocked,
                release_date_ms,
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
    
    let generation_spending_key = match spending_key {
        SpendingKey::Generation(key) => key,
        SpendingKey::Symmetric(_) => {
            return Ok(serde_json::to_string(&GetPendingBalanceResponse {
                success: true,
                pending_balance: "0".to_string(),
                error: Some("Symmetric spending keys not supported".to_string()),
            }).map_err(|e| format!("Failed to serialize response: {}", e))?);
        }
    };
    
    let mut pending_total = NativeCurrencyAmount::zero();
    
    let mempool_tx_ids_result = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "mempool_transactions",
            "params": [],
            "id": 1
        }))
        .send()
        .await;
    
    let tx_ids: Vec<String> = match mempool_tx_ids_result {
        Ok(response) => {
            let text = response.text().await
                .map_err(|e| format!("Failed to read mempool transactions response: {}", e))?;
            let json: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("Failed to parse mempool transactions: {}", e))?;
            
            if let Some(_error) = json.get("error") {
                return Ok(serde_json::to_string(&GetPendingBalanceResponse {
                    success: true,
                    pending_balance: "0".to_string(),
                    error: None,
                }).map_err(|e| format!("Failed to serialize response: {}", e))?);
            }
            
            let result = extract_rpc_result("mempool_transactions", &json)?;
            let arr = result
                .get("transactions")
                .ok_or_else(|| "mempool_transactions missing transactions field".to_string())?;
            
            let tx_kernel_ids: Vec<String> = serde_json::from_value(arr.clone())
                .map_err(|e| format!("Failed to deserialize transaction IDs: {}", e))?;
            tx_kernel_ids
        }
        Err(_e) => {
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
        let tx_kernel_result = client
            .post(&rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "mempool_getTransactionKernel",
                "params": [tx_id_hex],
                "id": 1
            }))
            .send()
            .await;
        
        let tx_kernel: Option<TransactionKernel> = match tx_kernel_result {
            Ok(response) => {
                let text = response.text().await
                    .map_err(|e| format!("Failed to read mempool tx kernel response: {}", e))?;
                let json: serde_json::Value = serde_json::from_str(&text)
                    .map_err(|e| format!("Failed to parse mempool tx kernel: {}", e))?;
                
                let kernel_result = extract_rpc_result("mempool_getTransactionKernel", &json)?;
                
                if kernel_result.get("kernel").is_none() || kernel_result["kernel"].is_null() {
                    continue;
                }
                
                let rpc_kernel: RpcTransactionKernel =
                    serde_json::from_value(kernel_result["kernel"].clone())
                        .map_err(|e| format!("Failed to deserialize RPC kernel: {}", e))?;

                let tx_kernel_proxy = TransactionKernelProxy {
                    inputs: vec![],
                    outputs: rpc_kernel
                        .outputs
                        .into_iter()
                        .map(|ar| ar.into())
                        .collect(),
                    announcements: rpc_kernel
                        .announcements
                        .into_iter()
                        .map(|rpc_bfes| {
                            let message: Vec<BFieldElement> = rpc_bfes.0.into();
                            Announcement::new(message)
                        })
                        .collect(),
                    fee: rpc_kernel.fee.0,
                    coinbase: rpc_kernel.coinbase.map(|c| c.0),
                    timestamp: rpc_kernel.timestamp,
                    mutator_set_hash: rpc_kernel.mutator_set_hash,
                    merge_bit: rpc_kernel.merge_bit,
                };

                Some(tx_kernel_proxy.into_kernel())
            }
            Err(_e) => {
                continue;
            }
        };
        
        if let Some(tx_kernel) = tx_kernel {
            use neptune_privacy::prelude::twenty_first::prelude::BFieldElement;
            use neptune_privacy::state::wallet::address::common::{ciphertext_from_announcement, receiver_identifier_from_announcement, key_type_from_announcement};
            use neptune_privacy::state::wallet::address::KeyType;
            use neptune_privacy::util_types::mutator_set::addition_record::AdditionRecord;
            use neptune_privacy::util_types::mutator_set::commit;
            use neptune_privacy::prelude::twenty_first::tip5::Tip5;
            
            let receiver_identifier = generation_spending_key.receiver_identifier();
            let generation_key_type_flag: BFieldElement = KeyType::Generation.into();
            let generation_subaddr_flag: BFieldElement = KeyType::GenerationSubAddr.into();

            for announcement in &tx_kernel.announcements {
                if !matches!(key_type_from_announcement(announcement), Ok(kt) if kt == generation_key_type_flag || kt == generation_subaddr_flag) {
                    continue;
                }

                if !matches!(receiver_identifier_from_announcement(announcement), Ok(rid) if rid == receiver_identifier) {
                    continue;
                }

                let ciphertext_bfes = match ciphertext_from_announcement(announcement) {
                    Ok(ct) => ct,
                    Err(_) => continue,
                };

                if let Ok((utxo, _sender_randomness, _payment_id)) = generation_spending_key.decrypt(&ciphertext_bfes) {
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

