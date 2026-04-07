//! Integration tests for philiprehberger-retry-kit.
//!
//! These tests exercise the public API end-to-end, complementing the
//! unit tests in `src/lib.rs`.

use std::cell::Cell;
use std::fmt;
use std::sync::atomic::{AtomicU32, Ordering};
use std::time::{Duration, Instant};

use philiprehberger_retry_kit::{retry, Backoff, RetryOptions};

#[derive(Debug)]
struct TestError(String);

impl fmt::Display for TestError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.0)
    }
}

impl std::error::Error for TestError {}

#[test]
fn retry_succeeds_after_transient_failures() {
    let counter: Cell<u32> = Cell::new(0);

    let opts = RetryOptions::default()
        .max_attempts(5)
        .backoff(Backoff::Fixed)
        .initial_delay(Duration::from_millis(1))
        .max_delay(Duration::from_millis(1))
        .jitter(false);

    let result = retry(opts, || {
        let n = counter.get() + 1;
        counter.set(n);
        if n < 3 {
            Err(TestError(format!("transient {n}")))
        } else {
            Ok::<_, TestError>(n)
        }
    });

    assert_eq!(result.unwrap(), 3);
    assert_eq!(counter.get(), 3);
}

#[test]
fn retry_exhaustion_returns_error_with_max_attempts() {
    let counter = AtomicU32::new(0);

    let opts = RetryOptions::default()
        .max_attempts(4)
        .backoff(Backoff::Fixed)
        .initial_delay(Duration::from_millis(1))
        .max_delay(Duration::from_millis(1))
        .jitter(false);

    let result = retry(opts, || {
        counter.fetch_add(1, Ordering::SeqCst);
        Err::<i32, _>(TestError("always fails".into()))
    });

    let err = result.expect_err("expected exhaustion");
    assert_eq!(err.attempts, 4);
    assert_eq!(counter.load(Ordering::SeqCst), 4);
}

#[test]
fn exponential_backoff_elapsed_time_in_expected_range() {
    // 3 attempts, initial=20ms, exponential: sleeps ~20ms + ~40ms = ~60ms.
    let opts = RetryOptions::default()
        .max_attempts(3)
        .backoff(Backoff::Exponential)
        .initial_delay(Duration::from_millis(20))
        .max_delay(Duration::from_secs(5))
        .jitter(false);

    let start = Instant::now();
    let result = retry(opts, || Err::<i32, _>(TestError("fail".into())));
    let elapsed = start.elapsed();

    assert!(result.is_err());
    // Lower bound: sum of sleeps (~60ms). Upper bound generous for CI.
    assert!(
        elapsed >= Duration::from_millis(55),
        "elapsed {elapsed:?} shorter than expected lower bound"
    );
    assert!(
        elapsed < Duration::from_millis(1500),
        "elapsed {elapsed:?} longer than expected upper bound"
    );
}

#[test]
fn jitter_keeps_delay_within_bounds() {
    // With jitter enabled, calculate_delay() is internal, so we measure
    // total elapsed time across two identical runs and assert both fall
    // within the jitter bounds (0.5x..1.0x of the capped base delay).
    //
    // 3 attempts, initial=40ms, fixed backoff => base sleeps = 2 * 40ms = 80ms.
    // Jitter scales each sleep by [0.5, 1.0], so total sleeps range over
    // roughly [40ms, 80ms].
    let make_opts = || {
        RetryOptions::default()
            .max_attempts(3)
            .backoff(Backoff::Fixed)
            .initial_delay(Duration::from_millis(40))
            .max_delay(Duration::from_millis(40))
            .jitter(true)
    };

    let run_once = || {
        let start = Instant::now();
        let _ = retry(make_opts(), || Err::<i32, _>(TestError("fail".into())));
        start.elapsed()
    };

    let a = run_once();
    let b = run_once();

    // Bounds: lower 0.5 * 80ms = 40ms (minus slack), upper generous for CI.
    for e in [a, b] {
        assert!(
            e >= Duration::from_millis(35),
            "elapsed {e:?} below jitter lower bound"
        );
        assert!(
            e < Duration::from_millis(1500),
            "elapsed {e:?} above jitter upper bound"
        );
    }
}

#[cfg(feature = "async")]
mod async_tests {
    use super::*;
    use philiprehberger_retry_kit::retry_async;

    #[tokio::test]
    async fn retry_async_succeeds_after_transient_failures() {
        let counter = AtomicU32::new(0);

        let opts = RetryOptions::default()
            .max_attempts(5)
            .backoff(Backoff::Fixed)
            .initial_delay(Duration::from_millis(1))
            .max_delay(Duration::from_millis(1))
            .jitter(false);

        let result = retry_async(opts, || async {
            let n = counter.fetch_add(1, Ordering::SeqCst) + 1;
            if n < 3 {
                Err(TestError(format!("transient {n}")))
            } else {
                Ok::<_, TestError>(n)
            }
        })
        .await;

        assert_eq!(result.unwrap(), 3);
        assert_eq!(counter.load(Ordering::SeqCst), 3);
    }

    #[tokio::test]
    async fn retry_async_exhaustion_returns_error() {
        let opts = RetryOptions::default()
            .max_attempts(3)
            .backoff(Backoff::Fixed)
            .initial_delay(Duration::from_millis(1))
            .max_delay(Duration::from_millis(1))
            .jitter(false);

        let result = retry_async(opts, || async {
            Err::<i32, _>(TestError("always fails".into()))
        })
        .await;

        let err = result.expect_err("expected exhaustion");
        assert_eq!(err.attempts, 3);
    }
}
