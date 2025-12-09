use neptune_privacy::prelude::twenty_first::math::b_field_element::BFieldElement;
use neptune_privacy::prelude::triton_vm::prelude::Tip5;
use neptune_privacy::protocol::consensus::transaction::announcement::Announcement;
use neptune_privacy::protocol::consensus::transaction::transaction_kernel::{TransactionKernel, TransactionKernelProxy};
use neptune_privacy::state::wallet::address::view_key::ViewKey;
use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::protocol::proof_abstractions::timestamp::Timestamp;
use neptune_privacy::prelude::twenty_first::tip5::digest::Digest;
use neptune_privacy::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use neptune_privacy::application::json_rpc::core::model::message::GetArchivalMutatorSetResponse;
use num_traits::Zero;
use reqwest::Client;
use serde::{Deserialize, Serialize};

use crate::utxo_cache;
use crate::transaction;

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScanUtxosRequest {
    pub view_key_hex: String,
    pub start_block: u64,
    pub end_block: u64,
    pub rpc_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ScanUtxosResponse {
    pub success: bool,
    pub utxos: Vec<UtxoInfo>,
    pub last_scanned_block: u64,
    pub error: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UtxoInfo {
    pub block_height: u64,
    pub amount: String,
    pub addition_record_hex: String,
    pub utxo_hash: String,
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

    Ok(msa_response.archival_mutator_set)
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
    request: ScanUtxosRequest,
) -> Result<String, String> {
    // Create client with timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let rpc_url = request.rpc_url;
    
    // Deserialize view key (stays on device!)
    let view_key_bytes = hex::decode(
        request.view_key_hex.trim_start_matches("0x")
    )
    .map_err(|e| format!("Invalid view key hex: {}", e))?;
    
    let view_key: ViewKey = bincode::deserialize(&view_key_bytes)
        .map_err(|e| format!("Failed to deserialize view key: {}", e))?;
    
    // Get current mutator set accumulator to verify UTXOs are unspent
    let mutator_set_accumulator = get_current_mutator_set_accumulator(&client, &rpc_url).await?;
    
    let mut all_utxos = Vec::new();
    let mut last_scanned = request.start_block;
    
    // Scan blocks in batches using the new range endpoint
    let batch_size = 1000; // Request 1000 blocks per batch
    let mut current_block = request.start_block;
    
    // timing accumulators removed
    
    while current_block <= request.end_block {
        let batch_end = std::cmp::min(current_block + batch_size - 1, request.end_block);
        
        // Request announcements for the entire range in one call
        let mut retries = 3;
        let mut response_result = None;
        
        while retries > 0 {
            match client
                .post(&rpc_url)
                .json(&serde_json::json!({
                    "jsonrpc": "2.0",
                    "method": "archival_getBlockAnnouncementsRange",
                    "params": [current_block, batch_end],
                    "id": 1
                }))
                .send()
                .await
            {
                Ok(resp) => {
                    response_result = Some(resp);
                    break;
                }
                Err(e) => {
                    retries -= 1;
                    if retries == 0 {
                        log::warn!("[UTXO Scan] Batch request failed for blocks {} to {} after retries: {}. Skipping batch.", 
                            current_block, batch_end, e);
                        response_result = None;
                        break;
                    }
                    // Wait before retry (exponential backoff)
                    let delay_ms = 1000 * (3 - retries);
                    tokio::time::sleep(tokio::time::Duration::from_millis(delay_ms)).await;
                }
            }
        }
        
        let response = match response_result {
            Some(r) => r,
            None => {
                // Batch failed, skip to next batch
                current_block = batch_end + 1;
                continue;
            }
        };
        
        let status = response.status();
        let response_text = match response.text().await {
            Ok(text) => text,
            Err(e) => {
                log::warn!("[UTXO Scan] Failed to read batch response for blocks {} to {}: {}. Skipping batch.", 
                    current_block, batch_end, e);
                current_block = batch_end + 1;
                continue;
            }
        };
        
        if !status.is_success() {
            log::warn!("[UTXO Scan] Non-success status {} for batch {} to {}: {}", 
                status, current_block, batch_end, response_text);
            current_block = batch_end + 1;
            continue;
        }
        
        if response_text.trim().is_empty() {
            log::warn!("[UTXO Scan] Empty response for batch {} to {}", current_block, batch_end);
            current_block = batch_end + 1;
            continue;
        }
        
        // Parse batch response
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
        // Extract result from RPC response
        let result = match extract_rpc_result("archival_getBlockAnnouncementsRange", &response_json) {
            Ok(r) => r,
            Err(e) => {
                log::warn!("[UTXO Scan] Error getting batch announcements for blocks {} to {}: {}. Skipping batch.", 
                    current_block, batch_end, e);
                current_block = batch_end + 1;
                continue;
            }
        };
        
        // Process each block in the batch response
        let blocks_array = match result.get("blocks") {
            Some(serde_json::Value::Array(arr)) => arr,
            _ => {
                log::warn!("[UTXO Scan] Invalid blocks array in response for batch {} to {}", current_block, batch_end);
                current_block = batch_end + 1;
                continue;
            }
        };
        
        for block_entry in blocks_array {
            let height = match block_entry.get("height") {
                Some(serde_json::Value::Number(n)) => n.as_u64().unwrap_or(0),
                _ => {
                    log::warn!("[UTXO Scan] Missing height in block entry");
                    continue;
                }
            };
            
            let announcements = match block_entry.get("announcements") {
                Some(serde_json::Value::Array(arr)) => arr,
                Some(serde_json::Value::Null) => {
                    // Block not found or no announcements
                    last_scanned = height;
                    continue;
                }
                _ => {
                    continue;
                }
            };
            
            if announcements.is_empty() {
                last_scanned = height;
                continue;
            }
            
            // Convert RPC announcements to Announcement structs
            let mut announcement_vec = Vec::new();
            let mut parse_errors = 0;
            
            for ann in announcements.iter() {
                let hex_str = match ann.as_str() {
                    Some(s) => s,
                    None => {
                        parse_errors += 1;
                        continue;
                    }
                };
                
                match parse_rpc_bfield_elements(hex_str) {
                    Ok(elements) => {
                        if elements.is_empty() {
                            parse_errors += 1;
                            continue;
                        }
                        announcement_vec.push(Announcement::new(elements));
                    }
                    Err(_e) => {
                        parse_errors += 1;
                    }
                }
            }
            
            if announcement_vec.is_empty() {
                last_scanned = height;
                continue;
            }
            
            let announcement_count = announcement_vec.len();
            
            if parse_errors > 0 && parse_errors == announcements.len() {
                log::warn!("[UTXO Scan] Block {}: Failed to parse all {} announcements", 
                    height, announcements.len());
            }
            
            // Build TransactionKernel with announcements using TransactionKernelProxy
            let tx_kernel = TransactionKernelProxy {
                inputs: vec![],
                outputs: vec![],
                announcements: announcement_vec,
                fee: NativeCurrencyAmount::zero(),
                coinbase: None,
                timestamp: Timestamp::zero(),
                mutator_set_hash: Digest::default(),
                merge_bit: false,
            }
            .into_kernel();
            
            // CLIENT-SIDE FILTERING (view key never left device!)
            let scan_result = view_key.scan_for_announced_utxos(&tx_kernel);
            match scan_result {
                Ok(incoming_utxos) => {
                    // Verify UTXOs are unspent and store them
                    let mut cached_utxos = Vec::new();
                    let mut unspent_count = 0;
                    
                    for utxo in &incoming_utxos {
                        // Verify UTXO is unspent by checking membership proof against current accumulator
                        let item = Tip5::hash(utxo.utxo());
                        
                        // Generate membership proof client-side (same as in transaction building)
                        let msmp = match transaction::generate_membership_proof_client_side(
                            utxo,
                            &mutator_set_accumulator,
                            &rpc_url,
                            &client,
                        ).await {
                            Ok(proof) => proof,
                            Err(_e) => {
                                continue; // Skip this UTXO if we can't generate proof
                            }
                        };
                        
                        // Verify the membership proof against current accumulator
                        if !mutator_set_accumulator.verify(item, &msmp) {
                            continue; // UTXO has been spent, skip it
                        }
                        
                        // UTXO is unspent, add it to results
                        unspent_count += 1;
                        let amount = utxo.utxo().get_native_currency_amount();
                        let addition_record = utxo.addition_record();
                        
                        // Compute UTXO hash using Tip5::hash
                        let utxo_hash = Tip5::hash(utxo.utxo());
                        
                        // Convert amount from nau to coins as a decimal string
                        // Use Display implementation which formats as decimal coins
                        let amount_str = format!("{}", amount);
                        
                        all_utxos.push(UtxoInfo {
                            block_height: height,
                            amount: amount_str,
                            addition_record_hex: format!(
                                "0x{}",
                                hex::encode(bincode::serialize(&addition_record)
                                    .map_err(|e| format!("Failed to serialize addition record: {}", e))?)
                            ),
                            utxo_hash: format!("0x{}", utxo_hash.to_hex()),
                        });
                        
                        cached_utxos.push(utxo.clone());
                    }
                    
                    if unspent_count > 0 {
                        log::info!("[UTXO Scan] Block {}: Found {} unspent UTXO(s)", height, unspent_count);
                    }
                    
                    // Store unspent UTXOs in cache for later use in transaction building
                    if !cached_utxos.is_empty() {
                        utxo_cache::store_utxos(cached_utxos).await;
                    }
                }
                Err(_e) => {
                    // Silently skip blocks with scanning errors
                }
            }
            
            last_scanned = height;
        }
        
        current_block = batch_end + 1;
    }
    
    let response = ScanUtxosResponse {
        success: true,
        utxos: all_utxos,
        last_scanned_block: last_scanned,
        error: None,
    };
    
    Ok(serde_json::to_string(&response)
        .map_err(|e| format!("Failed to serialize response: {}", e))?)
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct GetPendingBalanceRequest {
    pub view_key_hex: String,
    pub rpc_url: String,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct GetPendingBalanceResponse {
    pub success: bool,
    pub pending_balance: String,
    pub error: Option<String>,
}

pub async fn get_pending_balance_from_mempool(
    request: GetPendingBalanceRequest,
) -> Result<String, String> {
    // Create client with timeout
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let rpc_url = request.rpc_url;
    
    // Deserialize view key (stays on device!)
    let view_key_bytes = hex::decode(
        request.view_key_hex.trim_start_matches("0x")
    )
    .map_err(|e| format!("Invalid view key hex: {}", e))?;
    
    let view_key: ViewKey = bincode::deserialize(&view_key_bytes)
        .map_err(|e| format!("Failed to deserialize view key: {}", e))?;
    
    // Get mempool transaction IDs and kernels to scan for pending UTXOs
    // Note: This requires mempool_txIds and mempool_txKernel JSON-RPC methods
    // If these methods are not available, pending balance will be 0
    
    let mut pending_total = NativeCurrencyAmount::zero();
    
    // Get mempool transaction IDs via JSON-RPC
    let mempool_tx_ids_result = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "mempool_getMempoolTxIds",
            "params": [],
            "id": 1
        }))
        .send()
        .await;
    
    let tx_ids: Vec<String> = match mempool_tx_ids_result {
        Ok(response) => {
            let text = response.text().await
                .map_err(|e| format!("Failed to read mempool tx IDs response: {}", e))?;
            let json: serde_json::Value = serde_json::from_str(&text)
                .map_err(|e| format!("Failed to parse mempool tx IDs: {}", e))?;
            
            // Check for RPC errors
            if let Some(error) = json.get("error") {
                log::warn!("Mempool tx IDs method error: {}", error);
                return Ok(serde_json::to_string(&GetPendingBalanceResponse {
                    success: true,
                    pending_balance: "0".to_string(),
                    error: Some(format!("Mempool method error: {}", error)),
                }).map_err(|e| format!("Failed to serialize response: {}", e))?);
            }
            
            let result = extract_rpc_result("mempool_getMempoolTxIds", &json)?;
            serde_json::from_value(result)
                .map_err(|e| format!("Failed to deserialize tx IDs: {}", e))?
        }
        Err(e) => {
            // Request failed
            log::warn!("Failed to get mempool tx IDs: {}", e);
            return Ok(serde_json::to_string(&GetPendingBalanceResponse {
                success: true,
                pending_balance: "0".to_string(),
                error: Some(format!("Failed to get mempool tx IDs: {}", e)),
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
    
    // Get transaction kernels for each mempool transaction
    for tx_id_hex in tx_ids {
        let tx_kernel_result = client
            .post(&rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "mempool_getMempoolTxKernel",
                "params": {
                    "txId": tx_id_hex
                },
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
                
                let kernel_result = extract_rpc_result("mempool_getMempoolTxKernel", &json)?;
                
                // Check if transaction kernel exists (might be null if transaction was already mined)
                if kernel_result.get("kernel").is_none() || kernel_result["kernel"].is_null() {
                    continue;
                }
                
                // The kernel is returned as an RpcTransactionKernel object
                // We need to convert it to TransactionKernel
                let rpc_kernel: crate::application::json_rpc::core::model::block::transaction_kernel::RpcTransactionKernel = 
                    serde_json::from_value(kernel_result["kernel"].clone())
                        .map_err(|e| format!("Failed to deserialize RPC kernel: {}", e))?;
                
                // Convert RpcTransactionKernel to TransactionKernel using TransactionKernelProxy
                // For scanning, we only need announcements, so we can simplify the conversion
                use neptune_privacy::protocol::consensus::transaction::transaction_kernel::TransactionKernelProxy;
                
                let tx_kernel_proxy = TransactionKernelProxy {
                    inputs: vec![], // We skip inputs for scanning - we only need announcements
                    outputs: rpc_kernel.outputs.into_iter().map(|ar| ar.into()).collect(),
                    announcements: rpc_kernel.announcements.into_iter().map(|ann| {
                        use neptune_privacy::protocol::consensus::transaction::announcement::Announcement;
                        Announcement::new(ann.0)
                    }).collect(),
                    fee: rpc_kernel.fee.into(),
                    coinbase: rpc_kernel.coinbase.map(|c| c.into()),
                    timestamp: rpc_kernel.timestamp,
                    mutator_set_hash: rpc_kernel.mutator_set_hash,
                    merge_bit: rpc_kernel.merge_bit,
                };
                
                Some(tx_kernel_proxy.into_kernel())
            }
            Err(_e) => {
                // Skip this transaction if we can't get its kernel
                continue;
            }
        };
        
        if let Some(tx_kernel) = tx_kernel {
            // CLIENT-SIDE SCANNING (view key never left device!)
            match view_key.scan_for_announced_utxos(&tx_kernel) {
                Ok(incoming_utxos) => {
                    // Sum up amounts from pending incoming UTXOs
                    for utxo in incoming_utxos {
                        pending_total = pending_total + utxo.utxo().get_native_currency_amount();
                    }
                }
                Err(_e) => {
                    // Silently skip transactions that can't be scanned
                    continue;
                }
            }
        }
    }
    
    // Convert to decimal string (coins)
    let pending_balance_str = format!("{}", pending_total);
    
    Ok(serde_json::to_string(&GetPendingBalanceResponse {
        success: true,
        pending_balance: pending_balance_str,
        error: None,
    }).map_err(|e| format!("Failed to serialize response: {}", e))?)
}

