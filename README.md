# rs-retry-kit

Async retry with exponential backoff and circuit breaker for Rust.

## Installation

```toml
[dependencies]
philiprehberger-retry-kit = "0.1"
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

## License

MIT
