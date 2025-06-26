use std::sync::{Arc, Mutex as StdMutex};
use std::time::{Duration, Instant};
use tokio::sync::{Mutex, Semaphore};
use tokio::time::sleep;

/// A token bucket rate limiter that enforces global rate limits across all agents
#[derive(Clone, Debug)]
pub struct TokenBucketRateLimiter {
    inner: Arc<RateLimiterInner>,
}

#[derive(Debug)]
struct RateLimiterInner {
    /// Maximum number of tokens in the bucket
    capacity: u32,
    /// Current number of tokens available
    tokens: Mutex<u32>,
    /// Rate at which tokens are replenished (tokens per second)
    refill_rate: f64,
    /// Last time tokens were refilled
    last_refill: Mutex<Instant>,
    /// Semaphore to control concurrent access and waiting
    semaphore: Arc<Semaphore>,
}

impl TokenBucketRateLimiter {
    /// Create a new token bucket rate limiter
    ///
    /// # Arguments
    /// * `requests_per_second` - Maximum number of requests allowed per second
    /// * `burst_capacity` - Maximum burst capacity (defaults to requests_per_second if None)
    pub fn new(requests_per_second: f64, burst_capacity: Option<u32>) -> Self {
        let capacity = burst_capacity.unwrap_or(requests_per_second.ceil() as u32);

        Self {
            inner: Arc::new(RateLimiterInner {
                capacity,
                tokens: Mutex::new(capacity),
                refill_rate: requests_per_second,
                last_refill: Mutex::new(Instant::now()),
                semaphore: Arc::new(Semaphore::new(capacity as usize)),
            }),
        }
    }

    /// Acquire a permit to make a request
    /// This method will block until a token is available
    pub async fn acquire(&self) -> RateLimitPermit {
        // First, acquire a semaphore permit to limit concurrent waiters
        let _permit = self
            .inner
            .semaphore
            .clone()
            .acquire_owned()
            .await
            .expect("Semaphore should not be closed");

        // Wait until we can get a token
        loop {
            if self.try_acquire_token().await {
                return RateLimitPermit { _permit };
            }

            // Calculate how long to wait before trying again
            let wait_time = self.calculate_wait_time().await;
            sleep(wait_time).await;
        }
    }

    /// Try to acquire a token without blocking
    async fn try_acquire_token(&self) -> bool {
        self.refill_tokens().await;

        let mut tokens = self.inner.tokens.lock().await;
        if *tokens > 0 {
            *tokens -= 1;
            true
        } else {
            false
        }
    }

    /// Refill tokens based on elapsed time
    async fn refill_tokens(&self) {
        let now = Instant::now();
        let mut last_refill = self.inner.last_refill.lock().await;
        let elapsed = now.duration_since(*last_refill);

        if elapsed.as_millis() > 0 {
            let tokens_to_add = (elapsed.as_secs_f64() * self.inner.refill_rate) as u32;

            if tokens_to_add > 0 {
                let mut tokens = self.inner.tokens.lock().await;
                *tokens = (*tokens + tokens_to_add).min(self.inner.capacity);
                *last_refill = now;
            }
        }
    }

    /// Calculate how long to wait before the next token becomes available
    async fn calculate_wait_time(&self) -> Duration {
        let tokens = self.inner.tokens.lock().await;
        if *tokens > 0 {
            // Tokens available, no need to wait
            Duration::from_millis(1)
        } else {
            // Calculate time until next token
            let time_per_token = 1.0 / self.inner.refill_rate;
            Duration::from_secs_f64(time_per_token)
        }
    }

    /// Get current statistics about the rate limiter
    pub async fn stats(&self) -> RateLimiterStats {
        let tokens = *self.inner.tokens.lock().await;
        let available_permits = self.inner.semaphore.available_permits();

        RateLimiterStats {
            available_tokens: tokens,
            capacity: self.inner.capacity,
            refill_rate: self.inner.refill_rate,
            waiting_requests: self.inner.capacity as usize - available_permits,
        }
    }
}

/// A permit that represents permission to make one request
/// The permit is automatically released when dropped
pub struct RateLimitPermit {
    _permit: tokio::sync::OwnedSemaphorePermit,
}

/// Statistics about the rate limiter's current state
#[derive(Debug, Clone)]
pub struct RateLimiterStats {
    pub available_tokens: u32,
    pub capacity: u32,
    pub refill_rate: f64,
    pub waiting_requests: usize,
}

/// Global rate limiter instance
static GLOBAL_RATE_LIMITER: StdMutex<Option<TokenBucketRateLimiter>> = StdMutex::new(None);

/// Initialize the global rate limiter
pub fn initialize_global_rate_limiter(requests_per_second: f64, burst_capacity: Option<u32>) {
    let limiter = TokenBucketRateLimiter::new(requests_per_second, burst_capacity);
    *GLOBAL_RATE_LIMITER.lock().unwrap() = Some(limiter);
}

/// Disable the global rate limiter (for cases where rate limiting is not needed)
pub fn disable_global_rate_limiter() {
    *GLOBAL_RATE_LIMITER.lock().unwrap() = None;
}

/// Get the global rate limiter instance
pub fn get_global_rate_limiter() -> Option<TokenBucketRateLimiter> {
    GLOBAL_RATE_LIMITER.lock().unwrap().clone()
}

/// Acquire a permit from the global rate limiter if it's enabled
pub async fn acquire_global_permit() -> Option<RateLimitPermit> {
    match get_global_rate_limiter() {
        Some(limiter) => Some(limiter.acquire().await),
        None => None,
    }
}

/// Get statistics from the global rate limiter if it's enabled
pub async fn global_rate_limiter_stats() -> Option<RateLimiterStats> {
    match get_global_rate_limiter() {
        Some(limiter) => Some(limiter.stats().await),
        None => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::{sleep, Duration};

    #[tokio::test]
    async fn test_token_bucket_basic() {
        let limiter = TokenBucketRateLimiter::new(2.0, Some(2));

        // Should be able to acquire 2 tokens immediately
        let _permit1 = limiter.acquire().await;
        let _permit2 = limiter.acquire().await;

        // Third acquisition should take some time
        let start = Instant::now();
        let _permit3 = limiter.acquire().await;
        let elapsed = start.elapsed();

        // Should have waited approximately 0.5 seconds (1/2 tokens per second)
        assert!(elapsed >= Duration::from_millis(400));
        assert!(elapsed <= Duration::from_millis(600));
    }

    #[tokio::test]
    async fn test_token_bucket_refill() {
        let limiter = TokenBucketRateLimiter::new(10.0, Some(1));

        // Acquire the only token
        let _permit1 = limiter.acquire().await;

        // Wait for refill
        sleep(Duration::from_millis(200)).await;

        // Should be able to acquire another token quickly
        let start = Instant::now();
        let _permit2 = limiter.acquire().await;
        let elapsed = start.elapsed();

        // Should be very fast since token was refilled
        assert!(elapsed <= Duration::from_millis(50));
    }

    #[tokio::test]
    async fn test_stats() {
        let limiter = TokenBucketRateLimiter::new(5.0, Some(10));

        let stats = limiter.stats().await;
        assert_eq!(stats.capacity, 10);
        assert_eq!(stats.available_tokens, 10);
        assert_eq!(stats.refill_rate, 5.0);
        assert_eq!(stats.waiting_requests, 0);

        // Acquire some tokens
        let _permit1 = limiter.acquire().await;
        let _permit2 = limiter.acquire().await;

        let stats = limiter.stats().await;
        assert_eq!(stats.available_tokens, 8);
    }

    #[tokio::test]
    async fn test_global_rate_limiter() {
        // Initialize global rate limiter
        initialize_global_rate_limiter(1.0, Some(1));

        // Should be able to acquire a permit
        let permit = acquire_global_permit().await;
        assert!(permit.is_some());

        // Stats should be available
        let stats = global_rate_limiter_stats().await;
        assert!(stats.is_some());
        assert_eq!(stats.unwrap().capacity, 1);
    }
}
