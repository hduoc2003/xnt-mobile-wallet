//! Funds at legacy-format (pre-v0.3) dCTIDH locks.

use std::cmp::Reverse;

use neptune_privacy::api::export::TransactionDetails;
use neptune_privacy::api::tx_initiation::builder::transaction_builder::TransactionBuilder;
use neptune_privacy::api::tx_initiation::migrate_dctidh::{
    fee_share, migration_fee, num_inputs_worth_moving,
};
use neptune_privacy::application::config::network::Network;
use neptune_privacy::application::json_rpc::core::model::wallet::transaction::RpcTransaction;
use neptune_privacy::prelude::twenty_first::prelude::Digest;
use neptune_privacy::protocol::consensus::block::block_height::BlockHeight;
use neptune_privacy::protocol::consensus::transaction::transaction_proof::TransactionProof;
use neptune_privacy::protocol::consensus::type_scripts::native_currency_amount::NativeCurrencyAmount;
use neptune_privacy::protocol::proof_abstractions::timestamp::Timestamp;
use neptune_privacy::state::transaction::tx_proving_capability::TxProvingCapability;
use neptune_privacy::state::wallet::address::dctidh_address::{
    dCTIDHSpendingKey, legacy_dctidh_cutoff,
};
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::transaction_input::TxInputList;
use neptune_privacy::state::wallet::transaction_output::{TxOutput, TxOutputList};
use num_traits::CheckedSub;
use rand::RngExt;
use serde::Serialize;
use tauri::AppHandle;

use crate::transaction::{self, ValidatedInputs, MAX_TX_INPUTS};
use crate::{rpc_client, rpc_config, utxo_cache, utxo_scanning};

/// The node's default `--min-relay-pctx-fee-per-input`.
const MIN_RELAY_FEE_PER_INPUT: NativeCurrencyAmount =
    NativeCurrencyAmount::from_nau(NativeCurrencyAmount::coin_as_nau() / 2000);

pub(crate) fn legacy_key(key: &SpendingKey) -> Option<SpendingKey> {
    match key {
        SpendingKey::dCTIDH(k) => Some(SpendingKey::dCTIDH(dCTIDHSpendingKey::legacy_from_seed(
            k.seed(),
        ))),
        SpendingKey::Generation(_) | SpendingKey::Symmetric(_) => None,
    }
}

/// Whether a UTXO at a legacy lock, confirmed at `height`, counts (main net: only below the v8 fork).
pub(crate) fn recognized_at(network: Network, height: u64) -> bool {
    legacy_dctidh_cutoff(network).is_none_or(|cutoff| BlockHeight::from(height) < cutoff)
}

/// The fee core's migration charges a proof-collection prover.
fn fee_for(num_inputs: usize) -> NativeCurrencyAmount {
    migration_fee(
        TxProvingCapability::ProofCollection,
        MIN_RELAY_FEE_PER_INPUT,
        num_inputs,
    )
}

fn inputs_worth_moving(
    amounts: &[NativeCurrencyAmount],
    max_inputs: usize,
) -> Option<(usize, NativeCurrencyAmount)> {
    num_inputs_worth_moving(amounts, max_inputs, fee_for)
}

/// A legacy UTXO worth no more than this is dust: never worth its share of a full migration's fee.
pub(crate) fn dust_threshold() -> NativeCurrencyAmount {
    fee_share(fee_for(MAX_TX_INPUTS), MAX_TX_INPUTS)
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
struct MigrationResult {
    success: bool,
    transaction: serde_json::Value,
    output_utxo_digests: Vec<String>,
    tx_id: String,
    to_address: String,
    num_inputs: usize,
    amount: String,
    fee: String,
    /// Legacy UTXOs left for a later migration, `num_dust` of them included.
    num_waiting: usize,
    /// Left UTXOs at or below [`dust_threshold`]; never moved.
    num_dust: usize,
}

pub async fn build_migration(
    app: &AppHandle,
    payload: serde_json::Value,
) -> Result<String, String> {
    let key_hex = payload["dctidhKeyHex"]
        .as_str()
        .ok_or("Missing dctidhKeyHex in payload")?;
    let low_memory_proving = payload["lowMemoryProving"]
        .as_bool()
        .ok_or("Missing lowMemoryProving in payload")?;
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    let client = transaction::http_client()?;
    let network = transaction::fetch_network(&client, &rpc_url).await?;

    let bytes = hex::decode(key_hex.trim_start_matches("0x"))
        .map_err(|e| format!("Invalid key hex: {e}"))?;
    // Deserializing a dCTIDH key derives it too, so both run off the async runtime.
    let (key, legacy) = tokio::task::spawn_blocking(move || {
        let key: SpendingKey = bincode::deserialize(&bytes)
            .map_err(|e| format!("Undeserializable spending key: {e}"))?;
        let legacy = legacy_key(&key).ok_or("Not a dCTIDH key")?;
        Ok::<_, String>((key, legacy))
    })
    .await
    .map_err(|e| format!("Key derivation task failed: {e}"))??;
    let destination = key.to_address();

    let now = Timestamp::now();
    let legacy_preimage = legacy.privacy_preimage();
    let legacy_lock = legacy.lock_script_hash();
    let held: Vec<_> = utxo_cache::get_all_utxos()
        .await
        .into_iter()
        .filter(|u| {
            u.utxo.receiver_preimage == legacy_preimage
                && u.utxo.utxo.lock_script_hash() == legacy_lock
                && recognized_at(network, u.block_height)
        })
        .filter(|u| {
            u.utxo
                .utxo
                .release_date()
                .is_none_or(|release| release <= now)
        })
        .collect();
    let unspent = utxo_scanning::validate_utxos_with_rpc(&client, &rpc_url, &held, false).await?;
    let mut candidates = utxo_scanning::filter_utxos_spent_in_mempool(&client, &rpc_url, &unspent)
        .await
        .map_err(|e| format!("Could not check the mempool for double spends: {e}"))?;
    candidates.sort_by_key(|u| Reverse(u.utxo.utxo.get_native_currency_amount()));

    let amounts: Vec<_> = candidates
        .iter()
        .map(|u| u.utxo.utxo.get_native_currency_amount())
        .collect();
    let (num_inputs, fee) = inputs_worth_moving(&amounts, MAX_TX_INPUTS)
        .ok_or("No legacy funds worth more than the fee to move them")?;
    let num_waiting = amounts.len() - num_inputs;
    let dust_threshold = dust_threshold();
    let num_dust = amounts[num_inputs..]
        .iter()
        .filter(|amount| **amount <= dust_threshold)
        .count();
    candidates.truncate(num_inputs);
    let total: NativeCurrencyAmount = amounts[..num_inputs].iter().copied().sum();
    let amount = total
        .checked_sub(&fee)
        .ok_or("The fee exceeds the moved funds")?;
    log::info!(
        "[Migrate] Moving {num_inputs} legacy dCTIDH UTXO(s) | amount={amount} | fee={fee} | waiting={num_waiting} | dust={num_dust}"
    );

    let inputs = ValidatedInputs {
        keys: vec![legacy.clone(); candidates.len()],
        utxos: candidates,
        primary_key: legacy,
        total,
    };
    let mutator_set_accumulator = rpc_client::get_archival_mutator_set(&client, &rpc_url).await?;
    let tx_inputs =
        transaction::unlock_inputs(&client, &rpc_url, &inputs, &mutator_set_accumulator).await?;
    // On-chain even though it is ours: an off-chain note would be lost on a restore from seed.
    let tx_outputs = TxOutputList::from(vec![TxOutput::onchain_native_currency(
        amount,
        rand::rng().random::<Digest>(),
        destination.clone(),
        true,
    )]);
    tx_outputs
        .validate_recipients()
        .map_err(|e| format!("Invalid recipient: {e}"))?;
    let details = TransactionDetails::new_without_coinbase(
        TxInputList::from(tx_inputs),
        tx_outputs,
        fee,
        now,
        mutator_set_accumulator,
        network,
    );

    let primitive_witness = details.primitive_witness();
    let kernel = primitive_witness.kernel.clone();
    let tip_height = rpc_client::get_chain_height(&client, &rpc_url).await?;
    let proof_collection = transaction::prove(
        primitive_witness,
        network,
        BlockHeight::from(tip_height),
        low_memory_proving,
    )
    .await?;
    let transaction = TransactionBuilder::new()
        .transaction_kernel(kernel)
        .transaction_proof(TransactionProof::ProofCollection(proof_collection))
        .build()
        .map_err(|e| format!("Failed to build transaction: {e}"))?;

    let result = MigrationResult {
        success: true,
        output_utxo_digests: transaction
            .kernel
            .outputs
            .iter()
            .map(|record| record.canonical_commitment.to_hex())
            .collect(),
        tx_id: transaction.txid().to_string(),
        transaction: serde_json::to_value(RpcTransaction::from(transaction))
            .map_err(|e| format!("Failed to serialize the transaction: {e}"))?,
        to_address: destination
            .to_bech32m(network)
            .map_err(|e| format!("Failed to encode the address: {e}"))?,
        num_inputs,
        amount: amount.to_string(),
        fee: fee.to_string(),
        num_waiting,
        num_dust,
    };
    serde_json::to_string(&result).map_err(|e| format!("Failed to serialize response: {e}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn coins(amount: &str) -> NativeCurrencyAmount {
        NativeCurrencyAmount::coins_from_str(amount).unwrap()
    }

    #[test]
    fn worth_moving_takes_the_most_inputs_above_their_fee_share() {
        // Fee 0.1 for up to 200 inputs: a share of 0.02 at 5 inputs, 0.025 at 4.
        let amounts = [
            coins("5"),
            coins("1"),
            coins("0.04"),
            coins("0.021"),
            coins("0.01"),
        ];
        assert_eq!(inputs_worth_moving(&amounts, 5), Some((3, coins("0.1"))));
        assert_eq!(
            inputs_worth_moving(&amounts[..1], 5),
            Some((1, coins("0.1")))
        );
        assert_eq!(inputs_worth_moving(&amounts, 2), Some((2, coins("0.1"))));
    }

    #[test]
    fn dust_alone_is_not_worth_moving() {
        assert_eq!(inputs_worth_moving(&[coins("0.1")], 5), None);
        assert_eq!(inputs_worth_moving(&[], 5), None);
    }

    #[test]
    fn main_net_recognizes_legacy_locks_only_before_the_fork() {
        let cutoff = legacy_dctidh_cutoff(Network::Main).unwrap().value();
        assert!(recognized_at(Network::Main, cutoff - 1));
        assert!(!recognized_at(Network::Main, cutoff));
    }

    #[test]
    fn legacy_key_is_the_legacy_derivation_of_the_same_index() {
        use neptune_privacy::state::wallet::wallet_entropy::WalletEntropy;
        let entropy = WalletEntropy::devnet_wallet();
        let current = SpendingKey::dCTIDH(entropy.nth_dctidh_spending_key(1));
        let expected = SpendingKey::dCTIDH(entropy.nth_legacy_dctidh_spending_key(1));
        let legacy = legacy_key(&current).unwrap();
        assert_eq!(legacy.privacy_preimage(), expected.privacy_preimage());
        assert_eq!(legacy.lock_script_hash(), expected.lock_script_hash());
        assert_ne!(legacy.lock_script_hash(), current.lock_script_hash());
        assert!(legacy_key(&SpendingKey::Generation(
            entropy.nth_generation_spending_key(1)
        ))
        .is_none());
    }
}
