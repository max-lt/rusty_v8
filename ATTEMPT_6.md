# Attempt 6

**Tag**: `v142.2.2-test-g-6`
**Date**: 2026-01-14

## Goal
Debug `bindgen` failure by inspecting the exact arguments passed to Clang.
In Attempt 5, compilation succeeded but binding generation failed with `fatal error: 'bits/wordsize.h' file not found`, indicating `bindgen` is not using the correct sysroot or include paths.

## Changes
- **`build.rs`**: Added `println!("cargo:warning=CLANG_ARGS: {:?}", clang_args);` to `build_binding()` to log the arguments.
- **`Cargo.toml`**: Bump version to `142.2.2-test-g-6`.

## Status
**PENDING**

## Expectations
- CI will likely fail again with the same panic.
- I will inspect the logs to see the value of `CLANG_ARGS`.
- I specifically want to see if `--sysroot` is present and if it's an absolute path.
