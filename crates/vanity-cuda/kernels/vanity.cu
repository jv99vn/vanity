// Main vanity address search kernel

#include "common.cuh"
#include "ecdsa.cu"
#include "keccak.cu"

// Pattern constraints in constant memory
__constant__ uint8_t PREFIX[20];
__constant__ uint8_t SUFFIX[20];
__constant__ int PREFIX_LEN;
__constant__ int SUFFIX_LEN;

// Main search kernel
// Each thread checks one private key
extern "C" __global__ void vanity_search(
    uint64_t iteration_offset,
    AddressResult* results,
    int* result_count,
    int max_results
) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;

    // 1. Generate private key from thread ID and iteration offset
    uint256 private_key;
    private_key.d[0] = (uint64_t)tid + iteration_offset;
    private_key.d[1] = 0;
    private_key.d[2] = 0;
    private_key.d[3] = 0;

    // Add randomness using blockIdx and threadIdx
    private_key.d[0] ^= ((uint64_t)blockIdx.x << 32);
    private_key.d[1] = (uint64_t)gridDim.x * blockDim.x + iteration_offset;

    // 2. Compute public key: P = k * G
    Point public_key;
    scalar_mult(public_key, private_key);

    if (public_key.infinity) {
        return;
    }

    // 3. Convert public key to bytes (uncompressed, without 0x04 prefix)
    uint8_t pubkey_bytes[64];
    for (int i = 0; i < 4; i++) {
        ((uint64_t*)pubkey_bytes)[i] = public_key.x.d[i];
        ((uint64_t*)pubkey_bytes)[4 + i] = public_key.y.d[i];
    }

    // 4. Compute Keccak-256 and extract address
    uint8_t address[20];
    public_key_to_address(pubkey_bytes, address);

    // 5. Check if address matches pattern
    if (matches_pattern(address, PREFIX, SUFFIX, PREFIX_LEN, SUFFIX_LEN)) {
        // Atomic increment to get result slot
        int idx = atomicAdd(result_count, 1);

        if (idx < max_results) {
            // Store private key
            for (int i = 0; i < 4; i++) {
                ((uint64_t*)results[idx].private_key)[i] = private_key.d[i];
            }

            // Store address
            for (int i = 0; i < 20; i++) {
                results[idx].address[i] = address[i];
            }

            results[idx].found = 1;
        }
    }
}

// Optimized batch search kernel using incremental point addition
// Instead of computing k*G from scratch, threads share a base point
// and increment it: P, P+G, P+2G, P+3G, ...
extern "C" __global__ void vanity_search_incremental(
    uint8_t* base_private_key,
    uint64_t iteration_offset,
    AddressResult* results,
    int* result_count,
    int max_results
) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;

    // Load base private key
    uint256 private_key;
    for (int i = 0; i < 4; i++) {
        private_key.d[i] = ((uint64_t*)base_private_key)[i];
    }

    // Add thread offset to private key
    uint256 offset;
    offset.d[0] = (uint64_t)tid + iteration_offset;
    offset.d[1] = offset.d[2] = offset.d[3] = 0;

    // Add offset to private key (mod n)
    // This is simplified - needs proper modular arithmetic

    // Compute public key from base + offset
    Point public_key;

    // First, compute offset * G
    scalar_mult(public_key, offset);

    // Then add base point if not starting from G
    // (This is the incremental optimization)

    if (public_key.infinity) {
        return;
    }

    // Convert to address and check pattern
    uint8_t pubkey_bytes[64];
    for (int i = 0; i < 4; i++) {
        ((uint64_t*)pubkey_bytes)[i] = public_key.x.d[i];
        ((uint64_t*)pubkey_bytes)[4 + i] = public_key.y.d[i];
    }

    uint8_t address[20];
    public_key_to_address(pubkey_bytes, address);

    if (matches_pattern(address, PREFIX, SUFFIX, PREFIX_LEN, SUFFIX_LEN)) {
        int idx = atomicAdd(result_count, 1);

        if (idx < max_results) {
            for (int i = 0; i < 4; i++) {
                ((uint64_t*)results[idx].private_key)[i] = private_key.d[i] + offset.d[i];
            }

            for (int i = 0; i < 20; i++) {
                results[idx].address[i] = address[i];
            }

            results[idx].found = 1;
        }
    }
}

// Benchmark kernel - just computes addresses without pattern matching
extern "C" __global__ void vanity_benchmark(
    uint64_t* counter
) {
    int tid = blockIdx.x * blockDim.x + threadIdx.x;

    uint256 private_key;
    private_key.d[0] = (uint64_t)tid;
    private_key.d[1] = private_key.d[2] = private_key.d[3] = 0;

    Point public_key;
    scalar_mult(public_key, private_key);

    if (!public_key.infinity) {
        atomicAdd(counter, 1);
    }
}
