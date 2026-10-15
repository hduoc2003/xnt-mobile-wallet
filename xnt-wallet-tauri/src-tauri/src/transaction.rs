use neptune_privacy::api::export::{ReceivingAddress, TransactionDetails};
use neptune_privacy::api::tx_initiation::builder::transaction_builder::TransactionBuilder;
use neptune_privacy::api::tx_initiation::builder::tx_output_list_builder::OutputFormat;
use neptune_privacy::application::config::network::Network;
use neptune_privacy::prelude::twenty_first::prelude::Digest;
use neptune_privacy::protocol::consensus::transaction::primitive_witness::PrimitiveWitness;
use neptune_privacy::protocol::consensus::transaction::transaction_proof::TransactionProof;
use neptune_privacy::protocol::consensus::block::block_height::BlockHeight;
use neptune_privacy::protocol::consensus::transaction::validity::proof_collection::ProofCollection;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::protocol::proof_abstractions::timestamp::Timestamp;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
use neptune_privacy::state::wallet::transaction_input::{TxInput, TxInputList};
use neptune_privacy::state::wallet::transaction_output::{TxOutput, TxOutputList};
use neptune_privacy::state::wallet::unlocked_utxo::UnlockedUtxo;
use neptune_privacy::state::wallet::utxo_notification::UtxoNotificationMedium;
use neptune_privacy::util_types::mutator_set::ms_membership_proof::MsMembershipProof;
use neptune_privacy::util_types::mutator_set::removal_record::absolute_index_set::AbsoluteIndexSet;
use neptune_privacy::prelude::twenty_first::prelude::Tip5;
use neptune_privacy::prelude::triton_vm::config::{overwrite_lde_trace_caching_to, CacheDecision};
use neptune_privacy::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use num_traits::identities::Zero;
use num_traits::ops::checked::CheckedSub;
use rand::Rng;
use reqwest::Client;
use tauri::AppHandle;
use std::str::FromStr;
use log::error;
use neptune_privacy::application::json_rpc::core::model::wallet::mutator_set::RpcMsMembershipSnapshot;
use crate::rpc_config;

use crate::rpc_client;
use crate::utxo_cache;
use crate::utxo_scanning;

pub(crate) async fn generate_membership_proof_client_side(
    incoming_utxo: &IncomingUtxo,
    mutator_set_accumulator: &MutatorSetAccumulator,
    rpc_url: &str,
    client: &Client,
) -> Result<MsMembershipProof, String> {
    let addition_record = incoming_utxo.addition_record();
    let commitment = addition_record.canonical_commitment;
    let commitment_hex = commitment.to_hex();
    
    let indices = rpc_client::get_aocl_leaf_indices(client, rpc_url, &[commitment_hex]).await?;
    
    if indices.is_empty() || indices[0].is_none() {
        return Err("UTXO not found in AOCL (may not be confirmed yet)".to_string());
    }
    
    let aocl_leaf_index = indices[0]
        .ok_or_else(|| "UTXO not found in AOCL (may not be confirmed yet)".to_string())?;
    
    let item = Tip5::hash(&incoming_utxo.utxo);
    let absolute_index_set = AbsoluteIndexSet::compute(
        item,
        incoming_utxo.sender_randomness,
        incoming_utxo.receiver_preimage,
        aocl_leaf_index,
    );
    
    let absolute_index_set_json = serde_json::to_value(&absolute_index_set)
        .map_err(|e| format!("Failed to serialize AbsoluteIndexSet to JSON: {}", e))?;
    
    let restore_result = rpc_client::restore_membership_proof(
        client,
        rpc_url,
        serde_json::json!([absolute_index_set_json]),
    ).await?;
    
    let snapshot = restore_result
        .get("snapshot")
        .cloned()
        .ok_or_else(|| "Missing snapshot in response".to_string())?;
    let snapshot: RpcMsMembershipSnapshot = serde_json::from_value(snapshot)
        .map_err(|e| format!("Invalid membership proof snapshot: {}", e))?;
    let rpc_proof = snapshot
        .membership_proofs
        .into_iter()
        .next()
        .ok_or_else(|| "No membership proof returned".to_string())?;
    rpc_proof
        .extract_ms_membership_proof(
            aocl_leaf_index,
            incoming_utxo.sender_randomness,
            incoming_utxo.receiver_preimage,
        )
        .ok_or_else(|| format!("Missing AOCL auth path for leaf index {}", aocl_leaf_index))
}

pub async fn build_transaction(app: AppHandle, payload: serde_json::Value) -> Result<String, String> {
    let result = build_transaction_inner(&app, payload).await;
    if let Err(ref err) = result {
        error!("[BuildTx] ✖ {}", err);
    }
    result
}

/// Whether a failed mempool double-spend check may be skipped.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MempoolCheck {
    BestEffort,
    Required,
}

/// Inputs per transaction; bounds proving time and memory on phones.
pub(crate) const MAX_TX_INPUTS: usize = 5;

/// Spendable, unspent inputs named by `inputAdditionRecords`, with the key that unlocks each.
pub(crate) struct ValidatedInputs {
    pub utxos: Vec<utxo_cache::CachedUtxo>,
    pub keys: Vec<SpendingKey>,
    /// Primary spending key from `spendingKeyHex`; change goes to its address.
    pub primary_key: SpendingKey,
    pub total: NativeCurrencyAmount,
}

/// `low_memory` proves without caching the LDE trace; otherwise triton-vm decides from free memory.
pub(crate) async fn prove(
    primitive_witness: PrimitiveWitness,
    network: Network,
    block_height: BlockHeight,
    low_memory: bool,
) -> Result<ProofCollection, String> {
    if !low_memory {
        return ProofCollection::produce_in_process(&primitive_witness, network, block_height)
            .await
            .map_err(|e| format!("Failed to generate proof collection: {e}"));
    }
    // triton-vm keeps this choice per thread, and core proves on its own spawn_blocking thread.
    tokio::task::spawn_blocking(move || {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .on_thread_start(|| overwrite_lde_trace_caching_to(CacheDecision::NoCache))
            .build()
            .map_err(|e| format!("Failed to start the prover runtime: {e}"))?;
        runtime
            .block_on(ProofCollection::produce_in_process(
                &primitive_witness,
                network,
                block_height,
            ))
            .map_err(|e| format!("Failed to generate proof collection: {e}"))
    })
    .await
    .map_err(|e| format!("Prover task failed: {e}"))?
}

pub(crate) fn http_client() -> Result<Client, String> {
    Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))
}

pub(crate) async fn fetch_network(client: &Client, rpc_url: &str) -> Result<Network, String> {
    let network_result = rpc_client::call_json_rpc(client, rpc_url, "node_network", serde_json::json!([])).await?;
    let network_str = network_result.get("network")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing network in response".to_string())?;
    Network::from_str(network_str)
        .map_err(|e| format!("Failed to parse network '{}': {}", network_str, e))
}

fn parse_spending_key(hex_str: &str) -> Result<SpendingKey, String> {
    let bytes = hex::decode(hex_str.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid spending key hex: {}", e))?;
    let key: SpendingKey = bincode::deserialize(&bytes)
        .map_err(|e| format!("Failed to deserialize spending key: {}", e))?;
    if matches!(key, SpendingKey::Symmetric(_)) {
        return Err("Symmetric spending keys not supported".to_string());
    }
    Ok(key)
}

pub(crate) async fn validate_inputs(
    client: &Client,
    rpc_url: &str,
    payload: &serde_json::Value,
    mempool_check: MempoolCheck,
) -> Result<ValidatedInputs, String> {
    let spending_key_hex = payload["spendingKeyHex"]
        .as_str()
        .ok_or_else(|| "Missing spendingKeyHex in payload".to_string())?;
    
    let input_addition_records: Vec<String> = payload["inputAdditionRecords"]
        .as_array()
        .ok_or_else(|| "Missing inputAdditionRecords in payload".to_string())?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();

    // Enforce a hard limit on the number of UTXOs per transaction
    if input_addition_records.len() > MAX_TX_INPUTS {
        return Err(format!("Too many UTXOs selected for a single transaction. Maximum allowed is {MAX_TX_INPUTS}."));
    }

    let spending_key = parse_spending_key(spending_key_hex)?;

    // Optional per-input spending keys (for mixed-key transactions, e.g. merging
    // Generation + dCTIDH UTXOs).  Each element corresponds positionally to the
    // matching entry in inputAdditionRecords.
    let per_input_key_hexes: Option<Vec<String>> = payload["spendingKeyHexes"]
        .as_array()
        .map(|arr| arr.iter().filter_map(|v| v.as_str().map(|s| s.to_string())).collect());

    let key_type_label = match &spending_key {
        SpendingKey::Generation(_) => "Generation",
        SpendingKey::dCTIDH(_)    => "dCTIDH",
        SpendingKey::Symmetric(_) => "Symmetric",
    };
    log::info!(
        "[BuildTx] Validating inputs | key_type={} | per_input_keys={} | inputs={}",
        key_type_label,
        per_input_key_hexes.as_ref().map(|v| v.len()).unwrap_or(0),
        input_addition_records.len(),
    );
    // Get UTXOs from cache instead of calling server
    let matched_utxos = utxo_cache::get_utxos_by_addition_records(&input_addition_records).await;
    
    if matched_utxos.len() != input_addition_records.len() {
        return Err(format!(
            "Not all UTXOs found in cache. Found {}/{} UTXOs. Please scan UTXOs first.",
            matched_utxos.len(),
            input_addition_records.len()
        ));
    }
    
    // Validate UTXOs using archival_getSpentStatus RPC before building transaction
    let validated_utxos = utxo_scanning::validate_utxos_with_rpc(client, rpc_url, &matched_utxos).await
        .map_err(|e| format!("Failed to validate UTXOs: {}. Some UTXOs may have been spent.", e))?;

    if validated_utxos.len() != matched_utxos.len() {
        return Err(format!(
            "Some UTXOs have been spent. Validated {}/{} UTXOs. Please refresh and try again.",
            validated_utxos.len(),
            matched_utxos.len()
        ));
    }

    // Additionally filter out UTXOs that are currently referenced as inputs in
    // pending mempool transactions.  archival_getSpentStatus only covers confirmed
    // blocks, so without this check a UTXO that is already in an unconfirmed tx
    // could be selected again, causing a double-spend attempt.
    let validated_utxos = match utxo_scanning::filter_utxos_spent_in_mempool(client, rpc_url, &validated_utxos).await {
        Ok(filtered) => {
            if filtered.len() != validated_utxos.len() {
                return Err(format!(
                    "{} of your selected UTXOs are already referenced in a pending mempool transaction. \
                     Wait for that transaction to confirm or be dropped, then rescan and try again.",
                    validated_utxos.len() - filtered.len()
                ));
            }
            filtered
        }
        Err(e) if mempool_check == MempoolCheck::Required => {
            return Err(format!("Could not check the mempool for double spends: {}", e));
        }
        Err(e) => {
            // If we cannot reach the mempool endpoint, log a warning but do not
            // block the transaction – the node will reject any actual double-spend.
            log::warn!("[BuildTx] Mempool UTXO filter failed (proceeding anyway): {}", e);
            validated_utxos
        }
    };
    
    let mut total_input_amount = NativeCurrencyAmount::zero();
    for utxo in &matched_utxos {
        total_input_amount = total_input_amount + utxo.utxo.utxo.get_native_currency_amount();
    }

    // Build per-input spending keys: use spendingKeyHexes if provided (mixed-key
    // transaction), otherwise repeat the primary spending_key for every input.
    let per_input_keys: Vec<SpendingKey> = match per_input_key_hexes {
        Some(key_hexes) => {
            if key_hexes.len() != validated_utxos.len() {
                return Err(format!(
                    "spendingKeyHexes count ({}) must match inputAdditionRecords count ({})",
                    key_hexes.len(),
                    validated_utxos.len()
                ));
            }
            key_hexes
                .iter()
                .map(|hex_str| parse_spending_key(hex_str))
                .collect::<Result<Vec<_>, String>>()?
        }
        None => vec![spending_key.clone(); validated_utxos.len()],
    };

    Ok(ValidatedInputs {
        utxos: validated_utxos,
        keys: per_input_keys,
        primary_key: spending_key,
        total: total_input_amount,
    })
}

/// Generates membership proofs client-side and unlocks each input with its own key.
pub(crate) async fn unlock_inputs(
    client: &Client,
    rpc_url: &str,
    inputs: &ValidatedInputs,
    mutator_set_accumulator: &MutatorSetAccumulator,
) -> Result<Vec<TxInput>, String> {
    let mut tx_inputs = Vec::new();
    for (incoming_utxo, input_key) in inputs.utxos.iter().zip(inputs.keys.iter()) {
        let lsaw = input_key.lock_script_and_witness();
        let msmp = generate_membership_proof_client_side(
            &incoming_utxo.utxo,
            mutator_set_accumulator,
            rpc_url,
            client,
        ).await?;

        tx_inputs.push(TxInput::from(UnlockedUtxo::unlock(
            incoming_utxo.utxo.utxo.clone(),
            lsaw,
            msmp,
        )));
    }
    Ok(tx_inputs)
}

async fn build_transaction_inner(app: &AppHandle, payload: serde_json::Value) -> Result<String, String> {
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    
    let _min_block_height = payload["minBlockHeight"].as_i64().unwrap_or(0);
    
    let output_addresses: Vec<String> = payload["outputAddresses"]
        .as_array()
        .ok_or_else(|| "Missing outputAddresses in payload".to_string())?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    
    let output_amounts: Vec<String> = payload["outputAmounts"]
        .as_array()
        .ok_or_else(|| "Missing outputAmounts in payload".to_string())?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    
    let transaction_timestamp = Timestamp::now();
    let timelock_release_date: Option<Timestamp> = payload.get("timelockReleaseDate")
        .and_then(|v| {
            if let Some(ts_ms) = v.as_u64() {
                let release_date = Timestamp::millis(ts_ms);
                if release_date > transaction_timestamp {
                    Some(release_date)
                } else {
                    None
                }
            } else if let Some(ts_str) = v.as_str() {
                ts_str.parse::<u64>()
                    .ok()
                    .and_then(|ts_ms| {
                        let release_date = Timestamp::millis(ts_ms);
                        if release_date > transaction_timestamp {
                            Some(release_date)
                        } else {
                            None
                        }
                    })
            } else {
                None
            }
        });
    
    let fee = payload["fee"]
        .as_str()
        .ok_or_else(|| "Missing fee in payload".to_string())?;
    let low_memory_proving = payload["lowMemoryProving"]
        .as_bool()
        .ok_or("Missing lowMemoryProving in payload")?;
    
    let client = http_client()?;
    let network = fetch_network(&client, &rpc_url).await?;
    let inputs = validate_inputs(&client, &rpc_url, &payload, MempoolCheck::BestEffort).await?;
    let spending_key = inputs.primary_key.clone();
    let total_input_amount = inputs.total;
    log::info!(
        "[BuildTx] Building transaction | inputs={} | outputs={} | fee={}",
        inputs.utxos.len(),
        output_addresses.len(),
        fee,
    );
    
    let fee_amount = NativeCurrencyAmount::coins_from_str(fee)
        .map_err(|e| format!("Invalid fee: {}", e))?;
    
    let mut validated_outputs: Vec<OutputFormat> = output_addresses
        .iter()
        .zip(output_amounts.iter())
        .map(|(addr, amt)| {
            Ok(OutputFormat::AddressAndAmountAndMedium(
                ReceivingAddress::from_bech32m(addr, network)
                    .map_err(|e| format!("Failed to parse address {}: {}", addr, e))?,
                NativeCurrencyAmount::coins_from_str(amt)
                    .map_err(|e| format!("Invalid amount {}: {}", amt, e))?,
                UtxoNotificationMedium::OnChain,
            ))
        })
        .collect::<Result<Vec<_>, String>>()?;
    
    let total_output_amount = fee_amount + validated_outputs.iter()
        .map(|o| o.native_currency_amount())
        .sum::<NativeCurrencyAmount>();
    
    let change_amount = total_input_amount
        .checked_sub(&total_output_amount)
        .ok_or_else(|| format!("Insufficient funds: inputs={}, outputs+fee={}", total_input_amount, total_output_amount))?;
    
    if !change_amount.is_zero() {
        validated_outputs.push(OutputFormat::AddressAndAmountAndMedium(
            spending_key.clone().to_address(),
            change_amount,
            UtxoNotificationMedium::OnChain,
        ));
    }
    
    let mutator_set_accumulator = rpc_client::get_archival_mutator_set(&client, &rpc_url).await?;
    let tx_inputs = unlock_inputs(&client, &rpc_url, &inputs, &mutator_set_accumulator).await?;
    
    let my_address: ReceivingAddress = spending_key.clone().to_address();
    let tx_outputs: Vec<TxOutput> = validated_outputs
        .into_iter()
        .map(|output| {
            let (address, amount) = match output {
                OutputFormat::AddressAndAmountAndMedium(addr, amt, _) => (addr, amt),
                _ => return Err("Invalid output format".to_string()),
            };
            
            let sender_randomness = rand::rng().random::<Digest>();
            let is_change = address == my_address && amount == change_amount;
            
            let mut tx_output = if is_change {
                TxOutput::onchain_native_currency_as_change(amount, sender_randomness, address)
            } else {
                TxOutput::onchain_native_currency(amount, sender_randomness, address, false)
            };
            
            if !is_change {
                if let Some(release_date) = timelock_release_date {
                    tx_output = tx_output.with_time_lock(release_date);
                }
            }
            
            Ok(tx_output)
        })
        .collect::<Result<Vec<_>, String>>()?;
    
    let tx_details = TransactionDetails::new_without_coinbase(
        TxInputList::from(tx_inputs),
        TxOutputList::from(tx_outputs),
        fee_amount,
        Timestamp::now(),
        mutator_set_accumulator,
        network,
    );
    
    let primitive_witness: PrimitiveWitness = tx_details.primitive_witness();
    let kernel = primitive_witness.kernel.clone();
    let block_height = rpc_client::get_chain_height(&client, &rpc_url).await?;
    let proof_collection = prove(
        primitive_witness,
        network,
        BlockHeight::from(block_height),
        low_memory_proving,
    )
    .await?;
    
    let proof = TransactionProof::ProofCollection(proof_collection);
    let transaction = TransactionBuilder::new()
        .transaction_kernel(kernel)
        .transaction_proof(proof)
        .build()
        .map_err(|e| format!("Failed to build transaction: {}", e))?;
    
    // Compute output UTXO digests from transaction kernel outputs (client-side)
    // Output UTXO digest = canonical_commitment from AdditionRecord
    let output_utxo_digests: Vec<String> = transaction.kernel.outputs
        .iter()
        .map(|addition_record| addition_record.canonical_commitment.to_hex())
        .collect();
    
    let tx_id = transaction.txid().to_string();

    // Convert Transaction to RpcTransaction for JSON-RPC
    use neptune_privacy::application::json_rpc::core::model::wallet::transaction::RpcTransaction;
    let rpc_transaction: RpcTransaction = transaction.into();
    
    // Serialize RpcTransaction to JSON
    let transaction_json = serde_json::to_value(&rpc_transaction)
        .map_err(|e| format!("Failed to serialize RpcTransaction: {}", e))?;
    
    Ok(serde_json::json!({
        "success": true,
        "transaction": transaction_json,
        "outputUtxoDigests": output_utxo_digests,
        "txId": tx_id,
    })
    .to_string())
}
