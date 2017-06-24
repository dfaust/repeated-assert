# repeated-assert

[![Build Status](https://travis-ci.org/dfaust/repeated-assert.svg?branch=master)](https://travis-ci.org/dfaust/repeated-assert)
[![Windows build status](https://ci.appveyor.com/api/projects/status/github/dfaust/repeated-assert?svg=true)](https://ci.appveyor.com/project/dfaust/repeated-assert)
[![Crate version](https://img.shields.io/crates/v/repeated-assert.svg)](https://crates.io/crates/repeated-assert)
[![Documentation](https://img.shields.io/badge/documentation-docs.rs-df3600.svg)](https://docs.rs/repeated-assert)

An assertion macro that tries to assert expressions multiple times

`repeated_assert!` re-tries to assert expressions until either all expressions are `true`
or the maximum amount of repetitions has been reached.
The current thread will be blocked between tries.

This is useful when waiting for events from another thread (or process).
Waiting for a short time might result in a failing test, while waiting too long is a waste of time.

## Examples

Wait for a file to appear, calculate the checksum and then assert the checksum is to equal to `1234` (re-try up to 10 times, wait 50 ms between tries)

```rust
repeated_assert!{ 10, Duration::from_millis(50);
    if Path::new("should_appear_soon.txt").exists();
    let checksum = crc("should_appear_soon.txt");
    eq checksum, 1234;
};
```
