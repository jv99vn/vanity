// Metal Compute Shaders for Ethereum Vanity Address Generation
// Optimized for Apple Silicon (M1/M2/M3/M4)

#include <metal_stdlib>
#include <simd/simd.h>

using namespace metal;

// ============================================================================
// Constants
// ============================================================================

// Keccak-256 round constants
constant static constexpr uint64_t KECCAK_RC[24] = {
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

// Rotation offsets for Rho step
constant static constexpr int RHO_OFFSETS[25] = {
    0, 1, 62, 28, 27,
    36, 44, 6, 55, 20,
    3, 10, 43, 25, 39,
    41, 45, 15, 21, 8,
    18, 2, 61, 56, 14
};

// secp256k1 field prime: p = 2^256 - 2^32 - 977
constant static constexpr uint64_t FIELD_P[4] = {
    0xFFFFFFFEFFFFFC2FULL, 0xFFFFFFFFFFFFFFFFULL,
    0xFFFFFFFFFFFFFFFFULL, 0xFFFFFFFFFFFFFFFFULL
};

// secp256k1 curve order
constant static constexpr uint64_t CURVE_N[4] = {
    0xBFD25E8CD0364141ULL, 0xBAAEDCE6AF48A03BULL,
    0xFFFFFFFFFFFFFFFEULL, 0xFFFFFFFFFFFFFFFFULL
};

// Generator point G.x
constant static constexpr uint64_t GX[4] = {
    0x59F2815B16F81798ULL, 0x029BFCDB2DCE28D9ULL,
    0x55A06295CE870B07ULL, 0x79BE667EF9DCBBACULL
};

// Generator point G.y
constant static constexpr uint64_t GY[4] = {
    0x9B10D4B8E2788A8FULL, 0xE1108A8FD17B448AULL,
    0xFBFC0E1108A8FD17ULL, 0x483ADA7726A3C465ULL
};

// 2^256 for field reduction
constant static constexpr uint64_t TWO_256[4] = {
    0x00000001000003D1ULL, 0x0000000000000000ULL,
    0x0000000000000000ULL, 0x0000000000000000ULL
};

// ============================================================================
// Utility Functions
// ============================================================================

// Rotate left 64-bit
DEVICE_INLINE uint64_t rotl64(uint64_t x, int r) {
    return (x << r) | (x >> (64 - r));
}

// XOR two 256-bit numbers
DEVICE_INLINE void xor256(thread uint64_t* result,
                          const thread uint64_t* a,
                          const thread uint64_t* b) {
    result[0] = a[0] ^ b[0];
    result[1] = a[1] ^ b[1];
    result[2] = a[2] ^ b[2];
    result[3] = a[3] ^ b[3];
}

// Add two 256-bit numbers with carry
DEVICE_INLINE uint64_t add256(thread uint64_t* result,
                              const thread uint64_t* a,
                              const thread uint64_t* b) {
    uint64_t carry = 0;

    #pragma unroll
    for (int i = 0; i < 4; i++) {
        uint64_t sum = a[i] + b[i] + carry;
        carry = (sum < a[i]) || (carry && sum == a[i]) ? 1 : 0;
        result[i] = sum;
    }

    return carry;
}

// Subtract two 256-bit numbers
DEVICE_INLINE void sub256(thread uint64_t* result,
                          const thread uint64_t* a,
                          const thread uint64_t* b) {
    uint64_t borrow = 0;

    #pragma unroll
    for (int i = 0; i < 4; i++) {
        uint64_t diff = a[i] - b[i] - borrow;
        borrow = (a[i] < b[i]) || (borrow && a[i] == b[i]) ? 1 : 0;
        result[i] = diff;
    }
}

// Compare two 256-bit numbers: returns 1 if a >= b
DEVICE_INLINE int gte256(const thread uint64_t* a, const thread uint64_t* b) {
    for (int i = 3; i >= 0; i--) {
        if (a[i] > b[i]) return 1;
        if (a[i] < b[i]) return 0;
    }
    return 1; // Equal
}

// ============================================================================
// Keccak-256 Implementation
// ============================================================================

DEVICE_INLINE void keccak_f1600(thread uint64_t* state) {
    uint64_t C[5], D[5], temp;

    for (int round = 0; round < 24; round++) {
        // Theta - calculate parity of columns
        #pragma unroll
        for (int i = 0; i < 5; i++) {
            C[i] = state[i] ^ state[i + 5] ^ state[i + 10] ^ state[i + 15] ^ state[i + 20];
        }

        #pragma unroll
        for (int i = 0; i < 5; i++) {
            D[i] = C[(i + 4) % 5] ^ rotl64(C[(i + 1) % 5], 1);
        }

        #pragma unroll
        for (int i = 0; i < 5; i++) {
            #pragma unroll
            for (int j = 0; j < 5; j++) {
                state[i + 5 * j] ^= D[i];
            }
        }

        // Rho and Pi
        temp = state[1];
        int pi_lane[24] = {
            10, 7, 11, 17, 18, 3, 5, 16, 8, 21, 24, 4,
            15, 23, 19, 13, 12, 2, 20, 14, 22, 9, 6, 1
        };

        #pragma unroll
        for (int i = 0; i < 24; i++) {
            int lane = pi_lane[i];
            uint64_t t = state[lane];
            state[lane] = rotl64(temp, RHO_OFFSETS[lane]);
            temp = t;
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

// Keccak-256 hash of 64-byte input (uncompressed public key without 04 prefix)
DEVICE_INLINE void keccak256(const thread uint8_t* input, thread uint8_t* output) {
    uint64_t state[25] = {0};

    // Absorb phase - 64 bytes into first 8 lanes
    const uint64_t* input64 = (const thread uint64_t*)input;
    #pragma unroll
    for (int i = 0; i < 8; i++) {
        state[i] = input64[i];
    }

    // Padding
    state[8] = 0x01;  // 0x01 after message
    state[16] = 0x8000000000000000ULL;  // 0x80 at end

    // Permutation
    keccak_f1600(state);

    // Squeeze - output 32 bytes (4 lanes)
    uint64_t* output64 = (thread uint64_t*)output;
    #pragma unroll
    for (int i = 0; i < 4; i++) {
        output64[i] = state[i];
    }
}

// ============================================================================
// secp256k1 Field Arithmetic
// ============================================================================

// Modular addition: result = (a + b) mod p
DEVICE_INLINE void field_add(thread uint64_t* result,
                              const thread uint64_t* a,
                              const thread uint64_t* b) {
    uint64_t carry = add256(result, a, b);

    // If result >= p or overflow, subtract p
    if (carry || gte256(result, FIELD_P)) {
        sub256(result, result, FIELD_P);
    }
}

// Modular subtraction: result = (a - b) mod p
DEVICE_INLINE void field_sub(thread uint64_t* result,
                              const thread uint64_t* a,
                              const thread uint64_t* b) {
    sub256(result, a, b);

    // If underflow, add p back
    if (a[3] < b[3] || (a[3] == b[3] && a[2] < b[2]) ||
        (a[3] == b[3] && a[2] == b[2] && a[1] < b[1]) ||
        (a[3] == b[3] && a[2] == b[2] && a[1] == b[1] && a[0] < b[0])) {
        uint64_t temp[4];
        add256(temp, result, FIELD_P);
        result[0] = temp[0];
        result[1] = temp[1];
        result[2] = temp[2];
        result[3] = temp[3];
    }
}

// 256-bit multiplication (result is 512-bit in [0..7])
DEVICE_INLINE void mul256_full(thread uint64_t* result,
                               const thread uint64_t* a,
                               const thread uint64_t* b) {
    result[0] = result[1] = result[2] = result[3] = 0;
    result[4] = result[5] = result[6] = result[7] = 0;

    for (int i = 0; i < 4; i++) {
        uint64_t carry = 0;
        for (int j = 0; j < 4; j++) {
            ulong prod = (ulong)a[i] * (ulong)b[j] + result[i + j] + carry;
            result[i + j] = (uint64_t)prod;
            carry = (uint64_t)(prod >> 64);
        }
        result[i + 4] = carry;
    }
}

// Fast modular reduction for secp256k1 (p = 2^256 - 2^32 - 977)
// Based on the fact that 2^256 ≡ 2^32 + 977 (mod p)
DEVICE_INLINE void field_reduce(thread uint64_t* result, thread uint64_t* prod) {
    // prod[0..7] is 512-bit product
    // We need to reduce the upper 256 bits

    uint64_t t[4];  // Temp for upper part
    t[0] = prod[4];
    t[1] = prod[5];
    t[2] = prod[6];
    t[3] = prod[7];

    // First reduction: multiply upper by (2^32 + 977) and add to lower
    // This is a simplified version - full implementation needs careful handling
    result[0] = prod[0];
    result[1] = prod[1];
    result[2] = prod[2];
    result[3] = prod[3];

    // Add t * 2^32 to result
    uint64_t temp[4];
    temp[0] = t[0] << 32;
    temp[1] = (t[0] >> 32) | (t[1] << 32);
    temp[2] = (t[1] >> 32) | (t[2] << 32);
    temp[3] = (t[2] >> 32) | (t[3] << 32);

    uint64_t carry = add256(result, result, temp);

    // Add t * 977
    temp[0] = t[0] * 977;
    temp[1] = t[1] * 977 + (t[0] >> 32) * 977;
    temp[2] = t[2] * 977 + (t[1] >> 32) * 977;
    temp[3] = t[3] * 977 + (t[2] >> 32) * 977;

    carry += add256(result, result, temp);

    // Final reduction if needed
    while (carry || gte256(result, FIELD_P)) {
        sub256(result, result, FIELD_P);
        carry = 0;
    }
}

// Modular multiplication
DEVICE_INLINE void field_mul(thread uint64_t* result,
                              const thread uint64_t* a,
                              const thread uint64_t* b) {
    uint64_t prod[8];
    mul256_full(prod, a, b);
    field_reduce(result, prod);
}

// Modular squaring (slightly faster than mul)
DEVICE_INLINE void field_sqr(thread uint64_t* result, const thread uint64_t* a) {
    field_mul(result, a, a);
}

// ============================================================================
// secp256k1 Point Operations (Jacobian Coordinates)
// ============================================================================

// Point in Jacobian coordinates: (X, Y, Z) where affine = (X/Z^2, Y/Z^3)
struct PointJac {
    uint64_t x[4];
    uint64_t y[4];
    uint64_t z[4];
    bool infinity;
};

// Point in affine coordinates
struct PointAff {
    uint64_t x[4];
    uint64_t y[4];
    bool infinity;
};

// Initialize point to infinity
DEVICE_INLINE void point_set_infinity(thread PointJac& p) {
    p.infinity = true;
    p.x[0] = p.x[1] = p.x[2] = p.x[3] = 0;
    p.y[0] = p.y[1] = p.y[2] = p.y[3] = 0;
    p.z[0] = p.z[1] = p.z[2] = p.z[3] = 0;
}

// Set point to generator G
DEVICE_INLINE void point_set_g(thread PointJac& p) {
    p.x[0] = GX[0]; p.x[1] = GX[1]; p.x[2] = GX[2]; p.x[3] = GX[3];
    p.y[0] = GY[0]; p.y[1] = GY[1]; p.y[2] = GY[2]; p.y[3] = GY[3];
    p.z[0] = 1; p.z[1] = 0; p.z[2] = 0; p.z[3] = 0;
    p.infinity = false;
}

// Point doubling in Jacobian coordinates
DEVICE_INLINE void point_double(thread PointJac& result, const thread PointJac& p) {
    if (p.infinity) {
        result.infinity = true;
        return;
    }

    // delta = Z^2
    uint64_t delta[4];
    field_sqr(delta, p.z);

    // gamma = Y^2
    uint64_t gamma[4];
    field_sqr(gamma, p.y);

    // beta = X * gamma
    uint64_t beta[4];
    field_mul(beta, p.x, gamma);

    // alpha = 3 * (X - delta) * (X + delta)
    uint64_t t1[4], t2[4], t3[4], alpha[4];
    field_sub(t1, p.x, delta);  // X - delta
    field_add(t2, p.x, delta);  // X + delta
    field_mul(t3, t1, t2);      // (X - delta) * (X + delta)
    field_add(alpha, t3, t3);   // 2 * ...
    field_add(alpha, alpha, t3); // 3 * ...

    // X' = alpha^2 - 8*beta
    uint64_t x_new[4], temp[4];
    field_sqr(x_new, alpha);
    field_add(temp, beta, beta); // 2*beta
    field_add(temp, temp, temp); // 4*beta
    field_add(temp, temp, temp); // 8*beta
    field_sub(x_new, x_new, temp);

    // Z' = (Y + Z)^2 - gamma - delta
    uint64_t z_new[4];
    field_add(z_new, p.y, p.z);
    field_sqr(z_new, z_new);
    field_sub(z_new, z_new, gamma);
    field_sub(z_new, z_new, delta);

    // Y' = alpha * (4*beta - X') - 8*gamma^2
    uint64_t y_new[4];
    field_add(temp, beta, beta); // 2*beta
    field_add(temp, temp, temp); // 4*beta
    field_sub(temp, temp, x_new); // 4*beta - X'
    field_mul(y_new, alpha, temp); // alpha * (4*beta - X')
    field_sqr(temp, gamma);      // gamma^2
    field_add(temp, temp, temp); // 2*gamma^2
    field_add(temp, temp, temp); // 4*gamma^2
    field_add(temp, temp, temp); // 8*gamma^2
    field_sub(y_new, y_new, temp);

    // Store results
    result.x[0] = x_new[0]; result.x[1] = x_new[1];
    result.x[2] = x_new[2]; result.x[3] = x_new[3];
    result.y[0] = y_new[0]; result.y[1] = y_new[1];
    result.y[2] = y_new[2]; result.y[3] = y_new[3];
    result.z[0] = z_new[0]; result.z[1] = z_new[1];
    result.z[2] = z_new[2]; result.z[3] = z_new[3];
    result.infinity = false;
}

// Point addition: result = p1 + p2 (mixed addition, p2 in Jacobian)
DEVICE_INLINE void point_add(thread PointJac& result,
                              const thread PointJac& p1,
                              const thread PointJac& p2) {
    if (p1.infinity) {
        result = p2;
        return;
    }
    if (p2.infinity) {
        result = p1;
        return;
    }

    // Z1^2, Z1^3
    uint64_t z1_sq[4], z1_cu[4];
    field_sqr(z1_sq, p1.z);
    field_mul(z1_cu, z1_sq, p1.z);

    // Z2^2, Z2^3
    uint64_t z2_sq[4], z2_cu[4];
    field_sqr(z2_sq, p2.z);
    field_mul(z2_cu, z2_sq, p2.z);

    // U1 = X1 * Z2^2
    uint64_t u1[4];
    field_mul(u1, p1.x, z2_sq);

    // U2 = X2 * Z1^2
    uint64_t u2[4];
    field_mul(u2, p2.x, z1_sq);

    // S1 = Y1 * Z2^3
    uint64_t s1[4];
    field_mul(s1, p1.y, z2_cu);

    // S2 = Y2 * Z1^3
    uint64_t s2[4];
    field_mul(s2, p2.y, z1_cu);

    // H = U2 - U1
    uint64_t h[4];
    field_sub(h, u2, u1);

    // R = S2 - S1
    uint64_t r[4];
    field_sub(r, s2, s1);

    // Check if points are equal
    uint64_t zero[4] = {0, 0, 0, 0};
    bool h_zero = (h[0] == 0 && h[1] == 0 && h[2] == 0 && h[3] == 0);
    bool r_zero = (r[0] == 0 && r[1] == 0 && r[2] == 0 && r[3] == 0);

    if (h_zero && r_zero) {
        // Points are equal, double
        point_double(result, p1);
        return;
    }

    if (h_zero) {
        // Points are negations
        result.infinity = true;
        return;
    }

    // H^2
    uint64_t h_sq[4];
    field_sqr(h_sq, h);

    // H^3
    uint64_t h_cu[4];
    field_mul(h_cu, h_sq, h);

    // X3 = R^2 - H^3 - 2*U1*H^2
    uint64_t x3[4], temp[4];
    field_sqr(x3, r);
    field_sub(x3, x3, h_cu);
    field_mul(temp, u1, h_sq);
    field_add(temp, temp, temp); // 2*U1*H^2
    field_sub(x3, x3, temp);

    // Y3 = R * (U1*H^2 - X3) - S1*H^3
    uint64_t y3[4];
    field_mul(temp, u1, h_sq);
    field_sub(temp, temp, x3);
    field_mul(y3, r, temp);
    field_mul(temp, s1, h_cu);
    field_sub(y3, y3, temp);

    // Z3 = H * Z1 * Z2
    uint64_t z3[4];
    field_mul(z3, h, p1.z);
    field_mul(z3, z3, p2.z);

    result.x[0] = x3[0]; result.x[1] = x3[1];
    result.x[2] = x3[2]; result.x[3] = x3[3];
    result.y[0] = y3[0]; result.y[1] = y3[1];
    result.y[2] = y3[2]; result.y[3] = y3[3];
    result.z[0] = z3[0]; result.z[1] = z3[1];
    result.z[2] = z3[2]; result.z[3] = z3[3];
    result.infinity = false;
}

// Scalar multiplication using double-and-add
DEVICE_INLINE void scalar_mult(thread PointJac& result,
                                const thread uint64_t* scalar,
                                const thread PointJac& base) {
    result.infinity = true;

    PointJac temp = base;

    for (int i = 0; i < 256; i++) {
        int word_idx = i / 64;
        int bit_idx = i % 64;

        // Double result
        point_double(result, result);

        // If bit is set, add temp
        if ((scalar[word_idx] >> bit_idx) & 1) {
            point_add(result, result, temp);
        }
    }
}

// Add G to point (optimized)
DEVICE_INLINE void add_g(thread PointJac& p) {
    PointJac g;
    point_set_g(g);
    point_add(p, p, g);
}

// ============================================================================
// Random Number Generation (Xorshift128+)
// ============================================================================

struct RngState {
    uint64_t s0;
    uint64_t s1;
};

DEVICE_INLINE uint64_t xorshift128plus(thread RngState& state) {
    uint64_t s1 = state.s0;
    uint64_t s0 = state.s1;
    uint64_t result = s0 + s1;
    state.s0 = s0;
    s1 ^= s1 << 23;
    state.s1 = s1 ^ s0 ^ (s1 >> 17) ^ (s0 >> 26);
    return result;
}

DEVICE_INLINE void rng_init(thread RngState& state, uint64_t seed1, uint64_t seed2) {
    state.s0 = seed1 ^ 0x9E3779B97F4A7C15ULL;
    state.s1 = seed2 ^ 0xBF58476D1CE4E5B9ULL;
    // Warm up
    xorshift128plus(state);
    xorshift128plus(state);
    xorshift128plus(state);
}

// ============================================================================
// Pattern Matching
// ============================================================================

// Check if address matches prefix pattern
DEVICE_INLINE bool matches_prefix(const thread uint8_t* address,
                                   const device uint8_t* pattern,
                                   int pattern_len) {
    // Each byte of address = 2 hex chars
    int full_bytes = pattern_len / 2;
    int remaining_nibbles = pattern_len % 2;

    // Check full bytes
    for (int i = 0; i < full_bytes; i++) {
        uint8_t expected = pattern[i];
        uint8_t actual = address[i];
        if (expected != actual) return false;
    }

    // Check remaining nibbles (high nibble only)
    if (remaining_nibbles > 0) {
        uint8_t expected_high = pattern[full_bytes] >> 4;
        uint8_t actual_high = address[full_bytes] >> 4;
        if (expected_high != actual_high) return false;
    }

    return true;
}

// Check if address matches suffix pattern
DEVICE_INLINE bool matches_suffix(const thread uint8_t* address,
                                   const device uint8_t* pattern,
                                   int pattern_len) {
    // Address is 20 bytes, suffix starts from end
    int full_bytes = pattern_len / 2;
    int remaining_nibbles = pattern_len % 2;

    int addr_start = 20 - full_bytes;

    // Check remaining nibbles (low nibble only)
    if (remaining_nibbles > 0) {
        uint8_t expected_low = pattern[0] & 0x0F;
        uint8_t actual_low = address[addr_start - 1] & 0x0F;
        if (expected_low != actual_low) return false;
    }

    // Check full bytes from the end
    int pattern_start = remaining_nibbles;
    for (int i = 0; i < full_bytes; i++) {
        uint8_t expected = pattern[pattern_start + i];
        uint8_t actual = address[addr_start + i];
        if (expected != actual) return false;
    }

    return true;
}

// ============================================================================
// Main Kernel
// ============================================================================

kernel void vanity_search(
    device const uint8_t* prefix_pattern [[buffer(0)]],
    device const uint8_t* suffix_pattern [[buffer(1)]],
    constant int& prefix_len [[buffer(2)]],
    constant int& suffix_len [[buffer(3)]],
    device uint8_t* results_private_keys [[buffer(4)]],
    device uint8_t* results_addresses [[buffer(5)]],
    device atomic_uint& result_count [[buffer(6)]],
    constant uint& max_results [[buffer(7)]],
    constant ulong& batch_offset [[buffer(8)]],
    constant ulong& seed [[buffer(9)]],
    uint tid [[thread_position_in_grid]],
    uint grid_size [[threads_per_grid]])
{
    // Initialize RNG
    RngState rng;
    rng_init(rng, seed + tid, batch_offset + tid);

    // Generate random private key
    uint64_t private_key[4];
    private_key[0] = xorshift128plus(rng);
    private_key[1] = xorshift128plus(rng);
    private_key[2] = xorshift128plus(rng);
    private_key[3] = xorshift128plus(rng);

    // Reduce mod n (curve order)
    while (gte256(private_key, CURVE_N)) {
        sub256(private_key, private_key, CURVE_N);
    }

    // Compute P = k * G
    PointJac base;
    point_set_g(base);

    PointJac pubkey;
    scalar_mult(pubkey, private_key, base);

    if (pubkey.infinity) return;

    // Convert to affine coordinates
    // Z^-1
    uint64_t z_inv[4];
    // For simplicity, output in Jacobian (conversion would need field inverse)

    // Output: X and Y as 64 bytes (will be hashed to get address)
    uint8_t pubkey_bytes[64];

    // For now, just use X and Y directly (simplified - needs proper Z^2, Z^3 division)
    // In production, implement modular inverse
    #pragma unroll
    for (int i = 0; i < 4; i++) {
        pubkey_bytes[i * 8 + 0] = (pubkey.x[i] >> 0) & 0xFF;
        pubkey_bytes[i * 8 + 1] = (pubkey.x[i] >> 8) & 0xFF;
        pubkey_bytes[i * 8 + 2] = (pubkey.x[i] >> 16) & 0xFF;
        pubkey_bytes[i * 8 + 3] = (pubkey.x[i] >> 24) & 0xFF;
        pubkey_bytes[i * 8 + 4] = (pubkey.x[i] >> 32) & 0xFF;
        pubkey_bytes[i * 8 + 5] = (pubkey.x[i] >> 40) & 0xFF;
        pubkey_bytes[i * 8 + 6] = (pubkey.x[i] >> 48) & 0xFF;
        pubkey_bytes[i * 8 + 7] = (pubkey.x[i] >> 56) & 0xFF;
    }
    #pragma unroll
    for (int i = 0; i < 4; i++) {
        pubkey_bytes[32 + i * 8 + 0] = (pubkey.y[i] >> 0) & 0xFF;
        pubkey_bytes[32 + i * 8 + 1] = (pubkey.y[i] >> 8) & 0xFF;
        pubkey_bytes[32 + i * 8 + 2] = (pubkey.y[i] >> 16) & 0xFF;
        pubkey_bytes[32 + i * 8 + 3] = (pubkey.y[i] >> 24) & 0xFF;
        pubkey_bytes[32 + i * 8 + 4] = (pubkey.y[i] >> 32) & 0xFF;
        pubkey_bytes[32 + i * 8 + 5] = (pubkey.y[i] >> 40) & 0xFF;
        pubkey_bytes[32 + i * 8 + 6] = (pubkey.y[i] >> 48) & 0xFF;
        pubkey_bytes[32 + i * 8 + 7] = (pubkey.y[i] >> 56) & 0xFF;
    }

    // Compute Keccak-256 hash
    uint8_t hash[32];
    keccak256(pubkey_bytes, hash);

    // Extract Ethereum address (last 20 bytes of hash)
    uint8_t address[20];
    #pragma unroll
    for (int i = 0; i < 20; i++) {
        address[i] = hash[12 + i];
    }

    // Check pattern match
    bool match = true;

    if (prefix_len > 0) {
        match = match && matches_prefix(address, prefix_pattern, prefix_len);
    }

    if (suffix_len > 0) {
        match = match && matches_suffix(address, suffix_pattern, suffix_len);
    }

    if (match) {
        // Atomically get result slot
        uint slot = atomic_fetch_add_explicit(&result_count, 1, memory_order_relaxed);

        if (slot < max_results) {
            // Store private key (32 bytes)
            device uint8_t* pk_dst = results_private_keys + slot * 32;
            #pragma unroll
            for (int i = 0; i < 4; i++) {
                pk_dst[i * 8 + 0] = (private_key[i] >> 0) & 0xFF;
                pk_dst[i * 8 + 1] = (private_key[i] >> 8) & 0xFF;
                pk_dst[i * 8 + 2] = (private_key[i] >> 16) & 0xFF;
                pk_dst[i * 8 + 3] = (private_key[i] >> 24) & 0xFF;
                pk_dst[i * 8 + 4] = (private_key[i] >> 32) & 0xFF;
                pk_dst[i * 8 + 5] = (private_key[i] >> 40) & 0xFF;
                pk_dst[i * 8 + 6] = (private_key[i] >> 48) & 0xFF;
                pk_dst[i * 8 + 7] = (private_key[i] >> 56) & 0xFF;
            }

            // Store address (20 bytes)
            device uint8_t* addr_dst = results_addresses + slot * 20;
            #pragma unroll
            for (int i = 0; i < 20; i++) {
                addr_dst[i] = address[i];
            }
        }
    }
}
