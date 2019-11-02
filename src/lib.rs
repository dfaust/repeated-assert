//! Run assertions multiple times
//!
//! `repeated_assert` runs assertions until they either pass
//! or the maximum amount of repetitions has been reached.
//! The current thread will be blocked between tries.
//!
//! This is useful when waiting for events from another thread (or process).
//! Waiting for a short time might result in a failing test, while waiting too long is a waste of time.
//!
//! # Examples
//!
//! Waiting for a file to appear (re-try up to 10 times, wait 50 ms between tries)
//!
//! ```rust,ignore
//! repeated_assert::that(10, Duration::from_millis(50), || {
//!     assert!(Path::new("should_appear_soon.txt").exists());
//! });
//! ```
//!
//! Waiting for variable `x` to equal `3`
//!
//! ```rust,ignore
//! repeated_assert::that(10, Duration::from_millis(50), || {
//!     assert_eq!(x, 3);
//! });
//! ```
//!
//! Temporary variables
//!
//! ```rust,ignore
//! repeated_assert::that(10, Duration::from_millis(50), || {
//!     let checksum = crc("should_appear_soon.txt");
//!     assert_eq!(checksum, 1234);
//! });
//! ```
//!
//! Return result
//!
//! ```rust,ignore
//! repeated_assert::that(10, Duration::from_millis(50), || -> Result<_, Box<dyn std::error::Error>> {
//!     let checksum = crc("should_appear_soon.txt")?;
//!     assert_eq!(checksum, 1234);
//! })?;
//! ```
//!
//! # Catch failing tests
//!
//! It's also possible to "catch" failing tests by executing some code if the expressions couldn't be asserted in order to trigger an alternate strategy.
//! This can be useful if the tested program relies on an unreliable service.
//! This counters the idea of a test to some degree, so use it only if the unreliable service is not critical for your program.
//!
//! Poke unreliable service after 5 unsuccessful assertion attempts
//!
//! ```rust,ignore
//! repeated_assert::with_catch(10, Duration::from_millis(50), 5,
//!     || {
//!         // poke unreliable service
//!     },
//!     || {
//!         assert!(Path::new("should_appear_soon.txt").exists());
//!     }
//! );
//! ```

use lazy_static::lazy_static;

use std::{collections::HashSet, panic, sync::Mutex, thread, time::Duration};

mod macros;

lazy_static! {
    static ref IGNORE_THREADS: Mutex<HashSet<String>> = {
        // get original panic hook
        let panic_hook = panic::take_hook();
        // set custom panic hook
        panic::set_hook(Box::new(move |panic_info| {
            let ignore_threads = IGNORE_THREADS.lock().expect("lock ignore threads");
            if let Some(thread_name) = thread::current().name() {
                if !ignore_threads.contains(thread_name) {
                    // call original panic hook
                    panic_hook(panic_info);
                }
            } else {
                // call original panic hook
                panic_hook(panic_info);
            }
        }));
        Mutex::new(HashSet::new())
    };
}

struct IgnoreGuard;

impl IgnoreGuard {
    fn new() -> IgnoreGuard {
        if let Some(thread_name) = thread::current().name() {
            IGNORE_THREADS
                .lock()
                .expect("lock ignore threads")
                .insert(thread_name.to_string());
        }
        IgnoreGuard
    }
}

impl Drop for IgnoreGuard {
    fn drop(&mut self) {
        if let Some(thread_name) = thread::current().name() {
            IGNORE_THREADS
                .lock()
                .expect("lock ignore threads")
                .remove(thread_name);
        }
    }
}

/// Run the provided function `assert` up to `repetitions` times with a `delay` in between tries.
///
/// Panics (including failed assertions) will be caught and ignored until the last try is executed.
///
/// # Examples
///
/// Waiting for a file to appear (re-try up to 10 times, wait 50 ms between tries)
///
/// ```rust,ignore
/// repeated_assert::that(10, Duration::from_millis(50), || {
///     assert!(Path::new("should_appear_soon.txt").exists());
/// });
/// ```
///
/// # Info
///
/// Behind the scene `std::panic::set_hook` is used to set a custom panic handler.
/// For every iteration but the last, panics are ignored and re-tried after a delay.
/// Only when the last iteration is reached, panics are handled by the panic handler that was registered prior to calling `repeated_assert`.
///
/// The panic handler can only be registerd for the entire process, and it is done on demand the first time `repeated_assert` is used.
/// `repeated_assert` works with multiple threads. Each thread is identified by its name, which is automatically set for tests.
pub fn that<A, R>(repetitions: usize, delay: Duration, assert: A) -> R
where
    A: Fn() -> R,
{
    // add current thread to ignore list
    let ignore_guard = IgnoreGuard::new();

    for _ in 0..(repetitions - 1) {
        // run assertions, catching panics
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| assert()));
        // return if assertions succeeded
        if let Ok(value) = result {
            return value;
        }
        // or sleep until the next try
        thread::sleep(delay);
    }

    // remove current thread from ignore list
    drop(ignore_guard);

    // run assertions without catching panics
    assert()
}

/// Run the provided function `assert` up to `repetitions` times with a `delay` in between tries.
/// Execute the provided function `catch` after `repetitions_catch` failed tries in order to trigger an alternate strategy.
///
/// Panics (including failed assertions) will be caught and ignored until the last try is executed.
///
/// # Examples
///
/// ```rust,ignore
/// repeated_assert::with_catch(10, Duration::from_millis(50), 5,
///     || {
///         // poke unreliable service
///     },
///     || {
///         assert!(Path::new("should_appear_soon.txt").exists());
///     }
/// );
/// ```
///
/// # Info
///
/// See [`that`].
pub fn with_catch<A, C, R>(
    repetitions: usize,
    delay: Duration,
    repetitions_catch: usize,
    catch: C,
    assert: A,
) -> R
where
    A: Fn() -> R,
    C: FnOnce() -> (),
{
    let ignore_guard = IgnoreGuard::new();

    for _ in 0..repetitions_catch {
        // run assertions, catching panics
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| assert()));
        // return if assertions succeeded
        if let Ok(value) = result {
            return value;
        }
        // or sleep until the next try
        thread::sleep(delay);
    }

    let thread_name = thread::current()
        .name()
        .unwrap_or("<unnamed thread>")
        .to_string();
    println!("{}: executing repeated-assert catch block", thread_name);
    catch();

    for _ in repetitions_catch..(repetitions - 1) {
        // run assertions, catching panics
        let result = panic::catch_unwind(panic::AssertUnwindSafe(|| assert()));
        // return if assertions succeeded
        if let Ok(value) = result {
            return value;
        }
        // or sleep until the next try
        thread::sleep(delay);
    }

    // remove current thread from ignore list
    drop(ignore_guard);

    // run assertions without catching panics
    assert()
}

#[cfg(test)]
mod tests {
    use crate as repeated_assert;
    use std::sync::{Arc, Mutex};
    use std::thread;
    use std::time::Duration;

    static STEP_MS: u64 = 100;

    fn spawn_thread(x: Arc<Mutex<i32>>) {
        thread::spawn(move || loop {
            thread::sleep(Duration::from_millis(10 * STEP_MS));
            if let Ok(mut x) = x.lock() {
                *x += 1;
            }
        });
    }

    // #[test]
    // fn slow() {
    //     let x = Arc::new(Mutex::new(0));

    //     spawn_thread(x.clone());

    //     repeated_assert::that(10, Duration::from_millis(10 * STEP_MS), || {
    //         assert!(*x.lock().unwrap() > 5);
    //     });
    // }

    // #[test]
    // fn panic() {
    //     let x = Arc::new(Mutex::new(0));

    //     spawn_thread(x.clone());

    //     repeated_assert::that(3, Duration::from_millis(10 * STEP_MS), || {
    //         assert!(*x.lock().unwrap() > 5);
    //     });
    // }

    #[test]
    fn single_success() {
        let x = Arc::new(Mutex::new(0));

        spawn_thread(x.clone());

        repeated_assert::that(5, Duration::from_millis(5 * STEP_MS), || {
            assert!(*x.lock().unwrap() > 0);
        });
    }

    #[test]
    #[should_panic(expected = "assertion failed: *x.lock().unwrap() > 0")]
    fn single_failure() {
        let x = Arc::new(Mutex::new(0));

        spawn_thread(x.clone());

        repeated_assert::that(3, Duration::from_millis(1 * STEP_MS), || {
            assert!(*x.lock().unwrap() > 0);
        });
    }

    #[test]
    fn multiple_success() {
        let x = Arc::new(Mutex::new(0));
        let a = 11;
        let b = 11;

        spawn_thread(x.clone());

        repeated_assert::that(5, Duration::from_millis(5 * STEP_MS), || {
            assert!(*x.lock().unwrap() > 0);
            assert_eq!(a, b);
        });
    }

    #[test]
    #[should_panic(expected = "assertion failed: *x.lock().unwrap() > 0")]
    fn multiple_failure_1() {
        let x = Arc::new(Mutex::new(0));
        let a = 11;
        let b = 11;

        spawn_thread(x.clone());

        repeated_assert::that(3, Duration::from_millis(1 * STEP_MS), || {
            assert!(*x.lock().unwrap() > 0);
            assert_eq!(a, b);
        });
    }

    #[test]
    #[should_panic(expected = "assertion failed: `(left == right)")]
    fn multiple_failure_2() {
        let x = Arc::new(Mutex::new(0));
        let a = 11;
        let b = 12;

        spawn_thread(x.clone());

        repeated_assert::that(5, Duration::from_millis(5 * STEP_MS), || {
            assert!(*x.lock().unwrap() > 0);
            assert_eq!(a, b);
        });
    }

    #[test]
    fn catch() {
        let x = Arc::new(Mutex::new(-1_000));

        spawn_thread(x.clone());

        repeated_assert::with_catch(
            10,
            Duration::from_millis(5 * STEP_MS),
            5,
            || {
                *x.lock().unwrap() = 0;
            },
            || {
                assert!(*x.lock().unwrap() > 0);
            },
        );
    }
}
