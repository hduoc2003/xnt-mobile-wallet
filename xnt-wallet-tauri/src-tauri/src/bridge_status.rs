use std::collections::{HashMap, HashSet};
use std::path::{Path, PathBuf};

use reqwest::{Client, StatusCode};
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager};
use tokio::sync::Mutex;

use crate::{rpc_client, rpc_config};

const RECORDS_FILE: &str = "bridge-deposits.json";
/// Consecutive polls with the deposit neither in the mempool nor in the AOCL before it counts as dropped.
const DROP_AFTER_MISSES: u8 = 3;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(
    tag = "state",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum DepositStatus {
    Submitted { misses: u8 },
    Confirmed { block_height: Option<u64> },
    Minted { block_height: Option<u64> },
    Dropped,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum RelayerState {
    Minted,
    Submitted,
    Refused,
    Waiting,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum RefusalReason {
    Invalid,
    BelowMintFee,
    NoSnapshot,
}

/// Body of the relayer's deposit status, served at `{edge}/bridge/deposits/{commitment_hex}`.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RelayerDepositStatus {
    pub minted: bool,
    pub status: RelayerState,
    pub reason: Option<RefusalReason>,
    /// Fee charged (submitted/minted) or required (below_mint_fee refusal), WXNT base units.
    pub mint_fee: Option<u64>,
    /// WXNT in the depositor's note once the mint was submitted, base units.
    pub minted_wxnt: Option<u64>,
    pub mintable_up_to: Option<u64>,
    pub paused: bool,
    /// Leviathan mint tx and the depositor's WXNT P2ID note; only once the mint was submitted.
    pub mint_tx_id: Option<String>,
    pub mint_note_id: Option<String>,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct DepositRecord {
    /// Hex of the bridge-lock UTXO's canonical commitment; the deposit id.
    pub utxo_commitment: String,
    pub tx_id: String,
    pub account_id: [u64; 2],
    /// Recipient as the user entered it; absent in records written before it was stored.
    #[serde(default, alias = "midenAddress")]
    pub leviathan_address: Option<String>,
    pub amount: String,
    /// Absent in records written before fees were stored.
    #[serde(default)]
    pub fee: Option<String>,
    /// Canonical commitments of all the deposit tx's outputs (bridge lock + change).
    #[serde(default)]
    pub output_commitments: Vec<String>,
    pub created_ms: u64,
    pub status: DepositStatus,
    pub relayer: Option<RelayerDepositStatus>,
    /// Why the last relayer check failed; cleared by the next successful one.
    pub relayer_error: Option<String>,
    /// The wallet account that made the deposit; records from before accounts belong to Account 1.
    #[serde(default = "account_1")]
    pub wallet_account: String,
}

fn account_1() -> String {
    crate::accounts::account_id(crate::accounts::MAIN_SEED, 0)
}

pub struct Observation {
    pub in_mempool: bool,
    pub in_aocl: bool,
    pub block_height: Option<u64>,
    pub minted: bool,
}

pub fn next_status(current: &DepositStatus, obs: &Observation) -> DepositStatus {
    match current {
        DepositStatus::Submitted { misses } => {
            if obs.in_aocl {
                DepositStatus::Confirmed {
                    block_height: obs.block_height,
                }
            } else if obs.in_mempool {
                DepositStatus::Submitted { misses: 0 }
            } else if misses + 1 >= DROP_AFTER_MISSES {
                DepositStatus::Dropped
            } else {
                DepositStatus::Submitted { misses: misses + 1 }
            }
        }
        DepositStatus::Confirmed { block_height } if obs.minted => DepositStatus::Minted {
            block_height: *block_height,
        },
        // A mined deposit can outlive a false drop, so dropped ones keep watching the AOCL.
        DepositStatus::Dropped if obs.in_aocl => DepositStatus::Confirmed {
            block_height: obs.block_height,
        },
        unchanged => unchanged.clone(),
    }
}

/// Serializes read-modify-write cycles of the records file.
static RECORDS_FILE_LOCK: Mutex<()> = Mutex::const_new(());

async fn records_path(app: &AppHandle) -> Result<PathBuf, String> {
    let dir = app
        .path()
        .app_data_dir()
        .map_err(|e| format!("No app data dir: {e}"))?;
    tokio::fs::create_dir_all(&dir)
        .await
        .map_err(|e| format!("Failed to create {}: {e}", dir.display()))?;
    Ok(dir.join(RECORDS_FILE))
}

pub async fn load_records_from(path: &Path) -> Result<Vec<DepositRecord>, String> {
    let text = match tokio::fs::read_to_string(path).await {
        Ok(text) => text,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(Vec::new()),
        Err(e) => return Err(format!("Failed to read {}: {e}", path.display())),
    };
    serde_json::from_str(&text).map_err(|e| format!("Corrupt {}: {e}", path.display()))
}

pub async fn save_records_to(path: &Path, records: &[DepositRecord]) -> Result<(), String> {
    let tmp = path.with_extension("json.tmp");
    let json = serde_json::to_string_pretty(records).map_err(|e| e.to_string())?;
    tokio::fs::write(&tmp, json)
        .await
        .map_err(|e| format!("Failed to write {}: {e}", tmp.display()))?;
    tokio::fs::rename(&tmp, path)
        .await
        .map_err(|e| format!("Failed to replace {}: {e}", path.display()))
}

pub async fn list(app: &AppHandle) -> Result<Vec<DepositRecord>, String> {
    load_records_from(&records_path(app).await?).await
}

/// Records a proven deposit before it is broadcast, so a crash after a successful broadcast
/// can't lose it; one that never reaches the mempool ages into `Dropped`. Idempotent per commitment.
pub async fn add(app: &AppHandle, record: DepositRecord) -> Result<(), String> {
    let _guard = RECORDS_FILE_LOCK.lock().await;
    let path = records_path(app).await?;
    let mut records = load_records_from(&path).await?;
    if records
        .iter()
        .any(|r| r.utxo_commitment == record.utxo_commitment)
    {
        return Ok(());
    }
    records.push(record);
    save_records_to(&path, &records).await
}

pub async fn remove_for_accounts(app: &AppHandle, account_ids: &[String]) -> Result<(), String> {
    let _guard = RECORDS_FILE_LOCK.lock().await;
    let path = records_path(app).await?;
    let mut records = load_records_from(&path).await?;
    records.retain(|r| !account_ids.contains(&r.wallet_account));
    save_records_to(&path, &records).await
}

pub async fn discard(app: &AppHandle, commitment: &str) -> Result<Vec<DepositRecord>, String> {
    let _guard = RECORDS_FILE_LOCK.lock().await;
    let path = records_path(app).await?;
    let mut records = load_records_from(&path).await?;
    records.retain(|r| r.utxo_commitment != commitment);
    save_records_to(&path, &records).await?;
    Ok(records)
}

async fn relayer_status(
    client: &Client,
    edge_base: &str,
    commitment: &str,
) -> Result<RelayerDepositStatus, String> {
    let url = format!("{edge_base}/bridge/deposits/{commitment}");
    let response = client
        .get(&url)
        .send()
        .await
        .map_err(|e| format!("Relayer unreachable: {e}"))?;
    match response.status() {
        StatusCode::OK => response
            .json::<RelayerDepositStatus>()
            .await
            .map_err(|e| format!("Relayer sent an unexpected body: {e}")),
        StatusCode::SERVICE_UNAVAILABLE => Err("Relayer can't check mints right now".to_string()),
        status => {
            let body = response
                .text()
                .await
                .map_err(|e| format!("Relayer answered {status}, body unreadable: {e}"))?;
            Err(format!("Relayer answered {status}: {body}"))
        }
    }
}

/// Polls one deposit; node and relayer failures specific to it are logged, not fatal.
async fn observe(
    client: &Client,
    rpc_url: &str,
    edge_base: &str,
    record: &DepositRecord,
    in_aocl: bool,
    mempool: Option<&HashSet<String>>,
) -> DepositRecord {
    let mut updated = record.clone();
    let commitment = &record.utxo_commitment;
    let becomes_confirmed = in_aocl
        && matches!(
            record.status,
            DepositStatus::Submitted { .. } | DepositStatus::Dropped
        );
    let block_height = if becomes_confirmed {
        rpc_client::output_block_height(client, rpc_url, commitment)
            .await
            .unwrap_or_else(|e| {
                log::warn!("[Bridge] block height of {commitment}: {e}");
                None
            })
    } else {
        None
    };
    let check_mint = in_aocl || matches!(record.status, DepositStatus::Confirmed { .. });
    let minted = check_mint
        && match relayer_status(client, edge_base, commitment).await {
            Ok(status) => {
                updated.relayer_error = None;
                let minted = status.minted;
                updated.relayer = Some(status);
                minted
            }
            Err(e) => {
                log::warn!("[Bridge] mint status of {commitment}: {e}");
                updated.relayer_error = Some(e);
                false
            }
        };
    let obs = Observation {
        in_mempool: mempool.is_some_and(|m| m.contains(commitment)),
        in_aocl,
        block_height,
        minted,
    };
    updated.status = next_status(&record.status, &obs);
    updated
}

/// Polls every deposit that isn't minted and persists the result. Network calls run without
/// the file lock; updates are merged by commitment so concurrent submits are kept.
pub async fn refresh(app: &AppHandle) -> Result<Vec<DepositRecord>, String> {
    let path = records_path(app).await?;
    let snapshot = {
        let _guard = RECORDS_FILE_LOCK.lock().await;
        load_records_from(&path).await?
    };
    let to_check: Vec<&DepositRecord> = snapshot
        .iter()
        .filter(|r| !matches!(r.status, DepositStatus::Minted { .. }))
        .collect();
    if to_check.is_empty() {
        return Ok(snapshot);
    }
    let rpc_url = rpc_config::resolve_rpc_url(app).await?;
    let edge_base = rpc_config::edge_base_url(&rpc_url)?;
    let client = Client::builder()
        .timeout(std::time::Duration::from_secs(30))
        .build()
        .map_err(|e| format!("Failed to create HTTP client: {e}"))?;
    let commitments: Vec<String> = to_check.iter().map(|r| r.utxo_commitment.clone()).collect();
    let aocl = rpc_client::get_aocl_leaf_indices(&client, &rpc_url, &commitments).await?;
    if aocl.len() != commitments.len() {
        return Err(format!(
            "archival_getAoclLeafIndices returned {} entries for {} commitments",
            aocl.len(),
            commitments.len()
        ));
    }
    let mempool = if to_check
        .iter()
        .any(|r| matches!(r.status, DepositStatus::Submitted { .. }))
    {
        Some(rpc_client::get_mempool_output_commitments(&client, &rpc_url).await?)
    } else {
        None
    };
    let mut updates = HashMap::new();
    for (record, index) in to_check.into_iter().zip(aocl) {
        let updated = observe(
            &client,
            &rpc_url,
            &edge_base,
            record,
            index.is_some(),
            mempool.as_ref(),
        )
        .await;
        updates.insert(updated.utxo_commitment.clone(), updated);
    }
    let _guard = RECORDS_FILE_LOCK.lock().await;
    let mut records = load_records_from(&path).await?;
    for record in records.iter_mut() {
        if let Some(updated) = updates.remove(&record.utxo_commitment) {
            *record = updated;
        }
    }
    save_records_to(&path, &records).await?;
    Ok(records)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn obs(
        in_mempool: bool,
        in_aocl: bool,
        block_height: Option<u64>,
        minted: bool,
    ) -> Observation {
        Observation {
            in_mempool,
            in_aocl,
            block_height,
            minted,
        }
    }

    #[test]
    fn submitted_to_confirmed_on_aocl() {
        let next = next_status(
            &DepositStatus::Submitted { misses: 2 },
            &obs(false, true, Some(180), false),
        );
        assert_eq!(
            next,
            DepositStatus::Confirmed {
                block_height: Some(180)
            }
        );
    }

    #[test]
    fn submitted_stays_while_in_mempool() {
        let next = next_status(
            &DepositStatus::Submitted { misses: 2 },
            &obs(true, false, None, false),
        );
        assert_eq!(next, DepositStatus::Submitted { misses: 0 });
    }

    #[test]
    fn dropped_after_three_misses() {
        let mut status = DepositStatus::Submitted { misses: 0 };
        for _ in 0..2 {
            status = next_status(&status, &obs(false, false, None, false));
            assert!(matches!(status, DepositStatus::Submitted { .. }));
        }
        assert_eq!(
            next_status(&status, &obs(false, false, None, false)),
            DepositStatus::Dropped
        );
    }

    #[test]
    fn relayer_down_keeps_confirmed_and_height() {
        let next = next_status(
            &DepositStatus::Confirmed {
                block_height: Some(180),
            },
            &obs(false, true, None, false),
        );
        assert_eq!(
            next,
            DepositStatus::Confirmed {
                block_height: Some(180)
            }
        );
    }

    #[test]
    fn submitted_never_jumps_to_minted() {
        let next = next_status(
            &DepositStatus::Submitted { misses: 0 },
            &obs(false, true, Some(180), true),
        );
        assert_eq!(
            next,
            DepositStatus::Confirmed {
                block_height: Some(180)
            }
        );
    }

    #[test]
    fn dropped_recovers_when_mined() {
        let next = next_status(&DepositStatus::Dropped, &obs(false, true, Some(182), false));
        assert_eq!(
            next,
            DepositStatus::Confirmed {
                block_height: Some(182)
            }
        );
        assert_eq!(
            next_status(&DepositStatus::Dropped, &obs(true, false, None, true)),
            DepositStatus::Dropped
        );
    }

    #[test]
    fn minted_from_relayer() {
        let next = next_status(
            &DepositStatus::Confirmed {
                block_height: Some(180),
            },
            &obs(false, true, None, true),
        );
        assert_eq!(
            next,
            DepositStatus::Minted {
                block_height: Some(180)
            }
        );
    }

    #[test]
    fn minted_stays() {
        let minted = DepositStatus::Minted {
            block_height: Some(5),
        };
        assert_eq!(
            next_status(&minted, &obs(false, false, None, false)),
            minted
        );
    }

    async fn one_shot_server(status_line: &'static str, body: &'static str) -> String {
        use tokio::io::{AsyncReadExt, AsyncWriteExt};
        let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
        let addr = listener.local_addr().unwrap();
        tokio::spawn(async move {
            let (mut socket, _) = listener.accept().await.unwrap();
            let mut buf = [0u8; 1024];
            let _ = socket.read(&mut buf).await.unwrap();
            let response = format!(
                "HTTP/1.1 {status_line}\r\ncontent-type: application/json\r\ncontent-length: {}\r\nconnection: close\r\n\r\n{body}",
                body.len()
            );
            socket.write_all(response.as_bytes()).await.unwrap();
        });
        format!("http://{addr}")
    }

    #[tokio::test]
    async fn relayer_ok_is_parsed() {
        let base = one_shot_server(
            "200 OK",
            r#"{"minted":false,"status":"waiting","mintableUpTo":170,"paused":false}"#,
        )
        .await;
        let status = relayer_status(&Client::new(), &base, "ab").await.unwrap();
        assert_eq!(status.status, RelayerState::Waiting);
        assert_eq!(status.mintable_up_to, Some(170));
    }

    #[tokio::test]
    async fn relayer_stale_snapshot_is_an_error() {
        let base = one_shot_server("503 Service Unavailable", r#"{"error":"stale"}"#).await;
        let err = relayer_status(&Client::new(), &base, "ab")
            .await
            .unwrap_err();
        assert!(err.contains("can't check"), "{err}");
    }

    #[tokio::test]
    async fn relayer_bad_request_surfaces_body() {
        let base = one_shot_server("400 Bad Request", r#"{"error":"bad hex"}"#).await;
        let err = relayer_status(&Client::new(), &base, "zz")
            .await
            .unwrap_err();
        assert!(err.contains("400") && err.contains("bad hex"), "{err}");
    }

    #[test]
    fn record_without_fee_or_outputs_still_loads() {
        let json = format!(
            r#"{{"utxoCommitment":"{c}","txId":"{c}","accountId":[1,2],"amount":"1","createdMs":7,"status":{{"state":"submitted","misses":0}},"relayer":null,"relayerError":null}}"#,
            c = "ab".repeat(40)
        );
        let record: DepositRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.fee, None);
        assert_eq!(record.leviathan_address, None);
        assert!(record.output_commitments.is_empty());
    }

    #[test]
    fn record_with_the_old_recipient_field_keeps_it() {
        let json = format!(
            r#"{{"utxoCommitment":"{c}","txId":"{c}","accountId":[1,2],"midenAddress":"mtst1old","amount":"1","createdMs":7,"status":{{"state":"submitted","misses":0}},"relayer":null,"relayerError":null}}"#,
            c = "ab".repeat(40)
        );
        let record: DepositRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.leviathan_address.as_deref(), Some("mtst1old"));
        assert!(serde_json::to_string(&record)
            .unwrap()
            .contains(r#""leviathanAddress":"mtst1old""#));
    }

    #[test]
    fn legacy_record_belongs_to_account_1() {
        let json = format!(
            r#"{{"utxoCommitment":"{c}","txId":"{c}","accountId":[1,2],"amount":"1","createdMs":7,"status":{{"state":"submitted","misses":0}},"relayer":null,"relayerError":null}}"#,
            c = "ab".repeat(40)
        );
        let record: DepositRecord = serde_json::from_str(&json).unwrap();
        assert_eq!(record.wallet_account, "main:0");
    }

    #[test]
    fn status_wire_format_matches_frontend() {
        let json = serde_json::to_string(&DepositStatus::Confirmed {
            block_height: Some(3),
        })
        .unwrap();
        assert_eq!(json, r#"{"state":"confirmed","blockHeight":3}"#);
    }

    #[test]
    fn relayer_body_carries_mint_ids() {
        let minted: RelayerDepositStatus = serde_json::from_str(
            r#"{"minted":true,"status":"minted","mintableUpTo":9,"paused":false,"mintTxId":"0xaa","mintNoteId":"0xbb"}"#,
        )
        .unwrap();
        assert_eq!(minted.mint_tx_id.as_deref(), Some("0xaa"));
        assert_eq!(minted.mint_note_id.as_deref(), Some("0xbb"));
        assert_eq!(minted.minted_wxnt, None);
    }

    #[test]
    fn relayer_body_parses_contract() {
        let refused: RelayerDepositStatus = serde_json::from_str(
            r#"{"minted":false,"status":"refused","reason":"below_mint_fee","mintFee":42,"mintableUpTo":null,"paused":false}"#,
        )
        .unwrap();
        assert_eq!(refused.reason, Some(RefusalReason::BelowMintFee));
        assert_eq!(refused.mint_fee, Some(42));
        assert_eq!(refused.mintable_up_to, None);
        let minted: RelayerDepositStatus = serde_json::from_str(
            r#"{"minted":true,"status":"minted","mintableUpTo":190,"paused":false}"#,
        )
        .unwrap();
        assert!(minted.minted);
        assert_eq!(minted.reason, None);
        assert_eq!(minted.mint_tx_id, None);
    }

    #[tokio::test]
    async fn records_round_trip_on_disk() {
        let dir = std::env::temp_dir().join(format!("bridge-status-test-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join(RECORDS_FILE);
        assert!(load_records_from(&path).await.unwrap().is_empty());
        let record = DepositRecord {
            utxo_commitment: "ab".repeat(40),
            tx_id: "cd".repeat(40),
            account_id: [1, 2],
            leviathan_address: Some("mtst1test".to_string()),
            amount: "1".to_string(),
            fee: Some("0.01".to_string()),
            output_commitments: vec!["ab".repeat(40)],
            created_ms: 7,
            status: DepositStatus::Confirmed {
                block_height: Some(9),
            },
            relayer: None,
            relayer_error: Some("down".to_string()),
            wallet_account: "imp-5:2".to_string(),
        };
        save_records_to(&path, &[record.clone(), record.clone()])
            .await
            .unwrap();
        assert_eq!(
            load_records_from(&path).await.unwrap(),
            vec![record.clone(), record]
        );
        std::fs::remove_dir_all(&dir).unwrap();
    }
}
