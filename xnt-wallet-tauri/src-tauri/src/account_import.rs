use neptune_privacy::state::wallet::secret_key_material::SecretKeyMaterial;
use neptune_privacy::state::wallet::wallet_file::WalletFile;
use serde::Deserialize;
use std::path::Path;

use crate::accounts::{self, SeedEntry, SeedSource, MAIN_SEED};
use crate::keystore;

pub const DUPLICATE_SEED: &str = "This seed is already in the wallet";

#[derive(Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "camelCase",
    rename_all_fields = "camelCase"
)]
pub enum ImportSource {
    Phrase {
        words: Vec<String>,
    },
    /// File contents, not paths: Android's file picker returns `content://` URIs.
    BackupFile {
        content: String,
        file_password: String,
    },
    NodeWalletDat {
        content: String,
    },
}

/// Leaves out the words, the file contents and the password.
impl std::fmt::Debug for ImportSource {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(match self {
            Self::Phrase { .. } => "Phrase",
            Self::BackupFile { .. } => "BackupFile",
            Self::NodeWalletDat { .. } => "NodeWalletDat",
        })
    }
}

impl ImportSource {
    fn seed_source(&self) -> SeedSource {
        match self {
            Self::Phrase { .. } => SeedSource::Phrase,
            Self::BackupFile { .. } => SeedSource::BackupFile,
            Self::NodeWalletDat { .. } => SeedSource::NodeWalletDat,
        }
    }
}

/// Two phrases are the same seed iff these are equal (`to_phrase` does not round-trip every phrase).
/// The error never quotes the words.
fn parse_phrase(words: &[String]) -> Result<SecretKeyMaterial, String> {
    let words: Vec<String> = words.iter().map(|w| w.trim().to_lowercase()).collect();
    SecretKeyMaterial::from_phrase(&words).map_err(|_| "Invalid seed phrase".to_string())
}

fn words(phrase: &str) -> Vec<String> {
    phrase.split_whitespace().map(String::from).collect()
}

/// Blocking: a backup file costs a PBKDF2 run.
pub fn seed_from_source(source: &ImportSource) -> Result<SecretKeyMaterial, String> {
    match source {
        ImportSource::Phrase { words } => parse_phrase(words),
        ImportSource::BackupFile {
            content,
            file_password,
        } => {
            let mnemonic = keystore::mnemonic_from_keystore(content, file_password)
                .map_err(|e| format!("Backup file: {e}"))?;
            parse_phrase(&words(&mnemonic))
        }
        ImportSource::NodeWalletDat { content } => {
            let wallet: WalletFile =
                serde_json::from_str(content).map_err(|_| "Not a node wallet file".to_string())?;
            Ok(wallet.secret_key())
        }
    }
}

/// Blocking. A main keystore that doesn't open is a wrong password; another group is skipped.
fn ensure_new_seed(
    seed: &SecretKeyMaterial,
    keystores: Vec<(String, Result<String, String>)>,
    wallet_password: &str,
) -> Result<(), String> {
    for (seed_id, content) in keystores {
        let stored = content
            .and_then(|c| keystore::mnemonic_from_keystore(&c, wallet_password))
            .and_then(|mnemonic| parse_phrase(&words(&mnemonic)));
        match stored {
            Ok(stored) if stored == *seed => return Err(DUPLICATE_SEED.into()),
            Ok(_) => {}
            Err(e) if seed_id == MAIN_SEED => return Err(format!("Wallet password: {e}")),
            Err(e) => {
                log::warn!("[Import] Not compared with seed {seed_id}, it does not open: {e}")
            }
        }
    }
    Ok(())
}

/// On an error nothing is written, except the one-time registry migration.
pub async fn import_at(
    data_dir: &Path,
    source: ImportSource,
    wallet_password: String,
) -> Result<SeedEntry, String> {
    let seed_source = source.seed_source();
    let _guard = accounts::lock().await;
    let mut registry = accounts::load_or_migrate_at(data_dir).await?;
    let mut keystores = Vec::new();
    for entry in &registry.seeds {
        let path = accounts::keystore_path(data_dir, entry)?;
        let content = tokio::fs::read_to_string(&path)
            .await
            .map_err(|e| format!("Failed to read {}: {e}", path.display()));
        keystores.push((entry.id.clone(), content));
    }
    let password = wallet_password.clone();
    let seed = tokio::task::spawn_blocking(move || {
        let seed = seed_from_source(&source)?;
        ensure_new_seed(&seed, keystores, &password)?;
        Ok::<_, String>(seed)
    })
    .await
    .map_err(|e| format!("Import task failed: {e}"))??;

    let id = format!("imp-{}", chrono::Utc::now().timestamp_millis());
    let entry = SeedEntry {
        keystore_file: format!("{id}.json"),
        id,
        source: seed_source,
        backed_up: false,
    };
    registry.seeds.push(entry.clone());
    registry.add(&entry.id, 0)?;

    let path = accounts::keystore_path(data_dir, &entry)?;
    let written = async {
        tokio::fs::create_dir_all(accounts::imported_dir(data_dir))
            .await
            .map_err(|e| format!("Failed to create the imported keystore directory: {e}"))?;
        keystore::write_keystore_file(&path, &seed.to_phrase().join(" "), &wallet_password).await?;
        accounts::save_at(data_dir, &registry).await
    }
    .await;
    if let Err(e) = written {
        match tokio::fs::remove_file(&path).await {
            Err(cleanup) if cleanup.kind() != std::io::ErrorKind::NotFound => log::warn!(
                "[Import] Failed to remove {} after an error: {cleanup}",
                path.display()
            ),
            _ => {}
        }
        return Err(e);
    }
    log::info!("[Import] Added seed {} from {seed_source:?}", entry.id);
    Ok(entry)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::accounts::test_util::TempDir;

    const WALLET_PW: &str = "wallet-pw";
    const MAIN_PHRASE: &str = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent";
    /// The seed in `test_data/node_wallet.dat`.
    const OTHER_PHRASE: &str = "year zoo zone abandon abandon about winner zoo yellow abandon abandon acid useful zoo winter abandon abandon aim";
    const NODE_WALLET_DAT: &str = include_str!("../test_data/node_wallet.dat");

    fn phrase(text: &str) -> ImportSource {
        ImportSource::Phrase { words: words(text) }
    }

    fn node_wallet_dat() -> ImportSource {
        ImportSource::NodeWalletDat {
            content: NODE_WALLET_DAT.into(),
        }
    }

    /// A backup file of `OTHER_PHRASE` encrypted with "file-pw", opened with `file_password`.
    async fn backup_file(file_password: &str) -> ImportSource {
        let dir = TempDir::new("backup-file");
        let path = dir.0.join("backup.json");
        keystore::write_keystore_file(&path, OTHER_PHRASE, "file-pw")
            .await
            .unwrap();
        ImportSource::BackupFile {
            content: std::fs::read_to_string(path).unwrap(),
            file_password: file_password.into(),
        }
    }

    async fn data_dir(name: &str) -> TempDir {
        let dir = TempDir::new(name);
        keystore::write_keystore_file(
            &dir.0.join("wallets/xnt-wallet-1.json"),
            MAIN_PHRASE,
            WALLET_PW,
        )
        .await
        .unwrap();
        dir
    }

    fn imported_files(dir: &TempDir) -> usize {
        std::fs::read_dir(accounts::imported_dir(&dir.0))
            .unwrap()
            .count()
    }

    async fn import(dir: &TempDir, source: ImportSource) -> Result<SeedEntry, String> {
        import_at(&dir.0, source, WALLET_PW.into()).await
    }

    #[test]
    fn phrase_import_round_trips() {
        let seed = seed_from_source(&phrase(&format!(" {} ", MAIN_PHRASE.to_uppercase()))).unwrap();
        assert_eq!(seed.to_phrase().join(" "), MAIN_PHRASE);
    }

    #[test]
    fn invalid_phrase_is_an_error() {
        let checksum_broken = MAIN_PHRASE.replace("agent", "abandon");
        assert_eq!(
            seed_from_source(&phrase(&checksum_broken)),
            Err("Invalid seed phrase".into())
        );
        assert!(seed_from_source(&phrase("abandon")).is_err());
    }

    #[test]
    fn node_wallet_dat_gives_its_seed() {
        let seed = seed_from_source(&node_wallet_dat()).unwrap();
        assert_eq!(seed.to_phrase().join(" "), OTHER_PHRASE);
        let garbage = ImportSource::NodeWalletDat {
            content: "{\"secret_seed\": 1}".into(),
        };
        assert_eq!(
            seed_from_source(&garbage),
            Err("Not a node wallet file".into())
        );
    }

    #[tokio::test]
    async fn backup_file_decrypts_with_its_password() {
        let seed = seed_from_source(&backup_file("file-pw").await).unwrap();
        assert_eq!(seed.to_phrase().join(" "), OTHER_PHRASE);
        assert!(seed_from_source(&backup_file("nope").await).is_err());
    }

    #[tokio::test]
    async fn backup_file_wrong_password_writes_nothing() {
        let dir = data_dir("backup-wrong").await;
        accounts::load_at(&dir.0).await.unwrap();
        let before = std::fs::read(dir.0.join("accounts.json")).unwrap();
        assert!(import(&dir, backup_file("nope").await).await.is_err());
        assert_eq!(std::fs::read(dir.0.join("accounts.json")).unwrap(), before);
        assert_eq!(imported_files(&dir), 0);
    }

    #[tokio::test]
    async fn wrong_wallet_password_writes_nothing() {
        let dir = data_dir("wallet-wrong").await;
        accounts::load_at(&dir.0).await.unwrap();
        let before = std::fs::read(dir.0.join("accounts.json")).unwrap();
        let error = import_at(&dir.0, phrase(OTHER_PHRASE), "nope".into())
            .await
            .unwrap_err();
        assert!(error.starts_with("Wallet password"), "{error}");
        assert_eq!(std::fs::read(dir.0.join("accounts.json")).unwrap(), before);
        assert_eq!(imported_files(&dir), 0);
    }

    #[tokio::test]
    async fn rejects_duplicate_seed() {
        let dir = data_dir("duplicate").await;
        assert_eq!(
            import(&dir, phrase(MAIN_PHRASE)).await,
            Err(DUPLICATE_SEED.into())
        );
        import(&dir, phrase(OTHER_PHRASE)).await.unwrap();
        assert_eq!(
            import(&dir, node_wallet_dat()).await,
            Err(DUPLICATE_SEED.into())
        );
        assert_eq!(accounts::load_at(&dir.0).await.unwrap().seeds.len(), 2);
        assert_eq!(imported_files(&dir), 1);
    }

    #[tokio::test]
    async fn locked_group_does_not_block_imports() {
        let dir = data_dir("locked").await;
        let locked = import(&dir, phrase(OTHER_PHRASE)).await.unwrap();
        std::fs::write(accounts::keystore_path(&dir.0, &locked).unwrap(), "{}").unwrap();
        let third = bip39::Mnemonic::from_entropy(&[1u8; 24], bip39::Language::English).unwrap();
        import(&dir, phrase(third.phrase())).await.unwrap();
        assert_eq!(accounts::load_at(&dir.0).await.unwrap().seeds.len(), 3);
    }

    #[tokio::test]
    async fn import_adds_group_with_its_keystore() {
        let dir = data_dir("import").await;
        let entry = import(&dir, backup_file("file-pw").await).await.unwrap();
        assert!(entry.id.starts_with("imp-"));
        assert_eq!(entry.source, SeedSource::BackupFile);
        assert!(!entry.backed_up);

        let registry = accounts::load_at(&dir.0).await.unwrap();
        assert_eq!(registry.active, "main:0");
        let account = registry
            .accounts
            .iter()
            .find(|a| a.seed_id == entry.id)
            .unwrap();
        assert_eq!(
            (account.index, account.id.clone()),
            (0, format!("{}:0", entry.id))
        );

        let content =
            std::fs::read_to_string(accounts::keystore_path(&dir.0, &entry).unwrap()).unwrap();
        assert_eq!(
            keystore::mnemonic_from_keystore(&content, WALLET_PW).unwrap(),
            OTHER_PHRASE
        );
    }
}
