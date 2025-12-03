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
use neptune_privacy::state::wallet::address::view_key::ViewKey;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::incoming_utxo::IncomingUtxo;
use neptune_privacy::state::wallet::transaction_input::{TxInput, TxInputList};
use neptune_privacy::state::wallet::transaction_output::{TxOutput, TxOutputList};
use neptune_privacy::state::wallet::unlocked_utxo::UnlockedUtxo;
use neptune_privacy::state::wallet::utxo_notification::UtxoNotificationMedium;
use neptune_privacy::util_types::mutator_set::addition_record::AdditionRecord;
use neptune_privacy::util_types::mutator_set::ms_membership_proof::MsMembershipProof;
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
        .unwrap_or_else(|_| "http://zsmartex.com:9897".to_string());
    
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
    let view_key = ViewKey::from_spending_key(&spending_key_enum)
        .map_err(|e| format!("Failed to derive view key: {}", e))?;
    
    let view_key_hex = format!("0x{}", hex::encode(bincode::serialize(&view_key)
        .map_err(|e| format!("Failed to serialize view key: {}", e))?));
    
    let utxo_text = client
        .post(&rpc_url)
        .json(&serde_json::json!({
            "jsonrpc": "2.0",
            "method": "wallet_getUtxosFromViewKey",
            "params": {
                "viewKey": view_key_hex,
                "startBlock": min_block_height,
                "maxSearchDepth": MAX_SEARCH_DEPTH,
            },
            "id": 1
        }))
        .send()
        .await
        .map_err(|e| format!("Failed to get UTXOs: {}", e))?
        .text()
        .await
        .map_err(|e| format!("Failed to get UTXO response: {}", e))?;
    
    let utxo_json: serde_json::Value = serde_json::from_str(&utxo_text)
        .map_err(|e| format!("Failed to parse UTXO response: {}", e))?;
    let utxo_result = extract_rpc_result("wallet_getUtxosFromViewKey", &utxo_json)?;

    let all_utxos: Vec<IncomingUtxo> = utxo_result["utxos"]
        .as_array()
        .ok_or_else(|| "Missing utxos in response".to_string())?
        .iter()
        .map(|u| serde_json::from_value(u.clone()))
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| format!("Failed to deserialize UTXO: {}", e))?;
    
    let mut matched_utxos = Vec::new();
    let mut total_input_amount = NativeCurrencyAmount::zero();
    
    for requested_ar_hex in &input_addition_records {
        let requested_ar: AdditionRecord = bincode::deserialize(
            &hex::decode(requested_ar_hex.trim_start_matches("0x"))
                .map_err(|e| format!("Invalid addition record hex: {}", e))?
        )
        .map_err(|e| format!("Invalid addition record: {}", e))?;
        
        let matching_utxo = all_utxos
            .iter()
            .find(|utxo| utxo.addition_record().canonical_commitment == requested_ar.canonical_commitment)
            .ok_or_else(|| format!("UTXO with addition_record {} not found", requested_ar_hex))?;
        
        total_input_amount = total_input_amount + matching_utxo.utxo().get_native_currency_amount();
        matched_utxos.push(matching_utxo.clone());
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
    
    let mut tx_inputs = Vec::new();
    for incoming_utxo in matched_utxos {
        let serialize_hex = |data: &[u8]| format!("0x{}", hex::encode(data));
        
        let utxo_with_proof_text = client
            .post(&rpc_url)
            .json(&serde_json::json!({
                "jsonrpc": "2.0",
                "method": "archival_generateUtxoWithProof",
                "params": {
                    "utxoHex": serialize_hex(&bincode::serialize(incoming_utxo.utxo()).unwrap()),
                    "additionRecordHex": serialize_hex(&bincode::serialize(&incoming_utxo.addition_record()).unwrap()),
                    "senderRandomnessHex": serialize_hex(&bincode::serialize(&incoming_utxo.sender_randomness()).unwrap()),
                    "receiverPreimageHex": serialize_hex(&bincode::serialize(&incoming_utxo.receiver_preimage()).unwrap()),
                    "maxSearchDepth": MAX_SEARCH_DEPTH,
                },
                "id": 1
            }))
            .send()
            .await
            .map_err(|e| format!("Failed to generate UTXO with proof: {}", e))?
            .text()
            .await
            .map_err(|e| format!("Failed to get UTXO with proof response: {}", e))?;

        let utxo_with_proof_json: serde_json::Value =
            serde_json::from_str(&utxo_with_proof_text)
                .map_err(|e| format!("Failed to parse UTXO with proof: {}", e))?;

        let result =
            extract_rpc_result("archival_generateUtxoWithProof", &utxo_with_proof_json)?;

        let msmp_hex = result["msmpHex"]
            .as_str()
            .or_else(|| result["msmp_hex"].as_str())
            .ok_or_else(|| "Missing msmp_hex in response".to_string())?;
        
        let msmp: MsMembershipProof = bincode::deserialize(
            &hex::decode(msmp_hex.trim_start_matches("0x"))
                .map_err(|e| format!("Invalid msmp_hex: {}", e))?
        )
        .map_err(|e| format!("Failed to deserialize msmp: {}", e))?;
        
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

