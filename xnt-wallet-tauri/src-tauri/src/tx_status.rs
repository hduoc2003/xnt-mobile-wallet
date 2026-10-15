use serde::Serialize;
use tauri::AppHandle;

use crate::{rpc_client, rpc_config, transaction};

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum TxStatus {
    Pending,
    Confirmed { block_height: Option<u64> },
    Dropped,
}

#[derive(Debug, PartialEq)]
enum Seen {
    Mined,
    InMempool,
    Nowhere,
}

fn classify(in_mempool: bool, aocl_index: Option<u64>) -> Seen {
    match (aocl_index, in_mempool) {
        (Some(_), _) => Seen::Mined,
        (None, true) => Seen::InMempool,
        (None, false) => Seen::Nowhere,
    }
}

/// Status of the transactions that created each output commitment, from our node.
pub async fn statuses(app: &AppHandle, commitments: &[String]) -> Result<Vec<TxStatus>, String> {
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    let client = transaction::http_client()?;
    // Mempool before AOCL: a tx mined between the two calls then still shows as mined.
    let mempool = rpc_client::get_mempool_output_commitments(&client, &rpc_url).await?;
    let aocl = rpc_client::get_aocl_leaf_indices(&client, &rpc_url, commitments).await?;
    if aocl.len() != commitments.len() {
        return Err(format!(
            "archival_getAoclLeafIndices returned {} entries for {} commitments",
            aocl.len(),
            commitments.len()
        ));
    }
    let mut statuses = Vec::with_capacity(commitments.len());
    for (commitment, index) in commitments.iter().zip(aocl) {
        statuses.push(match classify(mempool.contains(commitment), index) {
            Seen::Mined => TxStatus::Confirmed {
                block_height: rpc_client::output_block_height(&client, &rpc_url, commitment)
                    .await
                    .unwrap_or_else(|e| {
                        log::warn!("[History] block height of {commitment}: {e}");
                        None
                    }),
            },
            Seen::InMempool => TxStatus::Pending,
            Seen::Nowhere => TxStatus::Dropped,
        });
    }
    Ok(statuses)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mined_wins_over_mempool() {
        assert_eq!(classify(true, Some(3)), Seen::Mined);
        assert_eq!(classify(false, Some(3)), Seen::Mined);
    }

    #[test]
    fn mempool_only_is_pending_and_neither_is_dropped() {
        assert_eq!(classify(true, None), Seen::InMempool);
        assert_eq!(classify(false, None), Seen::Nowhere);
    }

    #[test]
    fn wire_format_matches_frontend() {
        let json = serde_json::to_string(&TxStatus::Confirmed {
            block_height: Some(9),
        })
        .unwrap();
        assert_eq!(json, r#"{"state":"confirmed","blockHeight":9}"#);
        assert_eq!(
            serde_json::to_string(&TxStatus::Dropped).unwrap(),
            r#"{"state":"dropped"}"#
        );
    }
}
