# rs-retry-kit

[![CI](https://github.com/philiprehberger/rs-retry-kit/actions/workflows/ci.yml/badge.svg)](https://github.com/philiprehberger/rs-retry-kit/actions/workflows/ci.yml)
[![Crates.io](https://img.shields.io/crates/v/philiprehberger-retry-kit.svg)](https://crates.io/crates/philiprehberger-retry-kit)
[![Last updated](https://img.shields.io/github/last-commit/philiprehberger/rs-retry-kit)](https://github.com/philiprehberger/rs-retry-kit/commits/main)

Async retry with exponential backoff and circuit breaker for Rust

## Installation

```toml
[dependencies]
philiprehberger-retry-kit = "0.5.0"
```

## Usage

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

### Async Circuit Breaker

```rust
use philiprehberger_retry_kit::CircuitBreaker;
use std::time::Duration;

let mut cb = CircuitBreaker::new(5, Duration::from_secs(30));

let result = cb.call_async(|| async {
    fetch_data().await
}).await;
```

### Circuit State Change Callback

```rust
use philiprehberger_retry_kit::{CircuitBreaker, CircuitState};
use std::time::Duration;

let mut cb = CircuitBreaker::new(3, Duration::from_secs(30))
    .on_state_change(|from, to| {
        println!("Circuit: {:?} -> {:?}", from, to);
    });

let _ = cb.call(|| fetch_data());
```

### Retry with Fallback

Try a primary function with retries; if exhausted, try a fallback once:

```rust
use philiprehberger_retry_kit::{retry_with_fallback, RetryOptions};

let result = retry_with_fallback(
    RetryOptions::default(),
    || primary_db_query(),
    || replica_db_query(),  // fallback if primary exhausts retries
);
```

## API

| Function / Type | Description |
|-----------------|-------------|
| `retry(opts, f)` | Retry a synchronous function with the given options |
| `retry_if(opts, f, predicate)` | Retry a synchronous function only when the predicate returns true for the error |
| `retry_async(opts, f)` | Retry an async function (requires `async` feature) |
| `retry_with_fallback(opts, f, fallback)` | Retry primary function, then try fallback once on exhaustion |
| `RetryOptions` | Configuration for retry behavior (max attempts, backoff, delays, jitter, deadline) |
| `RetryOptions::default()` | Create default options (3 attempts, exponential backoff, 1s initial, 30s max, jitter on) |
| `Backoff` | Backoff strategy enum: `Exponential`, `Linear`, `Fixed` |
| `RetryError` | Error returned when all retry attempts are exhausted |
| `CircuitBreaker::new(threshold, timeout)` | Create a circuit breaker with failure threshold and reset timeout |
| `cb.call(f)` | Execute a function through the circuit breaker |
| `cb.call_async(f)` | Execute an async function through the circuit breaker |
| `cb.on_state_change(callback)` | Register a callback for circuit state transitions |
| `cb.reset()` | Manually reset the circuit breaker to closed state |
| `cb.half_open_max_attempts(n)` | Set max trial attempts allowed in half-open state |
| `cb.state()` | Get current circuit state |
| `cb.metrics()` | Get a snapshot of cumulative metrics |
| `cb.consecutive_failures()` | Get current consecutive failure count |
| `cb.last_failure_time()` | Get the time of the last recorded failure |
| `CircuitState` | Circuit state enum: `Closed`, `Open`, `HalfOpen` |
| `CircuitBreakerMetrics` | Snapshot of circuit breaker metrics (total calls, successes, failures, state) |
| `CircuitOpenError` | Error returned when the circuit breaker is open |
| `presets::aggressive()` | Preset: 5 attempts, 500ms initial, 5s max |
| `presets::gentle()` | Preset: 3 attempts, 2s initial, 30s max |
| `presets::network_request()` | Preset: 3 attempts, 1s initial, 10s max |
| `presets::database_query()` | Preset: 3 attempts, linear backoff, 500ms initial, no jitter |

## Development

```bash
cargo test
cargo clippy -- -D warnings
```

## Support

If you find this project useful:

⭐ [Star the repo](https://github.com/philiprehberger/rs-retry-kit)

🐛 [Report issues](https://github.com/philiprehberger/rs-retry-kit/issues?q=is%3Aissue+is%3Aopen+label%3Abug)

💡 [Suggest features](https://github.com/philiprehberger/rs-retry-kit/issues?q=is%3Aissue+is%3Aopen+label%3Aenhancement)

❤️ [Sponsor development](https://github.com/sponsors/philiprehberger)

🌐 [All Open Source Projects](https://philiprehberger.com/open-source-packages)

💻 [GitHub Profile](https://github.com/philiprehberger)

🔗 [LinkedIn Profile](https://www.linkedin.com/in/philiprehberger)

## License

[MIT](LICENSE)
