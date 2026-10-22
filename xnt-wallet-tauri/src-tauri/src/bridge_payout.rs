use neptune_privacy::prelude::twenty_first::prelude::{BFieldElement, Digest};
use neptune_privacy::protocol::consensus::bridge::aggregated_announcement::AggregatedAnnouncementData;
use neptune_privacy::protocol::consensus::transaction::announcement::Announcement;
use tauri::AppHandle;

use crate::{rpc_client, rpc_config, transaction, utxo_cache};

/// Leviathan burn serial for a cached UTXO paid by a bridge settlement, else `None`.
pub async fn burn_serial(app: &AppHandle, commitment: &str) -> Result<Option<String>, String> {
    let cached = utxo_cache::get_all_utxos()
        .await
        .into_iter()
        .find(|c| c.utxo.addition_record().canonical_commitment.to_hex() == commitment)
        .ok_or_else(|| format!("No cached UTXO with commitment {commitment}"))?;
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    let client = transaction::http_client()?;
    let kernel = rpc_client::get_block_transaction_kernel(&client, &rpc_url, cached.block_height)
        .await?
        .ok_or_else(|| format!("No block at height {}", cached.block_height))?;
    Ok(burn_serial_in(
        &kernel.announcements,
        cached.utxo.sender_randomness,
    ))
}

fn burn_serial_in(announcements: &[Announcement], sender_randomness: Digest) -> Option<String> {
    AggregatedAnnouncementData::find_in_announcements(announcements)?
        .burns
        .iter()
        .find(|burn| burn.sender_randomness() == sender_randomness)
        .map(|burn| word_hex(&burn.serial))
}

/// Word hex as Leviathan shows it: each element's little-endian bytes, in order.
fn word_hex(word: &[BFieldElement; 4]) -> String {
    let bytes: Vec<u8> = word.iter().flat_map(|e| e.value().to_le_bytes()).collect();
    format!("0x{}", hex::encode(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn block_252_announcements() -> Vec<Announcement> {
        let result: serde_json::Value =
            serde_json::from_str(include_str!("../test_data/block_252_tx_kernel.json")).unwrap();
        rpc_client::kernel_from_result(&result)
            .unwrap()
            .unwrap()
            .announcements
            .clone()
    }

    #[test]
    fn finds_the_burn_that_paid_a_settlement_output() {
        let sender_randomness = Digest::try_from_hex(
            "d486bafaaf6b91fed41f4eaa0176876f75a5dfa94cc47c9d003a91ce155d867785b6a418f53e74bd",
        )
        .unwrap();
        assert_eq!(
            burn_serial_in(&block_252_announcements(), sender_randomness).as_deref(),
            Some("0x0ab86b0664358344ca7ab00a14dca55fbdaf7a74ec933a190e83bf87a886192d"),
        );
    }

    #[test]
    fn other_outputs_have_no_burn() {
        assert_eq!(
            burn_serial_in(&block_252_announcements(), Digest::default()),
            None
        );
    }
}
