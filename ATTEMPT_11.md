# Attempt 11

**Tag**: `v142.2.2-test-c-11`
**Date**: 2026-01-17

## Goal
Fix SIGSEGV on ARM64 native runner by using QEMU emulation instead.

## Problem Analysis
Attempt 10 successfully:
- Cross-compiled ARM64 binaries on x86_64
- Created nextest archive
- Transferred to ARM64 native runner (`ubuntu-24.04-arm`)

But tests crashed with SIGSEGV when running on native ARM64:
```
SIGSEGV [1.677s] openworkers-v8::slots clear_all_context_slots
SIGSEGV [1.843s] openworkers-v8 scope::tests::deref_types
SIGSEGV [1.625s] openworkers-v8::slots dropped_context_slots
SIGSEGV [1.630s] openworkers-v8::slots context_slots
```

14 tests passed before the crash, all tests that crash involve creating a `v8::Isolate`.

## Root Cause Hypothesis
The upstream `denoland/rusty_v8` does NOT use native ARM64 runners for Linux. They use **QEMU emulation** on x86_64 to run ARM64 binaries. This is a proven approach.

The SIGSEGV on native ARM64 could be due to:
- Differences in libc/environment between cross-compile sysroot and native runner
- Some V8 ARM64-specific code paths that behave differently under emulation vs native

## Changes
- **`.github/workflows/ci.yml`**: Modified `test-arm64` job:
  - Changed runner from `ubuntu-24.04-arm` to `ubuntu-22.04` (x86_64)
  - Added QEMU installation (`qemu-user`, `binfmt-support`, `libc6-arm64-cross`)
  - Set `QEMU_LD_PREFIX=/usr/aarch64-linux-gnu` for proper library resolution
  - Increased timeout to 60 minutes (QEMU is slower than native)

- **`Cargo.toml`**: Bump version to `142.2.2-test-c-11`

## Status
**PENDING**

## Expectations
- Tests should run via QEMU emulation (slower but proven to work upstream)
- All tests should pass as they do on upstream rusty_v8
