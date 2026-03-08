//! Cryptographic operations for key generation.

use rand::RngCore;
use secp256k1::{PublicKey, SecretKey, Secp256k1};
use sha3::{Digest, Keccak256};

use crate::address::Address;

/// Result of keypair generation
#[derive(Clone, Debug)]
pub struct KeyPair {
    /// 32-byte private key
    pub private_key: [u8; 32],
    /// Ethereum address derived from public key
    pub address: Address,
}

impl KeyPair {
    /// Get private key as hex string
    pub fn private_key_hex(&self) -> String {
        hex::encode(self.private_key)
    }

    /// Get address as hex string with 0x prefix
    pub fn address_hex(&self) -> String {
        self.address.to_hex_prefixed()
    }
}

/// Generate a random keypair
pub fn generate_keypair() -> KeyPair {
    let secp = Secp256k1::new();

    // Generate random private key
    let mut private_key_bytes = [0u8; 32];
    rand::thread_rng().fill_bytes(&mut private_key_bytes);

    let secret_key = SecretKey::from_slice(&private_key_bytes).expect("Invalid private key");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);

    // Get uncompressed public key (65 bytes: 0x04 + x + y)
    let public_key_bytes = public_key.serialize_uncompressed();

    // Derive Ethereum address
    let address = public_key_to_address(&public_key_bytes);

    KeyPair {
        private_key: private_key_bytes,
        address,
    }
}

/// Generate keypair from a specific private key
pub fn keypair_from_private_key(private_key: &[u8; 32]) -> KeyPair {
    let secp = Secp256k1::new();
    let secret_key = SecretKey::from_slice(private_key).expect("Invalid private key");
    let public_key = PublicKey::from_secret_key(&secp, &secret_key);
    let public_key_bytes = public_key.serialize_uncompressed();
    let address = public_key_to_address(&public_key_bytes);

    KeyPair {
        private_key: *private_key,
        address,
    }
}

/// Derive Ethereum address from uncompressed public key bytes
pub fn public_key_to_address(public_key: &[u8]) -> Address {
    // For uncompressed public key (65 bytes), skip the 0x04 prefix
    let key_bytes = if public_key.len() == 65 && public_key[0] == 0x04 {
        &public_key[1..]
    } else if public_key.len() == 64 {
        public_key
    } else {
        public_key
    };

    let mut hasher = Keccak256::new();
    hasher.update(key_bytes);
    let hash = hasher.finalize();

    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);

    Address::new(address)
}

/// Increment a 256-bit big-endian integer by 1
/// Returns true if successful, false on overflow
pub fn increment_u256_be(bytes: &mut [u8; 32]) -> bool {
    for i in (0..32).rev() {
        if bytes[i] == 0xFF {
            bytes[i] = 0;
        } else {
            bytes[i] += 1;
            return true;
        }
    }
    false // Overflow
}

/// CPU-based vanity address searcher
pub struct CpuSearcher {
    secp: Secp256k1<secp256k1::All>,
}

impl CpuSearcher {
    pub fn new() -> Self {
        Self {
            secp: Secp256k1::new(),
        }
    }

    /// Search for a vanity address matching the pattern
    /// Returns found keypair and number of iterations
    pub fn search(
        &self,
        prefix: Option<&str>,
        suffix: Option<&str>,
        max_iterations: Option<u64>,
    ) -> Option<(KeyPair, u64)> {
        let pattern = crate::pattern::Pattern::new(prefix, suffix).ok()?;
        let max = max_iterations.unwrap_or(u64::MAX);

        for i in 0..max {
            let keypair = generate_keypair();

            if pattern.matches(&keypair.address.to_hex()) {
                return Some((keypair, i + 1));
            }

            // Progress is handled by CLI layer
        }

        None
    }

    /// Generate public key from private key bytes
    pub fn public_key_from_private(&self, private_key: &[u8; 32]) -> [u8; 65] {
        let secret_key = SecretKey::from_slice(private_key).expect("Invalid private key");
        let public_key = PublicKey::from_secret_key(&self.secp, &secret_key);
        public_key.serialize_uncompressed()
    }
}

impl Default for CpuSearcher {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_keypair() {
        let keypair = generate_keypair();
        assert_eq!(keypair.private_key.len(), 32);

        // Verify address is valid hex
        let addr = keypair.address.to_hex();
        assert_eq!(addr.len(), 40);
        assert!(addr.chars().all(|c| c.is_ascii_hexdigit()));
    }

    #[test]
    fn test_keypair_from_private_key() {
        // Known test vector
        let private_key = [1u8; 32]; // All ones for simplicity
        let keypair = keypair_from_private_key(&private_key);

        // Verify deterministic address generation
        let keypair2 = keypair_from_private_key(&private_key);
        assert_eq!(keypair.address, keypair2.address);
    }

    #[test]
    fn test_known_address() {
        // Private key: 0x0000000000000000000000000000000000000000000000000000000000000001
        let mut private_key = [0u8; 32];
        private_key[31] = 1;

        let keypair = keypair_from_private_key(&private_key);

        // Expected address: 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf
        let expected = "7e5f4552091a69125d5dfcb7b8c2659029395bdf";
        assert_eq!(keypair.address.to_hex(), expected);
    }

    #[test]
    fn test_increment_u256_be() {
        let mut bytes = [0u8; 32];
        bytes[31] = 0xFE;

        assert!(increment_u256_be(&mut bytes));
        assert_eq!(bytes[31], 0xFF);

        assert!(increment_u256_be(&mut bytes));
        assert_eq!(bytes[31], 0x00);
        assert_eq!(bytes[30], 0x01);
    }

    #[test]
    fn test_cpu_searcher_short_prefix() {
        let searcher = CpuSearcher::new();

        // Search for very short prefix (should find quickly)
        let result = searcher.search(Some("0"), None, Some(100_000));
        assert!(result.is_some());

        let (keypair, iterations) = result.unwrap();
        assert!(keypair.address.to_hex().starts_with('0'));
        assert!(iterations <= 100_000);
    }
}
