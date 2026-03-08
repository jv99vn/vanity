//! OpenCL-based vanity address generation (fallback for AMD/Apple Silicon)

use anyhow::{Context, Result};
use thiserror::Error;
use vanity_core::{Pattern, crypto::KeyPair};

#[derive(Error, Debug)]
pub enum OpenClError {
    #[error("OpenCL not available: {0}")]
    NotAvailable(String),

    #[error("Device not found: {0}")]
    DeviceNotFound(usize),

    #[error("Kernel compilation failed: {0}")]
    KernelCompilationFailed(String),

    #[error("Memory allocation failed: {0}")]
    MemoryAllocationFailed(String),
}

/// OpenCL-based vanity address searcher
pub struct OpenClSearcher {
    device_id: usize,
    device_name: String,
}

impl OpenClSearcher {
    /// Create a new OpenCL searcher for the specified device
    pub fn new(device_id: usize) -> Result<Self> {
        let device_name = format!("OpenCL Device {}", device_id);

        Ok(Self {
            device_id,
            device_name,
        })
    }

    /// Get the name of the GPU device
    pub fn device_name(&self) -> String {
        self.device_name.clone()
    }

    /// Search for addresses in a batch
    pub fn search_batch(&mut self, pattern: &Pattern, batch_size: u64) -> Result<Vec<KeyPair>> {
        // CPU fallback - full OpenCL implementation would load and run kernel
        let mut results = Vec::new();

        for _ in 0..batch_size {
            let keypair = vanity_core::crypto::generate_keypair();

            if pattern.matches(&keypair.address.to_hex()) {
                results.push(keypair);
            }
        }

        Ok(results)
    }
}

/// Check if OpenCL is available on this system
pub fn is_opencl_available() -> bool {
    // Check for OpenCL runtime
    cfg!(target_os = "macos") || std::path::Path::new("/usr/lib/libOpenCL.so").exists()
}

/// List available OpenCL devices
pub fn list_devices() -> Result<Vec<String>> {
    Ok(vec!["OpenCL Device 0 (Placeholder)".to_string()])
}
