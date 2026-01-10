// Copyright 2019-2021 the Deno authors. All rights reserved. MIT license.

//! Tests for v8::Locker, v8::Unlocker, and v8::UnenteredIsolate
//!
//! These bindings enable multi-threaded isolate pooling architectures
//! similar to Cloudflare Workers.
//!
//! NOTE: When using UnenteredIsolate with Locker, each thread that accesses
//! the isolate must call enter() before first use to set up V8's thread-local
//! state (like LocalHeap). This is a V8 requirement.

use std::pin::pin;
use std::sync::Arc;
use std::sync::Mutex;
use std::thread;

#[test]
fn test_locker_basic() {
  let _setup_guard = setup();

  // Create an unentered isolate (doesn't auto-enter like OwnedIsolate)
  let params = v8::CreateParams::default();
  let mut isolate = v8::Isolate::new_unentered(params);

  // Each thread must call enter() before first use to set up thread-local state
  unsafe {
    isolate.enter();
  }

  {
    // Lock the isolate for this thread
    let _locker = v8::Locker::new(&isolate);

    // Now it should be locked
    assert!(v8::Locker::is_locked(&isolate));

    // Create a HandleScope
    let scope = pin!(v8::HandleScope::new(&mut isolate));
    let scope = &mut scope.init();

    // Create a Context
    let _context = v8::Context::new(scope, Default::default());
  }

  // Clean up
  unsafe {
    isolate.exit();
  }
}

#[test]
fn test_unlocker() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default();
  let isolate = v8::Isolate::new_unentered(params);

  // Enter for this thread
  unsafe {
    (&isolate as &v8::Isolate).enter();
  }

  {
    let _locker = v8::Locker::new(&isolate);
    assert!(v8::Locker::is_locked(&isolate));

    {
      // Temporarily unlock
      let _unlocker = v8::Unlocker::new(&isolate);

      // Should be unlocked now
      assert!(!v8::Locker::is_locked(&isolate));
    }

    // After unlocker drops, locker re-locks
    assert!(v8::Locker::is_locked(&isolate));
  }

  // Final state: unlocked
  assert!(!v8::Locker::is_locked(&isolate));

  // Clean up
  unsafe {
    (&isolate as &v8::Isolate).exit();
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
  // This would panic with OwnedIsolate
  drop(isolate2);
  drop(isolate1);
  drop(isolate3);

  // Test passes if no panic
}

#[test]
fn test_locker_multithreaded() {
  let _setup_guard = setup();

  // Create an unentered isolate
  let params = v8::CreateParams::default();
  let isolate = v8::Isolate::new_unentered(params);

  // Wrap in Arc<Mutex<>> to share across threads
  let isolate = Arc::new(Mutex::new(isolate));

  let mut handles = vec![];

  // Spawn multiple threads that try to lock the isolate
  for i in 0..3 {
    let isolate_clone = Arc::clone(&isolate);

    let handle = thread::spawn(move || {
      // Acquire Rust mutex first
      let mut isolate = isolate_clone.lock().unwrap();

      // Each thread must enter the isolate to set up thread-local state
      unsafe {
        (&*isolate as &v8::Isolate).enter();
      }

      // Then acquire V8 locker
      let _locker = v8::Locker::new(&isolate);

      // Isolate should be locked on this thread
      assert!(v8::Locker::is_locked(&isolate));

      // Do some work in a separate block so scopes drop before exit()
      {
        let scope = pin!(v8::HandleScope::new(&mut *isolate));
        let scope = &mut scope.init();
        let context = v8::Context::new(scope, Default::default());
        let scope = &mut v8::ContextScope::new(scope, context);

        let code = v8::String::new(scope, &format!("1 + {}", i)).unwrap();
        let script = v8::Script::compile(scope, code, None).unwrap();
        let result = script.run(scope).unwrap();
        let result = result.to_integer(scope).unwrap();

        assert_eq!(result.value(), 1 + i);
      }

      // Clean up - now safe because scopes have been dropped
      unsafe {
        (&*isolate as &v8::Isolate).exit();
      }

      // Locker and mutex drop here
    });

    handles.push(handle);
  }

  // Wait for all threads
  for handle in handles {
    handle.join().unwrap();
  }

  // Isolate should be unlocked after all threads finish
  let isolate = isolate.lock().unwrap();
  assert!(!v8::Locker::is_locked(&isolate));
}

#[test]
fn test_locker_prevents_concurrent_access() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default();
  let isolate = v8::Isolate::new_unentered(params);
  let isolate = Arc::new(Mutex::new(isolate));

  let isolate1 = Arc::clone(&isolate);
  let isolate2 = Arc::clone(&isolate);

  let (tx, rx) = std::sync::mpsc::channel();

  // Thread 1: Lock and hold
  let handle1 = thread::spawn(move || {
    let isolate = isolate1.lock().unwrap();
    unsafe {
      (&*isolate as &v8::Isolate).enter();
    }
    let _locker = v8::Locker::new(&isolate);

    // Signal that we have the lock
    tx.send(()).unwrap();

    // Hold the lock for a bit
    thread::sleep(std::time::Duration::from_millis(100));

    unsafe {
      (&*isolate as &v8::Isolate).exit();
    }
    // Locker drops here
  });

  // Wait for thread 1 to acquire lock
  rx.recv().unwrap();

  // Thread 2: Try to lock (should wait until thread 1 releases)
  let start = std::time::Instant::now();
  let handle2 = thread::spawn(move || {
    let isolate = isolate2.lock().unwrap();
    unsafe {
      (&*isolate as &v8::Isolate).enter();
    }
    let _locker = v8::Locker::new(&isolate);

    // We should have waited for thread 1
    unsafe {
      (&*isolate as &v8::Isolate).exit();
    }
    // Locker drops here
  });

  handle1.join().unwrap();
  handle2.join().unwrap();

  let elapsed = start.elapsed();

  // Should have waited at least 50ms (thread 1 held for 100ms)
  assert!(
    elapsed >= std::time::Duration::from_millis(50),
    "Thread 2 should have waited for thread 1 to release lock"
  );
}

#[test]
fn test_unentered_isolate_with_context() {
  let _setup_guard = setup();

  let params = v8::CreateParams::default();
  let mut isolate = v8::Isolate::new_unentered(params);

  // Enter for this thread
  unsafe {
    isolate.enter();
  }

  {
    let _locker = v8::Locker::new(&isolate);

    // Create a context and execute code
    let scope = pin!(v8::HandleScope::new(&mut isolate));
    let scope = &mut scope.init();
    let context = v8::Context::new(scope, Default::default());
    let scope = &mut v8::ContextScope::new(scope, context);

    let code = v8::String::new(scope, "40 + 2").unwrap();
    let script = v8::Script::compile(scope, code, None).unwrap();
    let result = script.run(scope).unwrap();
    let result = result.to_integer(scope).unwrap();

    assert_eq!(result.value(), 42);
  }

  // Clean up
  unsafe {
    isolate.exit();
  }

  // Isolate can be dropped without issues
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
  // (just to ensure setup is called via RAII)
  struct Guard;
  impl Drop for Guard {
    fn drop(&mut self) {}
  }
  Guard
}
