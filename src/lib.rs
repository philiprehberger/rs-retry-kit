use std::fmt;
use std::time::Duration;

/// Backoff strategy for retry delays.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Backoff {
    Exponential,
    Linear,
    Fixed,
}

impl fmt::Display for Backoff {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Backoff::Exponential => write!(f, "exponential"),
            Backoff::Linear => write!(f, "linear"),
            Backoff::Fixed => write!(f, "fixed"),
        }
    }
}

/// Configuration for retry behavior.
pub struct RetryOptions {
    pub max_attempts: u32,
    pub backoff: Backoff,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
    on_retry: Option<Box<dyn Fn(u32, &Duration) + Send + Sync>>,
}

impl fmt::Debug for RetryOptions {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("RetryOptions")
            .field("max_attempts", &self.max_attempts)
            .field("backoff", &self.backoff)
            .field("initial_delay", &self.initial_delay)
            .field("max_delay", &self.max_delay)
            .field("jitter", &self.jitter)
            .field("on_retry", &self.on_retry.as_ref().map(|_| "Fn(u32, &Duration)"))
            .finish()
    }
}

impl Clone for RetryOptions {
    fn clone(&self) -> Self {
        Self {
            max_attempts: self.max_attempts,
            backoff: self.backoff,
            initial_delay: self.initial_delay,
            max_delay: self.max_delay,
            jitter: self.jitter,
            on_retry: None,
        }
    }
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: Backoff::Exponential,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter: true,
            on_retry: None,
        }
    }
}

impl RetryOptions {
    pub fn max_attempts(mut self, n: u32) -> Self {
        self.max_attempts = n;
        self
    }

    pub fn backoff(mut self, b: Backoff) -> Self {
        self.backoff = b;
        self
    }

    pub fn initial_delay(mut self, d: Duration) -> Self {
        self.initial_delay = d;
        self
    }

    pub fn max_delay(mut self, d: Duration) -> Self {
        self.max_delay = d;
        self
    }

    pub fn jitter(mut self, j: bool) -> Self {
        self.jitter = j;
        self
    }

    pub fn on_retry<F>(mut self, f: F) -> Self
    where
        F: Fn(u32, &Duration) + Send + Sync + 'static,
    {
        self.on_retry = Some(Box::new(f));
        self
    }
}

/// Error returned when all retry attempts are exhausted.
#[derive(Debug)]
pub struct RetryError {
    pub attempts: u32,
    pub last_error: Box<dyn std::error::Error + Send + Sync>,
}

impl fmt::Display for RetryError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "all {} attempts failed: {}", self.attempts, self.last_error)
    }
}

impl std::error::Error for RetryError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(self.last_error.as_ref())
    }
}

fn calculate_delay(attempt: u32, opts: &RetryOptions) -> Duration {
    let base = match opts.backoff {
        Backoff::Exponential => {
            let exponent = (attempt - 1).min(31);
            opts.initial_delay.saturating_mul(2u32.saturating_pow(exponent))
        }
        Backoff::Linear => opts.initial_delay.saturating_mul(attempt),
        Backoff::Fixed => opts.initial_delay,
    };

    let capped = base.min(opts.max_delay);

    if opts.jitter {
        let factor = 0.5 + rand::random::<f64>() * 0.5;
        capped.mul_f64(factor)
    } else {
        capped
    }
}

/// Retry a synchronous function.
pub fn retry<T, E, F>(opts: RetryOptions, mut f: F) -> Result<T, RetryError>
where
    E: std::error::Error + Send + Sync + 'static,
    F: FnMut() -> Result<T, E>,
{
    let mut last_error = None;

    for attempt in 1..=opts.max_attempts {
        match f() {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_error = Some(e);
                if attempt < opts.max_attempts {
                    let delay = calculate_delay(attempt, &opts);
                    if let Some(ref cb) = opts.on_retry {
                        cb(attempt, &delay);
                    }
                    std::thread::sleep(delay);
                }
            }
        }
    }

    Err(RetryError {
        attempts: opts.max_attempts,
        last_error: Box::new(last_error.unwrap()),
    })
}

/// Retry a synchronous function, but only retry when the predicate returns true for the error.
pub fn retry_if<T, E, F, P>(opts: RetryOptions, mut f: F, predicate: P) -> Result<T, RetryError>
where
    E: std::error::Error + Send + Sync + 'static,
    F: FnMut() -> Result<T, E>,
    P: Fn(&E) -> bool,
{
    let mut last_error = None;
    let mut actual_attempts = 0;

    for attempt in 1..=opts.max_attempts {
        actual_attempts = attempt;
        match f() {
            Ok(val) => return Ok(val),
            Err(e) => {
                let should_retry = predicate(&e);
                last_error = Some(e);
                if !should_retry || attempt >= opts.max_attempts {
                    break;
                }
                let delay = calculate_delay(attempt, &opts);
                if let Some(ref cb) = opts.on_retry {
                    cb(attempt, &delay);
                }
                std::thread::sleep(delay);
            }
        }
    }

    Err(RetryError {
        attempts: actual_attempts,
        last_error: Box::new(last_error.unwrap()),
    })
}

/// Retry an async function (requires `async` feature).
#[cfg(feature = "async")]
pub async fn retry_async<T, E, F, Fut>(opts: RetryOptions, mut f: F) -> Result<T, RetryError>
where
    E: std::error::Error + Send + Sync + 'static,
    F: FnMut() -> Fut,
    Fut: std::future::Future<Output = Result<T, E>>,
{
    let mut last_error = None;

    for attempt in 1..=opts.max_attempts {
        match f().await {
            Ok(val) => return Ok(val),
            Err(e) => {
                last_error = Some(e);
                if attempt < opts.max_attempts {
                    let delay = calculate_delay(attempt, &opts);
                    if let Some(ref cb) = opts.on_retry {
                        cb(attempt, &delay);
                    }
                    tokio::time::sleep(delay).await;
                }
            }
        }
    }

    Err(RetryError {
        attempts: opts.max_attempts,
        last_error: Box::new(last_error.unwrap()),
    })
}

/// Preset configurations.
pub mod presets {
    use super::*;

    pub fn aggressive() -> RetryOptions {
        RetryOptions::default()
            .max_attempts(5)
            .backoff(Backoff::Exponential)
            .initial_delay(Duration::from_millis(500))
            .max_delay(Duration::from_secs(5))
            .jitter(true)
    }

    pub fn gentle() -> RetryOptions {
        RetryOptions::default()
            .max_attempts(3)
            .backoff(Backoff::Exponential)
            .initial_delay(Duration::from_secs(2))
            .max_delay(Duration::from_secs(30))
            .jitter(true)
    }

    pub fn network_request() -> RetryOptions {
        RetryOptions::default()
            .max_attempts(3)
            .backoff(Backoff::Exponential)
            .initial_delay(Duration::from_secs(1))
            .max_delay(Duration::from_secs(10))
            .jitter(true)
    }

    pub fn database_query() -> RetryOptions {
        RetryOptions::default()
            .max_attempts(3)
            .backoff(Backoff::Linear)
            .initial_delay(Duration::from_millis(500))
            .max_delay(Duration::from_secs(5))
            .jitter(false)
    }
}

/// Circuit breaker state.
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum CircuitState {
    Closed,
    Open,
    HalfOpen,
}

impl fmt::Display for CircuitState {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CircuitState::Closed => write!(f, "closed"),
            CircuitState::Open => write!(f, "open"),
            CircuitState::HalfOpen => write!(f, "half_open"),
        }
    }
}

/// Error when the circuit breaker is open.
#[derive(Debug)]
pub struct CircuitOpenError;

impl fmt::Display for CircuitOpenError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "circuit breaker is open — request rejected")
    }
}

impl std::error::Error for CircuitOpenError {}

/// A circuit breaker that tracks failures and short-circuits when a threshold is reached.
pub struct CircuitBreaker {
    failure_threshold: u32,
    reset_timeout: Duration,
    half_open_max_attempts: u32,
    state: CircuitState,
    failures: u32,
    last_failure_time: Option<std::time::Instant>,
    half_open_attempts: u32,
}

impl fmt::Debug for CircuitBreaker {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("CircuitBreaker")
            .field("failure_threshold", &self.failure_threshold)
            .field("reset_timeout", &self.reset_timeout)
            .field("half_open_max_attempts", &self.half_open_max_attempts)
            .field("state", &self.state)
            .field("failures", &self.failures)
            .finish()
    }
}

impl CircuitBreaker {
    pub fn new(failure_threshold: u32, reset_timeout: Duration) -> Self {
        Self {
            failure_threshold,
            reset_timeout,
            half_open_max_attempts: 1,
            state: CircuitState::Closed,
            failures: 0,
            last_failure_time: None,
            half_open_attempts: 0,
        }
    }

    /// Set the maximum number of trial attempts allowed in the half-open state.
    pub fn half_open_max_attempts(mut self, n: u32) -> Self {
        self.half_open_max_attempts = n;
        self
    }

    pub fn state(&self) -> CircuitState {
        self.state
    }

    /// Returns the current failure count.
    pub fn failures(&self) -> u32 {
        self.failures
    }

    /// Returns the failure threshold.
    pub fn failure_threshold(&self) -> u32 {
        self.failure_threshold
    }

    /// Reset the circuit breaker to the closed state.
    pub fn reset(&mut self) {
        self.state = CircuitState::Closed;
        self.failures = 0;
        self.last_failure_time = None;
        self.half_open_attempts = 0;
    }

    pub fn call<T, E, F>(&mut self, f: F) -> Result<T, Box<dyn std::error::Error>>
    where
        E: std::error::Error + 'static,
        F: FnOnce() -> Result<T, E>,
    {
        if self.state == CircuitState::Open {
            if let Some(last) = self.last_failure_time {
                if last.elapsed() >= self.reset_timeout {
                    self.state = CircuitState::HalfOpen;
                    self.half_open_attempts = 0;
                } else {
                    return Err(Box::new(CircuitOpenError));
                }
            }
        }

        if self.state == CircuitState::HalfOpen
            && self.half_open_attempts >= self.half_open_max_attempts
        {
            return Err(Box::new(CircuitOpenError));
        }

        if self.state == CircuitState::HalfOpen {
            self.half_open_attempts += 1;
        }

        match f() {
            Ok(val) => {
                if self.state == CircuitState::HalfOpen {
                    self.state = CircuitState::Closed;
                }
                self.failures = 0;
                Ok(val)
            }
            Err(e) => {
                self.failures += 1;
                self.last_failure_time = Some(std::time::Instant::now());

                if self.state == CircuitState::HalfOpen || self.failures >= self.failure_threshold {
                    self.state = CircuitState::Open;
                }

                Err(Box::new(e))
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Debug)]
    struct TestError(String);

    impl fmt::Display for TestError {
        fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
            write!(f, "{}", self.0)
        }
    }

    impl std::error::Error for TestError {}

    #[test]
    fn test_retry_success_first_try() {
        let result = retry(
            RetryOptions::default().max_attempts(3).jitter(false),
            || Ok::<_, TestError>(42),
        );
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_retry_success_after_failures() {
        let mut attempts = 0;
        let result = retry(
            RetryOptions::default()
                .max_attempts(3)
                .initial_delay(Duration::from_millis(1))
                .jitter(false),
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(TestError("fail".into()))
                } else {
                    Ok(42)
                }
            },
        );
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_retry_all_attempts_exhausted() {
        let result = retry(
            RetryOptions::default()
                .max_attempts(2)
                .initial_delay(Duration::from_millis(1))
                .jitter(false),
            || Err::<i32, _>(TestError("always fails".into())),
        );
        let err = result.unwrap_err();
        assert_eq!(err.attempts, 2);
        assert!(err.to_string().contains("always fails"));
    }

    #[test]
    fn test_fixed_backoff_delay() {
        let opts = RetryOptions::default()
            .backoff(Backoff::Fixed)
            .initial_delay(Duration::from_millis(100))
            .jitter(false);
        let d1 = calculate_delay(1, &opts);
        let d2 = calculate_delay(5, &opts);
        assert_eq!(d1, Duration::from_millis(100));
        assert_eq!(d2, Duration::from_millis(100));
    }

    #[test]
    fn test_linear_backoff_delay() {
        let opts = RetryOptions::default()
            .backoff(Backoff::Linear)
            .initial_delay(Duration::from_millis(100))
            .max_delay(Duration::from_secs(10))
            .jitter(false);
        assert_eq!(calculate_delay(1, &opts), Duration::from_millis(100));
        assert_eq!(calculate_delay(3, &opts), Duration::from_millis(300));
    }

    #[test]
    fn test_exponential_backoff_delay() {
        let opts = RetryOptions::default()
            .backoff(Backoff::Exponential)
            .initial_delay(Duration::from_millis(100))
            .max_delay(Duration::from_secs(10))
            .jitter(false);
        assert_eq!(calculate_delay(1, &opts), Duration::from_millis(100));
        assert_eq!(calculate_delay(2, &opts), Duration::from_millis(200));
        assert_eq!(calculate_delay(3, &opts), Duration::from_millis(400));
    }

    #[test]
    fn test_delay_capped_at_max() {
        let opts = RetryOptions::default()
            .backoff(Backoff::Exponential)
            .initial_delay(Duration::from_secs(1))
            .max_delay(Duration::from_secs(5))
            .jitter(false);
        let delay = calculate_delay(10, &opts);
        assert!(delay <= Duration::from_secs(5));
    }

    #[test]
    fn test_exponential_overflow_protection() {
        let opts = RetryOptions::default()
            .backoff(Backoff::Exponential)
            .initial_delay(Duration::from_secs(1))
            .max_delay(Duration::from_secs(60))
            .jitter(false);
        // Should not panic even with high attempt counts
        let delay = calculate_delay(100, &opts);
        assert!(delay <= Duration::from_secs(60));
    }

    #[test]
    fn test_backoff_display() {
        assert_eq!(format!("{}", Backoff::Exponential), "exponential");
        assert_eq!(format!("{}", Backoff::Linear), "linear");
        assert_eq!(format!("{}", Backoff::Fixed), "fixed");
    }

    #[test]
    fn test_presets() {
        let a = presets::aggressive();
        assert_eq!(a.max_attempts, 5);

        let g = presets::gentle();
        assert_eq!(g.max_attempts, 3);
        assert_eq!(g.initial_delay, Duration::from_secs(2));

        let n = presets::network_request();
        assert_eq!(n.max_delay, Duration::from_secs(10));

        let d = presets::database_query();
        assert_eq!(d.backoff, Backoff::Linear);
        assert!(!d.jitter);
    }

    #[test]
    fn test_circuit_breaker_closed_on_success() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let result = cb.call(|| Ok::<_, TestError>(42));
        assert_eq!(result.unwrap(), 42);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_opens_on_threshold() {
        let mut cb = CircuitBreaker::new(2, Duration::from_secs(30));
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        assert_eq!(cb.state(), CircuitState::Closed);
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_breaker_rejects_when_open() {
        let mut cb = CircuitBreaker::new(1, Duration::from_secs(300));
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        assert_eq!(cb.state(), CircuitState::Open);
        let result = cb.call(|| Ok::<_, TestError>(42));
        assert!(result.is_err());
    }

    #[test]
    fn test_circuit_breaker_half_open_recovery() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(1));
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        assert_eq!(cb.state(), CircuitState::Open);

        std::thread::sleep(Duration::from_millis(10));

        let result = cb.call(|| Ok::<_, TestError>(42));
        assert_eq!(result.unwrap(), 42);
        assert_eq!(cb.state(), CircuitState::Closed);
    }

    #[test]
    fn test_circuit_breaker_reset() {
        let mut cb = CircuitBreaker::new(1, Duration::from_secs(300));
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        assert_eq!(cb.state(), CircuitState::Open);

        cb.reset();
        assert_eq!(cb.state(), CircuitState::Closed);

        let result = cb.call(|| Ok::<_, TestError>(42));
        assert_eq!(result.unwrap(), 42);
    }

    #[test]
    fn test_circuit_breaker_configurable_half_open() {
        let mut cb = CircuitBreaker::new(1, Duration::from_millis(1)).half_open_max_attempts(2);
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        std::thread::sleep(Duration::from_millis(10));

        // First half-open attempt fails
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        // With max_attempts=2, it would re-open on failure in half-open
        assert_eq!(cb.state(), CircuitState::Open);
    }

    #[test]
    fn test_circuit_state_display() {
        assert_eq!(format!("{}", CircuitState::Closed), "closed");
        assert_eq!(format!("{}", CircuitState::Open), "open");
        assert_eq!(format!("{}", CircuitState::HalfOpen), "half_open");
    }

    #[test]
    fn test_retry_error_source() {
        let result = retry(
            RetryOptions::default()
                .max_attempts(1)
                .jitter(false),
            || Err::<i32, _>(TestError("root cause".into())),
        );
        let err = result.unwrap_err();
        assert!(std::error::Error::source(&err).is_some());
    }

    #[test]
    fn test_retry_if_skips_non_retryable() {
        let mut attempts = 0;
        let result = retry_if(
            RetryOptions::default()
                .max_attempts(5)
                .initial_delay(Duration::from_millis(1))
                .jitter(false),
            || {
                attempts += 1;
                Err::<i32, _>(TestError("permanent".into()))
            },
            |e: &TestError| e.0 != "permanent",
        );
        assert!(result.is_err());
        assert_eq!(attempts, 1);
    }

    #[test]
    fn test_retry_if_retries_retryable() {
        let mut attempts = 0;
        let result = retry_if(
            RetryOptions::default()
                .max_attempts(3)
                .initial_delay(Duration::from_millis(1))
                .jitter(false),
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(TestError("transient".into()))
                } else {
                    Ok(42)
                }
            },
            |e: &TestError| e.0 == "transient",
        );
        assert_eq!(result.unwrap(), 42);
        assert_eq!(attempts, 3);
    }

    #[test]
    fn test_on_retry_callback() {
        use std::sync::{Arc, Mutex};
        let log = Arc::new(Mutex::new(Vec::new()));
        let log2 = log.clone();

        let mut attempts = 0;
        let _ = retry(
            RetryOptions::default()
                .max_attempts(3)
                .initial_delay(Duration::from_millis(1))
                .jitter(false)
                .on_retry(move |attempt, _delay| {
                    log2.lock().unwrap().push(attempt);
                }),
            || {
                attempts += 1;
                if attempts < 3 {
                    Err(TestError("fail".into()))
                } else {
                    Ok(42)
                }
            },
        );

        let logged = log.lock().unwrap();
        assert_eq!(*logged, vec![1, 2]);
    }

    #[test]
    fn test_circuit_breaker_debug() {
        let cb = CircuitBreaker::new(3, Duration::from_secs(30));
        let debug = format!("{:?}", cb);
        assert!(debug.contains("CircuitBreaker"));
        assert!(debug.contains("failure_threshold"));
    }

    #[test]
    fn test_circuit_breaker_failures_getter() {
        let mut cb = CircuitBreaker::new(3, Duration::from_secs(30));
        assert_eq!(cb.failures(), 0);
        assert_eq!(cb.failure_threshold(), 3);
        let _ = cb.call(|| Err::<i32, _>(TestError("fail".into())));
        assert_eq!(cb.failures(), 1);
    }
}
