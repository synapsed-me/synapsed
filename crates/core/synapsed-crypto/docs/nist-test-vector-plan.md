# NIST FIPS 203 Test Vector Implementation Plan

## Overview
This document outlines the plan for implementing official NIST test vectors for the ML-KEM (Kyber) implementation in synapsed-crypto.

## Current Status

### ✅ Verified Compliance:

1. **Secret Key Sizes are CORRECT per FIPS 203:**
   - Kyber512: 1632 bytes (768 + 800 + 32 + 32)
   - Kyber768: 2400 bytes (1152 + 1184 + 32 + 32)
   - Kyber1024: 3168 bytes (1536 + 1568 + 32 + 32)

2. **z Parameter Size is CORRECT:**
   - The z parameter is 32 bytes (as confirmed by FIPS 203 specification)
   - Secret key format: [s || pk || H(pk) || z] where z is 32 bytes
   - Total seed in ML-KEM is 64 bytes (d || z), each 32 bytes

3. **Key Sizes Match NIST Standard:**
   - All public key, ciphertext, and shared secret sizes match FIPS 203

## Implementation Plan

### Phase 1: Test Vector Infrastructure

1. **Create Test Vector Data Structures**
   ```rust
   struct MLKEMTestVector {
       test_id: u32,
       seed_d: [u8; 32],
       seed_z: [u8; 32],
       public_key: Vec<u8>,
       secret_key: Vec<u8>,
       encaps_randomness: [u8; 32],
       ciphertext: Vec<u8>,
       shared_secret: [u8; 32],
   }
   ```

2. **Test Vector Loader**
   - Support JSON format from NIST
   - Support test vector files from NIST ACVP (Automated Cryptographic Validation Protocol)
   - Add parsing for both deterministic and probabilistic test vectors

### Phase 2: Test Categories

1. **Key Generation Tests**
   - Deterministic key generation from seed (d, z)
   - Verify public/secret key byte representation
   - Verify key sizes match expected values

2. **Encapsulation Tests**
   - Deterministic encapsulation with fixed randomness
   - Verify ciphertext format and size
   - Verify shared secret generation

3. **Decapsulation Tests**
   - Valid ciphertext decapsulation
   - Invalid ciphertext handling (implicit rejection)
   - Verify shared secret matches expected value

4. **Cross-Operation Tests**
   - Full KEM cycle (keygen → encaps → decaps)
   - Multiple encapsulations with same key
   - Interoperability with reference implementation

### Phase 3: Test Vector Sources

1. **NIST ACVP Vectors**
   - Download from: https://csrc.nist.gov/projects/cryptographic-algorithm-validation-program
   - Covers all parameter sets (ML-KEM-512, ML-KEM-768, ML-KEM-1024)

2. **NIST KAT (Known Answer Tests)**
   - From FIPS 203 test vectors appendix
   - Deterministic test cases for validation

3. **Reference Implementation Vectors**
   - Compare against NIST reference implementation
   - Ensure bit-for-bit compatibility

### Phase 4: Implementation Steps

1. **Update test_vectors.rs**
   ```rust
   // Add actual NIST test vector loading
   fn load_nist_mlkem_vectors(param_set: &str) -> Vec<MLKEMTestVector> {
       // Load from JSON/CSV files
   }
   ```

2. **Create Comprehensive Test Suite**
   ```rust
   #[test]
   fn test_mlkem512_nist_vectors() {
       let vectors = load_nist_mlkem_vectors("ML-KEM-512");
       for vector in vectors {
           run_mlkem_test_vector::<2>(&vector);
       }
   }
   ```

3. **Add Validation Functions**
   - Verify intermediate values (matrix A, polynomials s, e)
   - Check NTT operations match reference
   - Validate compression/decompression operations

### Phase 5: Edge Cases and Security Tests

1. **Implicit Rejection Tests**
   - Test with malformed ciphertexts
   - Verify constant-time behavior
   - Check z parameter usage in rejection cases

2. **Boundary Conditions**
   - Maximum/minimum coefficient values
   - Edge cases in polynomial arithmetic
   - Compression boundary cases

3. **Side-Channel Resistance**
   - Timing analysis for decapsulation
   - Memory access patterns
   - Constant-time operations verification

## Test File Organization

```
tests/
├── nist_vectors/
│   ├── ml_kem_512.json
│   ├── ml_kem_768.json
│   └── ml_kem_1024.json
├── test_vectors.rs (updated)
├── nist_compliance_tests.rs (new)
└── security_tests.rs (new)
```

## Success Criteria

1. All NIST test vectors pass
2. 100% compatibility with reference implementation
3. Constant-time decapsulation verified
4. Performance benchmarks documented
5. Security properties validated

## Timeline

- Week 1: Infrastructure setup and test vector parsing
- Week 2: Key generation and encapsulation tests
- Week 3: Decapsulation and edge case tests
- Week 4: Security validation and documentation

## References

- NIST FIPS 203: Module-Lattice-Based Key-Encapsulation Mechanism Standard
- NIST ACVP Documentation
- ML-KEM Reference Implementation