//! Output formatting and display utilities.

use colored::Colorize;
use std::fs::{File, OpenOptions};
use std::io::{self, Write};
use std::path::PathBuf;
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
    pub fn print(&self, index: usize, total: usize) {
        println!();
        println!("{}", "┌─────────────────────────────────────────────────────────────────────────────┐".bright_green());
        println!(
            "{}",
            format!(
                "│  🎯 MATCH {}/{} - Found at iteration {}",
                index + 1,
                total,
                format_number(self.iteration)
            )
            .bright_green()
            .bold()
        );
        println!("{}", "├─────────────────────────────────────────────────────────────────────────────┤".bright_green());

        println!(
            "│  {} {}",
            "Address:".cyan().bold(),
            self.keypair.address_hex().bright_white()
        );
        println!(
            "│  {} {}",
            "Private:".yellow().bold(),
            self.keypair.private_key_hex()
        );
        println!("{}", "└─────────────────────────────────────────────────────────────────────────────┘".bright_green());
        println!();
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
            (Some(p), Some(s)) => format!("{}_{}.txt", p, s),
            (Some(p), None) => format!("{}.txt", p),
            (None, Some(s)) => format!("{}.txt", s),
            (None, None) => "results.txt".to_string(),
        };

        let path = PathBuf::from(&filename);

        // Create/truncate file
        let mut file = File::create(&path)?;
        writeln!(file, "# Vanity Address Results")?;
        writeln!(file, "# Pattern: {}",
            match (prefix, suffix) {
                (Some(p), Some(s)) => format!("{}...{}", p, s),
                (Some(p), None) => format!("{}...", p),
                (None, Some(s)) => format!("...{}", s),
                (None, None) => "any".to_string(),
            }
        )?;
        writeln!(file, "# Generated: {}", chrono_timestamp())?;
        writeln!(file, "#")?;
        writeln!(file)?;

        Ok(Self { path })
    }

    pub fn save_result(&mut self, result: &SearchResult) -> io::Result<()> {
        let mut file = OpenOptions::new()
            .append(true)
            .open(&self.path)?;
        write!(file, "{}", result.to_file_format())?;
        Ok(())
    }

    pub fn path(&self) -> &PathBuf {
        &self.path
    }
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
            update_interval: Duration::from_millis(100), // Update every 100ms for smoother display
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
        target: usize,
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

        // Move cursor to top and clear
        print!("\x1B[H\x1B[2J");

        // Header
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".bright_cyan());
        println!("{}", "║             🚀 ETHEREUM VANITY ADDRESS GENERATOR - LIVE SEARCH                  ║".bright_cyan());
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".bright_cyan());
        println!();

        // Pattern info
        println!("  {} {}", "🎯 Pattern:".bold(), pattern.green().bold());
        println!("  {} {:.2e} (1 in {:.0})", "📊 Difficulty:", difficulty, difficulty);
        println!("  {} {} / {}", "🎯 Target:".bold(), matches.to_string().bright_green(), target.to_string().bright_white());
        println!();

        // Performance table
        println!("{}", "┌──────────────────────────────────────────────────────────────────────────────────┐".cyan());
        println!("{}", "│                            📈 LIVE PERFORMANCE                                   │".cyan());
        println!("{}", "├──────────────────────────────────────────────────────────────────────────────────┤".cyan());

        // Stats
        let iter_str = format_number(iterations);
        let speed_str = format_speed(rate);
        let time_str = format_duration(elapsed);

        println!(
            "│  {:<22} {:<52}│",
            "Addresses Checked:",
            iter_str.bright_white().bold()
        );
        println!(
            "│  {:<22} {:<52}│",
            "Speed:",
            speed_str.green().bold()
        );
        println!(
            "│  {:<22} {:<52}│",
            "Time Elapsed:",
            time_str.yellow().bold()
        );
        println!(
            "│  {:<22} {:<52}│",
            "Matches Found:",
            format!("{}", matches).bright_green().bold()
        );

        // Progress bar for matches
        let match_progress = (matches as f64 / target as f64).min(1.0);
        let bar_width = 50;
        let filled = (match_progress * bar_width as f64) as usize;
        let empty = bar_width - filled;

        print!("│  {:<22} ", "Progress:");
        print!("{}", "█".repeat(filled).green());
        print!("{}", "░".repeat(empty).dimmed());
        println!(" {:5.1}% {:>8}│", match_progress * 100.0, format!("({}/{})", matches, target));

        println!("{}", "└──────────────────────────────────────────────────────────────────────────────────┘".cyan());
        println!();

        // Live log area - show recent activity
        println!("{}", "┌──────────────────────────────────────────────────────────────────────────────────┐".dimmed());
        println!("{}", "│                              📝 LIVE ACTIVITY LOG                                │".dimmed());
        println!("{}", "├──────────────────────────────────────────────────────────────────────────────────┤".dimmed());

        // Current status
        let timestamp = chrono_timestamp();
        println!(
            "│  {} [{}] {}",
            "⏱".yellow(),
            timestamp.dimmed(),
            format!("Searching... ({} addresses/sec)", format_speed(rate)).white()
        );

        if matches > 0 {
            println!(
                "│  {} [{}] Found {} match(es) - saved to file",
                "✅".bright_green(),
                timestamp.dimmed(),
                matches.to_string().bright_green()
            );
        }

        println!(
            "│  {} {}",
            "📁".blue(),
            format!("Results will be saved to output file").dimmed()
        );

        println!("{}", "└──────────────────────────────────────────────────────────────────────────────────┘".dimmed());

        // Instructions
        println!();
        println!("  {} Press Ctrl+C to stop searching", "ℹ️".dimmed());

        // Flush output
        io::stdout().flush().ok();
    }

    /// Print a match notification immediately
    pub fn print_match_found(&self, result: &SearchResult, match_num: usize, total: usize) {
        // Print above the current display
        println!();
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".bright_green());
        println!(
            "{}",
            format!(
                "║  🎉 MATCH #{}/{} FOUND at iteration {}",
                match_num,
                total,
                format_number(result.iteration)
            )
            .bright_green()
            .bold()
        );
        println!("{}", "╠══════════════════════════════════════════════════════════════════════════════════╣".bright_green());
        println!(
            "║  {}: {}",
            "Address".cyan(),
            result.keypair.address_hex().bright_white()
        );
        println!(
            "║  {}: {}",
            "Private".yellow(),
            result.keypair.private_key_hex()
        );
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".bright_green());
        println!();
        io::stdout().flush().ok();
    }
}

impl Default for PerformanceTable {
    fn default() -> Self {
        Self::new()
    }
}

/// Print final statistics
pub struct Stats {
    pub addresses_checked: u64,
    pub elapsed_secs: f64,
    pub matches_found: usize,
    pub target: usize,
    pub output_file: String,
}

impl Stats {
    pub fn new(
        addresses_checked: u64,
        elapsed_secs: f64,
        matches_found: usize,
        target: usize,
        output_file: &str,
    ) -> Self {
        Self {
            addresses_checked,
            elapsed_secs,
            matches_found,
            target,
            output_file: output_file.to_string(),
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
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".cyan());
        println!("{}", "║                            📊 FINAL STATISTICS                                   ║".cyan());
        println!("{}", "╠══════════════════════════════════════════════════════════════════════════════════╣".cyan());
        println!(
            "│  {:<25} {:>48}  │",
            "Total Addresses:".dimmed(),
            format_number(self.addresses_checked).bright_white().bold()
        );
        println!(
            "│  {:<25} {:>48}  │",
            "Time Elapsed:".dimmed(),
            format_duration(self.elapsed_secs).yellow().bold()
        );
        println!(
            "│  {:<25} {:>48}  │",
            "Average Speed:".dimmed(),
            rate_str.green().bold()
        );
        println!(
            "│  {:<25} {:>48}  │",
            "Matches Found:".dimmed(),
            format!("{}/{}", self.matches_found, self.target).bright_green().bold()
        );
        println!(
            "│  {:<25} {:>48}  │",
            "Output File:".dimmed(),
            self.output_file.bright_blue().bold()
        );
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".cyan());
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

/// Format current timestamp
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

    #[test]
    fn test_format_duration() {
        assert_eq!(format_duration(30.0), "30.0s");
        assert_eq!(format_duration(90.0), "1m 30s");
        assert_eq!(format_duration(3661.0), "1h 1m");
        assert_eq!(format_duration(90061.0), "1d 1h");
    }
}
