# repeated-assert

[![Crate version](https://img.shields.io/crates/d/repeated-assert)](https://crates.io/crates/repeated-assert)
[![Documentation](https://img.shields.io/docsrs/repeated-assert)](https://docs.rs/repeated-assert)

Run assertions multiple times

`repeated_assert` runs assertions until they either pass
or the maximum amount of repetitions has been reached.
The current thread will be blocked between tries.

This is useful when waiting for events from another thread (or process).
Waiting for a short time might result in a failing test, while waiting too long is a waste of time.

## Crate features

* **async** - Enables the `that_async` and `with_catch_async` functions. It depends on the `futures` and `tokio` crates, which is why it's disabled by default.

## Examples

Waiting for a file to appear (re-try up to 10 times, wait 50 ms between tries)

```rust,ignore
repeated_assert::that(10, Duration::from_millis(50), || {
    assert!(Path::new("should_appear_soon.txt").exists());
});
```

Waiting for variable `x` to equal `3`

```rust,ignore
repeated_assert::that(10, Duration::from_millis(50), || {
    assert_eq!(x, 3);
});
```

Temporary variables

```rust,ignore
repeated_assert::that(10, Duration::from_millis(50), || {
    let checksum = crc("should_appear_soon.txt");
    assert_eq!(checksum, 1234);
});
```

Return result

```rust,ignore
repeated_assert::that(10, Duration::from_millis(50), || -> Result<_, Box<dyn std::error::Error>> {
    let checksum = crc("should_appear_soon.txt")?;
    assert_eq!(checksum, 1234);
})?;
```

Async

```rust,ignore
repeated_assert::that_async(10, Duration::from_millis(50), || async {
    let status = query_db().await;
    assert_eq!(status, "success");
}).await;
```

## Catch failing tests

It's also possible to "catch" failing tests by executing some code if the expressions couldn't be asserted in order to trigger an alternate strategy.
This can be useful if the tested program relies on an unreliable service.
This counters the idea of a test to some degree, so use it only if the unreliable service is not critical for your program.

Poke unreliable service after 5 unsuccessful assertion attempts

```rust,ignore
repeated_assert::with_catch(10, Duration::from_millis(50), 5,
    || {
        // poke unreliable service
    },
    || {
        assert!(Path::new("should_appear_soon.txt").exists());
    }
);
```
