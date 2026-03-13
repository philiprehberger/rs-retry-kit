# Changelog

## 0.2.0

- Add `Display` trait for `Backoff` enum
- Add `PartialEq` and `Eq` derives for `Backoff`
- Add `reset()` method to `CircuitBreaker` for manual state reset
- Add configurable `half_open_max_attempts()` on `CircuitBreaker`
- Add overflow protection for exponential backoff calculation
- Add comprehensive test suite covering retry, backoff strategies, circuit breaker states

## 0.1.0

- Initial release
