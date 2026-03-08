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

    // Determine backend name
    let backend_name = match args.backend.as_str() {
        "metal" => "Metal GPU (Apple Silicon)",
        "cuda" => "CUDA GPU (NVIDIA)",
        "opencl" => "OpenCL GPU",
        "cpu" => "CPU",
        _ => "Unknown",
    };

    // Print header once
    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║             🚀 ETHEREUM VANITY ADDRESS GENERATOR - LIVE SEARCH                  ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("  {} {}", "🎯 Pattern:".bold(), pattern_str.green().bold());
    println!("  {} 1 in {:.0} addresses", "📊 Difficulty:", difficulty);
    println!("  {} {} addresses", "🎯 Target:".bold(), target_count.to_string().bright_green());
    println!("  {} {}", "⚡ Backend:", backend_name.bright_magenta());
    println!("  {} {}", "📁 Output:", result_file.path().display().to_string().bright_blue());
    println!();
    println!("{}", "┌──────────────────────────────────────────────────────────────────────────────────┐".cyan());
    println!("{}", "│                              📝 LIVE SEARCH LOG                                  │".cyan());
    println!("{}", "├──────────────────────────────────────────────────────────────────────────────────┤".cyan());
    io::stdout().flush().ok();

    // Run search based on backend
    let results = match args.backend.as_str() {
        "cpu" => run_cpu_search(
            &pattern,
            &mut result_file,
            running,
            target_count,
        )?,
        "metal" => {
            #[cfg(target_os = "macos")]
            {
                run_metal_search(
                    &pattern,
                    &mut result_file,
                    running,
                    target_count,
                    args.device,
                )?
            }
            #[cfg(not(target_os = "macos"))]
            {
                println!("│  {} Metal only available on macOS, falling back to CPU", "⚠️".yellow());
                run_cpu_search(
                    &pattern,
                    &mut result_file,
                    running,
                    target_count,
                )?
            }
        }
        _ => {
            println!("│  {} Unknown backend, falling back to CPU", "⚠️".yellow());
            run_cpu_search(
                &pattern,
                &mut result_file,
                running,
                target_count,
            )?
        }
    };

    // Print final summary
    if !results.is_empty() {
        let total_iterations = results.iter().last().map(|r| r.iteration).unwrap_or(0);
        println!("{}", "├──────────────────────────────────────────────────────────────────────────────────┤".cyan());
        println!(
            "│  {} Search complete! Found {} / {} addresses                                ",
            "✅".bright_green(),
            results.len().to_string().bright_green(),
            target_count
        );
        println!("{}", "└──────────────────────────────────────────────────────────────────────────────────┘".cyan());
        println!();
        println!("  {} Total addresses checked: {}", "📊", output::format_number(total_iterations));
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
) -> Result<Vec<output::SearchResult>> {
    let start = Instant::now();
    let mut results = Vec::new();
    let mut iterations = 0u64;
    let mut last_log_iterations = 0u64;
    let log_interval = 100_000u64;

    loop {
        if results.len() >= target_count || !running.load(Ordering::SeqCst) {
            break;
        }

        let keypair = generate_keypair();
        iterations += 1;

        if pattern.matches(&keypair.address.to_hex()) {
            let result = output::SearchResult::new(keypair, iterations);
            results.push(result.clone());

            if let Err(e) = result_file.save_result(&result) {
                eprintln!("{} Failed to save result: {}", "❌".red(), e);
            }

            let elapsed = start.elapsed().as_secs_f64();
            let rate = iterations as f64 / elapsed.max(0.001);

            println!(
                "│  {} [{}] {} MATCH #{}/{} │ Iteration: {} │ Speed: {}",
                "🎯".bright_green(),
                output::chrono_timestamp().dimmed(),
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

            if results.len() >= target_count {
                break;
            }
        }

        if iterations - last_log_iterations >= log_interval {
            last_log_iterations = iterations;
            let elapsed = start.elapsed().as_secs_f64();
            let rate = iterations as f64 / elapsed.max(0.001);

            println!(
                "│  {} [{}] Checked {} addresses │ Speed: {} │ Found: {}/{}",
                "⏳".yellow(),
                output::chrono_timestamp().dimmed(),
                output::format_number(iterations).white(),
                output::format_speed(rate).cyan(),
                results.len().to_string().bright_green(),
                target_count
            );
            io::stdout().flush().ok();
        }
    }

    Ok(results)
}

#[cfg(target_os = "macos")]
fn run_metal_search(
    pattern: &Pattern,
    result_file: &mut ResultFile,
    running: Arc<AtomicBool>,
    target_count: usize,
    device_id: usize,
) -> Result<Vec<output::SearchResult>> {
    use vanity_metal::MetalSearcher;

    // Initialize Metal
    let searcher = MetalSearcher::new(device_id);
    let device_name = match &searcher {
        Ok(s) => s.device_name(),
        Err(_) => "CPU Fallback".to_string(),
    };

    println!(
        "│  {} Using Metal GPU: {}",
        "🚀".bright_magenta(),
        device_name.bright_white()
    );
    println!("│");
    io::stdout().flush().ok();

    // If Metal failed to initialize, fall back to CPU
    let mut searcher = match searcher {
        Ok(s) => s,
        Err(e) => {
            println!("│  {} Metal init failed: {}, using CPU", "⚠️".yellow(), e);
            return run_cpu_search(pattern, result_file, running, target_count);
        }
    };

    let start = Instant::now();
    let mut results = Vec::new();
    let mut total_iterations = 0u64;
    let batch_size = 1_000_000u64; // 1M per batch
    let log_interval = 10_000_000u64;

    while results.len() < target_count && running.load(Ordering::SeqCst) {
        // Search batch on GPU
        let batch_results = searcher.search_batch(pattern, batch_size)?;

        for gpu_result in batch_results {
            let keypair = gpu_result.to_keypair();
            let result = output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1);
            results.push(result.clone());

            if let Err(e) = result_file.save_result(&result) {
                eprintln!("{} Failed to save result: {}", "❌".red(), e);
            }

            let elapsed = start.elapsed().as_secs_f64();
            let rate = total_iterations as f64 / elapsed.max(0.001);

            println!(
                "│  {} [{}] {} MATCH #{}/{} │ Iteration: {} │ Speed: {}",
                "🎯".bright_green(),
                output::chrono_timestamp().dimmed(),
                "FOUND".bright_green().bold(),
                results.len().to_string().bright_green(),
                target_count,
                output::format_number(result.iteration).bright_white(),
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

            if results.len() >= target_count {
                break;
            }
        }

        total_iterations += batch_size;

        if total_iterations % log_interval == 0 {
            let elapsed = start.elapsed().as_secs_f64();
            let rate = total_iterations as f64 / elapsed.max(0.001);

            println!(
                "│  {} [{}] Checked {} addresses │ Speed: {} │ Found: {}/{}",
                "⏳".yellow(),
                output::chrono_timestamp().dimmed(),
                output::format_number(total_iterations).white(),
                output::format_speed(rate).cyan(),
                results.len().to_string().bright_green(),
                target_count
            );
            io::stdout().flush().ok();
        }
    }

    Ok(results)
}
