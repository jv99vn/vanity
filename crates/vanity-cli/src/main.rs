//! Vanity address generator CLI

mod args;
mod output;

use std::time::Instant;

use anyhow::{Context, Result};
use args::Args;
use indicatif::{ProgressBar, ProgressStyle};
use vanity_core::Pattern;

fn main() -> Result<()> {
    let args = Args::parse_args();

    // Validate and create pattern
    let pattern = Pattern::new(args.prefix.as_deref(), args.suffix.as_deref())
        .context("Invalid pattern")?;

    let _difficulty = pattern.difficulty_f64();

    // Select backend and run search
    let results = match args.backend.as_str() {
        "cpu" => run_cpu_search(&args, &pattern)?,
        #[cfg(feature = "cuda")]
        "cuda" => run_cuda_search(&args, &pattern)?,
        #[cfg(not(feature = "cuda"))]
        "cuda" => {
            eprintln!("CUDA support not compiled in. Use --backend cpu or rebuild with --features cuda");
            std::process::exit(1);
        }
        #[cfg(feature = "opencl")]
        "opencl" => run_opencl_search(&args, &pattern)?,
        #[cfg(not(feature = "opencl"))]
        "opencl" => {
            eprintln!("OpenCL support not compiled in. Use --backend cpu or rebuild with --features opencl");
            std::process::exit(1);
        }
        _ => unreachable!("Invalid backend (should be caught by clap)"),
    };

    // Print results
    for (i, result) in results.iter().enumerate() {
        result.print(i);
    }

    Ok(())
}

fn run_cpu_search(args: &Args, pattern: &Pattern) -> Result<Vec<output::SearchResult>> {
    use vanity_core::crypto::generate_keypair;

    output::print_progress(
        args.prefix.as_deref(),
        args.suffix.as_deref(),
        pattern.difficulty_f64(),
        "CPU",
    );

    let start = Instant::now();
    let mut results = Vec::new();
    let mut iterations = 0u64;
    let max_iterations = if args.max_iterations > 0 {
        Some(args.max_iterations)
    } else {
        None
    };

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

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

            if args.stats {
                let stats = output::Stats::new(
                    iterations,
                    start.elapsed().as_secs_f64(),
                    results.len(),
                );
                stats.print();
            }
        }

        if iterations % 10_000 == 0 {
            let rate = iterations as f64 / start.elapsed().as_secs_f64().max(0.001);
            pb.set_message(format!(
                "Checked {} addresses ({:.0}/s)",
                output::format_number(iterations),
                rate
            ));
        }
    }

    pb.finish();

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
fn run_cuda_search(args: &Args, pattern: &Pattern) -> Result<Vec<output::SearchResult>> {
    use vanity_cuda::CudaSearcher;

    let mut searcher = CudaSearcher::new(args.device)
        .context("Failed to initialize CUDA")?;

    let device_name = searcher.device_name();
    output::print_progress(
        args.prefix.as_deref(),
        args.suffix.as_deref(),
        pattern.difficulty_f64(),
        &device_name,
    );

    let start = Instant::now();
    let mut results = Vec::new();
    let mut total_iterations = 0u64;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    while results.len() < args.count {
        if args.max_iterations > 0 && total_iterations >= args.max_iterations {
            break;
        }

        let batch_results = searcher.search_batch(
            pattern,
            10_000_000, // 10M per batch
        )?;

        for keypair in batch_results {
            results.push(output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1));
        }

        total_iterations += 10_000_000;

        let elapsed = start.elapsed().as_secs_f64();
        let rate = total_iterations as f64 / elapsed.max(0.001);
        pb.set_message(format!(
            "Checked {} addresses ({:.2} M/s)",
            output::format_number(total_iterations),
            rate / 1_000_000.0
        ));
    }

    pb.finish();

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
fn run_opencl_search(args: &Args, pattern: &Pattern) -> Result<Vec<output::SearchResult>> {
    use vanity_opencl::OpenClSearcher;

    let mut searcher = OpenClSearcher::new(args.device)
        .context("Failed to initialize OpenCL")?;

    let device_name = searcher.device_name();
    output::print_progress(
        args.prefix.as_deref(),
        args.suffix.as_deref(),
        pattern.difficulty_f64(),
        &device_name,
    );

    let start = Instant::now();
    let mut results = Vec::new();
    let mut total_iterations = 0u64;

    let pb = ProgressBar::new_spinner();
    pb.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {msg}")
            .unwrap(),
    );

    while results.len() < args.count {
        if args.max_iterations > 0 && total_iterations >= args.max_iterations {
            break;
        }

        let batch_results = searcher.search_batch(
            pattern,
            10_000_000,
        )?;

        for keypair in batch_results {
            results.push(output::SearchResult::new(keypair, total_iterations + results.len() as u64 + 1));
        }

        total_iterations += 10_000_000;

        let elapsed = start.elapsed().as_secs_f64();
        let rate = total_iterations as f64 / elapsed.max(0.001);
        pb.set_message(format!(
            "Checked {} addresses ({:.2} M/s)",
            output::format_number(total_iterations),
            rate / 1_000_000.0
        ));
    }

    pb.finish();

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
