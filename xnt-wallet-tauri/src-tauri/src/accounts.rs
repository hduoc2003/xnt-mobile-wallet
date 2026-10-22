use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};
use tauri::Manager;

use crate::keystore;

pub const MAIN_SEED: &str = "main";
const REGISTRY_FILE: &str = "accounts.json";
/// Imported keystores live here so `newest_keystore_file` never picks one as the main keystore.
const IMPORTED_DIR: &str = "imported";

static REGISTRY_LOCK: tokio::sync::Mutex<()> = tokio::sync::Mutex::const_new(());

#[derive(Serialize, Deserialize, Clone, Copy, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum SeedSource {
    Created,
    Phrase,
    BackupFile,
    NodeWalletDat,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct SeedEntry {
    pub id: String,
    pub keystore_file: String,
    pub source: SeedSource,
    pub backed_up: bool,
}

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountEntry {
    pub id: String,
    pub seed_id: String,
    pub index: u64,
    /// `None` shows the default "Account N".
    pub name: Option<String>,
    pub hidden: bool,
}

/// Contents of `accounts.json`; holds no secrets.
#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct Registry {
    pub seeds: Vec<SeedEntry>,
    pub accounts: Vec<AccountEntry>,
    pub active: String,
}

pub fn account_id(seed_id: &str, index: u64) -> String {
    format!("{seed_id}:{index}")
}

impl Registry {
    pub fn for_main(keystore_file: String) -> Self {
        let active = account_id(MAIN_SEED, 0);
        Self {
            seeds: vec![SeedEntry {
                id: MAIN_SEED.into(),
                keystore_file,
                source: SeedSource::Created,
                backed_up: true,
            }],
            accounts: vec![AccountEntry {
                id: active.clone(),
                seed_id: MAIN_SEED.into(),
                index: 0,
                name: None,
                hidden: false,
            }],
            active,
        }
    }

    /// A new main wallet: main seed accounts go back to Account 1; imported groups stay.
    pub fn replace_main(&mut self, keystore_file: String) {
        let fresh = Self::for_main(keystore_file);
        self.seeds.retain(|s| s.id != MAIN_SEED);
        self.seeds.insert(0, fresh.seeds[0].clone());
        self.accounts.retain(|a| a.seed_id != MAIN_SEED);
        self.accounts.insert(0, fresh.accounts[0].clone());
        self.active = fresh.active;
    }

    fn account_mut(&mut self, id: &str) -> Result<&mut AccountEntry, String> {
        self.accounts
            .iter_mut()
            .find(|a| a.id == id)
            .ok_or_else(|| format!("Unknown account {id}"))
    }

    fn seed_mut(&mut self, seed_id: &str) -> Result<&mut SeedEntry, String> {
        self.seeds
            .iter_mut()
            .find(|s| s.id == seed_id)
            .ok_or_else(|| format!("Unknown seed {seed_id}"))
    }

    pub fn add(&mut self, seed_id: &str, index: u64) -> Result<AccountEntry, String> {
        self.seed_mut(seed_id)?;
        let id = account_id(seed_id, index);
        if self.accounts.iter().any(|a| a.id == id) {
            return Err(format!("Account {id} already exists"));
        }
        let entry = AccountEntry {
            id,
            seed_id: seed_id.into(),
            index,
            name: None,
            hidden: false,
        };
        self.accounts.push(entry.clone());
        Ok(entry)
    }

    pub fn rename(&mut self, id: &str, name: Option<String>) -> Result<(), String> {
        let name = match name {
            Some(n) if n.trim().is_empty() => return Err("Name cannot be empty".into()),
            Some(n) => Some(n.trim().to_string()),
            None => None,
        };
        self.account_mut(id)?.name = name;
        Ok(())
    }

    pub fn set_hidden(&mut self, id: &str, hidden: bool) -> Result<(), String> {
        if hidden && self.active == id {
            return Err("Switch to another account before hiding this one".into());
        }
        self.account_mut(id)?.hidden = hidden;
        Ok(())
    }

    pub fn set_active(&mut self, id: &str) -> Result<(), String> {
        let account = self.account_mut(id)?;
        account.hidden = false;
        self.active = id.into();
        Ok(())
    }

    pub fn mark_backed_up(&mut self, seed_id: &str) -> Result<(), String> {
        self.seed_mut(seed_id)?.backed_up = true;
        Ok(())
    }

    pub fn remove_seed(&mut self, seed_id: &str) -> Result<SeedEntry, String> {
        if seed_id == MAIN_SEED {
            return Err("The main seed cannot be removed".into());
        }
        let position = self
            .seeds
            .iter()
            .position(|s| s.id == seed_id)
            .ok_or_else(|| format!("Unknown seed {seed_id}"))?;
        let seed = self.seeds.remove(position);
        self.accounts.retain(|a| a.seed_id != seed_id);
        if !self.accounts.iter().any(|a| a.id == self.active) {
            self.set_active(&account_id(MAIN_SEED, 0))?;
        }
        Ok(seed)
    }

    fn validate(&self) -> Result<(), String> {
        let main = account_id(MAIN_SEED, 0);
        if !self.seeds.iter().any(|s| s.id == MAIN_SEED)
            || !self.accounts.iter().any(|a| a.id == main)
        {
            return Err("has no main seed or Account 1".into());
        }
        let mut seed_ids = std::collections::HashSet::new();
        for seed in &self.seeds {
            if !seed_ids.insert(seed.id.as_str()) {
                return Err(format!("lists seed {} twice", seed.id));
            }
            keystore_file_name(&seed.keystore_file)?;
        }
        let mut account_ids = std::collections::HashSet::new();
        for account in &self.accounts {
            if account.id != account_id(&account.seed_id, account.index)
                || !seed_ids.contains(account.seed_id.as_str())
                || !account_ids.insert(account.id.as_str())
            {
                return Err(format!("has an inconsistent account {}", account.id));
            }
        }
        match self.accounts.iter().find(|a| a.id == self.active) {
            Some(active) if !active.hidden => Ok(()),
            _ => Err(format!(
                "has an active account {} that is missing or hidden",
                self.active
            )),
        }
    }
}

/// Refuses anything but a bare file name, so a tampered registry cannot reach outside `wallets/`.
fn keystore_file_name(file: &str) -> Result<&str, String> {
    if Path::new(file).file_name().and_then(|n| n.to_str()) == Some(file) {
        Ok(file)
    } else {
        Err(format!("has an invalid keystore file name {file:?}"))
    }
}

pub fn keystore_path(data_dir: &Path, seed: &SeedEntry) -> Result<PathBuf, String> {
    let file = keystore_file_name(&seed.keystore_file)?;
    Ok(if seed.id == MAIN_SEED {
        data_dir.join("wallets").join(file)
    } else {
        imported_dir(data_dir).join(file)
    })
}

pub fn imported_dir(data_dir: &Path) -> PathBuf {
    data_dir.join("wallets").join(IMPORTED_DIR)
}

async fn read_at(data_dir: &Path) -> Result<Option<Registry>, String> {
    let path = data_dir.join(REGISTRY_FILE);
    match tokio::fs::read_to_string(&path).await {
        Ok(text) => {
            let registry: Registry = serde_json::from_str(&text)
                .map_err(|e| format!("Unreadable {}: {e}", path.display()))?;
            registry
                .validate()
                .map_err(|e| format!("{} {e}", path.display()))?;
            Ok(Some(registry))
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(format!("Failed to read {}: {e}", path.display())),
    }
}

/// Synced temp file + rename: unlock depends on this file, so a power cut must not empty it.
pub async fn save_at(data_dir: &Path, registry: &Registry) -> Result<(), String> {
    use tokio::io::AsyncWriteExt;
    // A saved registry that fails validation would block every later unlock.
    registry
        .validate()
        .map_err(|e| format!("Refusing to save a registry that {e}"))?;
    let path = data_dir.join(REGISTRY_FILE);
    let tmp = path.with_extension("json.tmp");
    let text = serde_json::to_string_pretty(registry).map_err(|e| e.to_string())?;
    let write_err = |e: std::io::Error| format!("Failed to write {}: {e}", tmp.display());
    let mut file = tokio::fs::File::create(&tmp).await.map_err(write_err)?;
    file.write_all(text.as_bytes()).await.map_err(write_err)?;
    file.sync_all().await.map_err(write_err)?;
    tokio::fs::rename(&tmp, &path)
        .await
        .map_err(|e| format!("Failed to write {}: {e}", path.display()))
}

pub async fn lock() -> tokio::sync::MutexGuard<'static, ()> {
    REGISTRY_LOCK.lock().await
}

/// Callers hold `lock()`.
pub async fn load_or_migrate_at(data_dir: &Path) -> Result<Registry, String> {
    let wallets = data_dir.join("wallets");
    if let Some(mut registry) = read_at(data_dir).await? {
        if let Some(newer) = newer_main_keystore(data_dir, &registry).await? {
            log::warn!("[Accounts] {newer} is newer than the registered main keystore; it becomes the main wallet");
            registry.replace_main(newer);
            save_at(data_dir, &registry).await?;
        }
        return Ok(registry);
    }
    let keystore_file = keystore::newest_keystore_file(&wallets).await?;
    let registry = Registry::for_main(keystore_file);
    save_at(data_dir, &registry).await?;
    log::info!("[Accounts] Created the registry for the existing wallet as Account 1");
    Ok(registry)
}

/// A keystore newer than the registry that isn't its main (failed registry update, older app).
async fn newer_main_keystore(
    data_dir: &Path,
    registry: &Registry,
) -> Result<Option<String>, String> {
    let wallets = data_dir.join("wallets");
    let newest = keystore::newest_keystore_file(&wallets).await?;
    let main = registry
        .seeds
        .iter()
        .find(|s| s.id == MAIN_SEED)
        .ok_or("The registry has no main seed")?;
    if newest == main.keystore_file {
        return Ok(None);
    }
    let modified = |path: PathBuf| async move {
        tokio::fs::metadata(&path)
            .await
            .and_then(|m| m.modified())
            .map_err(|e| format!("Failed to read {}: {e}", path.display()))
    };
    let main_exists = tokio::fs::try_exists(wallets.join(&main.keystore_file))
        .await
        .map_err(|e| format!("Failed to check the main keystore: {e}"))?;
    if !main_exists
        || modified(wallets.join(&newest)).await? > modified(data_dir.join(REGISTRY_FILE)).await?
    {
        Ok(Some(newest))
    } else {
        Ok(None)
    }
}

pub async fn load_at(data_dir: &Path) -> Result<Registry, String> {
    let _guard = lock().await;
    load_or_migrate_at(data_dir).await
}

pub async fn update_at<T>(
    data_dir: &Path,
    change: impl FnOnce(&mut Registry) -> Result<T, String>,
) -> Result<T, String> {
    let _guard = lock().await;
    let mut registry = load_or_migrate_at(data_dir).await?;
    let result = change(&mut registry)?;
    save_at(data_dir, &registry).await?;
    Ok(result)
}

/// Saves the registry before deleting the keystore; on a later error the caller must reload.
pub async fn remove_seed_at(data_dir: &Path, seed_id: &str) -> Result<Registry, String> {
    let (registry, file) = update_at(data_dir, |r| {
        let seed = r.remove_seed(seed_id)?;
        Ok((r.clone(), keystore_path(data_dir, &seed)?))
    })
    .await?;
    match tokio::fs::remove_file(&file).await {
        Ok(()) => Ok(registry),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(registry),
        Err(e) => Err(format!(
            "Removed the seed, but failed to delete {}: {e}",
            file.display()
        )),
    }
}

/// Imported groups stay listed (locked if their password differs), not silently orphaned.
pub async fn reset_for_main_at(data_dir: &Path, keystore_file: String) -> Result<Registry, String> {
    keystore_file_name(&keystore_file)?;
    let _guard = lock().await;
    let registry = match read_at(data_dir).await {
        Ok(Some(mut registry)) => {
            registry.replace_main(keystore_file);
            registry
        }
        Ok(None) => Registry::for_main(keystore_file),
        Err(e) => {
            log::warn!("[Accounts] Replacing an unreadable registry: {e}");
            Registry::for_main(keystore_file)
        }
    };
    save_at(data_dir, &registry).await?;
    Ok(registry)
}

/// The words of a seed's keystore; changes no state.
pub async fn seed_phrase_at(
    data_dir: &Path,
    seed_id: &str,
    wallet_password: String,
) -> Result<Vec<String>, String> {
    let registry = load_at(data_dir).await?;
    let seed = registry
        .seeds
        .iter()
        .find(|s| s.id == seed_id)
        .ok_or_else(|| format!("Unknown seed {seed_id}"))?;
    let path = keystore_path(data_dir, seed)?;
    let content = tokio::fs::read_to_string(&path)
        .await
        .map_err(|e| format!("Failed to read {}: {e}", path.display()))?;
    let mnemonic = tokio::task::spawn_blocking(move || {
        keystore::mnemonic_from_keystore(&content, &wallet_password)
    })
    .await
    .map_err(|e| format!("Keystore task failed: {e}"))??;
    Ok(mnemonic.split_whitespace().map(String::from).collect())
}

/// Guards against a node that reports activity for every receiver.
const MAX_DISCOVERED: u64 = 1000;

/// Stops after two inactive indices in a row.
pub async fn active_indices<F, Fut>(start: u64, mut probe: F) -> Result<Vec<u64>, String>
where
    F: FnMut(u64) -> Fut,
    Fut: std::future::Future<Output = Result<bool, String>>,
{
    let mut found = Vec::new();
    let mut inactive = 0;
    let mut index = start;
    while inactive < 2 {
        if probe(index).await? {
            found.push(index);
            inactive = 0;
        } else {
            inactive += 1;
        }
        index += 1;
    }
    Ok(found)
}

/// Ids added meanwhile are skipped.
pub async fn discover_at(
    data_dir: &Path,
    client: &reqwest::Client,
    rpc_url: &str,
    seed_id: &str,
    wallet_password: String,
) -> Result<Vec<AccountEntry>, String> {
    use neptune_privacy::state::wallet::address::SpendingKey;
    use neptune_privacy::state::wallet::wallet_entropy::WalletEntropy;

    let words = seed_phrase_at(data_dir, seed_id, wallet_password).await?;
    let entropy =
        WalletEntropy::from_phrase(&words).map_err(|_| "Invalid seed phrase".to_string())?;
    let registry = load_at(data_dir).await?;
    let start = registry
        .accounts
        .iter()
        .filter(|a| a.seed_id == seed_id)
        .map(|a| a.index + 1)
        .max()
        .ok_or_else(|| format!("Seed {seed_id} has no account"))?;
    let tip = crate::rpc_client::get_chain_height(client, rpc_url).await?;

    let indices = active_indices(start, |index| {
        let entropy = entropy.clone();
        async move {
            if index >= start + MAX_DISCOVERED {
                return Err(format!("Discovery stopped after {MAX_DISCOVERED} accounts"));
            }
            let generation = crate::utxo_scanning::receiver_id_hash_hex(&SpendingKey::Generation(
                entropy.nth_generation_spending_key(index),
            ));
            if crate::utxo_scanning::has_activity(client, rpc_url, &generation, tip).await? {
                return Ok(true);
            }
            // dCTIDH derivation is slow on phones, so only when the Generation address is unused.
            let (dctidh, legacy) = tokio::task::spawn_blocking(move || {
                let (key, legacy) = entropy.nth_dctidh_spending_key_with_legacy(index);
                (
                    crate::utxo_scanning::receiver_id_hash_hex(&SpendingKey::dCTIDH(key)),
                    crate::utxo_scanning::receiver_id_hash_hex(&SpendingKey::dCTIDH(legacy)),
                )
            })
            .await
            .map_err(|e| format!("Key derivation task failed: {e}"))?;
            if crate::utxo_scanning::has_activity(client, rpc_url, &dctidh, tip).await? {
                return Ok(true);
            }
            crate::utxo_scanning::has_activity(client, rpc_url, &legacy, tip).await
        }
    })
    .await?;
    log::info!("[Accounts] Discovery for seed {seed_id} found indices {indices:?}");

    update_at(data_dir, |r| {
        let mut added = Vec::new();
        for &index in &indices {
            if !r
                .accounts
                .iter()
                .any(|a| a.id == account_id(seed_id, index))
            {
                added.push(r.add(seed_id, index)?);
            }
        }
        Ok(added)
    })
    .await
}

pub fn data_dir(app: &tauri::AppHandle) -> Result<PathBuf, String> {
    app.path()
        .app_data_dir()
        .map_err(|e| format!("Failed to get app data directory: {e}"))
}

#[cfg(test)]
pub(crate) mod test_util {
    use std::path::PathBuf;

    /// A data dir with `wallets/imported/`, removed on drop.
    pub(crate) struct TempDir(pub(crate) PathBuf);

    impl TempDir {
        pub(crate) fn new(name: &str) -> Self {
            static NEXT: std::sync::atomic::AtomicU32 = std::sync::atomic::AtomicU32::new(0);
            let n = NEXT.fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let dir =
                std::env::temp_dir().join(format!("accounts-{name}-{}-{n}", std::process::id()));
            let _ = std::fs::remove_dir_all(&dir);
            std::fs::create_dir_all(dir.join("wallets").join(super::IMPORTED_DIR)).unwrap();
            Self(dir)
        }
    }

    impl Drop for TempDir {
        fn drop(&mut self) {
            let _ = std::fs::remove_dir_all(&self.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::test_util::TempDir;
    use super::*;

    /// A keystore file dated `secs` after the epoch, so tests control which one is newest.
    fn keystore_at(dir: &TempDir, file: &str, secs: u64) {
        let f = std::fs::File::create(dir.0.join("wallets").join(file)).unwrap();
        f.set_modified(std::time::UNIX_EPOCH + std::time::Duration::from_secs(secs))
            .unwrap();
    }

    fn with_imported_seed() -> Registry {
        let mut registry = Registry::for_main("xnt-wallet-1.json".into());
        registry.seeds.push(SeedEntry {
            id: "imp-5".into(),
            keystore_file: "imp-5.json".into(),
            source: SeedSource::Phrase,
            backed_up: false,
        });
        registry.add("imp-5", 0).unwrap();
        registry
    }

    #[tokio::test]
    async fn migrates_single_keystore_to_account_1() {
        let dir = TempDir::new("migrate");
        std::fs::write(dir.0.join("wallets/xnt-wallet-1.json"), "{}").unwrap();
        let registry = load_or_migrate_at(&dir.0).await.unwrap();
        assert_eq!(registry, Registry::for_main("xnt-wallet-1.json".into()));
        assert_eq!(registry.active, "main:0");
        assert_eq!(registry.accounts[0].index, 0);
        assert!(registry.seeds[0].backed_up);
        assert_eq!(read_at(&dir.0).await.unwrap(), Some(registry));
    }

    #[tokio::test]
    async fn migration_picks_the_newest_keystore() {
        let dir = TempDir::new("newest");
        let old = std::fs::File::create(dir.0.join("wallets/xnt-wallet-2.json")).unwrap();
        old.set_modified(std::time::SystemTime::UNIX_EPOCH + std::time::Duration::from_secs(1_000))
            .unwrap();
        std::fs::write(dir.0.join("wallets/xnt-wallet-1.json"), "{}").unwrap();
        let registry = load_or_migrate_at(&dir.0).await.unwrap();
        assert_eq!(registry.seeds[0].keystore_file, "xnt-wallet-1.json");
    }

    #[tokio::test]
    async fn inconsistent_registry_is_refused() {
        let dir = TempDir::new("invalid");
        keystore_at(&dir, "xnt-wallet-1.json", 1_000);
        let write = |registry: &Registry| {
            let text = serde_json::to_string(registry).unwrap();
            std::fs::write(dir.0.join(REGISTRY_FILE), text).unwrap();
        };
        let mut registry = with_imported_seed();
        registry.active = "main:9".into();
        assert!(save_at(&dir.0, &registry).await.is_err());
        write(&registry);
        assert!(load_or_migrate_at(&dir.0).await.is_err());
        registry.active = "main:0".into();
        registry.seeds[1].keystore_file = "../xnt-wallet-1.json".into();
        write(&registry);
        assert!(load_or_migrate_at(&dir.0).await.is_err());
    }

    #[tokio::test]
    async fn reset_for_main_replaces_the_registry() {
        let dir = TempDir::new("reset");
        let mut old = with_imported_seed();
        old.add(MAIN_SEED, 1).unwrap();
        save_at(&dir.0, &old).await.unwrap();
        let registry = reset_for_main_at(&dir.0, "xnt-wallet-7.json".into())
            .await
            .unwrap();
        let ids: Vec<_> = registry.accounts.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["main:0", "imp-5:0"]);
        assert_eq!(registry.seeds[0].keystore_file, "xnt-wallet-7.json");
        assert_eq!(registry.seeds[1].id, "imp-5");
        assert_eq!(registry.active, "main:0");
        assert_eq!(read_at(&dir.0).await.unwrap(), Some(registry));
    }

    #[test]
    fn removing_the_active_seed_unhides_account_1() {
        let mut registry = with_imported_seed();
        registry.set_active("imp-5:0").unwrap();
        registry.set_hidden("main:0", true).unwrap();
        registry.remove_seed("imp-5").unwrap();
        assert_eq!(registry.active, "main:0");
        assert!(!registry.accounts[0].hidden);
        assert_eq!(registry.validate(), Ok(()));
    }

    #[tokio::test]
    async fn load_returns_saved_registry() {
        let dir = TempDir::new("load");
        keystore_at(&dir, "xnt-wallet-1.json", 1_000);
        keystore_at(&dir, "xnt-wallet-9.json", 2_000);
        let saved = with_imported_seed();
        save_at(&dir.0, &saved).await.unwrap();
        assert_eq!(load_or_migrate_at(&dir.0).await.unwrap(), saved);
    }

    #[tokio::test]
    async fn keystore_newer_than_the_registry_becomes_main() {
        let dir = TempDir::new("heal");
        keystore_at(&dir, "xnt-wallet-1.json", 1_000);
        let mut saved = with_imported_seed();
        saved.add(MAIN_SEED, 1).unwrap();
        save_at(&dir.0, &saved).await.unwrap();
        let later = std::time::SystemTime::now() + std::time::Duration::from_secs(60);
        let f = std::fs::File::create(dir.0.join("wallets/xnt-wallet-2.json")).unwrap();
        f.set_modified(later).unwrap();

        let registry = load_or_migrate_at(&dir.0).await.unwrap();
        assert_eq!(registry.seeds[0].keystore_file, "xnt-wallet-2.json");
        let ids: Vec<_> = registry.accounts.iter().map(|a| a.id.as_str()).collect();
        assert_eq!(ids, ["main:0", "imp-5:0"]);
        assert_eq!(read_at(&dir.0).await.unwrap(), Some(registry));
    }

    #[tokio::test]
    async fn missing_main_keystore_falls_back_to_the_newest() {
        let dir = TempDir::new("gone");
        keystore_at(&dir, "xnt-wallet-3.json", 1_000);
        save_at(&dir.0, &with_imported_seed()).await.unwrap();
        let registry = load_or_migrate_at(&dir.0).await.unwrap();
        assert_eq!(registry.seeds[0].keystore_file, "xnt-wallet-3.json");
    }

    #[tokio::test]
    async fn missing_keystore_is_an_error() {
        let dir = TempDir::new("missing");
        assert!(load_or_migrate_at(&dir.0).await.is_err());
        assert_eq!(read_at(&dir.0).await.unwrap(), None);
    }

    #[tokio::test]
    async fn seed_phrase_reads_the_registered_keystore() {
        let dir = TempDir::new("phrase");
        let words = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon agent";
        keystore::write_keystore_file(&dir.0.join("wallets/xnt-wallet-1.json"), words, "pw")
            .await
            .unwrap();
        let phrase = seed_phrase_at(&dir.0, MAIN_SEED, "pw".into())
            .await
            .unwrap();
        assert_eq!(phrase.join(" "), words);
        assert_eq!(
            seed_phrase_at(&dir.0, MAIN_SEED, "nope".into()).await,
            Err("Invalid password".into())
        );
        assert!(seed_phrase_at(&dir.0, "imp-1", "pw".into()).await.is_err());
    }

    #[tokio::test]
    async fn discovery_stops_after_two_inactive() {
        let active = [1, 2, 5];
        let probed = std::cell::RefCell::new(Vec::new());
        let found = active_indices(1, |i| {
            probed.borrow_mut().push(i);
            std::future::ready(Ok(active.contains(&i)))
        })
        .await
        .unwrap();
        assert_eq!(found, vec![1, 2]);
        assert_eq!(*probed.borrow(), vec![1, 2, 3, 4]);
    }

    #[tokio::test]
    async fn discovery_probe_error_is_an_error() {
        let result = active_indices(1, |i| {
            std::future::ready(if i == 2 {
                Err("node down".to_string())
            } else {
                Ok(true)
            })
        })
        .await;
        assert_eq!(result, Err("node down".into()));
    }

    #[test]
    fn add_rejects_existing_index() {
        let mut registry = Registry::for_main("k.json".into());
        assert_eq!(registry.add("main", 1).unwrap().id, "main:1");
        assert!(registry.add("main", 1).is_err());
        assert!(registry.add("main", 0).is_err());
        assert!(registry.add("imp-1", 0).is_err());
    }

    #[test]
    fn rename_none_restores_default_name() {
        let mut registry = Registry::for_main("k.json".into());
        registry
            .rename("main:0", Some("  Savings ".into()))
            .unwrap();
        assert_eq!(registry.accounts[0].name.as_deref(), Some("Savings"));
        assert!(registry.rename("main:0", Some(" ".into())).is_err());
        registry.rename("main:0", None).unwrap();
        assert_eq!(registry.accounts[0].name, None);
    }

    #[test]
    fn set_active_rejects_unknown_id() {
        let mut registry = Registry::for_main("k.json".into());
        assert_eq!(
            registry.set_active("main:7"),
            Err("Unknown account main:7".into())
        );
        assert_eq!(registry.active, "main:0");
    }

    #[test]
    fn active_account_cannot_be_hidden() {
        let mut registry = Registry::for_main("k.json".into());
        registry.add("main", 1).unwrap();
        assert!(registry.set_hidden("main:0", true).is_err());
        registry.set_hidden("main:1", true).unwrap();
        registry.set_active("main:1").unwrap();
        assert!(!registry.accounts[1].hidden);
    }

    #[test]
    fn remove_seed_refuses_main() {
        let mut registry = with_imported_seed();
        assert!(registry.remove_seed("main").is_err());
        assert_eq!(registry, with_imported_seed());
    }

    #[tokio::test]
    async fn remove_seed_deletes_keystore_and_accounts() {
        let dir = TempDir::new("remove");
        keystore_at(&dir, "xnt-wallet-1.json", 1_000);
        let mut saved = with_imported_seed();
        saved.set_active("imp-5:0").unwrap();
        save_at(&dir.0, &saved).await.unwrap();
        let keystore = imported_dir(&dir.0).join("imp-5.json");
        std::fs::write(&keystore, "{}").unwrap();

        let registry = remove_seed_at(&dir.0, "imp-5").await.unwrap();
        assert_eq!(registry.seeds.len(), 1);
        assert!(registry.accounts.iter().all(|a| a.seed_id == "main"));
        assert_eq!(registry.active, "main:0");
        assert!(!keystore.exists());
        assert_eq!(read_at(&dir.0).await.unwrap(), Some(registry));
    }
}
