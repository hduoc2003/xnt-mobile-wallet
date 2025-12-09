use neptune_privacy::api::export::{ReceivingAddress, TransactionDetails};
use neptune_privacy::api::tx_initiation::builder::transaction_builder::TransactionBuilder;
use neptune_privacy::api::tx_initiation::builder::tx_output_list_builder::OutputFormat;
use neptune_privacy::application::config::network::Network;
use neptune_privacy::application::json_rpc::core::model::message::{
    GetArchivalMutatorSetResponse, StateInfo,
};
use neptune_privacy::prelude::twenty_first::prelude::Digest;
use neptune_privacy::protocol::consensus::shared::Hash;
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
use neptune_privacy::state::wallet::address::generation_address::GenerationSpendingKey;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
use neptune_privacy::state::wallet::transaction_input::{TxInput, TxInputList};
use neptune_privacy::state::wallet::transaction_output::{TxOutput, TxOutputList};
use neptune_privacy::state::wallet::unlocked_utxo::UnlockedUtxo;
use neptune_privacy::state::wallet::utxo_notification::UtxoNotificationMedium;
use neptune_privacy::util_types::mutator_set::addition_record::AdditionRecord;
use neptune_privacy::util_types::mutator_set::ms_membership_proof::MsMembershipProof;
use neptune_privacy::util_types::mutator_set::removal_record::absolute_index_set::AbsoluteIndexSet;
use neptune_privacy::util_types::mutator_set::removal_record::chunk::Chunk;
use neptune_privacy::util_types::mutator_set::removal_record::chunk_dictionary::ChunkDictionary;
use neptune_privacy::util_types::mutator_set::shared::CHUNK_SIZE;
use neptune_privacy::prelude::twenty_first::prelude::Tip5;
use neptune_privacy::util_types::mutator_set::mutator_set_accumulator::MutatorSetAccumulator;
use neptune_privacy::prelude::twenty_first::util_types::mmr::mmr_membership_proof::MmrMembershipProof;
use num_traits::identities::Zero;
use num_traits::ops::checked::CheckedSub;
use rand::Rng;
use reqwest::Client;
use std::str::FromStr;
use log::error;
use neptune_privacy::prelude::triton_vm::prelude::{Program, Claim, NonDeterminism, PublicInput};
use neptune_privacy::prelude::triton_vm::stark::Stark;
use neptune_privacy::prelude::triton_vm::vm::VM;
use neptune_privacy::protocol::consensus::transaction::transaction_kernel::TransactionKernelField;
use neptune_privacy::protocol::proof_abstractions::mast_hash::MastHash;
use neptune_privacy::protocol::proof_abstractions::tasm::program::ConsensusProgram;
use neptune_privacy::protocol::proof_abstractions::SecretWitness;

const MAX_SEARCH_DEPTH: u64 = 5000;

/// Generate membership proof client-side without sending receiver_preimage to server
pub(crate) async fn generate_membership_proof_client_side(
    incoming_utxo: &IncomingUtxo,
    mutator_set_accumulator: &MutatorSetAccumulator,
    rpc_url: &str,
    client: &Client,
) -> Result<MsMembershipProof, String> {
    let serialize_hex = |data: &[u8]| format!("0x{}", hex::encode(data));
    
    // Step 1: Find AOCL leaf index
    let addition_record = incoming_utxo.addition_record();
    let addition_record_hex = serialize_hex(&bincode::serialize(&addition_record)
        .map_err(|e| format!("Failed to serialize addition record: {}", e))?);
    
    let creation_block_text = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getUtxoCreationBlock",
            "params": {
                "additionRecord": addition_record_hex,
                "maxSearchDepth": MAX_SEARCH_DEPTH,
            },
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get UTXO creation block: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get response: {}", e))?;
    
    let creation_block_json: serde_json::Value = serde_json::from_str(&creation_block_text)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    
    let creation_result = extract_rpc_result("archival_getUtxoCreationBlock", &creation_block_json)?;
    
    let last_aocl_index = creation_result["lastAoclIndexInBlock"]
        .as_u64()
        .ok_or_else(|| "Missing lastAoclIndexInBlock".to_string())?;
    
    let num_outputs = creation_result["numOutputsInBlock"]
        .as_u64()
        .ok_or_else(|| "Missing numOutputsInBlock".to_string())?;
    
    // Find exact AOCL leaf index by searching backwards
    let mut aocl_leaf_index = last_aocl_index;
    let min_aocl_leaf_index = last_aocl_index.saturating_sub(num_outputs) + 1;
    
    while aocl_leaf_index >= min_aocl_leaf_index {
        let digest_text = client
            .post(rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "archival_getUtxoDigest",
                "params": [aocl_leaf_index],
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to get AOCL digest: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to get response: {}", e))?;
        
        let digest_json: serde_json::Value = serde_json::from_str(&digest_text)
            .map_err(|e| format!("Failed to parse: {}", e))?;
        
        let digest_result = extract_rpc_result("archival_getUtxoDigest", &digest_json)?;
        
        if let Some(digest_opt) = digest_result.get("digest") {
            if let Some(digest_hex) = digest_opt.as_str() {
                let digest_bytes = hex::decode(digest_hex.trim_start_matches("0x"))
                    .map_err(|e| format!("Invalid digest hex: {}", e))?;
                let digest: Digest = bincode::deserialize(&digest_bytes)
                    .map_err(|e| format!("Failed to deserialize digest: {}", e))?;
                
                if digest == addition_record.canonical_commitment {
                    break;
                }
            }
        }
        
        if aocl_leaf_index == 0 {
            break;
        }
        aocl_leaf_index -= 1;
    }
    
    if aocl_leaf_index < min_aocl_leaf_index {
        return Err("UTXO addition record not found in block".to_string());
    }
    
    // Step 2: Get AOCL authentication path
    let auth_path_text = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getAoclAuthenticationPath",
            "params": [aocl_leaf_index],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get AOCL auth path: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get response: {}", e))?;
    
    let auth_path_json: serde_json::Value = serde_json::from_str(&auth_path_text)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    
    let auth_path_result = extract_rpc_result("archival_getAoclAuthenticationPath", &auth_path_json)?;
    
    let auth_path_aocl_hex = auth_path_result["authPathHex"]
        .as_str()
        .ok_or_else(|| "Missing authPathHex".to_string())?;
    
    let auth_path_aocl: MmrMembershipProof = 
        bincode::deserialize(
            &hex::decode(auth_path_aocl_hex.trim_start_matches("0x"))
                .map_err(|e| format!("Invalid hex: {}", e))?
        )
        .map_err(|e| format!("Failed to deserialize auth path: {}", e))?;
    
    // Step 3: Compute chunk indices needed (client-side, uses receiver_preimage - never sent to server!)
    let item = Tip5::hash(incoming_utxo.utxo());
    let swbf_indices = AbsoluteIndexSet::compute(
        item,
        incoming_utxo.sender_randomness(),
        incoming_utxo.receiver_preimage(),
        aocl_leaf_index,
    );
    
    let batch_index = mutator_set_accumulator.get_batch_index();
    let window_start = u128::from(batch_index) * u128::from(CHUNK_SIZE);
    
    let chunk_indices: Vec<u64> = swbf_indices
        .to_array()
        .iter()
        .filter(|bi| **bi < window_start)
        .map(|bi| (*bi / u128::from(CHUNK_SIZE)) as u64)
        .collect();
    
    // Step 4: Get chunks and their auth paths
    let chunks_text = client
        .post(rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getChunksAndAuthPaths",
            "params": {
                "chunkIndices": chunk_indices,
            },
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get chunks: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get response: {}", e))?;
    
    let chunks_json: serde_json::Value = serde_json::from_str(&chunks_text)
        .map_err(|e| format!("Failed to parse: {}", e))?;
    
    let chunks_result = extract_rpc_result("archival_getChunksAndAuthPaths", &chunks_json)?;
    
    let chunks_array = chunks_result["chunks"]
        .as_array()
        .ok_or_else(|| "Missing chunks array".to_string())?;
    
    let mut target_chunks = ChunkDictionary::default();
    
    for chunk_data in chunks_array {
        let chunk_index = chunk_data["chunkIndex"]
            .as_u64()
            .ok_or_else(|| "Missing chunkIndex".to_string())?;
        
        let chunk_hex = chunk_data["chunkHex"]
            .as_str()
            .ok_or_else(|| "Missing chunkHex".to_string())?;
        
        let chunk_bytes = hex::decode(chunk_hex.trim_start_matches("0x"))
            .map_err(|e| format!("Invalid chunk hex: {}", e))?;
        
        let chunk: Chunk = bincode::deserialize(&chunk_bytes)
            .map_err(|e| format!("Failed to deserialize chunk: {}", e))?;
        
        let auth_path_hex = chunk_data["authPathHex"]
            .as_str()
            .ok_or_else(|| "Missing authPathHex".to_string())?;
        
        let chunk_auth_path: MmrMembershipProof = 
            bincode::deserialize(
                &hex::decode(auth_path_hex.trim_start_matches("0x"))
                    .map_err(|e| format!("Invalid auth path hex: {}", e))?
            )
            .map_err(|e| format!("Failed to deserialize auth path: {}", e))?;
        
        target_chunks.insert(chunk_index, (chunk_auth_path, chunk));
    }
    
    // Step 5: Build membership proof (all client-side, receiver_preimage never sent to server!)
    Ok(MsMembershipProof {
        sender_randomness: incoming_utxo.sender_randomness(),
        receiver_preimage: incoming_utxo.receiver_preimage(),
        auth_path_aocl,
        aocl_leaf_index,
        target_chunks,
    })
}

pub async fn build_transaction(payload: serde_json::Value) -> Result<String, String> {
    match build_transaction_inner(payload).await {
        Ok(result) => Ok(result),
        Err(err) => {
            error!("[BuildTx] ✖ {}", err);
            Err(err)
        }
    }
}

async fn build_transaction_inner(payload: serde_json::Value) -> Result<String, String> {
    let rpc_url = std::env::var("VITE_APP_API")
        .unwrap_or_else(|_| "https://66b4b9bd1e4c.ngrok-free.app".to_string());
    
    let spending_key_hex = payload["spendingKeyHex"]
        .as_str()
        .ok_or_else(|| "Missing spendingKeyHex in payload".to_string())?;
    
    let input_addition_records: Vec<String> = payload["inputAdditionRecords"]
        .as_array()
        .ok_or_else(|| "Missing inputAdditionRecords in payload".to_string())?
        .iter()
        .filter_map(|v| v.as_str().map(|s| s.to_string()))
        .collect();
    
    let min_block_height = payload["minBlockHeight"].as_i64().unwrap_or(0);
    
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
    
    let fee = payload["fee"]
        .as_str()
        .ok_or_else(|| "Missing fee in payload".to_string())?;
    
    let client = Client::new();
    let state_text = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "node_getState",
            "params": [],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get state: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get state response: {}", e))?;

    let state_json: serde_json::Value = serde_json::from_str(&state_text)
        .map_err(|e| format!("Failed to parse state: {}", e))?;

    let state_result = extract_rpc_result("node_getState", &state_json)?;
    let state_info: StateInfo = serde_json::from_value(state_result["state"].clone())
        .map_err(|e| format!("Failed to deserialize state: {}", e))?;
    let network = Network::from_str(&state_info.network)
        .map_err(|e| format!("Failed to parse network: {}", e))?;
    
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
    // Get UTXOs from cache instead of calling server (PRIVACY: no view key sent!)
    use crate::utxo_cache;
    let matched_utxos = utxo_cache::get_utxos_by_addition_records(&input_addition_records).await;
    
    if matched_utxos.len() != input_addition_records.len() {
        return Err(format!(
            "Not all UTXOs found in cache. Found {}/{} UTXOs. Please scan UTXOs first.",
            matched_utxos.len(),
            input_addition_records.len()
        ));
    }
    
    let mut total_input_amount = NativeCurrencyAmount::zero();
    for utxo in &matched_utxos {
        total_input_amount = total_input_amount + utxo.utxo().get_native_currency_amount();
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
    
    // Get mutator set accumulator first (needed for client-side proof generation)
    let archival_text = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "archival_getArchivalMutatorSet",
            "params": [],
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get archival mutator set: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get archival response: {}", e))?;

    let archival_json: serde_json::Value =
        serde_json::from_str(&archival_text)
            .map_err(|e| format!("Failed to parse archival response: {}", e))?;

    let result_obj =
        extract_rpc_result("archival_getArchivalMutatorSet", &archival_json)?;

    if result_obj.get("archivalMutatorSet").is_none()
        && result_obj.get("archival_mutator_set").is_none()
    {
        return Err("archival_getArchivalMutatorSet missing archivalMutatorSet field".to_string());
    }
    
    let mutator_set_accumulator = serde_json::from_value::<GetArchivalMutatorSetResponse>(result_obj.clone())
        .map_err(|e| format!("Failed to deserialize archival mutator set: {}", e))?
        .archival_mutator_set;
    
    // Generate membership proofs client-side (receiver_preimage never sent to server!)
    let mut tx_inputs = Vec::new();
    for incoming_utxo in matched_utxos {
        let msmp = generate_membership_proof_client_side(
            &incoming_utxo,
            &mutator_set_accumulator,
            &rpc_url,
            &client,
        ).await?;
        
        tx_inputs.push(TxInput::from(UnlockedUtxo::unlock(
            incoming_utxo.utxo().clone(),
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
            
            Ok(if is_change {
                TxOutput::onchain_native_currency_as_change(amount, sender_randomness, address)
            } else {
                TxOutput::onchain_native_currency(amount, sender_randomness, address, false)
            })
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
    
    let tx_bytes = bincode::serialize(&transaction)
        .map_err(|e| format!("Failed to serialize transaction: {}", e))?;
    
    let tx_hex_with_prefix = format!("0x{}", hex::encode(&tx_bytes));
    
    Ok(serde_json::json!({
        "success": true,
        "transaction": tx_hex_with_prefix,
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
    
    let merge_bit_mast_path = primitive_witness
        .kernel
        .mast_path(TransactionKernelField::MergeBit);
    
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

fn extract_rpc_result<'a>(
    method: &str,
    response: &'a serde_json::Value,
) -> Result<&'a serde_json::Value, String> {
    if let Some(error) = response.get("error") {
        if !error.is_null() {
            return Err(format!("{method} RPC error: {}", error));
        }
    }

    response
        .get("result")
        .ok_or_else(|| format!("{method} missing result field"))
}

