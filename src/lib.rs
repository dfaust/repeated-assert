//! An assertion macro that tries to assert expressions multiple times
//!
//! `repeated_assert!` re-tries to assert expressions until either all expressions are `true`
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
//! repeated_assert!{ 10, Duration::from_millis(50);
//!     if Path::new("should_appear_soon.txt").exists();
//! };
//! ```
//!
//! Waiting for variable `x` to equal `3`
//!
//! ```rust,ignore
//! repeated_assert!{ 10, Duration::from_millis(50);
//!     eq x, 3;
//! };
//! ```
//!
//! Multiple assertions
//!
//! ```rust,ignore
//! repeated_assert!{ 10, Duration::from_millis(50);
//!     if Path::new("should_appear_soon.txt").exists();
//!     eq x, 3;
//! };
//! ```
//!
//! Temporary variables
//!
//! ```rust,ignore
//! repeated_assert!{ 10, Duration::from_millis(50);
//!     let checksum = crc("file_is_being_written.txt");
//!     eq checksum, 1234;
//! };
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
//! repeated_assert!{ 10, Duration::from_millis(50), 5, {
//!         // poke unreliable service
//!     };
//!     if Path::new("should_appear_soon.txt").exists();
//! };
//! ```
#![warn(missing_docs)]

#[macro_export]
macro_rules! repeated_assert {
    ($repetitions:expr, $delay:expr; $($tt:tt)*) => {
        for i in 0..$repetitions {
            if i == $repetitions - 1 {
                __repeated_assert!{ @final, $($tt)* }
            } else {
                if __repeated_assert!{ $($tt)* } {
                    break;
                }
                ::std::thread::sleep($delay);
            }
        }
    };
    ($repetitions:expr, $delay:expr, $repetitions_catch:expr, $catch:block; $($tt:tt)*) => {
        for i in 0..$repetitions {
            if i == $repetitions - 1 {
                __repeated_assert!{ @final, $($tt)* }
            } else if i == $repetitions_catch {
                let thread_name = ::std::thread::current().name().unwrap_or("unnamed thread").to_owned();
                println!("{}: executing repeated-assert catch block", thread_name);
                $catch
                ::std::thread::sleep($delay);
            } else {
                if __repeated_assert!{ $($tt)* } {
                    break;
                }
                ::std::thread::sleep($delay);
            }
        }
    };
}

#[doc(hidden)]
#[macro_export]
macro_rules! __repeated_assert {
    (@final, if $expr:expr;) => {
        assert!($expr);
    };
    (@final, eq $left:expr, $right:expr;) => {
        assert_eq!($left, $right, stringify!($left != $right));
    };
    (@final, if $expr:expr; $($tt:tt)+) => {
        assert!($expr);
        __repeated_assert!{ @final, $($tt)+ }
    };
    (@final, eq $left:expr, $right:expr; $($tt:tt)+) => {
        assert_eq!($left, $right, stringify!($left != $right));
        __repeated_assert!{ @final, $($tt)+ }
    };
    (@final, let $($pat:pat)|+ = $expr:expr; $($tt:tt)+) => {
        match $expr {
            $($pat)|+ => { __repeated_assert!{ @final, $($tt)+ } }
        }
    };
    (if $expr:expr;) => {
        $expr
    };
    (eq $left:expr, $right:expr;) => {
        $left == $right
    };
    (if $expr:expr; $($tt:tt)+) => {
        if $expr {
            __repeated_assert!{ $($tt)+ }
        } else {
            false
        }
    };
    (eq $left:expr, $right:expr; $($tt:tt)+) => {
        if $left == $right {
            __repeated_assert!{ $($tt)+ }
        } else {
            false
        }
    };
    (let $($pat:pat)|+ = $expr:expr; $($tt:tt)+) => {
        match $expr {
            $($pat)|+ => { __repeated_assert!{ $($tt)+ } }
        }
    };
}

#[cfg(test)]
mod tests {
    use std::thread;
    use std::sync::{Arc, Mutex};
    use std::time::Duration;

    static STEP_MS: u64 = 100;

    fn spawn_thread(x: Arc<Mutex<i32>>) {
        thread::spawn(move || {
            loop {
                thread::sleep(Duration::from_millis(10 * STEP_MS));
                if let Ok(mut x) = x.lock() {
                    *x += 1;
                }
            }
        });
    }

    #[test]
    fn single_success() {
        let x = Arc::new(Mutex::new(0));

        spawn_thread(x.clone());

        repeated_assert!{ 5, Duration::from_millis(5 * STEP_MS);
            if *x.lock().unwrap() > 0;
        };
    }

    #[test]
    #[should_panic(expected = "assertion failed: *x.lock().unwrap() > 0")]
    fn single_failure() {
        let x = Arc::new(Mutex::new(0));

        spawn_thread(x.clone());

        repeated_assert!{ 3, Duration::from_millis(1 * STEP_MS);
            if *x.lock().unwrap() > 0;
        };
    }

    #[test]
    fn multiple_success() {
        let x = Arc::new(Mutex::new(0));
        let a = 11;
        let b = 11;

        spawn_thread(x.clone());

        repeated_assert!{ 5, Duration::from_millis(5 * STEP_MS);
            if *x.lock().unwrap() > 0;
            eq a, b;
        };
    }

    #[test]
    #[should_panic(expected = "assertion failed: *x.lock().unwrap() > 0")]
    fn multiple_failure_1() {
        let x = Arc::new(Mutex::new(0));
        let a = 11;
        let b = 11;

        spawn_thread(x.clone());

        repeated_assert!{ 3, Duration::from_millis(1 * STEP_MS);
            if *x.lock().unwrap() > 0;
            eq a, b;
        };
    }

    #[test]
    #[should_panic(expected = "a != b")]
    fn multiple_failure_2() {
        let x = Arc::new(Mutex::new(0));
        let a = 11;
        let b = 12;

        spawn_thread(x.clone());

        repeated_assert!{ 5, Duration::from_millis(5 * STEP_MS);
            if *x.lock().unwrap() > 0;
            eq a, b;
        };
    }

    #[test]
    fn let_success() {
        let x = Arc::new(Mutex::new(0));

        spawn_thread(x.clone());

        repeated_assert!{ 5, Duration::from_millis(5 * STEP_MS);
            let y = *x.lock().unwrap();
            if y > 0;
        };
    }

    #[test]
    #[should_panic(expected = "assertion failed: y > 0")]
    fn let_failure() {
        let x = Arc::new(Mutex::new(0));

        spawn_thread(x.clone());

        repeated_assert!{ 3, Duration::from_millis(1 * STEP_MS);
            let y = *x.lock().unwrap();
            if y > 0;
        };
    }

    #[test]
    fn catch() {
        let x = Arc::new(Mutex::new(-1_000));

        spawn_thread(x.clone());

        repeated_assert!{ 10, Duration::from_millis(5 * STEP_MS), 5, {
                *x.lock().unwrap() = 0;
            };
            if *x.lock().unwrap() > 0;
        };
    }
}
