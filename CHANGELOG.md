# Changelog

## 0.6.0 (2026-04-06)

- Upgrade `rand` from 0.8 to 0.10
- Update CI checkout action to v6

## 0.5.0 (2026-04-06)

- Add `call_async()` on `CircuitBreaker` for async circuit breaker support (requires `async` feature)
- Add `on_state_change()` callback on `CircuitBreaker` for observing state transitions
- Add `retry_with_fallback()` for retrying with a fallback function on exhaustion

## 0.4.4 (2026-03-31)

- Standardize README to 3-badge format with emoji Support section
- Update CI checkout action to v5 for Node.js 24 compatibility

## 0.4.3 (2026-03-27)

- Add GitHub issue templates, PR template, and dependabot configuration
- Update README badges and add Support section

## 0.4.2 (2026-03-22)

- Fix README and CHANGELOG compliance

## 0.4.1 (2026-03-20)

- Add crate-level doc comment with usage example

## 0.4.0 (2026-03-17)

- Add `with_deadline()` on `RetryOptions` — absolute deadline after which retries stop
- Add `with_total_timeout()` on `RetryOptions` — relative timeout from the start of execution
- Deadline checks apply to `retry()`, `retry_if()`, and `retry_async()`
- Add `CircuitBreakerMetrics` struct with `total_calls`, `successes`, `failures`, `consecutive_failures`, and `state`
- Add `metrics()` method on `CircuitBreaker` returning a metrics snapshot
- Add `consecutive_failures()` getter on `CircuitBreaker`
- Add `last_failure_time()` getter on `CircuitBreaker`

## 0.3.5 (2026-03-17)

- Add readme, rust-version, documentation to Cargo.toml
- Add Development section to README

## 0.3.4 (2026-03-16)

- Update install snippet to use full version

## 0.3.3 (2026-03-16)

- Add README badges
- Synchronize version across Cargo.toml, README, and CHANGELOG

## 0.3.0 (2026-03-13)

- Add `retry_if()` function — retries only when a predicate returns true for the error
- Add `on_retry` callback to `RetryOptions` — observe retries with attempt number and delay
- Add `Debug` impl for `CircuitBreaker`
- Add `failures()` and `failure_threshold()` getters on `CircuitBreaker`

## 0.2.0 (2026-03-12)

- Add `Display` trait for `Backoff` enum
- Add `PartialEq` and `Eq` derives for `Backoff`
- Add `reset()` method to `CircuitBreaker` for manual state reset
- Add configurable `half_open_max_attempts()` on `CircuitBreaker`
- Add overflow protection for exponential backoff calculation
- Add comprehensive test suite covering retry, backoff strategies, circuit breaker states

## 0.1.0 (2026-03-09)

- Initial release
