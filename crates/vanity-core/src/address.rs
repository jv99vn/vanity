//! Ethereum address utilities for vanity address generation.

use sha3::{Digest, Keccak256};

/// Represents an Ethereum address (20 bytes)
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Address([u8; 20]);

impl Address {
    /// Create a new address from 20 bytes
    pub fn new(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Create an address from a slice (must be exactly 20 bytes)
    pub fn from_slice(slice: &[u8]) -> Option<Self> {
        if slice.len() == 20 {
            let mut arr = [0u8; 20];
            arr.copy_from_slice(slice);
            Some(Self(arr))
        } else {
            None
        }
    }

    /// Parse an address from hex string (with or without 0x prefix)
    pub fn from_hex(hex: &str) -> Option<Self> {
        let hex = hex.strip_prefix("0x").unwrap_or(hex);
        if hex.len() != 40 {
            return None;
        }
        let bytes = hex::decode(hex).ok()?;
        Self::from_slice(&bytes)
    }

    /// Get the address bytes
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Convert to hex string (without 0x prefix)
    pub fn to_hex(&self) -> String {
        hex::encode(self.0)
    }

    /// Convert to hex string with 0x prefix
    pub fn to_hex_prefixed(&self) -> String {
        format!("0x{}", self.to_hex())
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_hex())
    }
}

/// Derive Ethereum address from uncompressed public key
///
/// The address is the last 20 bytes of the Keccak-256 hash
/// of the 64-byte uncompressed public key (without the 0x04 prefix)
pub fn public_key_to_address(public_key: &[u8]) -> Address {
    // For uncompressed public key (65 bytes), skip the 0x04 prefix
    // For compressed (33 bytes), need to decompress first
    let key_bytes = if public_key.len() == 65 && public_key[0] == 0x04 {
        &public_key[1..] // Skip 0x04 prefix
    } else if public_key.len() == 64 {
        public_key // Already without prefix
    } else {
        // Fallback: hash whatever we have
        public_key
    };

    let mut hasher = Keccak256::new();
    hasher.update(key_bytes);
    let hash = hasher.finalize();

    // Take last 20 bytes
    let mut address = [0u8; 20];
    address.copy_from_slice(&hash[12..32]);

    Address::new(address)
}

/// CPU-based address generator for testing and fallback
pub struct AddressGenerator {
    rng: rand::rngs::ThreadRng,
}

impl AddressGenerator {
    /// Create a new address generator
    pub fn new() -> Self {
        Self {
            rng: rand::thread_rng(),
        }
    }
}

impl Default for AddressGenerator {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_from_hex() {
        let hex = "deadbeef12345678901234567890123456789012";
        let addr = Address::from_hex(hex).unwrap();
        assert_eq!(addr.to_hex(), hex);

        let hex_prefixed = "0xdeadbeef12345678901234567890123456789012";
        let addr2 = Address::from_hex(hex_prefixed).unwrap();
        assert_eq!(addr, addr2);
    }

    #[test]
    fn test_address_invalid_hex() {
        assert!(Address::from_hex("invalid").is_none());
        assert!(Address::from_hex("dead").is_none()); // Too short
    }

    #[test]
    fn test_public_key_to_address() {
        // Known test vector
        // Private key: 0x0000000000000000000000000000000000000000000000000000000000000001
        // Expected address: 0x7E5F4552091A69125d5DfCb7b8C2659029395Bdf

        let public_key = hex::decode("0479BE667EF9DCBBAC55A06295CE870B07029BFCDB2DCE28D959F2815B16F81798483ADA7726A3C4655DA4FBFC0E1108A8FD17B448A68554199C47D08FFB10D4B8").unwrap();

        let address = public_key_to_address(&public_key);
        let expected = "7e5f4552091a69125d5dfcb7b8c2659029395bdf";

        assert_eq!(address.to_hex(), expected);
    }
}
