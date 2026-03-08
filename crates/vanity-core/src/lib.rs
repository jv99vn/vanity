//! Vanity address generator core library
//!
//! Provides pattern parsing, difficulty calculation, and address generation
//! utilities for Ethereum vanity address generation.

pub mod address;
pub mod crypto;
pub mod pattern;

pub use address::{Address, AddressGenerator};
pub use crypto::generate_keypair;
pub use pattern::{Pattern, PatternError};
