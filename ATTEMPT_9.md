# Attempt 9

**Tag**: `v142.2.2-test-g-9`
**Date**: 2026-01-15

## Goal
Fix linker failure by installing and configuring the ARM64 cross-linker.
Attempt 8 passed `bindgen` (finding headers) but failed with `collect2: error: ld returned 1 exit status`, indicating the host linker was used for the target architecture.

## Changes
- **`.github/workflows/ci.yml`**:
  - Install `gcc-aarch64-linux-gnu` package.
  - Set `CARGO_TARGET_AARCH64_UNKNOWN_LINUX_GNU_LINKER=aarch64-linux-gnu-gcc` environment variable for the ARM64 job.
- **`Cargo.toml`**: Bump version to `142.2.2-test-g-9`.

## Status
**PENDING**

## Expectations
- Cargo should use the correct linker for the final binary.
- This *might* be the final piece for a successful cross-compilation.
