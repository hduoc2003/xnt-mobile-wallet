use neptune_privacy::application::config::network::Network;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::secret_key_material::SecretKeyMaterial;
use neptune_privacy::state::wallet::wallet_entropy::WalletEntropy;
use rand::RngExt;
use serde::Serialize;
use std::str::FromStr;

/// Generates a fresh 24-word BIP39 seed phrase and returns it alongside the
/// derived generation receiver identifier.
pub fn generate_seed() -> Result<String, String> {
    let mut rng = rand::rng();
    let entropy: [u8; 24] = rng.random();

    let mnemonic = bip39::Mnemonic::from_entropy(&entropy, bip39::Language::English)
        .map_err(|e| format!("Failed to generate mnemonic: {}", e))?;

    let seed_phrase: Vec<String> = mnemonic.phrase().split(' ').map(|s| s.to_string()).collect();

    let secret_key_material = SecretKeyMaterial::from_phrase(&seed_phrase)
        .map_err(|e| format!("Failed to create secret key from phrase: {}", e))?;
    let wallet_entropy = WalletEntropy::from(secret_key_material);

    let spending_key = wallet_entropy.nth_generation_spending_key(0);
    let receiver_id = spending_key.receiver_identifier();

    Ok(serde_json::json!({
        "success": true,
        "seed_phrase": seed_phrase,
        "receiver_identifier": format!("0x{:016x}", receiver_id.value()),
    })
    .to_string())
}

/// Derives the spending key and address for a seed phrase. `key_type` selects
/// the address family: "generation", or "dctidh" (the default for any other
/// value).
pub fn generate_keys_from_seed(
    seed_phrase: Vec<String>,
    network: Option<String>,
    key_type: Option<String>,
) -> Result<String, String> {
    let wallet_entropy = WalletEntropy::from_phrase(&seed_phrase)
        .map_err(|e| format!("Invalid seed phrase: {}", e))?;

    let network_str = network.as_deref().unwrap_or("main");
    let network_enum = parse_network(network_str)?;

    let key_type_str = key_type.as_deref().unwrap_or("generation");
    log::info!(
        "[Keys] Deriving spending key | key_type={} | network={}",
        key_type_str,
        network_enum
    );

    let (spending_key, key_type_label) = match key_type_str {
        "generation" => (
            SpendingKey::Generation(wallet_entropy.nth_generation_spending_key(0)),
            "generation",
        ),
        _ => (
            SpendingKey::dCTIDH(wallet_entropy.nth_dctidh_spending_key(0)),
            "dctidh",
        ),
    };
    let receiver_id = spending_key.receiver_identifier();
    let (spending_key_hex, address) = key_hex_and_address(spending_key, network_enum)?;
    Ok(serde_json::json!({
        "success": true,
        "spending_key_hex": spending_key_hex,
        "receiver_identifier": format!("0x{:016x}", receiver_id.value()),
        "address": address,
        "key_type": key_type_label,
    })
    .to_string())
}

/// Accepts the frontend's "mainnet" alias for xnt-core's "main".
pub fn parse_network(network: &str) -> Result<Network, String> {
    let normalized = if network == "mainnet" {
        "main"
    } else {
        network
    };
    Network::from_str(normalized).map_err(|e| format!("Invalid network '{}': {}", network, e))
}

fn key_hex_and_address(key: SpendingKey, network: Network) -> Result<(String, String), String> {
    let bytes =
        bincode::serialize(&key).map_err(|e| format!("Failed to serialize spending key: {}", e))?;
    let address = key
        .to_address()
        .to_display_bech32m(network)
        .map_err(|e| format!("Failed to encode address: {}", e))?;
    Ok((format!("0x{}", hex::encode(bytes)), address))
}

#[derive(Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AccountKeys {
    pub index: u64,
    pub generation_key_hex: String,
    pub dctidh_key_hex: String,
    pub generation_address: String,
    pub dctidh_address: String,
}

/// Leaves the key hex out, so a logged `AccountKeys` never leaks keys.
impl std::fmt::Debug for AccountKeys {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AccountKeys")
            .field("index", &self.index)
            .field("generation_address", &self.generation_address)
            .field("dctidh_address", &self.dctidh_address)
            .finish_non_exhaustive()
    }
}

/// Account N of a seed is key index N of both address families.
pub fn derive_account_keys(
    seed_phrase: &[String],
    index: u64,
    network: Network,
) -> Result<AccountKeys, String> {
    let entropy = WalletEntropy::from_phrase(seed_phrase)
        .map_err(|e| format!("Invalid seed phrase: {}", e))?;
    let (generation_key_hex, generation_address) = key_hex_and_address(
        SpendingKey::Generation(entropy.nth_generation_spending_key(index)),
        network,
    )?;
    let (dctidh_key_hex, dctidh_address) = key_hex_and_address(
        SpendingKey::dCTIDH(entropy.nth_dctidh_spending_key(index)),
        network,
    )?;
    Ok(AccountKeys {
        index,
        generation_key_hex,
        dctidh_key_hex,
        generation_address,
        dctidh_address,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// BIP39 phrase of 24 zero bytes.
    fn phrase() -> Vec<String> {
        let mut words = vec!["abandon".to_string(); 17];
        words.push("agent".to_string());
        words
    }

    fn legacy(key_type: &str) -> serde_json::Value {
        let json = generate_keys_from_seed(phrase(), Some("testnet".into()), Some(key_type.into()))
            .unwrap();
        serde_json::from_str(&json).unwrap()
    }

    /// Index 0 as derived before accounts existed, for `phrase()` on testnet (dCTIDH: v0.3 derivation).
    #[test]
    fn index_0_keeps_pre_account_keys() {
        use sha2::{Digest, Sha256};
        let generation = legacy("generation");
        assert_eq!(generation["spending_key_hex"], "0x000000000b0bf32f681fdd5187bb508797c23eba6d3a0b440a40d2fa504905bc334a8e12e01d661bba184326");
        assert_eq!(generation["receiver_identifier"], "0x460394d9564fcf16");
        assert_eq!(generation["key_type"], "generation");
        let address_hash = Sha256::digest(generation["address"].as_str().unwrap().as_bytes());
        assert_eq!(
            hex::encode(address_hash),
            "4f4a7fcc2b26a72ee061a7b84bca21ea2c653e128e836457c6944747397a8d70"
        );
        let dctidh = legacy("dctidh");
        assert_eq!(dctidh["spending_key_hex"], "0x02000000c3fcd70be232631c2c08bed9b5e4cf59e5f617d9d88a2710a53baf1c5fa55affc5df9075f4065d87");
        assert_eq!(dctidh["receiver_identifier"], "0x07a0c5d2798b1b53");
        let address_hash = Sha256::digest(dctidh["address"].as_str().unwrap().as_bytes());
        assert_eq!(
            hex::encode(address_hash),
            "5137e96040cf430cba5a840cc0cf0d31248cc860cca6dbbc9efebde4a4c4ec75"
        );
        assert_eq!(legacy("other"), dctidh);
    }

    /// Pre-v0.3 releases gave `phrase()`'s dCTIDH key receiver identifier 0xfdcc042ae89b7999.
    #[test]
    fn legacy_dctidh_key_keeps_the_pre_v0_3_receiver_identifier() {
        let keys = derive_account_keys(&phrase(), 0, parse_network("testnet").unwrap()).unwrap();
        let bytes = hex::decode(keys.dctidh_key_hex.trim_start_matches("0x")).unwrap();
        let key: SpendingKey = bincode::deserialize(&bytes).unwrap();
        let legacy = crate::legacy_dctidh::legacy_key(&key).unwrap();
        assert_eq!(
            format!("0x{:016x}", legacy.receiver_identifier().value()),
            "0xfdcc042ae89b7999"
        );
    }

    #[test]
    fn index_0_matches_existing_derivation() {
        let keys = derive_account_keys(&phrase(), 0, parse_network("testnet").unwrap()).unwrap();
        let generation = legacy("generation");
        let dctidh = legacy("dctidh");
        assert_eq!(keys.generation_key_hex, generation["spending_key_hex"]);
        assert_eq!(keys.generation_address, generation["address"]);
        assert_eq!(keys.dctidh_key_hex, dctidh["spending_key_hex"]);
        assert_eq!(keys.dctidh_address, dctidh["address"]);
    }

    #[test]
    fn different_indices_give_different_addresses() {
        let network = parse_network("testnet").unwrap();
        let first = derive_account_keys(&phrase(), 0, network).unwrap();
        let second = derive_account_keys(&phrase(), 1, network).unwrap();
        assert_eq!(second.index, 1);
        assert_ne!(first.generation_address, second.generation_address);
        assert_ne!(first.dctidh_address, second.dctidh_address);
    }

    #[test]
    fn derivation_is_deterministic() {
        let network = parse_network("testnet").unwrap();
        assert_eq!(
            derive_account_keys(&phrase(), 3, network).unwrap(),
            derive_account_keys(&phrase(), 3, network).unwrap()
        );
    }

    #[test]
    fn mainnet_alias_parses() {
        assert_eq!(parse_network("mainnet").unwrap(), Network::Main);
    }
}
