use neptune_privacy::api::export::{ReceivingAddress, TransactionDetails};
use neptune_privacy::api::tx_initiation::builder::transaction_builder::TransactionBuilder;
use neptune_privacy::api::tx_initiation::builder::tx_output_list_builder::OutputFormat;
use neptune_privacy::application::config::network::Network;
use neptune_privacy::prelude::twenty_first::prelude::Digest;
use neptune_privacy::protocol::consensus::Hash;
use neptune_privacy::protocol::consensus::transaction::primitive_witness::PrimitiveWitness;
use neptune_privacy::protocol::consensus::transaction::transaction_proof::TransactionProof;
use neptune_privacy::protocol::consensus::transaction::validity::collect_lock_scripts::{
    CollectLockScripts, CollectLockScriptsWitness,
};
use neptune_privacy::protocol::consensus::transaction::validity::collect_type_scripts::{
    CollectTypeScripts, CollectTypeScriptsWitness,
};
use neptune_privacy::protocol::consensus::transaction::validity::kernel_to_outputs::{
    KernelToOutputs, KernelToOutputsWitness,
};
use neptune_privacy::protocol::consensus::transaction::validity::neptune_proof::Proof as NeptuneProof;
use neptune_privacy::protocol::consensus::transaction::validity::proof_collection::ProofCollection;
use neptune_privacy::protocol::consensus::transaction::validity::removal_records_integrity::{
    RemovalRecordsIntegrity, RemovalRecordsIntegrityWitness,
};
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
use neptune_privacy::util_types::mutator_set::removal_record::chunk::Chunk;
use neptune_privacy::util_types::mutator_set::removal_record::chunk_dictionary::ChunkDictionary;
use neptune_privacy::prelude::twenty_first::prelude::Tip5;
use neptune_privacy::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use neptune_privacy::prelude::twenty_first::util_types::mmr::mmr_membership_proof::MmrMembershipProof;
use num_traits::identities::Zero;
use num_traits::ops::checked::CheckedSub;
use rand::Rng;
use reqwest::Client;
use tauri::AppHandle;
use std::str::FromStr;
use log::error;
use neptune_privacy::prelude::triton_vm::prelude::{Program, Claim, NonDeterminism, PublicInput};
use neptune_privacy::prelude::triton_vm::stark::Stark;
use neptune_privacy::prelude::triton_vm::vm::VM;
use neptune_privacy::protocol::proof_abstractions::mast_hash::MastHash;
use neptune_privacy::protocol::proof_abstractions::tasm::program::ConsensusProgram;
use neptune_privacy::protocol::proof_abstractions::SecretWitness;
use crate::rpc_config;

use crate::rpc_client;

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
    
    let snapshot = restore_result.get("snapshot")
        .ok_or_else(|| "Missing snapshot in response".to_string())?;
    
    let membership_proofs = snapshot.get("membershipProofs")
        .and_then(|v| v.as_array())
        .ok_or_else(|| "Missing membershipProofs array".to_string())?;
    
    if membership_proofs.is_empty() {
        return Err("No membership proof returned".to_string());
    }
    
    let rpc_proof = &membership_proofs[0];
    
    
    let aocl_auth_paths_value = rpc_proof.get("aoclAuthPaths")
        .ok_or_else(|| {
            log::error!("[BuildTx] rpc_proof structure: {}", serde_json::to_string_pretty(rpc_proof).unwrap_or_default());
            "Missing aoclAuthPaths".to_string()
        })?;
    
    let aocl_auth_paths_map = if let Some(arr) = aocl_auth_paths_value.as_array() {
        if arr.is_empty() {
            return Err("Empty aoclAuthPaths array".to_string());
        }
        arr[0].as_object()
            .ok_or_else(|| "aoclAuthPaths array element is not an object".to_string())?
    } else {
        aocl_auth_paths_value.as_object()
            .ok_or_else(|| "aoclAuthPaths is neither array nor object".to_string())?
    };
    
    let aocl_leaf_index_str = aocl_leaf_index.to_string();
    let aocl_auth_path_array = aocl_auth_paths_map.get(&aocl_leaf_index_str)
        .and_then(|v| v.as_array())
        .ok_or_else(|| format!("Missing AOCL auth path for leaf index {}", aocl_leaf_index))?;
    
    let mut auth_path_digests = Vec::new();
    for digest_hex in aocl_auth_path_array {
        let digest_str = digest_hex.as_str()
            .ok_or_else(|| "Invalid digest format".to_string())?;
        let digest_bytes = hex::decode(digest_str.trim_start_matches("0x"))
            .map_err(|e| format!("Invalid digest hex: {}", e))?;
        let digest: Digest = bincode::deserialize(&digest_bytes)
            .map_err(|e| format!("Failed to deserialize digest: {}", e))?;
        auth_path_digests.push(digest);
    }
    
    let auth_path_aocl = MmrMembershipProof::new(auth_path_digests);
    
    let target_chunks_value = rpc_proof.get("targetChunks")
        .ok_or_else(|| "Missing targetChunks".to_string())?;
    
    let target_chunks_map = if let Some(arr) = target_chunks_value.as_array() {
        if arr.is_empty() {
            return Err("Empty targetChunks array".to_string());
        }
        arr[0].as_object()
            .ok_or_else(|| "targetChunks array element is not an object".to_string())?
    } else {
        target_chunks_value.as_object()
            .ok_or_else(|| "targetChunks is neither array nor object".to_string())?
    };
    
    let mut target_chunks = ChunkDictionary::default();
    
    for (chunk_index_str, chunk_tuple) in target_chunks_map {
        let chunk_index = chunk_index_str.parse::<u64>()
            .map_err(|e| format!("Invalid chunk index: {}", e))?;
        
        // chunk_tuple is (RpcMmrMembershipProof, Vec<u32>), serializes as array [auth_path, relative_indices]
        let chunk_tuple_array = chunk_tuple.as_array()
            .ok_or_else(|| format!("Invalid chunk tuple format for chunk {}", chunk_index))?;
        
        if chunk_tuple_array.len() != 2 {
            return Err(format!("Invalid chunk tuple length for chunk {}", chunk_index));
        }
        
        // Extract auth path (first element)
        let auth_path_array = chunk_tuple_array[0]
            .as_array()
            .ok_or_else(|| format!("Missing auth path for chunk {}", chunk_index))?;
        
        let mut chunk_auth_path_digests = Vec::new();
        for digest_hex in auth_path_array {
            let digest_str = digest_hex.as_str()
                .ok_or_else(|| "Invalid digest format".to_string())?;
            let digest_bytes = hex::decode(digest_str.trim_start_matches("0x"))
                .map_err(|e| format!("Invalid digest hex: {}", e))?;
            let digest: Digest = bincode::deserialize(&digest_bytes)
                .map_err(|e| format!("Failed to deserialize digest: {}", e))?;
            chunk_auth_path_digests.push(digest);
        }
        
        let chunk_auth_path = MmrMembershipProof::new(chunk_auth_path_digests);
        
        // Extract relative_indices (second element)
        let relative_indices_array = chunk_tuple_array[1]
            .as_array()
            .ok_or_else(|| format!("Missing relative_indices for chunk {}", chunk_index))?;
        
        let mut relative_indices = Vec::new();
        for idx in relative_indices_array {
            let idx_val = idx.as_u64()
                .ok_or_else(|| "Invalid relative index".to_string())?;
            relative_indices.push(idx_val as u32);
        }
        
        let chunk = Chunk { relative_indices };
        
        target_chunks.insert(chunk_index, (chunk_auth_path, chunk));
    }
    
    Ok(MsMembershipProof {
        sender_randomness: incoming_utxo.sender_randomness,
        receiver_preimage: incoming_utxo.receiver_preimage,
        auth_path_aocl,
        aocl_leaf_index,
        target_chunks,
    })
}

pub async fn build_transaction(app: AppHandle, payload: serde_json::Value) -> Result<String, String> {
    let result = build_transaction_inner(&app, payload).await;
    if let Err(ref err) = result {
        error!("[BuildTx] ✖ {}", err);
    }
    result
}

async fn build_transaction_inner(app: &AppHandle, payload: serde_json::Value) -> Result<String, String> {
    let rpc_url = rpc_config::get_custom_rpc_endpoint(app)
        .or_else(|| {
            std::env::var("VITE_APP_API")
                .ok()
                .filter(|v| !v.trim().is_empty())
        })
        .unwrap_or_else(|| rpc_config::DEFAULT_RPC_URL.to_string());
    
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
    if input_addition_records.len() > 5 {
        return Err("Too many UTXOs selected for a single transaction. Maximum allowed is 5.".to_string());
    }
    
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
    
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(60))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {}", e))?;
    let network_result = rpc_client::call_json_rpc(&client, &rpc_url, "node_network", serde_json::json!([])).await?;
    let network_str = network_result.get("network")
        .and_then(|v| v.as_str())
        .ok_or_else(|| "Missing network in response".to_string())?;
    let network = Network::from_str(network_str)
        .map_err(|e| format!("Failed to parse network '{}': {}", network_str, e))?;
    
    // Parse spending key
    let spending_key_bytes = hex::decode(spending_key_hex.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid spending key hex: {}", e))?;
    
    let spending_key: SpendingKey = bincode::deserialize(&spending_key_bytes)
        .map_err(|e| format!("Failed to deserialize spending key: {}", e))?;
    
    let generation_spending_key = match spending_key {
        SpendingKey::Generation(key) => key,
        SpendingKey::Symmetric(_) => {
            return Err("Symmetric spending keys not supported".to_string());
        }
    };
    
    let spending_key_enum = SpendingKey::Generation(generation_spending_key.clone());
    let lock_script_and_witness = spending_key_enum.lock_script_and_witness();
    // Get UTXOs from cache instead of calling server
    use crate::utxo_cache;
    let matched_utxos = utxo_cache::get_utxos_by_addition_records(&input_addition_records).await;
    
    if matched_utxos.len() != input_addition_records.len() {
        return Err(format!(
            "Not all UTXOs found in cache. Found {}/{} UTXOs. Please scan UTXOs first.",
            matched_utxos.len(),
            input_addition_records.len()
        ));
    }
    
    // Validate UTXOs using archival_getSpentStatus RPC before building transaction
    use crate::utxo_scanning;
    let validated_utxos = utxo_scanning::validate_utxos_with_rpc(&client, &rpc_url, &matched_utxos).await
        .map_err(|e| format!("Failed to validate UTXOs: {}. Some UTXOs may have been spent.", e))?;
    
    if validated_utxos.len() != matched_utxos.len() {
        return Err(format!(
            "Some UTXOs have been spent. Validated {}/{} UTXOs. Please refresh and try again.",
            validated_utxos.len(),
            matched_utxos.len()
        ));
    }
    
    let mut total_input_amount = NativeCurrencyAmount::zero();
    for utxo in &matched_utxos {
        total_input_amount = total_input_amount + utxo.utxo.utxo.get_native_currency_amount();
    }
    
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
            generation_spending_key.to_address().into(),
            change_amount,
            UtxoNotificationMedium::OnChain,
        ));
    }
    
    let mutator_set_accumulator = rpc_client::get_archival_mutator_set(&client, &rpc_url).await?;
    
    // Generate membership proofs client-side
    let mut tx_inputs = Vec::new();
    for incoming_utxo in validated_utxos {
        let msmp = generate_membership_proof_client_side(
            &incoming_utxo.utxo,
            &mutator_set_accumulator,
            &rpc_url,
            &client,
        ).await?;
        
        tx_inputs.push(TxInput::from(UnlockedUtxo::unlock(
            incoming_utxo.utxo.utxo.clone(),
            lock_script_and_witness.clone(),
            msmp,
        )));
    }
    
    let my_address: ReceivingAddress = generation_spending_key.to_address().into();
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
    let proof_collection = produce_proof_collection_in_process(&primitive_witness, Some(23))
        .await
        .map_err(|e| format!("Failed to generate proof collection: {}", e))?;
    
    let proof = TransactionProof::ProofCollection(proof_collection);
    let transaction = TransactionBuilder::new()
        .transaction_kernel(primitive_witness.kernel.clone())
        .transaction_proof(proof)
        .build()
        .map_err(|e| format!("Failed to build transaction: {}", e))?;
    
    // Compute output UTXO digests from transaction kernel outputs (client-side)
    // Output UTXO digest = canonical_commitment from AdditionRecord
    let output_utxo_digests: Vec<String> = transaction.kernel.outputs
        .iter()
        .map(|addition_record| addition_record.canonical_commitment.to_hex())
        .collect();
    
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
    })
    .to_string())
}

async fn prove_in_process(
    program: Program,
    claim: Claim,
    nondeterminism: NonDeterminism,
    max_log2_padded_height: Option<u8>,
) -> std::result::Result<NeptuneProof, String> {
    let claim_input = claim.input.clone();
    let program_clone = program.clone();
    let nondeterminism_clone = nondeterminism.clone();
    let result = tokio::task::spawn_blocking(move || {
        VM::trace_execution(program_clone, claim_input.into(), nondeterminism_clone)
    })
    .await
    .map_err(|e| format!("VM execution task failed: {}", e))?;
    
    let (aet, public_output) = result.map_err(|e| format!("VM execution failed: {}", e))?;
    
    if public_output != claim.output {
        return Err("VM output does not match claim output".to_string());
    }
    
    let log2_padded_height = aet.padded_height().ilog2() as u8;
    if let Some(max) = max_log2_padded_height {
        if log2_padded_height > max {
            return Err(format!(
                "Proof complexity limit exceeded: padded height 2^{} exceeds limit 2^{}",
                log2_padded_height, max
            ));
        }
    }

    let stark = Stark::default();
    let proof = stark
        .prove(&claim, &aet)
        .map_err(|e| format!("Proof generation failed: {}", e))?;
    Ok(NeptuneProof::from(proof))
}

async fn produce_proof_collection_in_process(
    primitive_witness: &PrimitiveWitness,
    max_log2_padded_height: Option<u8>,
) -> std::result::Result<ProofCollection, String> {
    // Extract witnesses
    let removal_records_integrity_witness = RemovalRecordsIntegrityWitness::from(primitive_witness);
    let collect_lock_scripts_witness = CollectLockScriptsWitness::from(primitive_witness);
    let kernel_to_outputs_witness = KernelToOutputsWitness::from(primitive_witness);
    let collect_type_scripts_witness = CollectTypeScriptsWitness::from(primitive_witness);
    
    let txk_mast_hash = primitive_witness.kernel.mast_hash();
    let txk_mast_hash_as_input = PublicInput::new(txk_mast_hash.reversed().values().to_vec());
    let salted_inputs_hash = Hash::hash(&primitive_witness.input_utxos);
    let salted_outputs_hash = Hash::hash(&primitive_witness.output_utxos);
    
    // Prove RemovalRecordsIntegrity
    let removal_records_integrity = prove_in_process(
        RemovalRecordsIntegrity.program(),
        removal_records_integrity_witness.claim(),
        removal_records_integrity_witness.nondeterminism(),
        max_log2_padded_height,
    )
    .await?;
    
    // Prove CollectLockScripts
    let collect_lock_scripts = prove_in_process(
        CollectLockScripts.program(),
        collect_lock_scripts_witness.claim(),
        collect_lock_scripts_witness.nondeterminism(),
        max_log2_padded_height,
    )
    .await?;
    
    // Prove KernelToOutputs
    let kernel_to_outputs = prove_in_process(
        KernelToOutputs.program(),
        kernel_to_outputs_witness.claim(),
        kernel_to_outputs_witness.nondeterminism(),
        max_log2_padded_height,
    )
    .await?;
    
    // Prove CollectTypeScripts
    let collect_type_scripts = prove_in_process(
        CollectTypeScripts.program(),
        collect_type_scripts_witness.claim(),
        collect_type_scripts_witness.nondeterminism(),
        max_log2_padded_height,
    )
    .await?;
    
    // Prove lock scripts
    let mut lock_scripts_halt = Vec::new();
    for lock_script_and_witness in &primitive_witness.lock_scripts_and_witnesses {
        let claim = Claim::new(lock_script_and_witness.program.hash())
            .with_input(txk_mast_hash_as_input.individual_tokens.clone());
        let nondeterminism = lock_script_and_witness.nondeterminism();
        let program = lock_script_and_witness.program.clone();
        let proof =
            prove_in_process(program, claim, nondeterminism, max_log2_padded_height).await?;
        lock_scripts_halt.push(proof);
    }
    
    // Prove type scripts
    let mut type_scripts_halt = Vec::new();
    for tsaw in &primitive_witness.type_scripts_and_witnesses {
        let type_script_program = tsaw.program.clone();
        let public_input: Vec<_> = [txk_mast_hash, salted_inputs_hash, salted_outputs_hash]
            .into_iter()
            .flat_map(|d| d.reversed().values())
            .collect();
        let claim = Claim::new(type_script_program.hash()).with_input(public_input);
        let nondeterminism = tsaw.nondeterminism();
        let proof = prove_in_process(
            type_script_program,
            claim,
            nondeterminism,
            max_log2_padded_height,
        )
        .await?;
        type_scripts_halt.push(proof);
    }
    
    let lock_script_hashes = primitive_witness
        .lock_scripts_and_witnesses
        .iter()
        .map(|lsaw| lsaw.program.hash())
        .collect();
    let type_script_hashes = primitive_witness
        .type_scripts_and_witnesses
        .iter()
        .map(|tsaw| tsaw.program.hash())
        .collect();
    
    let merge_bit_mast_path = primitive_witness.kernel.mast_path(
        neptune_privacy::protocol::consensus::transaction::transaction_kernel::TransactionKernelField::MergeBit,
    );

    Ok(ProofCollection {
        removal_records_integrity,
        collect_lock_scripts,
        lock_scripts_halt,
        kernel_to_outputs,
        collect_type_scripts,
        type_scripts_halt,
        lock_script_hashes,
        type_script_hashes,
        kernel_mast_hash: txk_mast_hash,
        salted_inputs_hash,
        salted_outputs_hash,
        merge_bit_mast_path,
    })
}


