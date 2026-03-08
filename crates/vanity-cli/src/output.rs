//! Output formatting and display utilities.

use colored::Colorize;
use std::io::{self, Write};
use std::time::Duration;
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
            format!("iteration {}", format_number(self.iteration)).dimmed()
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
}

/// Performance table for live display
pub struct PerformanceTable {
    start_time: std::time::Instant,
    last_update: std::time::Instant,
    update_interval: Duration,
}

impl PerformanceTable {
    pub fn new() -> Self {
        Self {
            start_time: std::time::Instant::now(),
            last_update: std::time::Instant::now() - Duration::from_secs(1),
            update_interval: Duration::from_millis(200), // Update every 200ms
        }
    }

    /// Check if it's time to update the display
    pub fn should_update(&self) -> bool {
        self.last_update.elapsed() >= self.update_interval
    }

    /// Render the live performance table
    pub fn render(
        &mut self,
        iterations: u64,
        matches: usize,
        pattern: &str,
        difficulty: f64,
    ) {
        self.last_update = std::time::Instant::now();
        let elapsed = self.start_time.elapsed().as_secs_f64();
        let rate = if elapsed > 0.0 {
            iterations as f64 / elapsed
        } else {
            0.0
        };

        // Clear previous lines and render new table
        print!("{}", "\x1B[2J\x1B[1;1H"); // Clear screen and move cursor to top

        // Header
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════╗".bright_cyan());
        println!("{}", "║              🚀 ETHEREUM VANITY ADDRESS GENERATOR - LIVE SEARCH              ║".bright_cyan());
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════╝".bright_cyan());
        println!();

        // Pattern info
        println!("  {} {}", "🎯 Pattern:".bold(), pattern.green().bold());
        println!("  {} {:.2e} (1 in {:.0})", "📊 Difficulty:", difficulty, difficulty);
        println!();

        // Performance table
        println!("{}", "┌─────────────────────────────────────────────────────────────────────────────┐".cyan());
        println!("{}", "│                          📈 PERFORMANCE METRICS                              │".cyan());
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────┤".cyan());

        // Row 1: Iterations and Speed
        let iter_str = format_number(iterations);
        let speed_str = format_speed(rate);
        println!(
            "│  {:<20} {:<25}  │",
            format!("{}:", "Addresses Checked").dimmed(),
            iter_str.bright_white().bold()
        );
        println!(
            "│  {:<20} {:<25}  │",
            format!("{}:", "Speed").dimmed(),
            speed_str.green().bold()
        );

        // Row 2: Time and Matches
        let time_str = format_duration(elapsed);
        println!(
            "│  {:<20} {:<25}  │",
            format!("{}:", "Time Elapsed").dimmed(),
            time_str.yellow().bold()
        );
        println!(
            "│  {:<20} {:<25}  │",
            format!("{}:", "Matches Found").dimmed(),
            format!("{}", matches).bright_green().bold()
        );

        // Row 3: ETA estimate
        let remaining = difficulty as f64 - iterations as f64;
        let eta_str = if rate > 0.0 && remaining > 0.0 {
            let eta_secs = remaining / rate;
            format_duration(eta_secs)
        } else {
            "Calculating...".to_string()
        };
        println!(
            "│  {:<20} {:<25}  │",
            format!("{}:", "Est. Time Remaining").dimmed(),
            eta_str.bright_blue().bold()
        );

        println!("{}", "└─────────────────────────────────────────────────────────────────────────────┘".cyan());
        println!();

        // Progress bar
        let progress = (iterations as f64 / difficulty.max(1.0)).min(1.0);
        let bar_width = 60;
        let filled = (progress * bar_width as f64) as usize;
        let empty = bar_width - filled;

        print!("  [");
        print!("{}", "█".repeat(filled).green());
        print!("{}", "░".repeat(empty).dimmed());
        println!("] {:.2}%", progress * 100.0);
        println!();

        // Activity log
        println!("{}", "┌─────────────────────────────────────────────────────────────────────────────┐".dimmed());
        println!("{}", "│                              📝 ACTIVITY LOG                                 │".dimmed());
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────┤".dimmed());
        println!(
            "│  {} {}",
            "⏱".dimmed(),
            format!("[{}] Searching...", format_timestamp()).dimmed()
        );
        println!(
            "│  {} {}",
            "⚡".dimmed(),
            format!("Processing {:.0} keys/second", rate).dimmed()
        );
        if matches > 0 {
            println!(
                "│  {} {}",
                "✅".green(),
                format!("[{}] Found {} match(es) so far!", format_timestamp(), matches).green()
            );
        }
        println!("{}", "└─────────────────────────────────────────────────────────────────────────────┘".dimmed());

        // Flush output
        io::stdout().flush().ok();
    }

    /// Log a message during search
    pub fn log_message(&self, message: &str, msg_type: LogType) {
        let timestamp = format_timestamp();
        let prefix = match msg_type {
            LogType::Info => "ℹ️ ".blue(),
            LogType::Success => "✅ ".green(),
            LogType::Warning => "⚠️ ".yellow(),
            LogType::Match => "🎯 ".bright_green(),
        };
        eprintln!("{} [{}] {}", prefix, timestamp.dimmed(), message);
    }
}

impl Default for PerformanceTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Log message type
pub enum LogType {
    Info,
    Success,
    Warning,
    Match,
}

/// Print statistics (final summary)
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
        let rate_str = format_speed(rate);

        println!();
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════╗".cyan());
        println!("{}", "║                           📊 FINAL STATISTICS                                 ║".cyan());
        println!("{}", "╠══════════════════════════════════════════════════════════════════════════════╣".cyan());
        println!(
            "│  {:<25} {:>45}  │",
            "Total Addresses:".dimmed(),
            format_number(self.addresses_checked).bright_white().bold()
        );
        println!(
            "│  {:<25} {:>45}  │",
            "Time Elapsed:".dimmed(),
            format_duration(self.elapsed_secs).yellow().bold()
        );
        println!(
            "│  {:<25} {:>45}  │",
            "Average Speed:".dimmed(),
            rate_str.green().bold()
        );
        println!(
            "│  {:<25} {:>45}  │",
            "Matches Found:".dimmed(),
            format!("{}", self.matches_found).bright_green().bold()
        );
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════╝".cyan());
        println!();
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
    } else {
        format!("{:.0} /s", rate)
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

/// Format current timestamp
fn format_timestamp() -> String {
    let now = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
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
    fn test_stats_addresses_per_sec() {
        let stats = Stats::new(1_000_000, 1.0, 1);
        assert_eq!(stats.addresses_per_sec(), 1_000_000.0);
    }

    #[test]
    fn test_format_speed() {
        assert_eq!(format_speed(500.0), "500 /s");
        assert_eq!(format_speed(1500.0), "1.50 K/s");
        assert_eq!(format_speed(1_500_000.0), "1.50 M/s");
    }

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30.0), "30.0s");
        assert_eq!(format_duration(90.0), "1m 30s");
        assert_eq!(format_duration(3661.0), "1h 1m");
        assert_eq!(format_duration(90061.0), "1d 1h");
    }
}
