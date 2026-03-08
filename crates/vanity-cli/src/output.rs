//! Output formatting and display utilities.

use colored::Colorize;
use vanity_core::crypto::KeyPair;

/// Result of a vanity address search
pub struct SearchResult {
    pub keypair: KeyPair,
    pub iteration: u64,
}

impl SearchResult {
    pub fn new(keypair: KeyPair, iteration: u64) -> Self {
        Self { keypair, iteration }
    }

    /// Print the result to stdout
    pub fn print(&self, index: usize) {
        println!();
        println!("{}", format!("═{}", "═".repeat(70)).dimmed());
        println!(
            "{} {}",
            format!("[{}/]", index + 1).green().bold(),
            "Address Found!".green().bold()
        );
        println!("{}", format!("═{}", "═".repeat(70)).dimmed());

        println!(
            "  {} {}",
            "Address:".cyan().bold(),
            self.keypair.address_hex().bright_white()
        );
        println!(
            "  {} {}",
            "Private Key:".yellow().bold(),
            self.keypair.private_key_hex()
        );
        println!(
            "  {} {}",
            "Found at:".dimmed(),
            format!("iteration {}", self.iteration).dimmed()
        );
        println!("{}", format!("═{}", "═".repeat(70)).dimmed());
        println!();
    }

    /// Print in a simple format for scripting
    pub fn print_simple(&self) {
        println!("address={}", self.keypair.address_hex());
        println!("private_key={}", self.keypair.private_key_hex());
    }
}

/// Print search progress
pub fn print_progress(
    prefix: Option<&str>,
    suffix: Option<&str>,
    difficulty: f64,
    device_name: &str,
) {
    let pattern = match (prefix, suffix) {
        (Some(p), Some(s)) => format!("{}...{}", p, s),
        (Some(p), None) => format!("{}...", p),
        (None, Some(s)) => format!("...{}", s),
        (None, None) => "any".to_string(),
    };

    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════╗".cyan());
    println!("{}", "║         GPU-Accelerated Ethereum Vanity Address Generator        ║".cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════╝".cyan());
    println!();
    println!("  {} {}", "Pattern:".bold(), pattern.green());
    println!("  {} {:.2e} (1 in {:.0})", "Difficulty:".bold(), difficulty, difficulty);
    println!("  {} {}", "Device:".bold(), device_name);
    println!();
    println!("{}", "Searching...".dimmed());
}

/// Print statistics
pub struct Stats {
    pub addresses_checked: u64,
    pub elapsed_secs: f64,
    pub matches_found: usize,
}

impl Stats {
    pub fn new(addresses_checked: u64, elapsed_secs: f64, matches_found: usize) -> Self {
        Self {
            addresses_checked,
            elapsed_secs,
            matches_found,
        }
    }

    pub fn addresses_per_sec(&self) -> f64 {
        if self.elapsed_secs > 0.0 {
            self.addresses_checked as f64 / self.elapsed_secs
        } else {
            0.0
        }
    }

    pub fn print(&self) {
        let rate = self.addresses_per_sec();
        let rate_str = if rate >= 1_000_000.0 {
            format!("{:.2} M/s", rate / 1_000_000.0)
        } else if rate >= 1_000.0 {
            format!("{:.2} K/s", rate / 1_000.0)
        } else {
            format!("{:.2} /s", rate)
        };

        println!();
        println!("{}", "════════════════════════════════════════════════════════════════════".dimmed());
        println!("{}", "                          Statistics                                ".bold());
        println!("{}", "════════════════════════════════════════════════════════════════════".dimmed());
        println!("  {} {}", "Total addresses:".dimmed(), format!("{:.0}", self.addresses_checked));
        println!("  {} {}", "Time elapsed:".dimmed(), format!("{:.2}s", self.elapsed_secs));
        println!("  {} {}", "Speed:".dimmed(), rate_str.green());
        println!("  {} {}", "Matches found:".dimmed(), self.matches_found);
        println!("{}", "════════════════════════════════════════════════════════════════════".dimmed());
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
    fn test_stats_addresses_per_sec() {
        let stats = Stats::new(1_000_000, 1.0, 1);
        assert_eq!(stats.addresses_per_sec(), 1_000_000.0);
    }
}
