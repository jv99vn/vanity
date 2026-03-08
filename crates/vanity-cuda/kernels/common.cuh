// Common CUDA header for vanity address generation
// Shared definitions and utilities

#ifndef VANITY_COMMON_CUH
#define VANITY_COMMON_CUH

#include <cstdint>
#include <cuda_runtime.h>

// Constants
#define ADDRESS_SIZE 20
#define PRIVATE_KEY_SIZE 32
#define PUBLIC_KEY_SIZE 65
#define HASH_SIZE 32

// 256-bit unsigned integer (for secp256k1)
struct uint256 {
    uint64_t d[4];  // Little-endian: d[0] is least significant

    __device__ __host__ uint256() {
        d[0] = d[1] = d[2] = d[3] = 0;
    }

    __device__ __host__ uint256(uint64_t v) {
        d[0] = v;
        d[1] = d[2] = d[3] = 0;
    }

    __device__ bool is_zero() const {
        return (d[0] | d[1] | d[2] | d[3]) == 0;
    }

    __device__ void set_zero() {
        d[0] = d[1] = d[2] = d[3] = 0;
    }
};

// Affine point on secp256k1
struct Point {
    uint256 x;
    uint256 y;
    bool infinity;

    __device__ Point() : infinity(true) {}
};

// Result structure for found addresses
struct AddressResult {
    uint8_t private_key[PRIVATE_KEY_SIZE];
    uint8_t address[ADDRESS_SIZE];
    uint32_t found;
};

// Utility functions
__device__ __host__ inline uint32_t bswap32(uint32_t x) {
    return ((x >> 24) & 0xff) |
           ((x >> 8) & 0xff00) |
           ((x << 8) & 0xff0000) |
           ((x << 24) & 0xff000000);
}

// Convert hex char to value
__device__ __host__ inline int hex_to_val(char c) {
    if (c >= '0' && c <= '9') return c - '0';
    if (c >= 'a' && c <= 'f') return c - 'a' + 10;
    if (c >= 'A' && c <= 'F') return c - 'A' + 10;
    return -1;
}

// Check if address byte matches pattern
__device__ inline bool matches_pattern(
    const uint8_t* address,
    const uint8_t* prefix,
    const uint8_t* suffix,
    int prefix_len,
    int suffix_len
) {
    // Check prefix (first bytes)
    for (int i = 0; i < prefix_len / 2; i++) {
        if (address[i] != prefix[i]) return false;
    }
    // Handle odd-length prefix
    if (prefix_len % 2 == 1) {
        uint8_t expected = prefix[prefix_len / 2] >> 4;
        uint8_t actual = address[prefix_len / 2] >> 4;
        if (actual != expected) return false;
    }

    // Check suffix (last bytes)
    int suffix_start = 20 - (suffix_len + 1) / 2;
    for (int i = 0; i < suffix_len / 2; i++) {
        if (address[suffix_start + i] != suffix[i]) return false;
    }
    // Handle odd-length suffix
    if (suffix_len % 2 == 1) {
        uint8_t expected = suffix[suffix_len / 2] & 0x0F;
        uint8_t actual = address[suffix_start + suffix_len / 2] & 0x0F;
        if (actual != expected) return false;
    }

    return true;
}

// CUDA error checking macro
#define CUDA_CHECK(call) \
    do { \
        cudaError_t err = call; \
        if (err != cudaSuccess) { \
            printf("CUDA error at %s:%d: %s\n", __FILE__, __LINE__, \
                   cudaGetErrorString(err)); \
        } \
    } while (0)

#endif // VANITY_COMMON_CUH
