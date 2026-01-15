# Attempt 8

**Tag**: `v142.2.2-test-g-8`
**Date**: 2026-01-15

## Goal
Fix `bindgen` failure by injecting the missing `--target` flag for cross-compilation.
Attempt 7 failed with the same error, but revealed that while `--sysroot` was present, `--target` was missing from `CLANG_ARGS`.

## Changes
- **`build.rs`**: In `build_binding()`, alongside sysroot injection:
  - Add `--target=aarch64-linux-gnu` to `clang_args` when building for `aarch64`.
- **`Cargo.toml`**: Bump version to `142.2.2-test-g-8`.

## Status
**PENDING**

## Expectations
- `bindgen` should finally use the correct target architecture and find the headers in the provided sysroot.
- Build should pass `bindgen` and proceed to compilation.
