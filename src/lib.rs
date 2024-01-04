use std::fmt;
use std::time::Duration;

/// Backoff strategy for retry delays.
#[derive(Debug, Clone, Copy)]
pub enum Backoff {
    Exponential,
    Linear,
    Fixed,
}

/// Configuration for retry behavior.
#[derive(Debug, Clone)]
pub struct RetryOptions {
    pub max_attempts: u32,
    pub backoff: Backoff,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub jitter: bool,
}

impl Default for RetryOptions {
    fn default() -> Self {
        Self {
            max_attempts: 3,
            backoff: Backoff::Exponential,
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(30),
            jitter: true,
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
        Backoff::Exponential => opts.initial_delay.mul_f64(2f64.powi(attempt as i32 - 1)),
        Backoff::Linear => opts.initial_delay * attempt,
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

    pub fn state(&self) -> CircuitState {
        self.state
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

        if self.state == CircuitState::HalfOpen && self.half_open_attempts >= self.half_open_max_attempts {
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
