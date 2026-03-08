//! Vanity address generator CLI

mod args;
mod output;

use std::io::{self, Write};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Instant;

use anyhow::{Context, Result};
use args::Args;
use colored::Colorize;
use output::ResultFile;
use vanity_core::{Pattern, crypto::generate_keypair};

fn main() -> Result<()> {
    let args = Args::parse_args();

    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!();
        println!("{}", "🛑 Stopping search...".yellow());
    }).expect("Error setting Ctrl-C handler");

    // Validate and create pattern
    let pattern = Pattern::new(args.prefix.as_deref(), args.suffix.as_deref())
        .context("Invalid pattern")?;

    let difficulty = pattern.difficulty_f64();
    let pattern_str = pattern.to_string();
    let target_count = args.count;

    // Create output file
    let mut result_file = ResultFile::new(args.prefix.as_deref(), args.suffix.as_deref())
        .context("Failed to create output file")?;

    // Print header once
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║             🚀 ETHEREUM VANITY ADDRESS GENERATOR - LIVE SEARCH                  ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("  {} {}", "🎯 Pattern:".bold(), pattern_str.green().bold());
    println!("  {} 1 in {:.0} addresses", "📊 Difficulty:", difficulty);
    println!("  {} {} addresses", "🎯 Target:".bold(), target_count.to_string().bright_green());
    println!("  {} {}", "📁 Output:", result_file.path().display().to_string().bright_blue());
    println!();
    println!("{}", "┌──────────────────────────────────────────────────────────────────────────────────┐".cyan());
    println!("{}", "│                              📝 LIVE SEARCH LOG                                  │".cyan());
    println!("{}", "├──────────────────────────────────────────────────────────────────────────────────┤".cyan());
    io::stdout().flush().ok();

    // Run search
    let (results, total_iterations, elapsed) = run_cpu_search(
        &pattern,
        &mut result_file,
        running,
        target_count,
    )?;

    // Print final summary
    if !results.is_empty() {
        println!("{}", "├──────────────────────────────────────────────────────────────────────────────────┤".cyan());
        println!(
            "│  {} Search complete! Found {} / {} addresses in {}                ",
            "✅".bright_green(),
            results.len().to_string().bright_green(),
            target_count,
            output::format_duration(elapsed)
        );
        println!("{}", "└──────────────────────────────────────────────────────────────────────────────────┘".cyan());
        println!();
        println!("  {} Total addresses checked: {}", "📊", output::format_number(total_iterations));
        println!("  {} Average speed: {}", "⚡", output::format_speed(total_iterations as f64 / elapsed.max(0.001)));
        println!("  {} Results saved to: {}", "📁", result_file.path().display().to_string().bright_blue());
        println!();
    } else {
        println!("{}", "└──────────────────────────────────────────────────────────────────────────────────┘".cyan());
        println!();
        println!("{}", "  No matches found (search was interrupted)".yellow());
        println!();
    }

    Ok(())
}

fn run_cpu_search(
    pattern: &Pattern,
    result_file: &mut ResultFile,
    running: Arc<AtomicBool>,
    target_count: usize,
) -> Result<(Vec<output::SearchResult>, u64, f64)> {
    let start = Instant::now();
    let mut results = Vec::new();
    let mut iterations = 0u64;
    let mut last_log_iterations = 0u64;
    let log_interval = 100_000u64; // Log progress every 100k iterations

    // Main search loop - ONLY stops when:
    // 1. Found enough addresses (target_count)
    // 2. User presses Ctrl+C
    loop {
        // Check if we have enough results FIRST - STOP IMMEDIATELY
        if results.len() >= target_count {
            break;
        }

        // Check for Ctrl+C
        if !running.load(Ordering::SeqCst) {
            break;
        }

        // Generate one keypair
        let keypair = generate_keypair();
        iterations += 1;

        // Get address hex without 0x prefix for matching
        let addr_hex = keypair.address.to_hex();

        // Check if it matches the pattern
        if pattern.matches(&addr_hex) {
            let result = output::SearchResult::new(keypair, iterations);
            results.push(result.clone());

            // Save to file IMMEDIATELY
            if let Err(e) = result_file.save_result(&result) {
                eprintln!("{} Failed to save result: {}", "❌".red(), e);
            }

            // Print match notification IMMEDIATELY in real-time
            let elapsed = start.elapsed().as_secs_f64();
            let rate = iterations as f64 / elapsed.max(0.001);
            let timestamp = output::chrono_timestamp();

            println!(
                "│  {} [{}] {} MATCH #{}/{} │ Iteration: {} │ Speed: {}",
                "🎯".bright_green(),
                timestamp.dimmed(),
                "FOUND".bright_green().bold(),
                results.len().to_string().bright_green(),
                target_count,
                output::format_number(iterations).bright_white(),
                output::format_speed(rate).yellow()
            );
            println!(
                "│       📍 Address: {}",
                result.keypair.address_hex().bright_white()
            );
            println!(
                "│       🔑 Private: {}",
                result.keypair.private_key_hex().dimmed()
            );
            println!("│");
            io::stdout().flush().ok();

            // Check if we have enough - STOP IMMEDIATELY, no more iterations
            if results.len() >= target_count {
                break;
            }
        }

        // Log progress periodically (not clearing screen)
        if iterations - last_log_iterations >= log_interval {
            last_log_iterations = iterations;
            let elapsed = start.elapsed().as_secs_f64();
            let rate = iterations as f64 / elapsed.max(0.001);
            let timestamp = output::chrono_timestamp();

            println!(
                "│  {} [{}] Checked {} addresses │ Speed: {} │ Found: {}/{}",
                "⏳".yellow(),
                timestamp.dimmed(),
                output::format_number(iterations).white(),
                output::format_speed(rate).cyan(),
                results.len().to_string().bright_green(),
                target_count
            );
            io::stdout().flush().ok();
        }
    }

    let elapsed = start.elapsed().as_secs_f64();
    Ok((results, iterations, elapsed))
}
