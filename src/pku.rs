// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

//! PKRU (Protection Keys for Userspace) support for V8 isolate locking.
//!
//! Intel PKU allows restricting memory access via a per-thread CPU register
//! called PKRU. V8 uses PKU for memory protection (sandbox, JIT code, etc.),
//! but PKRU is per-thread, so threads entering V8 need their PKRU normalized
//! to match the baseline established during V8 initialization.
//!
//! Without this, threads may crash with SIGSEGV (si_code=SEGV_PKUERR) when
//! accessing V8-protected memory pages.
//!
//! This module is only active on Linux x86_64 with PKU-capable CPUs.
//! On other platforms or CPUs without PKU, it compiles to no-ops.

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
use std::sync::OnceLock;

/// Baseline PKRU value captured after V8 initialization.
/// None if PKU is not supported on this CPU/kernel.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
static BASELINE_PKRU: OnceLock<Option<u32>> = OnceLock::new();

/// Check if PKU is supported by the CPU and enabled by the OS.
/// Uses pkey_alloc syscall as recommended by kernel documentation.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn is_pku_supported() -> bool {
  // Try to allocate a protection key. If it succeeds, PKU is supported.
  // syscall numbers: pkey_alloc = 330, pkey_free = 331 on x86_64
  let pkey = unsafe { libc::syscall(libc::SYS_pkey_alloc, 0, 0) };

  if pkey >= 0 {
    // Free the key we just allocated
    unsafe { libc::syscall(libc::SYS_pkey_free, pkey) };
    true
  } else {
    false
  }
}

/// Capture the current PKRU as the baseline for V8 operations.
///
/// This should be called once after `V8::initialize()` on the main thread.
/// The captured value will be restored on every thread entering V8 via `Locker`.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub fn capture_baseline() {
  BASELINE_PKRU.get_or_init(|| {
    if is_pku_supported() {
      Some(read_pkru())
    } else {
      None
    }
  });
}

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub fn capture_baseline() {
  // No-op on non-Linux or non-x86_64
}

/// Read the current PKRU register value.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn read_pkru() -> u32 {
  let pkru: u32;

  unsafe {
    std::arch::asm!(
      "xor ecx, ecx",
      "rdpkru",
      out("eax") pkru,
      out("ecx") _,
      out("edx") _,
      options(nomem, nostack),
    );
  }

  pkru
}

/// Write a value to the PKRU register.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
fn write_pkru(pkru: u32) {
  unsafe {
    std::arch::asm!(
      "xor ecx, ecx",
      "xor edx, edx",
      "wrpkru",
      in("eax") pkru,
      out("ecx") _,
      out("edx") _,
      options(nomem, nostack),
    );
  }
}

/// RAII guard that saves the current PKRU and restores the V8 baseline.
///
/// When created, it saves the current thread's PKRU value and restores the
/// baseline captured during V8 initialization. When dropped, it restores
/// the original PKRU value.
///
/// On CPUs without PKU support, this is a no-op.
#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
pub struct PkruGuard {
  saved: Option<u32>,
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl PkruGuard {
  /// Create a new PKRU guard, saving the current PKRU and restoring baseline.
  pub fn new() -> Self {
    let saved = if let Some(Some(baseline)) = BASELINE_PKRU.get() {
      let current = read_pkru();

      if current != *baseline {
        write_pkru(*baseline);
      }

      Some(current)
    } else {
      None
    };

    Self { saved }
  }
}

#[cfg(all(target_os = "linux", target_arch = "x86_64"))]
impl Drop for PkruGuard {
  fn drop(&mut self) {
    if let Some(saved) = self.saved {
      let current = read_pkru();

      if current != saved {
        write_pkru(saved);
      }
    }
  }
}

/// No-op PKRU guard for non-Linux or non-x86_64 platforms.
#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
pub struct PkruGuard;

#[cfg(not(all(target_os = "linux", target_arch = "x86_64")))]
impl PkruGuard {
  pub fn new() -> Self {
    Self
  }
}
