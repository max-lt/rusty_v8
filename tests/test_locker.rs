// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

//! Tests for v8::Locker and v8::UnenteredIsolate
//!
//! These bindings enable multi-threaded isolate usage where
//! Locker ensures thread-safe access to isolates.
//!
//! Key design points:
//! - UnenteredIsolate does NOT deref to Isolate (safety)
//! - You MUST use Locker to access the Isolate
//! - V8's Locker handles Enter/Exit automatically

use std::pin::pin;

#[test]
fn test_locker_basic() {
  let _setup_guard = setup();

  // Create an unentered isolate (doesn't auto-enter like OwnedIsolate)
  let params = v8::CreateParams::default();
  let mut isolate = v8::Isolate::new_unentered(params);

  {
    // Lock the isolate - Locker takes &mut UnenteredIsolate
    // and derefs to Isolate. V8's Locker enters the isolate automatically.
    let mut locker = v8::Locker::new(&mut isolate);

    // Access Isolate through the Locker
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();

    // Create a Context
    let _context = v8::Context::new(scope, Default::default());
  }

  // Locker drops -> exits. UnenteredIsolate::Drop handles enter/exit internally.
}

#[test]
fn test_locker_with_script() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default();
  let mut isolate = v8::Isolate::new_unentered(params);

  {
    let mut locker = v8::Locker::new(&mut isolate);

    // Create a context and execute code
    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "40 + 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_integer(scope).unwrap();

    assert_eq!(result.value(), 42);
  }
}

#[test]
fn test_unentered_isolate_no_lifo_constraint() {
  let _setup_guard = setup();

  // Create multiple unentered isolates
  let params1 = v8::CreateParams::default();
  let isolate1 = v8::Isolate::new_unentered(params1);

  let params2 = v8::CreateParams::default();
  let isolate2 = v8::Isolate::new_unentered(params2);

  let params3 = v8::CreateParams::default();
  let isolate3 = v8::Isolate::new_unentered(params3);

  // Drop in arbitrary order (not LIFO)
  // This would panic with OwnedIsolate but works with UnenteredIsolate
  drop(isolate2);
  drop(isolate1);
  drop(isolate3);

  // Test passes if no panic
}

#[test]
fn test_locker_multiple_lock_unlock() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default();
  let mut isolate = v8::Isolate::new_unentered(params);

  // First lock/unlock cycle
  {
    let mut locker = v8::Locker::new(&mut isolate);

    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "1 + 1").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 2);
  }

  // Second lock/unlock cycle - same isolate, new locker
  {
    let mut locker = v8::Locker::new(&mut isolate);

    let scope = pin!(v8::HandleScope::new(&mut *locker));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "2 + 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    assert_eq!(result.to_integer(scope).unwrap().value(), 4);
  }
}

#[test]
fn test_locker_is_locked() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default();
  let mut isolate = v8::Isolate::new_unentered(params);

  // Before locking
  assert!(!v8::Locker::is_locked(&isolate));

  {
    let _locker = v8::Locker::new(&mut isolate);
    // Can't check is_locked here because we borrowed isolate mutably
  }

  // After unlocking
  assert!(!v8::Locker::is_locked(&isolate));
}

// Helper to setup V8 platform (only once per process)
fn setup() -> impl Drop {
  use std::sync::Once;
  static INIT: Once = Once::new();

  INIT.call_once(|| {
    let platform = v8::new_default_platform(0, false).make_shared();
    v8::V8::initialize_platform(platform);
    v8::V8::initialize();
  });

  // Return a guard that does nothing on drop
  struct Guard;
  impl Drop for Guard {
    fn drop(&mut self) {}
  }
  Guard
}
