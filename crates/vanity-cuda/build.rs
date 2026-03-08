//! Build script for compiling CUDA kernels with NVCC

use std::env;
use std::path::PathBuf;
use std::process::Command;

fn main() {
    // Check if CUDA toolkit is available
    let cuda_path = env::var("CUDA_PATH")
        .or_else(|_| env::var("CUDA_HOME"))
        .unwrap_or_else(|_| "/usr/local/cuda".to_string());

    let nvcc_path = PathBuf::from(&cuda_path).join("bin/nvcc");

    if !nvcc_path.exists() {
        println!("cargo:warning=CUDA compiler (nvcc) not found at {:?}", nvcc_path);
        println!("cargo:warning=CUDA kernels will not be compiled");
        println!("cargo:rerun-if-env-changed=CUDA_PATH");
        return;
    }

    // Kernel source files
    let kernels = ["vanity.cu"];

    let out_dir = env::var("OUT_DIR").unwrap();
    let kernel_dir = PathBuf::from("kernels");

    for kernel in &kernels {
        let src = kernel_dir.join(kernel);
        let dst = PathBuf::from(&out_dir).join(kernel.replace(".cu", ".ptx"));

        println!("cargo:rerun-if-changed={}", src.display());
        println!("cargo:rerun-if-changed=kernels/common.cuh");
        println!("cargo:rerun-if-changed=kernels/ecdsa.cu");
        println!("cargo:rerun-if-changed=kernels/keccak.cu");

        // Compile kernel to PTX
        let output = Command::new(&nvcc_path)
            .arg("-ptx")
            .arg("-arch=sm_75") // RTX 20xx/30xx, adjust as needed
            .arg("-O3")
            .arg("-I")
            .arg("kernels")
            .arg("-o")
            .arg(&dst)
            .arg(&src)
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    println!("cargo:warning=Failed to compile {:?}:", kernel);
                    println!("cargo:warning={}", String::from_utf8_lossy(&output.stderr));
                }
            }
            Err(e) => {
                println!("cargo:warning=Failed to run nvcc: {}", e);
            }
        }
    }

    // Tell cargo where to find the compiled PTX files
    println!("cargo:rustc-env=KERNEL_DIR={}", out_dir);
}
