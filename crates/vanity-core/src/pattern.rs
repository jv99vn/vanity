//! Pattern parsing and difficulty calculation for vanity addresses.

use std::fmt;
use thiserror::Error;

/// Errors that can occur during pattern parsing
#[derive(Error, Debug)]
pub enum PatternError {
    #[error("Invalid hex character: '{0}' at position {1}")]
    InvalidHexChar(char, usize),

    #[error("Pattern too long: {0} characters (max 19 for prefix + suffix combined)")]
    PatternTooLong(usize),

    #[error("Empty pattern: at least one of prefix or suffix must be specified")]
    EmptyPattern,
}

/// Represents a vanity address pattern with optional prefix and suffix
#[derive(Clone, Debug, PartialEq)]
pub struct Pattern {
    /// Hex characters that the address must start with (lowercase)
    prefix: Vec<u8>,
    /// Hex characters that the address must end with (lowercase)
    suffix: Vec<u8>,
    /// Pre-computed prefix mask for GPU matching
    prefix_mask: Vec<u8>,
    /// Pre-computed suffix mask for GPU matching
    suffix_mask: Vec<u8>,
}

impl Pattern {
    /// Create a new pattern with optional prefix and suffix
    pub fn new(prefix: Option<&str>, suffix: Option<&str>) -> Result<Self, PatternError> {
        let prefix = prefix.map(|s| s.trim()).filter(|s| !s.is_empty());
        let suffix = suffix.map(|s| s.trim()).filter(|s| !s.is_empty());

        if prefix.is_none() && suffix.is_none() {
            return Err(PatternError::EmptyPattern);
        }

        let prefix_bytes = if let Some(p) = prefix {
            Self::parse_hex(p)?
        } else {
            Vec::new()
        };

        let suffix_bytes = if let Some(s) = suffix {
            Self::parse_hex(s)?
        } else {
            Vec::new()
        };

        // Limit total pattern length (max 19 hex chars = 9.5 bytes, we round to 9)
        // Ethereum addresses are 40 hex chars, so we need at least 1 char for randomness
        let total_len = prefix_bytes.len() + suffix_bytes.len();
        if total_len > 19 {
            return Err(PatternError::PatternTooLong(total_len));
        }

        let prefix_mask = Self::create_mask(&prefix_bytes);
        let suffix_mask = Self::create_mask(&suffix_bytes);

        Ok(Self {
            prefix: prefix_bytes,
            suffix: suffix_bytes,
            prefix_mask,
            suffix_mask,
        })
    }

    /// Parse hex string to lowercase bytes
    fn parse_hex(s: &str) -> Result<Vec<u8>, PatternError> {
        let mut bytes = Vec::with_capacity(s.len());
        for (i, c) in s.chars().enumerate() {
            let lower = c.to_ascii_lowercase();
            match lower {
                '0'..='9' | 'a'..='f' => bytes.push(lower as u8),
                _ => return Err(PatternError::InvalidHexChar(c, i)),
            }
        }
        Ok(bytes)
    }

    /// Create a mask for the pattern (all bytes set to the pattern length)
    fn create_mask(bytes: &[u8]) -> Vec<u8> {
        bytes.to_vec()
    }

    /// Get the prefix bytes
    pub fn prefix(&self) -> &[u8] {
        &self.prefix
    }

    /// Get the suffix bytes
    pub fn suffix(&self) -> &[u8] {
        &self.suffix
    }

    /// Get the prefix length in hex characters
    pub fn prefix_len(&self) -> usize {
        self.prefix.len()
    }

    /// Get the suffix length in hex characters
    pub fn suffix_len(&self) -> usize {
        self.suffix.len()
    }

    /// Calculate the difficulty (expected number of iterations to find a match)
    /// Each hex character adds factor of 16
    pub fn difficulty(&self) -> u64 {
        let total_chars = self.prefix.len() + self.suffix.len();
        16u64.pow(total_chars as u32)
    }

    /// Calculate the difficulty as a floating point number for large patterns
    pub fn difficulty_f64(&self) -> f64 {
        let total_chars = self.prefix.len() + self.suffix.len();
        16f64.powi(total_chars as i32)
    }

    /// Check if an address matches this pattern
    pub fn matches(&self, address: &str) -> bool {
        let addr = address.to_ascii_lowercase();

        // Check prefix
        if !self.prefix.is_empty() {
            if !addr.starts_with(std::str::from_utf8(&self.prefix).unwrap()) {
                return false;
            }
        }

        // Check suffix
        if !self.suffix.is_empty() {
            if !addr.ends_with(std::str::from_utf8(&self.suffix).unwrap()) {
                return false;
            }
        }

        true
    }

    /// Check if address bytes match this pattern (for GPU results)
    pub fn matches_bytes(&self, address: &[u8; 20]) -> bool {
        // Convert address bytes to hex string for matching
        let hex = hex::encode(address);
        self.matches(&hex)
    }

    /// Get the pattern as a GPU-friendly format
    /// Returns (prefix_bytes, suffix_bytes) padded to fixed size
    pub fn to_gpu_format(&self) -> ([u8; 20], [u8; 20], usize, usize) {
        let mut prefix_bytes = [0u8; 20];
        let mut suffix_bytes = [0u8; 20];

        // Prefix goes at the start
        for (i, &b) in self.prefix.iter().enumerate() {
            if i < 20 {
                prefix_bytes[i] = b;
            }
        }

        // Suffix goes at the end (right-aligned)
        let suffix_start = 20 - self.suffix.len();
        for (i, &b) in self.suffix.iter().enumerate() {
            suffix_bytes[suffix_start + i] = b;
        }

        (prefix_bytes, suffix_bytes, self.prefix.len(), self.suffix.len())
    }
}

impl fmt::Display for Pattern {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let prefix = String::from_utf8_lossy(&self.prefix);
        let suffix = String::from_utf8_lossy(&self.suffix);

        if !self.prefix.is_empty() && !self.suffix.is_empty() {
            write!(f, "{}...{}", prefix, suffix)
        } else if !self.prefix.is_empty() {
            write!(f, "{}...", prefix)
        } else {
            write!(f, "...{}", suffix)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_pattern_creation() {
        let pattern = Pattern::new(Some("dead"), Some("beef")).unwrap();
        assert_eq!(pattern.prefix(), b"dead");
        assert_eq!(pattern.suffix(), b"beef");
    }

    #[test]
    fn test_pattern_uppercase() {
        let pattern = Pattern::new(Some("DEAD"), Some("BEEF")).unwrap();
        assert_eq!(pattern.prefix(), b"dead");
        assert_eq!(pattern.suffix(), b"beef");
    }

    #[test]
    fn test_pattern_prefix_only() {
        let pattern = Pattern::new(Some("cafe"), None).unwrap();
        assert_eq!(pattern.prefix(), b"cafe");
        assert!(pattern.suffix().is_empty());
    }

    #[test]
    fn test_pattern_suffix_only() {
        let pattern = Pattern::new(None, Some("face")).unwrap();
        assert!(pattern.prefix().is_empty());
        assert_eq!(pattern.suffix(), b"face");
    }

    #[test]
    fn test_pattern_empty() {
        assert!(matches!(
            Pattern::new(None, None),
            Err(PatternError::EmptyPattern)
        ));
    }

    #[test]
    fn test_invalid_hex() {
        assert!(matches!(
            Pattern::new(Some("xyz"), None),
            Err(PatternError::InvalidHexChar('x', 0))
        ));
    }

    #[test]
    fn test_difficulty() {
        // 4 hex chars = 16^4 = 65536
        let pattern = Pattern::new(Some("dead"), None).unwrap();
        assert_eq!(pattern.difficulty(), 65536);

        // 8 hex chars = 16^8 = 4,294,967,296
        let pattern = Pattern::new(Some("dead"), Some("beef")).unwrap();
        assert_eq!(pattern.difficulty(), 4294967296);
    }

    #[test]
    fn test_matches() {
        let pattern = Pattern::new(Some("dead"), Some("beef")).unwrap();

        // Should match
        assert!(pattern.matches("dead12345678901234567890123456beef"));
        assert!(pattern.matches("DEAD12345678901234567890123456BEEF"));

        // Should not match
        assert!(!pattern.matches("cafe12345678901234567890123456beef"));
        assert!(!pattern.matches("dead12345678901234567890123456cafe"));
    }

    #[test]
    fn test_display() {
        let pattern = Pattern::new(Some("dead"), Some("beef")).unwrap();
        assert_eq!(format!("{}", pattern), "dead...beef");

        let pattern = Pattern::new(Some("cafe"), None).unwrap();
        assert_eq!(format!("{}", pattern), "cafe...");

        let pattern = Pattern::new(None, Some("face")).unwrap();
        assert_eq!(format!("{}", pattern), "...face");
    }
}
