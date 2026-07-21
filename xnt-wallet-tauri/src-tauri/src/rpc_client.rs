use reqwest::Client;
use serde_json::Value;
use neptune_privacy::prelude::twenty_first::tip5::digest::Digest;
use neptune_privacy::application::json_rpc::core::model::message::GetArchivalMutatorSetResponse;
use neptune_privacy::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use neptune_privacy::protocol::consensus::transaction::transaction_kernel::TransactionKernel;

pub fn extract_rpc_result(method: &str, response: &Value) -> Result<Value, String> {
    if let Some(error) = response.get("error") {
        return Err(format!("RPC error for {}: {}", method, error));
    }
    
    response
        .get("result")
        .ok_or_else(|| format!("Missing result in RPC response for {}", method))
        .map(|v| v.clone())
}

pub async fn call_json_rpc(
    client: &Client,
    rpc_url: &str,
    method: &str,
    params: Value,
) -> Result<Value, String> {
    let body = serde_json::json!({
        "jsonrpc": "2.0",
        "method": method,
        "params": params,
        "id": 1,
    });

    let response = client
        .post(rpc_url)
        .json(&body)
        .send()
        .await
        .map_err(|e| format!("Failed to call {}: {}", method, e))?;

    let status = response.status();
    let text = response
        .text()
        .await
        .map_err(|e| format!("Failed to read {} response: {}", method, e))?;

    if !status.is_success() {
        return Err(format!("Non-success status {} for {}: {}", status, method, text));
    }

    let json: Value = serde_json::from_str(&text)
        .map_err(|e| format!("Failed to parse {} response: {}", method, e))?;

    extract_rpc_result(method, &json)
}

pub async fn get_archival_mutator_set(
    client: &Client,
    rpc_url: &str,
) -> Result<MutatorSetAccumulator, String> {
    let result = call_json_rpc(client, rpc_url, "archival_getArchivalMutatorSet", serde_json::json!([])).await?;
    
    let msa_response: GetArchivalMutatorSetResponse = serde_json::from_value(result)
        .map_err(|e| format!("Failed to deserialize mutator set accumulator: {}", e))?;

    Ok(msa_response.archival_mutator_set.into())
}

pub async fn get_spent_status(
    client: &Client,
    rpc_url: &str,
    absolute_index_set_hashes: &[Digest],
) -> Result<Vec<Option<u64>>, String> {
    let hashes_vec: Vec<String> = absolute_index_set_hashes
        .iter()
        .map(|d| d.to_hex())
        .collect();
    
    let result = call_json_rpc(
        client,
        rpc_url,
        "archival_getSpentStatus",
        serde_json::json!({
            "absoluteIndexSetHashes": hashes_vec
        }),
    ).await?;
    
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

pub async fn get_aocl_leaf_indices(
    client: &Client,
    rpc_url: &str,
    commitments: &[String],
) -> Result<Vec<Option<u64>>, String> {
    let result = call_json_rpc(
        client,
        rpc_url,
        "archival_getAoclLeafIndices",
        serde_json::json!({
            "commitments": commitments
        }),
    ).await?;
    
    let indices = result
        .get("indices")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing indices".to_string())?
        .iter()
        .map(|v| v.as_u64())
        .collect();

    Ok(indices)
}

pub async fn restore_membership_proof(
    client: &Client,
    rpc_url: &str,
    absolute_index_sets: Value,
) -> Result<Value, String> {
    call_json_rpc(
        client,
        rpc_url,
        "archival_restoreMembershipProof",
        serde_json::json!([absolute_index_sets]),
    ).await
}

pub async fn get_mempool_transactions(
    client: &Client,
    rpc_url: &str,
) -> Result<Vec<String>, String> {
    let result = call_json_rpc(
        client,
        rpc_url,
        "mempool_transactions",
        serde_json::json!([]),
    ).await?;
    
    let arr = result
        .get("transactions")
        .ok_or_else(|| "mempool_transactions missing transactions field".to_string())?;
    
    let tx_ids: Vec<String> = serde_json::from_value(arr.clone())
        .map_err(|e| format!("Failed to deserialize transaction IDs: {}", e))?;
    
    Ok(tx_ids)
}

pub async fn get_mempool_transaction_kernel(
    client: &Client,
    rpc_url: &str,
    tx_id: &str,
) -> Result<Option<TransactionKernel>, String> {
    let result = call_json_rpc(
        client,
        rpc_url,
        "mempool_getTransactionKernel",
        serde_json::json!([tx_id]),
    ).await?;

    if result.get("kernel").is_none() || result["kernel"].is_null() {
        return Ok(None);
    }

    let kernel_value = result.get("kernel").unwrap();
    let rpc_kernel: neptune_privacy::application::json_rpc::core::model::block::transaction_kernel::RpcTransactionKernel =
        serde_json::from_value(kernel_value.clone())
            .map_err(|e| format!("Failed to deserialize transaction kernel: {}", e))?;

    let kernel: TransactionKernel = rpc_kernel.into();
    Ok(Some(kernel))
}

/// Returns the set of AbsoluteIndexSet hashes (hex) for every removal record across
/// all pending transactions in the mempool.  These correspond to UTXOs that are
/// currently being spent but not yet confirmed on-chain.
pub async fn get_mempool_removal_hashes(
    client: &Client,
    rpc_url: &str,
) -> Result<std::collections::HashSet<String>, String> {
    use neptune_privacy::prelude::twenty_first::prelude::Tip5;

    let tx_ids = get_mempool_transactions(client, rpc_url).await?;
    let mut spent_hashes = std::collections::HashSet::new();

    for tx_id in &tx_ids {
        let kernel = match get_mempool_transaction_kernel(client, rpc_url, tx_id).await {
            Ok(Some(k)) => k,
            Ok(None) => continue,
            Err(_) => continue,
        };
        for removal_record in &kernel.inputs {
            let hash = Tip5::hash(&removal_record.absolute_indices);
            spent_hashes.insert(hash.to_hex());
        }
    }

    Ok(spent_hashes)
}
