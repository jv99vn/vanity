//! CLI argument parsing using clap.

use clap::Parser;

#[derive(Parser, Debug)]
#[command(
    name = "vanity",
    author,
    version,
    about = "GPU-accelerated Ethereum vanity address generator",
    long_about = "Generate Ethereum addresses with custom prefix and/or suffix.
Uses GPU acceleration for maximum performance.

Examples:
  vanity --prefix dead                  # Address starting with 'dead'
  vanity --suffix beef                  # Address ending with 'beef'
  vanity --prefix cafe --suffix face    # Both prefix and suffix
  vanity --prefix dead --count 5        # Find 5 matching addresses
  vanity --prefix dead --device 0       # Use specific GPU
  vanity --prefix dead --backend metal  # Use Metal (Apple Silicon)
  vanity --prefix dead --backend cpu    # Use CPU only"
)]
pub struct Args {
    /// Hex prefix for the address (e.g., "dead")
    #[arg(short, long)]
    pub prefix: Option<String>,

    /// Hex suffix for the address (e.g., "beef")
    #[arg(short, long)]
    pub suffix: Option<String>,

    /// Number of addresses to find
    #[arg(short = 'n', long, default_value = "1", alias = "count")]
    pub count: usize,

    /// GPU device index to use
    #[arg(short, long, default_value = "0")]
    pub device: usize,

    /// Backend to use: metal (Apple Silicon), cuda (NVIDIA), opencl, or cpu
    #[arg(short, long, default_value = "metal", value_parser = ["metal", "cuda", "opencl", "cpu"])]
    pub backend: String,

    /// Show performance statistics
    #[arg(long)]
    pub stats: bool,

    /// Verbose output
    #[arg(short, long)]
    pub verbose: bool,
}

impl Args {
    pub fn parse_args() -> Self {
        Self::parse()
    }
}
