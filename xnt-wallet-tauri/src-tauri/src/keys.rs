use neptune_privacy::application::config::network::Network;
use neptune_privacy::state::wallet::address::SpendingKey;
use neptune_privacy::state::wallet::secret_key_material::SecretKeyMaterial;
use neptune_privacy::state::wallet::wallet_entropy::WalletEntropy;
use rand::Rng;
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
    let network_str_normalized = match network_str {
        "mainnet" => "main",
        "testnet" => "testnet",
        other => other,
    };
    let network_enum = Network::from_str(network_str_normalized)
        .map_err(|e| format!("Invalid network '{}': {}", network_str, e))?;

    let key_type_str = key_type.as_deref().unwrap_or("generation");
    log::info!(
        "[Keys] Deriving spending key | key_type={} | network={}",
        key_type_str,
        network_str_normalized
    );

    match key_type_str {
        "generation" => {
            let spending_key = wallet_entropy.nth_generation_spending_key(0);
            let spending_key_enum = SpendingKey::Generation(spending_key.clone());
            let receiver_id = spending_key.receiver_identifier();
            let spending_key_bytes = bincode::serialize(&spending_key_enum)
                .map_err(|e| format!("Failed to serialize spending key: {}", e))?;
            let address_string = spending_key_enum
                .to_address()
                .to_display_bech32m(network_enum)
                .map_err(|e| format!("Failed to encode address: {}", e))?;
            Ok(serde_json::json!({
                "success": true,
                "spending_key_hex": format!("0x{}", hex::encode(&spending_key_bytes)),
                "receiver_identifier": format!("0x{:016x}", receiver_id.value()),
                "address": address_string,
                "key_type": "generation",
            })
            .to_string())
        }
        _ => {
            // Default: dctidh
            let dctidh_key = wallet_entropy.nth_dctidh_spending_key(0);
            let spending_key_enum = SpendingKey::dCTIDH(dctidh_key.clone());
            let receiver_id = dctidh_key.receiver_identifier();
            let spending_key_bytes = bincode::serialize(&spending_key_enum)
                .map_err(|e| format!("Failed to serialize spending key: {}", e))?;
            let address_string = spending_key_enum
                .to_address()
                .to_display_bech32m(network_enum)
                .map_err(|e| format!("Failed to encode address: {}", e))?;
            Ok(serde_json::json!({
                "success": true,
                "spending_key_hex": format!("0x{}", hex::encode(&spending_key_bytes)),
                "receiver_identifier": format!("0x{:016x}", receiver_id.value()),
                "address": address_string,
                "key_type": "dctidh",
            })
            .to_string())
        }
    }
}
