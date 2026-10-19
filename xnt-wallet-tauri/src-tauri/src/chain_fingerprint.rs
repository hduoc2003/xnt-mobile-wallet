use reqwest::Client;
use serde::{Deserialize, Serialize};
use std::path::Path;

use crate::{rpc_client, utxo_cache};

/// Block 1, not genesis: genesis survives a testnet reset.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ChainFingerprint {
    pub network: String,
    pub block_1_digest: String,
}

/// A block the next scan does not re-scan; a different digest there means a reorg past the cache.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct ScannedTip {
    pub height: u64,
    pub digest: String,
}

/// Contents of `chain.json`, written only after a scan has persisted the cache.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct ChainState {
    #[serde(flatten)]
    chain: ChainFingerprint,
    scanned_tip: Option<ScannedTip>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(tag = "state", rename_all = "camelCase")]
pub enum ChainCheck {
    Same,
    FirstRecord,
    Reset { previous: ChainFingerprint },
    Reorg { height: u64 },
}

impl ChainCheck {
    pub fn needs_full_rescan(&self) -> bool {
        matches!(self, Self::Reset { .. } | Self::Reorg { .. })
    }
}

/// `node_tip_digest`: `None` if the node is shorter than the stored tip.
fn decide(
    stored: Option<&ChainState>,
    node: &ChainFingerprint,
    node_tip_digest: Option<&str>,
) -> ChainCheck {
    let Some(stored) = stored else {
        return ChainCheck::FirstRecord;
    };
    if stored.chain != *node {
        return ChainCheck::Reset {
            previous: stored.chain.clone(),
        };
    }
    match &stored.scanned_tip {
        Some(tip) if node_tip_digest != Some(tip.digest.as_str()) => {
            ChainCheck::Reorg { height: tip.height }
        }
        _ => ChainCheck::Same,
    }
}

/// `null` means the node has no block at that height.
fn parse_digest(result: &serde_json::Value) -> Result<Option<String>, String> {
    match &result["digest"] {
        serde_json::Value::Null => Ok(None),
        serde_json::Value::String(digest) => Ok(Some(digest.clone())),
        other => Err(format!(
            "archival_getBlockDigest returned an unexpected digest: {other}"
        )),
    }
}

async fn block_digest(
    client: &Client,
    rpc_url: &str,
    height: u64,
) -> Result<Option<String>, String> {
    let result = rpc_client::call_json_rpc(
        client,
        rpc_url,
        "archival_getBlockDigest",
        serde_json::json!([height]),
    )
    .await?;
    parse_digest(&result)
}

async fn fetch_fingerprint(client: &Client, rpc_url: &str) -> Result<ChainFingerprint, String> {
    let network = rpc_client::call_json_rpc(client, rpc_url, "node_network", serde_json::json!([]))
        .await?["network"]
        .as_str()
        .ok_or("node_network returned no network")?
        .to_string();
    let block_1_digest = block_digest(client, rpc_url, 1)
        .await?
        .ok_or("The node has no block 1 yet")?;
    Ok(ChainFingerprint {
        network,
        block_1_digest,
    })
}

fn read_state(path: &Path) -> Result<Option<ChainState>, String> {
    match std::fs::read_to_string(path) {
        Ok(text) => serde_json::from_str(&text)
            .map(Some)
            .map_err(|e| format!("Unreadable {}: {e}", path.display())),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Failed to read {}: {e}", path.display())),
    }
}

/// Temp file + rename: a write cut short must not leave a file that blocks every later scan.
fn write_state(path: &Path, state: &ChainState) -> Result<(), String> {
    let text = serde_json::to_string(state).map_err(|e| e.to_string())?;
    let tmp = path.with_extension("json.tmp");
    std::fs::write(&tmp, text)
        .and_then(|()| std::fs::rename(&tmp, path))
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

/// Read-only: the caller rescans from 0 on `needs_full_rescan`, then `record`s the new chain.
pub async fn check(
    client: &Client,
    rpc_url: &str,
) -> Result<(ChainCheck, ChainFingerprint), String> {
    let node = fetch_fingerprint(client, rpc_url).await?;
    let stored = read_state(&utxo_cache::chain_file_path())?;
    let node_tip_digest = match stored.as_ref().and_then(|s| s.scanned_tip.as_ref()) {
        Some(tip) => block_digest(client, rpc_url, tip.height).await?,
        None => None,
    };
    let check = decide(stored.as_ref(), &node, node_tip_digest.as_deref());
    if check.needs_full_rescan() {
        log::warn!("[Chain] {check:?}: the UTXO cache no longer matches the node");
    }
    Ok((check, node))
}

pub async fn scanned_tip(
    client: &Client,
    rpc_url: &str,
    height: u64,
) -> Result<ScannedTip, String> {
    let digest = block_digest(client, rpc_url, height)
        .await?
        .ok_or_else(|| format!("The node has no block {height}"))?;
    Ok(ScannedTip { height, digest })
}

pub fn record(chain: ChainFingerprint, tip: ScannedTip) -> Result<(), String> {
    write_state(
        &utxo_cache::chain_file_path(),
        &ChainState {
            chain,
            scanned_tip: Some(tip),
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fp(network: &str, digest: &str) -> ChainFingerprint {
        ChainFingerprint {
            network: network.into(),
            block_1_digest: digest.into(),
        }
    }

    fn state(chain: ChainFingerprint, tip: Option<(u64, &str)>) -> ChainState {
        ChainState {
            chain,
            scanned_tip: tip.map(|(height, digest)| ScannedTip {
                height,
                digest: digest.into(),
            }),
        }
    }

    #[test]
    fn first_check_records_fingerprint() {
        assert_eq!(
            decide(None, &fp("testnet-0", "aa"), None),
            ChainCheck::FirstRecord
        );
    }

    #[test]
    fn same_chain_keeps_cache() {
        let stored = state(fp("testnet-0", "aa"), Some((300, "t1")));
        assert_eq!(
            decide(Some(&stored), &fp("testnet-0", "aa"), Some("t1")),
            ChainCheck::Same
        );
        assert!(!ChainCheck::Same.needs_full_rescan());
    }

    #[test]
    fn different_block_1_digest_is_a_reset() {
        let stored = state(fp("testnet-0", "aa"), Some((300, "t1")));
        let check = decide(Some(&stored), &fp("testnet-0", "bb"), Some("t1"));
        assert_eq!(
            check,
            ChainCheck::Reset {
                previous: fp("testnet-0", "aa")
            }
        );
        assert!(check.needs_full_rescan());
    }

    #[test]
    fn different_network_resets() {
        let stored = state(fp("testnet-0", "aa"), None);
        assert!(decide(Some(&stored), &fp("main", "aa"), None).needs_full_rescan());
    }

    #[test]
    fn changed_scanned_tip_is_a_reorg() {
        let stored = state(fp("testnet-0", "aa"), Some((300, "t1")));
        assert_eq!(
            decide(Some(&stored), &fp("testnet-0", "aa"), Some("t2")),
            ChainCheck::Reorg { height: 300 }
        );
    }

    #[test]
    fn node_shorter_than_scanned_tip_is_a_reorg() {
        let stored = state(fp("testnet-0", "aa"), Some((300, "t1")));
        assert_eq!(
            decide(Some(&stored), &fp("testnet-0", "aa"), None),
            ChainCheck::Reorg { height: 300 }
        );
    }

    #[test]
    fn state_round_trips_through_json() {
        let original = state(fp("testnet-0", "aa"), Some((300, "t1")));
        let text = serde_json::to_string(&original).unwrap();
        assert!(text.contains("\"block1Digest\":\"aa\"") && text.contains("\"scannedTip\""));
        assert_eq!(serde_json::from_str::<ChainState>(&text).unwrap(), original);
    }

    #[test]
    fn written_state_reads_back() {
        let dir = std::env::temp_dir().join(format!("chain-fp-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("chain.json");
        let stored = state(fp("testnet-0", "aa"), Some((300, "t1")));
        write_state(&path, &stored).unwrap();
        assert_eq!(read_state(&path).unwrap(), Some(stored));
        assert!(!path.with_extension("json.tmp").exists());
        std::fs::remove_dir_all(&dir).unwrap();
    }

    #[test]
    fn null_digest_is_a_missing_block_and_other_shapes_are_errors() {
        assert_eq!(parse_digest(&serde_json::json!({"digest": null})), Ok(None));
        assert_eq!(
            parse_digest(&serde_json::json!({"digest": "ab"})),
            Ok(Some("ab".into()))
        );
        assert!(parse_digest(&serde_json::json!({"digest": 7})).is_err());
    }
}
