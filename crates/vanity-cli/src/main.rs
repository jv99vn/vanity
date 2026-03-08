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
use output::{PerformanceTable, ResultFile};
use vanity_core::Pattern;

fn main() -> Result<()> {
    let args = Args::parse_args();

    // Setup Ctrl+C handler
    let running = Arc::new(AtomicBool::new(true));
    let r = running.clone();
    ctrlc::set_handler(move || {
        r.store(false, Ordering::SeqCst);
        println!("\n\n{}", "🛑 Stopping search...".yellow());
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

    println!();
    println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".bright_cyan());
    println!("{}", "║             🚀 ETHEREUM VANITY ADDRESS GENERATOR - LIVE SEARCH                  ║".bright_cyan());
    println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".bright_cyan());
    println!();
    println!("  {} {}", "🎯 Pattern:".bold(), pattern_str.green().bold());
    println!("  {} {:.2e} (1 in {:.0})", "📊 Difficulty:", difficulty, difficulty);
    println!("  {} {} addresses", "🎯 Target:".bold(), target_count.to_string().bright_green());
    println!("  {} {}", "📁 Output:", result_file.path().display().to_string().bright_blue());
    println!();
    println!("{}", "Starting search... (Press Ctrl+C to stop)".dimmed());
    println!();

    // Run search
    let results = match args.backend.as_str() {
        "cpu" => run_cpu_search(
            &args,
            &pattern,
            &pattern_str,
            difficulty,
            &mut result_file,
            running,
        )?,
        #[cfg(feature = "cuda")]
        "cuda" => run_cuda_search(
            &args,
            &pattern,
            &pattern_str,
            difficulty,
            &mut result_file,
            running,
        )?,
        #[cfg(not(feature = "cuda"))]
        "cuda" => {
            eprintln!("CUDA support not compiled in. Use --backend cpu or rebuild with --features cuda");
            std::process::exit(1);
        }
        #[cfg(feature = "opencl")]
        "opencl" => run_opencl_search(
            &args,
            &pattern,
            &pattern_str,
            difficulty,
            &mut result_file,
            running,
        )?,
        #[cfg(not(feature = "opencl"))]
        "opencl" => {
            eprintln!("OpenCL support not compiled in. Use --backend cpu or rebuild with --features opencl");
            std::process::exit(1);
        }
        _ => unreachable!("Invalid backend (should be caught by clap)"),
    };

    // Print final summary
    if !results.is_empty() {
        println!();
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════════╗".bright_green());
        println!("{}", "║                          🎉 SEARCH COMPLETE - RESULTS 🎉                         ║".bright_green());
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════════╝".bright_green());

        for (i, result) in results.iter().enumerate() {
            result.print(i, results.len());
        }

        println!("  {} Results saved to: {}", "📁".blue(), result_file.path().display().to_string().bright_blue());
        println!();
    } else {
        println!();
        println!("{}", "No matches found (search was interrupted)".yellow());
    }

    Ok(())
}

fn run_cpu_search(
    args: &Args,
    pattern: &Pattern,
    pattern_str: &str,
    difficulty: f64,
    result_file: &mut ResultFile,
    running: Arc<AtomicBool>,
) -> Result<Vec<output::SearchResult>> {
    use vanity_core::crypto::generate_keypair;

    let mut perf_table = PerformanceTable::new();
    let start = Instant::now();

    let mut results = Vec::new();
    let mut iterations = 0u64;
    let target_count = args.count;

    // Main search loop - runs until we have enough matches or user stops
    while results.len() < target_count && running.load(Ordering::SeqCst) {
        let keypair = generate_keypair();
        iterations += 1;

        if pattern.matches(&keypair.address.to_hex()) {
            let result = output::SearchResult::new(keypair, iterations);
            results.push(result);

            // Save to file immediately
            if let Err(e) = result_file.save_result(&results.last().unwrap()) {
                eprintln!("{} Failed to save result: {}", "❌".red(), e);
            }

            // Print match notification immediately (real-time!)
            perf_table.print_match_found(
                &results.last().unwrap(),
                results.len(),
                target_count,
            );

            // Flush stdout to ensure immediate output
            io::stdout().flush().ok();
        }

        // Update display periodically (every 100ms)
        if perf_table.should_update() {
            perf_table.render(iterations, results.len(), target_count, pattern_str, difficulty);
        }
    }

    // Final display
    perf_table.render(iterations, results.len(), target_count, pattern_str, difficulty);

    // Print final statistics
    let stats = output::Stats::new(
        iterations,
        start.elapsed().as_secs_f64(),
        results.len(),
        target_count,
        &result_file.path().display().to_string(),
    );
    stats.print();

    Ok(results)
}

#[cfg(feature = "cuda")]
fn run_cuda_search(
    args: &Args,
    pattern: &Pattern,
    pattern_str: &str,
    difficulty: f64,
    result_file: &mut ResultFile,
    running: Arc<AtomicBool>,
) -> Result<Vec<output::SearchResult>> {
    use vanity_cuda::CudaSearcher;

    let mut searcher = CudaSearcher::new(args.device)
        .context("Failed to initialize CUDA")?;

    let device_name = searcher.device_name();
    let mut perf_table = PerformanceTable::new();
    let start = Instant::now();

    let mut results = Vec::new();
    let mut total_iterations = 0u64;
    let batch_size = 1_000_000u64; // Smaller batches for more frequent updates
    let target_count = args.count;

    while results.len() < target_count && running.load(Ordering::SeqCst) {
        let batch_results = searcher.search_batch(pattern, batch_size)?;

        for keypair in batch_results {
            let result = output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1);
            results.push(result);

            // Save to file immediately
            if let Err(e) = result_file.save_result(&results.last().unwrap()) {
                eprintln!("{} Failed to save result: {}", "❌".red(), e);
            }

            // Print match notification immediately
            perf_table.print_match_found(
                &results.last().unwrap(),
                results.len(),
                target_count,
            );
            io::stdout().flush().ok();
        }

        total_iterations += batch_size;

        // Update display
        perf_table.render(total_iterations, results.len(), target_count, pattern_str, difficulty);
    }

    // Print final statistics
    let stats = output::Stats::new(
        total_iterations,
        start.elapsed().as_secs_f64(),
        results.len(),
        target_count,
        &result_file.path().display().to_string(),
    );
    stats.print();

    Ok(results)
}

#[cfg(feature = "opencl")]
fn run_opencl_search(
    args: &Args,
    pattern: &Pattern,
    pattern_str: &str,
    difficulty: f64,
    result_file: &mut ResultFile,
    running: Arc<AtomicBool>,
) -> Result<Vec<output::SearchResult>> {
    use vanity_opencl::OpenClSearcher;

    let mut searcher = OpenClSearcher::new(args.device)
        .context("Failed to initialize OpenCL")?;

    let device_name = searcher.device_name();
    let mut perf_table = PerformanceTable::new();
    let start = Instant::now();

    let mut results = Vec::new();
    let mut total_iterations = 0u64;
    let batch_size = 1_000_000u64;
    let target_count = args.count;

    while results.len() < target_count && running.load(Ordering::SeqCst) {
        let batch_results = searcher.search_batch(pattern, batch_size)?;

        for keypair in batch_results {
            let result = output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1);
            results.push(result);

            // Save to file immediately
            if let Err(e) = result_file.save_result(&results.last().unwrap()) {
                eprintln!("{} Failed to save result: {}", "❌".red(), e);
            }

            // Print match notification immediately
            perf_table.print_match_found(
                &results.last().unwrap(),
                results.len(),
                target_count,
            );
            io::stdout().flush().ok();
        }

        total_iterations += batch_size;

        // Update display
        perf_table.render(total_iterations, results.len(), target_count, pattern_str, difficulty);
    }

    // Print final statistics
    let stats = output::Stats::new(
        total_iterations,
        start.elapsed().as_secs_f64(),
        results.len(),
        target_count,
        &result_file.path().display().to_string(),
    );
    stats.print();

    Ok(results)
}
