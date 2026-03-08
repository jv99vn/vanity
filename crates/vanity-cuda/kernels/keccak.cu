// Keccak-256 hash implementation for CUDA
// Used to derive Ethereum addresses from public keys

#ifndef KECCAK_CU
#define KECCAK_CU

#include "common.cuh"

// Keccak-256 constants
__constant__ uint64_t KECCAK_RC[24] = {
    0x0000000000000001ULL, 0x0000000000008082ULL,
    0x800000000000808aULL, 0x8000000080008000ULL,
    0x000000000000808bULL, 0x0000000080000001ULL,
    0x8000000080008081ULL, 0x8000000000008009ULL,
    0x000000000000008aULL, 0x0000000000000088ULL,
    0x0000000080008009ULL, 0x000000008000000aULL,
    0x000000008000808bULL, 0x800000000000008bULL,
    0x8000000000008089ULL, 0x8000000000008003ULL,
    0x8000000000008002ULL, 0x8000000000000080ULL,
    0x000000000000800aULL, 0x800000008000000aULL,
    0x8000000080008081ULL, 0x8000000000008080ULL,
    0x0000000080000001ULL, 0x8000000080008008ULL
};

// Rotation offsets
__constant__ int KECCAK_ROT[24] = {
    1, 3, 6, 10, 15, 21, 28, 36, 45, 55, 2, 14,
    27, 41, 56, 8, 25, 43, 62, 18, 39, 61, 20, 44
};

__constant__ int KECCAK_PIL[24] = {
    10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4,
    15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1
};

// Rotate left
__device__ __host__ inline uint64_t rotl64(uint64_t x, int r) {
    return (x << r) | (x >> (64 - r));
}

// Keccak round function
__device__ void keccak_f(uint64_t* state) {
    uint64_t temp, C[5], D;

    for (int round = 0; round < 24; round++) {
        // Theta
        #pragma unroll
        for (int i = 0; i < 5; i++) {
            C[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }

        #pragma unroll
        for (int i = 0; i < 5; i++) {
            D = C[(i + 4) % 5] ^ rotl64(C[(i + 1) % 5], 1);
            #pragma unroll
            for (int j = 0; j < 5; j++) {
                state[i + 5 * j] ^= D;
            }
        }

        // Rho and Pi
        temp = state[1];
        #pragma unroll
        for (int i = 0; i < 24; i++) {
            int j = KECCAK_PIL[i];
            D = state[j];
            state[j] = rotl64(temp, KECCAK_ROT[i]);
            temp = D;
        }

        // Chi
        #pragma unroll
        for (int j = 0; j < 5; j++) {
            #pragma unroll
            for (int i = 0; i < 5; i++) {
                C[i] = state[i + 5 * j];
            }
            #pragma unroll
            for (int i = 0; i < 5; i++) {
                state[i + 5 * j] = C[i] ^ (~C[(i + 1) % 5] & C[(i + 2) % 5]);
            }
        }

        // Iota
        state[0] ^= KECCAK_RC[round];
    }
}

// Keccak-256 hash of 64 bytes (uncompressed public key without prefix)
// Output: 32 bytes
__device__ void keccak256(const uint8_t* input, size_t len, uint8_t* output) {
    uint64_t state[25] = {0};

    // Absorb phase
    size_t offset = 0;
    while (offset + 136 <= len) {
        #pragma unroll
        for (int i = 0; i < 17; i++) {
            state[i] ^= ((uint64_t*)input)[offset / 8 + i];
        }
        keccak_f(state);
        offset += 136;
    }

    // Final block with padding
    uint8_t final_block[136] = {0};
    for (size_t i = 0; i < len - offset; i++) {
        final_block[i] = input[offset + i];
    }
    final_block[len - offset] = 0x01;
    final_block[135] = 0x80;

    for (int i = 0; i < 17; i++) {
        state[i] ^= ((uint64_t*)final_block)[i];
    }
    keccak_f(state);

    // Squeeze phase (we only need 32 bytes)
    for (int i = 0; i < 4; i++) {
        ((uint64_t*)output)[i] = state[i];
    }
}

// Specialized function: Keccak-256 of 64-byte public key
// Returns address (last 20 bytes of hash)
__device__ void public_key_to_address(const uint8_t* public_key, uint8_t* address) {
    uint8_t hash[32];
    keccak256(public_key, 64, hash);

    // Take last 20 bytes
    #pragma unroll
    for (int i = 0; i < 20; i++) {
        address[i] = hash[12 + i];
    }
}

#endif // KECCAK_CU
