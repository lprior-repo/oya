//! Token-bucket rate limiter with periodic refill support.

use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

/// Token bucket limiter state.
#[derive(Debug, Clone)]
pub struct TokenBucket {
    capacity: u32,
    tokens: u32,
    refill_per_second: u32,
    last_refill: Instant,
}

impl TokenBucket {
    /// Create a token bucket with initial full capacity.
    pub fn new(capacity: u32, refill_per_second: u32) -> Self {
        Self {
            capacity,
            tokens: capacity,
            refill_per_second,
            last_refill: Instant::now(),
        }
    }

    /// Return currently available tokens.
    pub fn tokens(&self) -> u32 {
        self.tokens
    }

    /// Attempt to consume one token without blocking.
    pub fn try_acquire(&mut self) -> Option<()> {
        self.refill_from_elapsed();
        if self.tokens == 0 {
            return None;
        }
        self.tokens = self.tokens.saturating_sub(1);
        Some(())
    }

    /// Attempt to consume one token.
    pub fn allow(&mut self) -> bool {
        self.try_acquire().is_some()
    }

    /// Manually trigger one refill step.
    pub fn refill_tick(&mut self) {
        self.tokens = self
            .tokens
            .saturating_add(self.refill_per_second)
            .min(self.capacity);
        self.last_refill = Instant::now();
    }

    fn refill_from_elapsed(&mut self) {
        let elapsed = self.last_refill.elapsed();
        let seconds = elapsed.as_secs();
        if seconds == 0 {
            return;
        }

        let increase = seconds
            .saturating_mul(u64::from(self.refill_per_second))
            .min(u64::from(u32::MAX));
        self.tokens = self
            .tokens
            .saturating_add(increase as u32)
            .min(self.capacity);
        self.last_refill = Instant::now();
    }
}

/// Spawn a 1-second refill loop for a token bucket.
pub fn spawn_refill_timer(bucket: Arc<Mutex<TokenBucket>>) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut interval = tokio::time::interval(Duration::from_secs(1));
        loop {
            interval.tick().await;
            let mut guard = bucket.lock().await;
            guard.refill_tick();
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio_test::block_on;

    #[test]
    fn test_token_bucket_allows_until_empty() {
        let mut bucket = TokenBucket::new(3, 1);

        assert!(bucket.allow());
        assert!(bucket.allow());
        assert!(bucket.allow());
        assert!(!bucket.allow());
    }

    #[test]
    fn test_try_acquire_returns_none_when_empty() {
        let mut bucket = TokenBucket::new(1, 0);

        assert_eq!(bucket.try_acquire(), Some(()));
        assert_eq!(bucket.try_acquire(), None);
    }

    #[test]
    fn test_refill_tick_restores_tokens() {
        let mut bucket = TokenBucket::new(5, 2);
        let _ = bucket.allow();
        let _ = bucket.allow();
        assert_eq!(bucket.tokens(), 3);

        bucket.refill_tick();
        assert_eq!(bucket.tokens(), 5);
    }

    #[test]
    fn test_refill_timer_runs_every_second() {
        block_on(async {
            let bucket = Arc::new(Mutex::new(TokenBucket::new(10, 1)));

            {
                let mut guard = bucket.lock().await;
                for _ in 0..5 {
                    let _ = guard.allow();
                }
                assert_eq!(guard.tokens(), 5);
            }

            let handle = spawn_refill_timer(Arc::clone(&bucket));
            tokio::time::sleep(Duration::from_millis(1200)).await;
            handle.abort();

            let guard = bucket.lock().await;
            assert!(guard.tokens() >= 6);
        });
    }
}
