# rs-retry-kit

[![CI](https://github.com/philiprehberger/rs-retry-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-retry-kit/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-retry-kit.svg)](https://crates.io/crates/philiprehberger-retry-kit)
[![License](https://img.shields.io/github/license/philiprehberger/rs-retry-kit)](LICENSE)

Async retry with exponential backoff and circuit breaker for Rust.

## Installation

```toml
[dependencies]
philiprehberger-retry-kit = "0.4.0"
```

## Usage

### Sync Retry

```rust
use philiprehberger_retry_kit::{retry, RetryOptions, Backoff};
use std::time::Duration;

let result = retry(RetryOptions::default(), || {
    fetch_data()
});
```

### With Options

```rust
let opts = RetryOptions::default()
    .max_attempts(5)
    .backoff(Backoff::Exponential)
    .initial_delay(Duration::from_secs(1))
    .max_delay(Duration::from_secs(30))
    .jitter(true);

let result = retry(opts, || fetch_data());
```

### Async Retry

```rust
use philiprehberger_retry_kit::{retry_async, RetryOptions};

let result = retry_async(RetryOptions::default(), || async {
    fetch_data().await
}).await;
```

### Presets

```rust
use philiprehberger_retry_kit::presets;

let result = retry(presets::network_request(), || fetch_data());
let result = retry(presets::database_query(), || query_db());
let result = retry(presets::aggressive(), || critical_op());
```

### Circuit Breaker

```rust
use philiprehberger_retry_kit::CircuitBreaker;
use std::time::Duration;

let mut cb = CircuitBreaker::new(5, Duration::from_secs(30))
    .half_open_max_attempts(2); // allow 2 trial requests in half-open state

match cb.call(|| fetch_data()) {
    Ok(data) => println!("Got: {:?}", data),
    Err(e) => eprintln!("Failed: {}", e),
}

// Manually reset the circuit breaker
cb.reset();
```

### Conditional Retry

```rust
use philiprehberger_retry_kit::retry_if;

let result = retry_if(
    RetryOptions::default(),
    || might_fail(),
    |err| err.is_transient(),  // only retry transient errors
);
```

### Retry Callbacks

```rust
let opts = RetryOptions::default()
    .on_retry(|attempt, delay| {
        println!("Retry #{}, waiting {:?}", attempt, delay);
    });

let result = retry(opts, || fetch_data());
```

### Deadline / Total Timeout

Stop retrying after an absolute deadline or a relative timeout, regardless of remaining attempts:

```rust
use std::time::{Duration, Instant};

// Absolute deadline
let opts = RetryOptions::default()
    .max_attempts(10)
    .with_deadline(Instant::now() + Duration::from_secs(30));

let result = retry(opts, || fetch_data());

// Relative timeout (converted to a deadline when the retry loop starts)
let opts = RetryOptions::default()
    .max_attempts(10)
    .with_total_timeout(Duration::from_secs(30));

let result = retry(opts, || fetch_data());
```

Both options can be combined; the earlier of the two takes effect. Deadline support works with `retry()`, `retry_if()`, and `retry_async()`.

### Circuit Breaker Metrics

Inspect cumulative statistics and timing information from a `CircuitBreaker`:

```rust
use philiprehberger_retry_kit::CircuitBreaker;
use std::time::Duration;

let mut cb = CircuitBreaker::new(5, Duration::from_secs(30));

let _ = cb.call(|| ok_or_fail());

// Snapshot of cumulative metrics
let m = cb.metrics();
println!("calls={} ok={} err={}", m.total_calls, m.successes, m.failures);
println!("consecutive_failures={} state={}", m.consecutive_failures, m.state);

// Individual accessors
println!("consecutive: {}", cb.consecutive_failures());
if let Some(t) = cb.last_failure_time() {
    println!("last failure was {:?} ago", t.elapsed());
}
```

Metrics are cumulative and survive `reset()`; only the consecutive failure counter and state are cleared.

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## License

MIT
