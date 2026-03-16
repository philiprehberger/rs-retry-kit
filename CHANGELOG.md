# Changelog
n## 0.3.4 (2026-03-16)

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
