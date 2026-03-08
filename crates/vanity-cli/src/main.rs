//! Vanity address generator CLI

mod args;
mod output;

use std::time::Instant;

use anyhow::{Context, Result};
use args::Args;
use colored::Colorize;
use indicatif::{ProgressBar, ProgressStyle};
use output::{LogType, PerformanceTable};
use vanity_core::Pattern;

fn main() -> Result<()> {
    let args = Args::parse_args();

    // Validate and create pattern
    let pattern = Pattern::new(args.prefix.as_deref(), args.suffix.as_deref())
        .context("Invalid pattern")?;

    let difficulty = pattern.difficulty_f64();
    let pattern_str = pattern.to_string();

    // Select backend and run search
    let results = match args.backend.as_str() {
        "cpu" => run_cpu_search(&args, &pattern, &pattern_str, difficulty)?,
        #[cfg(feature = "cuda")]
        "cuda" => run_cuda_search(&args, &pattern, &pattern_str, difficulty)?,
        #[cfg(not(feature = "cuda"))]
        "cuda" => {
            eprintln!("CUDA support not compiled in. Use --backend cpu or rebuild with --features cuda");
            std::process::exit(1);
        }
        #[cfg(feature = "opencl")]
        "opencl" => run_opencl_search(&args, &pattern, &pattern_str, difficulty)?,
        #[cfg(not(feature = "opencl"))]
        "opencl" => {
            eprintln!("OpenCL support not compiled in. Use --backend cpu or rebuild with --features opencl");
            std::process::exit(1);
        }
        _ => unreachable!("Invalid backend (should be caught by clap)"),
    };

    // Print final results
    if !results.is_empty() {
        println!();
        println!("{}", "╔══════════════════════════════════════════════════════════════════════════════╗".bright_green());
        println!("{}", "║                        🎉 SEARCH COMPLETE - RESULTS 🎉                        ║".bright_green());
        println!("{}", "╚══════════════════════════════════════════════════════════════════════════════╝".bright_green());
    }

    for (i, result) in results.iter().enumerate() {
        result.print(i);
    }

    Ok(())
}

fn run_cpu_search(
    args: &Args,
    pattern: &Pattern,
    pattern_str: &str,
    difficulty: f64,
) -> Result<Vec<output::SearchResult>> {
    use vanity_core::crypto::generate_keypair;

    // Initialize performance table
    let mut perf_table = PerformanceTable::new();
    let start = Instant::now();

    // Initial display
    perf_table.render(0, 0, pattern_str, difficulty);

    let mut results = Vec::new();
    let mut iterations = 0u64;
    let max_iterations = if args.max_iterations > 0 {
        Some(args.max_iterations)
    } else {
        None
    };

    // Progress bar for verbose mode
    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    // Log interval (every 50k iterations)
    let log_interval = 50_000u64;
    let mut last_log = 0u64;

    while results.len() < args.count {
        if let Some(max) = max_iterations {
            if iterations >= max {
                break;
            }
        }

        let keypair = generate_keypair();
        iterations += 1;

        if pattern.matches(&keypair.address.to_hex()) {
            results.push(output::SearchResult::new(keypair, iterations));

            // Log match found
            perf_table.log_message(
                &format!(
                    "Match #{} found at iteration {}!",
                    results.len(),
                    output::format_number(iterations)
                ),
                LogType::Match,
            );
        }

        // Update display periodically
        if perf_table.should_update() {
            perf_table.render(iterations, results.len(), pattern_str, difficulty);
        }

        // Log progress every log_interval iterations
        if iterations - last_log >= log_interval {
            last_log = iterations;
            let elapsed = start.elapsed().as_secs_f64();
            let rate = iterations as f64 / elapsed.max(0.001);
            perf_table.log_message(
                &format!(
                    "Checked {} addresses ({})",
                    output::format_number(iterations),
                    output::format_speed(rate)
                ),
                LogType::Info,
            );
        }
    }

    pb.finish();

    // Final display
    perf_table.render(iterations, results.len(), pattern_str, difficulty);

    // Print final statistics
    if args.stats {
        let stats = output::Stats::new(
            iterations,
            start.elapsed().as_secs_f64(),
            results.len(),
        );
        stats.print();
    }

    Ok(results)
}

#[cfg(feature = "cuda")]
fn run_cuda_search(
    args: &Args,
    pattern: &Pattern,
    pattern_str: &str,
    difficulty: f64,
) -> Result<Vec<output::SearchResult>> {
    use vanity_cuda::CudaSearcher;

    let mut searcher = CudaSearcher::new(args.device)
        .context("Failed to initialize CUDA")?;

    let device_name = searcher.device_name();

    // Initialize performance table
    let mut perf_table = PerformanceTable::new();
    let start = Instant::now();

    // Log CUDA initialization
    perf_table.log_message(&format!("Initialized CUDA device: {}", device_name), LogType::Success);

    // Initial display
    perf_table.render(0, 0, pattern_str, difficulty);

    let mut results = Vec::new();
    let mut total_iterations = 0u64;
    let batch_size = 10_000_000u64;

    while results.len() < args.count {
        if args.max_iterations > 0 && total_iterations >= args.max_iterations {
            break;
        }

        let batch_results = searcher.search_batch(pattern, batch_size)?;

        for keypair in batch_results {
            results.push(output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1));
        }

        total_iterations += batch_size;

        // Update display
        perf_table.render(total_iterations, results.len(), pattern_str, difficulty);

        // Log batch completion
        perf_table.log_message(
            &format!(
                "Batch complete: {} total addresses checked",
                output::format_number(total_iterations)
            ),
            LogType::Info,
        );
    }

    // Final statistics
    if args.stats {
        let stats = output::Stats::new(
            total_iterations,
            start.elapsed().as_secs_f64(),
            results.len(),
        );
        stats.print();
    }

    Ok(results)
}

#[cfg(feature = "opencl")]
fn run_opencl_search(
    args: &Args,
    pattern: &Pattern,
    pattern_str: &str,
    difficulty: f64,
) -> Result<Vec<output::SearchResult>> {
    use vanity_opencl::OpenClSearcher;

    let mut searcher = OpenClSearcher::new(args.device)
        .context("Failed to initialize OpenCL")?;

    let device_name = searcher.device_name();

    // Initialize performance table
    let mut perf_table = PerformanceTable::new();
    let start = Instant::now();

    // Log OpenCL initialization
    perf_table.log_message(&format!("Initialized OpenCL device: {}", device_name), LogType::Success);

    // Initial display
    perf_table.render(0, 0, pattern_str, difficulty);

    let mut results = Vec::new();
    let mut total_iterations = 0u64;
    let batch_size = 10_000_000u64;

    while results.len() < args.count {
        if args.max_iterations > 0 && total_iterations >= args.max_iterations {
            break;
        }

        let batch_results = searcher.search_batch(pattern, batch_size)?;

        for keypair in batch_results {
            results.push(output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1));
        }

        total_iterations += batch_size;

        // Update display
        perf_table.render(total_iterations, results.len(), pattern_str, difficulty);

        // Log batch completion
        perf_table.log_message(
            &format!(
                "Batch complete: {} total addresses checked",
                output::format_number(total_iterations)
            ),
            LogType::Info,
        );
    }

    // Final statistics
    if args.stats {
        let stats = output::Stats::new(
            total_iterations,
            start.elapsed().as_secs_f64(),
            results.len(),
        );
        stats.print();
    }

    Ok(results)
}
