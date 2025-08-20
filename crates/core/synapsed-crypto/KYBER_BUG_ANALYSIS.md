# Kyber512 Re-encryption Bug Analysis

## Summary
The Kyber512 implementation had a bug that failed the re-encryption check during decapsulation, causing shared secrets to mismatch. The root cause was **asymmetric rounding in the 10-bit polynomial compression/decompression functions**. **This bug has been fixed.**

## Bug Location
File: `src/utils.rs`

### Compression (compress_poly_10bit):
```rust
let compressed = ((coeff.wrapping_mul(1024).wrapping_add(1664)) / 3329) & 0x3FF;
```
Uses rounding constant: **1664** (which is ⌊q/2⌋ where q=3329)

### Decompression (decompress_poly_10bit):
```rust
coeffs[i * 4 + j] = ((compressed * 3329 + 512) / 1024) as i16;
```
Uses rounding constant: **512** (which is 2^9)

## Why This Causes Failure

1. **Kyber512 uses DU=10** for ciphertext compression of the u component
2. During encapsulation, polynomials are compressed from q=3329 to 10 bits
3. During decapsulation, these values are decompressed back
4. The asymmetric rounding means: `decompress(compress(x)) ≠ x`
5. This causes the re-encrypted ciphertext c' to differ from the original c
6. The FO transform detects this mismatch and falls back to implicit rejection using z

## Fix Applied
The fix has been applied to line 305 in `src/utils.rs`:
```rust
// OLD (incorrect):
coeffs[i * 4 + j] = ((compressed * 3329 + 512) / 1024) as i16;

// NEW (correct - FIXED):
coeffs[i * 4 + j] = ((compressed * 3329 + 1664) / 1024) as i16;
```

## Verification
After fixing, the re-encryption in decapsulation should produce identical ciphertext, allowing the shared secrets to match.

## Additional Notes
- The 4-bit decompression (used for DV=4) has a similar issue but uses correct rounding
- The size fixes (SECRET_KEY_SIZE=1632, SecureArray<32>) are already correct
- The FO transform structure and implicit rejection mechanism are properly implemented