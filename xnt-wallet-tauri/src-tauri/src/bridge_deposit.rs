use log::error;
use neptune_privacy::api::export::{ConsensusRuleSet, Timestamp};
use neptune_privacy::api::tx_initiation::bridge_deposit::bridge_deposit_tx_details;
use neptune_privacy::api::tx_initiation::builder::transaction_builder::TransactionBuilder;
use neptune_privacy::application::config::network::Network;
use neptune_privacy::application::json_rpc::core::model::wallet::transaction::RpcTransaction;
use neptune_privacy::prelude::twenty_first::prelude::Digest;
use neptune_privacy::protocol::consensus::block::block_height::BlockHeight;
use neptune_privacy::protocol::consensus::bridge::deposit_announcement::{
    parse_leviathan_address, MIN_BRIDGE_AMOUNT,
};
use neptune_privacy::protocol::consensus::transaction::transaction_proof::TransactionProof;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::state::wallet::transaction_input::TxInputList;
use num_traits::{CheckedAdd, Zero};
use serde_json::Value;
use tauri::AppHandle;

use crate::bridge_status::{self, DepositRecord, DepositStatus};
use crate::transaction::{self, MempoolCheck, MAX_TX_INPUTS};
use crate::{rpc_client, rpc_config};

/// A raw account id has no checksum, so a typo still parses and the deposit is minted to nobody.
fn is_raw_account_id(address: &str) -> bool {
    let hex = address
        .strip_prefix("0x")
        .or_else(|| address.strip_prefix("0X"))
        .unwrap_or(address);
    !hex.is_empty() && hex.chars().all(|c| c.is_ascii_hexdigit())
}

#[derive(Debug)]
pub(crate) struct DepositRequest {
    pub account_id: [u64; 2],
    pub leviathan_address: String,
    pub amount: NativeCurrencyAmount,
    pub fee: NativeCurrencyAmount,
    pub low_memory_proving: bool,
    pub wallet_account: String,
}

/// Checks everything that needs no node, so bad input fails before any proving.
pub(crate) fn parse_deposit_request(
    payload: &Value,
    network: Network,
) -> Result<DepositRequest, String> {
    let address = payload["leviathanAddress"]
        .as_str()
        .ok_or("Missing leviathanAddress in payload")?
        .trim();
    let address = address.strip_prefix("miden:").unwrap_or(address).trim();
    if is_raw_account_id(address) {
        return Err("Enter the Leviathan address (bech32), not a raw account id".to_string());
    }
    let account_id = parse_leviathan_address(address, network)
        .map_err(|e| format!("Invalid Leviathan address: {e}"))?;
    let amount = parse_coins(payload, "amount")?;
    if amount < MIN_BRIDGE_AMOUNT {
        return Err(format!("Deposit must be at least {MIN_BRIDGE_AMOUNT} XNT"));
    }
    let fee = parse_coins(payload, "fee")?;
    if fee < NativeCurrencyAmount::zero() {
        return Err("Fee can't be negative".to_string());
    }
    let low_memory_proving = payload["lowMemoryProving"]
        .as_bool()
        .ok_or("Missing lowMemoryProving in payload")?;
    let wallet_account = payload["walletAccount"]
        .as_str()
        .filter(|id| !id.is_empty())
        .ok_or("Missing walletAccount in payload")?
        .to_string();
    let input_count = payload["inputUtxoIds"]
        .as_array()
        .ok_or("Missing inputUtxoIds in payload")?
        .len();
    if !(1..=MAX_TX_INPUTS).contains(&input_count) {
        return Err(format!(
            "A deposit uses 1 to {MAX_TX_INPUTS} UTXOs, got {input_count}"
        ));
    }
    Ok(DepositRequest {
        account_id,
        leviathan_address: address.to_string(),
        amount,
        fee,
        low_memory_proving,
        wallet_account,
    })
}

fn parse_coins(payload: &Value, field: &str) -> Result<NativeCurrencyAmount, String> {
    let value = payload[field]
        .as_str()
        .ok_or_else(|| format!("Missing {field} in payload"))?;
    NativeCurrencyAmount::coins_from_str(value).map_err(|e| format!("Invalid {field} {value}: {e}"))
}

pub async fn build_bridge_deposit(app: AppHandle, payload: Value) -> Result<String, String> {
    let result = build(&app, &payload).await;
    if let Err(ref err) = result {
        error!("[Bridge] ✖ {}", err);
    }
    result
}

async fn build(app: &AppHandle, payload: &Value) -> Result<String, String> {
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    let client = transaction::http_client()?;
    let network = transaction::fetch_network(&client, &rpc_url).await?;
    let request = parse_deposit_request(payload, network)?;
    // Before the bridge era a deposit is mined but never minted: the funds would only be locked.
    let tip_height = BlockHeight::from(rpc_client::get_chain_height(&client, &rpc_url).await?);
    if !ConsensusRuleSet::infer_for_next_block(network, tip_height).bav_active() {
        return Err("The bridge is not open on this network yet".into());
    }
    let inputs =
        transaction::validate_inputs(&client, &rpc_url, payload, MempoolCheck::Required).await?;

    let needed = request
        .amount
        .checked_add(&request.fee)
        .ok_or("Amount plus fee overflows")?;
    if inputs.total < needed {
        return Err(format!(
            "Insufficient funds: inputs={}, amount+fee={}",
            inputs.total, needed
        ));
    }

    let mutator_set_accumulator = rpc_client::get_archival_mutator_set(&client, &rpc_url).await?;
    let tx_inputs =
        transaction::unlock_inputs(&client, &rpc_url, &inputs, &mutator_set_accumulator).await?;
    let (details, deposit) = bridge_deposit_tx_details(
        TxInputList::from(tx_inputs),
        request.account_id,
        request.amount,
        request.fee,
        inputs.primary_key.to_address(),
        Timestamp::now(),
        mutator_set_accumulator,
        network,
    )
    .map_err(|e| format!("Failed to build deposit: {e}"))?;
    details
        .tx_outputs
        .validate_recipients()
        .map_err(|e| format!("Invalid recipient: {e}"))?;

    let primitive_witness = details.primitive_witness();
    let kernel = primitive_witness.kernel.clone();
    let tip_height = rpc_client::get_chain_height(&client, &rpc_url).await?;
    let proof_collection = transaction::prove(
        primitive_witness,
        network,
        BlockHeight::from(tip_height),
        request.low_memory_proving,
    )
    .await?;
    let transaction = TransactionBuilder::new()
        .transaction_kernel(kernel)
        .transaction_proof(TransactionProof::ProofCollection(proof_collection))
        .build()
        .map_err(|e| format!("Failed to build transaction: {e}"))?;

    let utxo_commitment = Digest::new(deposit.utxo_commitment).to_hex();
    let tx_id = transaction.txid().to_string();
    let created_ms = u64::try_from(chrono::Utc::now().timestamp_millis())
        .map_err(|e| format!("System clock before 1970: {e}"))?;
    bridge_status::add(
        app,
        DepositRecord {
            utxo_commitment: utxo_commitment.clone(),
            tx_id: tx_id.clone(),
            account_id: request.account_id,
            leviathan_address: Some(request.leviathan_address.clone()),
            amount: request.amount.to_string(),
            fee: Some(request.fee.to_string()),
            output_commitments: transaction
                .kernel
                .outputs
                .iter()
                .map(|output| output.canonical_commitment.to_hex())
                .collect(),
            created_ms,
            status: DepositStatus::Submitted { misses: 0 },
            relayer: None,
            relayer_error: None,
            wallet_account: request.wallet_account.clone(),
            bound_to_account: deposit.binds_account(),
        },
    )
    .await?;

    let rpc_transaction: RpcTransaction = transaction.into();
    let transaction_json = serde_json::to_value(&rpc_transaction)
        .map_err(|e| format!("Failed to serialize RpcTransaction: {e}"))?;
    Ok(serde_json::json!({
        "success": true,
        "transaction": transaction_json,
        "utxoCommitment": utxo_commitment,
        "txId": tx_id,
    })
    .to_string())
}

#[cfg(test)]
mod tests {
    use super::parse_deposit_request;
    use neptune_privacy::application::config::network::Network;
    use neptune_privacy::prelude::twenty_first::prelude::{BFieldElement, Digest};
    use neptune_privacy::protocol::consensus::bridge::deposit_announcement::deposit_receiver_preimage;
    use serde_json::json;

    const ADDRESS: &str = "mtst1ap85fypkz32ju5g9kjadv4ux05jn5qfz";

    fn payload(address: &str, amount: &str, inputs: usize) -> serde_json::Value {
        json!({
            "spendingKeyHex": "0x00",
            "inputUtxoIds": vec!["0x00:0"; inputs],
            "leviathanAddress": address,
            "amount": amount,
            "fee": "0.01",
            "lowMemoryProving": true,
            "walletAccount": "main:1",
        })
    }

    #[test]
    fn deposit_request_requires_wallet_account() {
        let network = Network::Testnet(0);
        let request = parse_deposit_request(&payload(ADDRESS, "1", 1), network).unwrap();
        assert_eq!(request.wallet_account, "main:1");
        for missing in [json!(null), json!("")] {
            let mut body = payload(ADDRESS, "1", 1);
            body["walletAccount"] = missing;
            assert_eq!(
                parse_deposit_request(&body, network).unwrap_err(),
                "Missing walletAccount in payload"
            );
        }
    }

    #[test]
    fn accepts_bech32_with_prefix_and_spaces() {
        let request = parse_deposit_request(
            &payload(&format!("  miden:{ADDRESS} "), "1", 1),
            Network::Testnet(0),
        )
        .unwrap();
        let plain = parse_deposit_request(&payload(ADDRESS, "1", 1), Network::Testnet(0)).unwrap();
        assert_eq!(request.account_id, plain.account_id);
        assert_eq!(request.leviathan_address, ADDRESS);
    }

    #[test]
    fn rejects_below_minimum() {
        let err =
            parse_deposit_request(&payload(ADDRESS, "0.99", 1), Network::Testnet(0)).unwrap_err();
        assert!(err.contains("at least 1"), "{err}");
    }

    #[test]
    fn rejects_too_many_or_no_inputs() {
        let err =
            parse_deposit_request(&payload(ADDRESS, "1", 6), Network::Testnet(0)).unwrap_err();
        assert!(err.contains("5"), "{err}");
        assert!(parse_deposit_request(&payload(ADDRESS, "1", 0), Network::Testnet(0)).is_err());
    }

    #[test]
    fn rejects_negative_fee() {
        let mut p = payload(ADDRESS, "1", 1);
        p["fee"] = json!("-0.01");
        assert!(parse_deposit_request(&p, Network::Testnet(0)).is_err());
    }

    #[test]
    fn rejects_testnet_address_on_mainnet() {
        assert!(parse_deposit_request(&payload(ADDRESS, "1", 1), Network::Main).is_err());
    }

    /// xnt-core's `receiver_preimage_derivation_is_pinned` vector.
    #[test]
    fn receiver_preimage_matches_core() {
        let preimage = deposit_receiver_preimage(
            [BFieldElement::new(10), BFieldElement::new(20)],
            Digest::new([BFieldElement::new(7); 5]),
        );
        assert_eq!(
            "4db6bac78b8882d7854bf9b4857a34adc6839e3fd4b95dd7fab703c27de1b3734f68e32f7e428624",
            preimage.to_hex()
        );
    }

    #[test]
    fn rejects_raw_account_ids() {
        for raw in [
            "0x7db0664f500c4c1069429324acfdec",
            "7db0664f500c4c1069429324acfdec",
            " miden:0X7db0664f500c4c1069429324acfdec",
            "miden: 0x7db0664f500c4c1069429324acfdec",
        ] {
            let err =
                parse_deposit_request(&payload(raw, "1", 1), Network::Testnet(0)).unwrap_err();
            assert!(err.contains("not a raw account id"), "{err}");
        }
    }

    #[test]
    fn rejects_garbage_address() {
        let err =
            parse_deposit_request(&payload("hello", "1", 1), Network::Testnet(0)).unwrap_err();
        assert!(err.contains("Invalid Leviathan address"), "{err}");
    }
}
