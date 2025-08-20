# Dilithium Memory Safety Fix

## Issue
The Dilithium implementation had a critical memory safety issue where the `crh()` function returns 48 bytes (as per NIST specification), but the code was trying to copy it into a buffer slice that was only 32 bytes long.

## Root Cause
In all three Dilithium implementations (Dilithium2, Dilithium3, Dilithium5), the following code was problematic:

```rust
let mu = crh(&mu_input);  // Returns 48 bytes
let mut rhoprime_input = SecureArray::<96>::zero();
rhoprime_input.as_mut()[64..96].copy_from_slice(&mu);  // Trying to copy 48 bytes into 32-byte slice!
```

The slice `[64..96]` is only 32 bytes long, but `mu` is 48 bytes, causing a panic.

## Fix Applied
Changed the buffer size from 96 to 112 bytes to accommodate the full 48-byte `mu`:

```rust
let mut rhoprime_input = SecureArray::<112>::zero();  // Increased from 96 to 112
rhoprime_input.as_mut()[64..112].copy_from_slice(&mu);  // Now correctly copies all 48 bytes
```

This fix was applied to:
- `/workspaces/playground/synapsed/core/synapsed-crypto/src/dilithium/dilithium2.rs` (line 513)
- `/workspaces/playground/synapsed/core/synapsed-crypto/src/dilithium/dilithium3.rs` (line 202)
- `/workspaces/playground/synapsed/core/synapsed-crypto/src/dilithium/dilithium5.rs` (line 202)

## Status
The memory safety issue has been resolved. The buffer now correctly accommodates the 48-byte output from the `crh()` function, preventing the slice size mismatch panic.

## Additional Issues Found
While fixing this issue, other problems were discovered:
1. Public key size calculations appear incorrect (failing assertions)
2. Arithmetic overflow in the `decompose` function
3. Incomplete implementations for key unpacking

These issues are separate from the memory safety fix and would require additional investigation.