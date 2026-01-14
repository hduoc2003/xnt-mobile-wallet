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
            }
            Err(_) => {
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
                        Ok(text) => text,
                        Err(_) => {
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
                Err(_) => {
                    retries -= 1;
                    if retries == 0 {
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
                current_block = batch_end + 1;
                continue;
            }
        };
        
        if !status.is_success() {
            current_block = batch_end + 1;
            continue;
        }
        
        if response_text.trim().is_empty() {
            current_block = batch_end + 1;
            continue;
        }
        
        let response_json: serde_json::Value = match serde_json::from_str(&response_text) {
            Ok(json) => json,
            Err(_) => {
                current_block = batch_end + 1;
                continue;
            }
        };
        let result = match rpc_client::extract_rpc_result("archival_getUtxosByReceiver", &response_json) {
            Ok(r) => r,
            Err(_) => {
                current_block = batch_end + 1;
                continue;
            }
        };
        
        let indexed_utxos = match result.get("utxos") {
            Some(serde_json::Value::Array(arr)) => arr,
            _ => {
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
                Err(_) => {
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
                    
                    if !cached_utxos.is_empty() {
                        utxo_cache::store_utxos(cached_utxos).await;
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

