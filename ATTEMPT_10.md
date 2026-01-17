# Attempt 10

**Tag**: `v142.2.2-test-c-10`
**Date**: 2026-01-17

## Goal
Fix test execution failure by separating build and test into different jobs.
Attempt 9 succeeded in cross-compiling ARM64 binaries on x86_64, but failed when `cargo nextest run` tried to **execute** the ARM64 test binaries on an x86_64 host.

## Problem Analysis
The error from Attempt 9 showed the x86_64 shell trying to interpret an ARM64 binary as a script:
```
/home/runner/.../v8-ebd126a52fd35178: 3: ���: not found
```

This happens because cross-compilation produces ARM64 binaries that cannot run on x86_64.

## Changes
- **`.github/workflows/ci.yml`**: Restructured into separate jobs:

  1. **`build` job** (ubuntu-22.04 x86_64):
     - Cross-compiles for `aarch64-unknown-linux-gnu`
     - Uses `cargo nextest archive` to create a portable test archive
     - Uploads the nextest archive as artifact
     - Publishes the library binary to GitHub releases

  2. **`test-arm64` job** (ubuntu-24.04-arm):
     - Runs on native ARM64 runner
     - Downloads the nextest archive from build job
     - Executes tests with `cargo nextest run --archive-file`

  3. **`publish` job**: Now depends on both build and test-arm64

- **`Cargo.toml`**: Bump version to `142.2.2-test-c-10`

## Key Technical Details
- Using `cargo nextest archive` to package compiled tests for cross-machine execution
- Native ARM64 GitHub runner: `ubuntu-24.04-arm`
- Artifact retention: 1 day (enough for the workflow)

## Status
**PENDING**

## Expectations
- Build job should complete successfully (cross-compilation worked in Attempt 9)
- Test job should run tests natively on ARM64 hardware
- If tests pass, the full pipeline (build → test → publish) should succeed
