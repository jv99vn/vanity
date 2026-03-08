# Vanity - GPU-Accelerated Ethereum Address Generator

A high-performance Ethereum vanity address generator using GPU acceleration (CUDA/OpenCL) with CPU fallback.

## Features

- **GPU Acceleration**: Uses CUDA (NVIDIA) or OpenCL (AMD/Apple Silicon) for maximum speed
- **Flexible Patterns**: Search by prefix, suffix, or both
- **High Performance**: Up to 100M+ addresses/second on modern GPUs
- **CPU Fallback**: Works without GPU support

## Installation

### Prerequisites

- Rust 1.70+
- For GPU support:
  - CUDA Toolkit 11+ (NVIDIA)
  - Or OpenCL runtime (AMD/Apple)

### Build

```bash
# CPU only (fastest build)
cargo build --release --no-default-features --features ""

# With CUDA support
cargo build --release --features cuda

# With OpenCL support
cargo build --release --features opencl
```

## Usage

```bash
# Find address starting with "dead"
vanity --prefix dead

# Find address ending with "beef"
vanity --suffix beef

# Find address with both prefix and suffix
vanity --prefix cafe --suffix face

# Find multiple addresses
vanity --prefix dead --count 5

# Use specific GPU device
vanity --prefix dead --device 0

# Use OpenCL backend
vanity --prefix dead --backend opencl

# CPU only mode
vanity --prefix dead --backend cpu

# Show statistics
vanity --prefix dead --stats
```

## Performance

Expected performance on modern GPUs:

| GPU | Speed |
|-----|-------|
| RTX 3080 | ~50M addrs/sec |
| RTX 4090 | ~100M addrs/sec |

Difficulty examples:

| Pattern | Difficulty | Time (RTX 4090) |
|---------|------------|-----------------|
| `dead` | 65,536 | ~0.001s |
| `deadbeef` | 4.3B | ~43s |
| `deadbeefcafe` | 280T | ~82 days |

## Project Structure

```
vanity/
├── crates/
│   ├── vanity-cli/      # CLI interface
│   ├── vanity-core/     # Core library (pattern parsing, CPU crypto)
│   ├── vanity-cuda/     # CUDA kernels and bindings
│   └── vanity-opencl/   # OpenCL kernels and bindings
```

## Algorithm

The generator uses the following algorithm:

1. Generate random 32-byte private key
2. Compute public key using secp256k1 ECDSA
3. Hash public key with Keccak-256
4. Take last 20 bytes as Ethereum address
5. Check if address matches pattern

GPU optimization uses incremental point addition:
- Instead of computing `k*G` from scratch each time
- Compute `P, P+G, P+2G, P+3G, ...` which is O(1) vs O(log n)

## Security Note

This tool is designed for generating vanity addresses for aesthetic purposes only.

**Do NOT use these addresses for:**
- Storing significant value
- Production applications
- Any security-sensitive context

The private key generation uses simple random numbers without additional entropy mixing or security hardening. For production wallets, use established wallet software.

## License

MIT
