//! Output formatting and display utilities.

use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
use vanity_core::crypto::KeyPair;

/// Result of a vanity address search
#[derive(Clone)]
pub struct SearchResult {
    pub keypair: KeyPair,
    pub iteration: u64,
}

impl SearchResult {
    pub fn new(keypair: KeyPair, iteration: u64) -> Self {
        Self { keypair, iteration }
    }

    /// Convert to file format string
    pub fn to_file_format(&self) -> String {
        format!(
            "Address: {}\nPrivate Key: {}\nIteration: {}\n---\n",
            self.keypair.address_hex(),
            self.keypair.private_key_hex(),
            self.iteration
        )
    }
}

/// File handler for saving results
pub struct ResultFile {
    path: PathBuf,
}

impl ResultFile {
    pub fn new(prefix: Option<&str>, suffix: Option<&str>) -> io::Result<Self> {
        let filename = match (prefix, suffix) {
            (Some(p), Some(s)) => format!("{}_{}.txt", p.to_lowercase(), s.to_lowercase()),
            (Some(p), None) => format!("{}.txt", p.to_lowercase()),
            (None, Some(s)) => format!("{}.txt", s.to_lowercase()),
            (None, None) => "results.txt".to_string(),
        };

        let path = PathBuf::from(&filename);

        // Create/truncate file with header
        let mut file = File::create(&path)?;
        writeln!(file, "# ==========================================")?;
        writeln!(file, "# Ethereum Vanity Address Results")?;
        writeln!(file, "# ==========================================")?;
        writeln!(file, "# Pattern: {}",
            match (prefix, suffix) {
                (Some(p), Some(s)) => format!("{}...{}", p, s),
                (Some(p), None) => format!("{}...", p),
                (None, Some(s)) => format!("...{}", s),
                (None, None) => "any".to_string(),
            }
        )?;
        writeln!(file, "# Generated: {}", chrono_timestamp())?;
        writeln!(file, "# ==========================================")?;
        writeln!(file)?;

        Ok(Self { path })
    }

    pub fn save_result(&self, result: &SearchResult) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)?;
        write!(file, "{}", result.to_file_format())?;
        file.sync_all()?; // Ensure data is written to disk immediately
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
}

/// Format a large number with commas
pub fn format_number(n: u64) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().enumerate() {
        if i > 0 && (s.len() - i) % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result
}

/// Format speed with appropriate unit
pub fn format_speed(rate: f64) -> String {
    if rate >= 1_000_000.0 {
        format!("{:.2} M/s", rate / 1_000_000.0)
    } else if rate >= 1_000.0 {
        format!("{:.2} K/s", rate / 1_000.0)
    } else if rate >= 1.0 {
        format!("{:.0} /s", rate)
    } else {
        format!("{:.1} /s", rate)
    }
}

/// Format duration in human-readable format
pub fn format_duration(secs: f64) -> String {
    if secs < 60.0 {
        format!("{:.1}s", secs)
    } else if secs < 3600.0 {
        let mins = (secs / 60.0) as u64;
        let s = (secs % 60.0) as u64;
        format!("{}m {}s", mins, s)
    } else if secs < 86400.0 {
        let hours = (secs / 3600.0) as u64;
        let mins = ((secs % 3600.0) / 60.0) as u64;
        format!("{}h {}m", hours, mins)
    } else {
        let days = (secs / 86400.0) as u64;
        let hours = ((secs % 86400.0) / 3600.0) as u64;
        format!("{}d {}h", days, hours)
    }
}

/// Format current timestamp (HH:MM:SS)
pub fn chrono_timestamp() -> String {
    use std::time::{SystemTime, UNIX_EPOCH};
    let now = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default();
    let secs = now.as_secs() % 86400;
    let hours = secs / 3600;
    let mins = (secs % 3600) / 60;
    let secs = secs % 60;
    format!("{:02}:{:02}:{:02}", hours, mins, secs)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_format_number() {
        assert_eq!(format_number(0), "0");
        assert_eq!(format_number(100), "100");
        assert_eq!(format_number(1000), "1,000");
        assert_eq!(format_number(1000000), "1,000,000");
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(500.0), "500 /s");
        assert_eq!(format_speed(1500.0), "1.50 K/s");
        assert_eq!(format_speed(1_500_000.0), "1.50 M/s");
    }
}
