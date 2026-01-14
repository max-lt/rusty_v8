# Attempt 7

**Tag**: `v142.2.2-test-g-7`
**Date**: 2026-01-15

## Goal
Fix `bindgen` failure by manually injecting the V8 ARM64 sysroot.
Attempt 6 confirmed that `bindgen` is receiving **no** sysroot configuration, causing it to fail finding standard headers.

## Changes
- **`build.rs`**: In `build_binding()`, manually detect if we are building for `aarch64-unknown-linux-gnu`.
  - If so, construct the absolute path to `build/linux/debian_bullseye_arm64-sysroot`.
  - Append `--sysroot=<path>` to `clang_args`.
- **`Cargo.toml`**: Bump version to `142.2.2-test-g-7`.

## Status
**PENDING**

## Expectations
- `bindgen` should now find `bits/wordsize.h` and other headers in the sysroot.
- Build should pass.
