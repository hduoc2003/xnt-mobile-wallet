use serde::{Deserialize, Serialize};
use tasm_lib::prelude::Digest;
use tasm_lib::triton_vm::prelude::BFieldElement;
use anyhow::{anyhow, Result};
use tasm_lib::twenty_first::math::lattice;
use crate::api::export::Utxo;
use super::ReceivingAddress;
use super::SpendingKey;
use super::generation_address::GenerationSpendingKey;
use crate::state::wallet::incoming_utxo::IncomingUtxo;

/// A view-only key that can decrypt UTXO announcements but cannot spend UTXOs.
///
/// ViewKey stores only the decryption components from a Generation SpendingKey:
/// - `receiver_identifier`: Used to filter announcements
/// - `decryption_key`: Lattice KEM private key for decrypting announcements
/// - `receiver_preimage`: Needed for membership proof restoration
///
/// Critically, ViewKey does NOT contain:
/// - `unlock_key_preimage`: Required for spending (lock script witness)
/// - `seed`: The root secret from which everything is derived
///
/// This allows a user to safely share their ViewKey with a server for UTXO scanning
/// without risking their funds. The server can decr`ypt announcements and identify UTXOs,
/// but cannot create valid lock script witnesses to spend them.
///
/// **Security Note:** Only Generation keys are supported. Symmetric keys cannot be
/// made view-only because the seed IS the unlock key.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ViewKey {
    /// Identifies which announcements belong to this key
    pub receiver_identifier: BFieldElement,

    /// Lattice KEM private key for decrypting UTXO announcements
    /// This is the decryption_key from GenerationSpendingKey
    pub decryption_key: Vec<u8>, // Serialized lattice::kem::SecretKey

    /// Receiver preimage (privacy preimage) needed for membership proof restoration
    pub receiver_preimage: Digest,
}

impl ViewKey {
    /// Create a ViewKey from a SpendingKey
    ///
    /// **Security:** Only Generation keys are supported. Symmetric keys cannot
    /// be made view-only because their seed is also their unlock key.
    ///
    /// # Errors
    /// Returns an error if the spending key is not a Generation key.
    pub fn from_spending_key(spending_key: &SpendingKey) -> Result<Self> {
        match spending_key {
            SpendingKey::Generation(gen_key) => {
                // Extract only the view-only components
                let receiver_identifier = gen_key.receiver_identifier();
                let receiver_preimage = gen_key.receiver_preimage();

                // Get the decryption key and serialize it
                let decryption_key_value = gen_key.decryption_key();
                let decryption_key_bytes = bincode::serialize(&decryption_key_value)
                    .map_err(|e| anyhow!("Failed to serialize decryption key: {}", e))?;

                Ok(Self {
                    receiver_identifier,
                    decryption_key: decryption_key_bytes,
                    receiver_preimage,
                })
            }
            SpendingKey::Symmetric(_) => {
                Err(anyhow!("Symmetric keys cannot be made view-only. The seed is also the unlock key, so sharing decryption capability would also share spending capability."))
            }
        }
    }

    /// Get the receiver identifier
    pub fn receiver_identifier(&self) -> BFieldElement {
        self.receiver_identifier
    }

    /// Get the receiver preimage (needed for membership proof restoration)
    pub fn receiver_preimage(&self) -> Digest {
        self.receiver_preimage
    }

    /// Decrypt a ciphertext into a UTXO and sender randomness
    ///
    /// This method provides decryption capability without exposing spending capability.
    /// It uses the stored decryption key to decrypt announcements.
    pub fn decrypt(&self, ciphertext: &[BFieldElement]) -> Result<(Utxo, Digest)> {
        // Deserialize the decryption key
        let decryption_key: lattice::kem::SecretKey = bincode::deserialize(&self.decryption_key)
            .map_err(|e| anyhow!("Failed to deserialize decryption key: {}", e))?;

        // Use the static decrypt_with_key method
        GenerationSpendingKey::decrypt_with_key(decryption_key, ciphertext)
    }

    /// Attempt to convert back to a SpendingKey for scanning
    ///
    /// **Warning:** This method cannot fully reconstruct a SpendingKey because
    /// the unlock_key_preimage and seed are not stored. This will fail or produce
    /// an incomplete key that cannot spend funds.
    ///
    /// This method exists only for backwards compatibility during migration.
    /// New code should use `decrypt()` directly or work with ViewKey-specific methods.
    #[deprecated(note = "ViewKey should not be converted back to SpendingKey. Use decrypt() or scan_for_announced_utxos() instead.")]
    pub fn to_spending_key(&self) -> Result<SpendingKey> {
        Err(anyhow!("ViewKey cannot be converted back to SpendingKey. The unlock_key_preimage and seed are not stored for security reasons. Use decrypt() or other ViewKey-specific methods instead."))
    }

    /// Get the receiving address
    ///
    /// This requires reconstructing the full address, which needs the encryption
    /// public key. We can derive this from the decryption key if needed.
    pub fn to_address(&self) -> Result<ReceivingAddress> {
        // To get the address, we need the encryption public key
        // This can be derived from the decryption key, but GenerationSpendingKey
        // doesn't expose this. We need to either:
        // 1. Store the encryption public key in ViewKey
        // 2. Add a method to derive it from decryption key
        // 3. Store the full address in ViewKey

        // For now, return an error - address derivation needs the full key
        Err(anyhow!("Cannot derive receiving address from ViewKey alone. Address generation requires the seed or encryption public key, which are not stored in ViewKey for security reasons."))
    }

    /// Scan announcements in a transaction kernel for UTXOs
    ///
    /// This method provides the same functionality as SpendingKey::scan_for_announced_utxos
    /// but works with a ViewKey, ensuring the spending key never needs to be reconstructed.
    pub fn scan_for_announced_utxos(
        &self,
        tx_kernel: &crate::protocol::consensus::transaction::transaction_kernel::TransactionKernel,
    ) -> Result<Vec<IncomingUtxo>> {

        use super::common;

        // Deserialize the decryption key once before the loop
        let decryption_key: lattice::kem::SecretKey = bincode::deserialize(&self.decryption_key)
            .map_err(|e| anyhow!("Failed to deserialize decryption key: {}", e))?;

        let mut incoming_utxos = Vec::new();

        // Scan all announcements in the transaction kernel
        for announcement in &tx_kernel.announcements {
            // Check if this announcement matches our key type (Generation)
            let key_type = match common::key_type_from_announcement(announcement) {
                Ok(kt) => kt,
                Err(_) => continue,
            };

            // Only process Generation key announcements
            if key_type != super::generation_address::GENERATION_FLAG {
                continue;
            }

            // Check if receiver_identifier matches
            let announcement_receiver_id = match common::receiver_identifier_from_announcement(announcement) {
                Ok(id) => id,
                Err(_) => continue,
            };

            if announcement_receiver_id != self.receiver_identifier {
                continue;
            }

            // Extract ciphertext from announcement
            let ciphertext = match common::ciphertext_from_announcement(announcement) {
                Ok(ct) => ct,
                Err(_) => continue,
            };

            // Decrypt the ciphertext (decryption_key is Copy, so we can use it multiple times)
            let (utxo, sender_randomness) = match GenerationSpendingKey::decrypt_with_key(decryption_key, &ciphertext) {
                Ok(result) => result,
                Err(_) => continue, // Decryption failed - not for this key or corrupted
            };

            // Construct IncomingUtxo
            incoming_utxos.push(IncomingUtxo {
                utxo,
                sender_randomness,
                receiver_preimage: self.receiver_preimage,
                is_guesser_fee: false,
            });
        }

        Ok(incoming_utxos)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::state::wallet::address::generation_address::GenerationSpendingKey;

    #[test]
    fn test_view_key_from_spending_key() {
        let seed = Digest::default();
        let spending_key = GenerationSpendingKey::derive_from_seed(seed);
        let spending_key_enum = SpendingKey::Generation(spending_key);

        let view_key = ViewKey::from_spending_key(&spending_key_enum).unwrap();

        assert_eq!(view_key.receiver_identifier(), spending_key.receiver_identifier());
        assert_eq!(view_key.receiver_preimage(), spending_key.receiver_preimage());

        // Security: ViewKey should NOT be convertible back to SpendingKey
        assert!(view_key.to_spending_key().is_err());

        // But it should be able to decrypt announcements
        // (We can't easily test this without a full transaction, so we'll skip for now)
    }
}
