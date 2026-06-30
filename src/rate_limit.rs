use std::sync::Arc;
use std::time::{Duration, Instant};

use tokio::sync::Mutex;

pub struct RateLimiter {
    next_slot: Mutex<Instant>,
    interval: Duration,
}

impl RateLimiter {
    pub fn new(rps: u64) -> Arc<Self> {
        Arc::new(Self {
            next_slot: Mutex::new(Instant::now()),
            interval: Duration::from_secs_f64(1.0 / rps as f64),
        })
    }

    pub async fn acquire(self: &Arc<Self>) {
        let wait = {
            let mut slot = self.next_slot.lock().await;
            let now = Instant::now();
            let scheduled = (*slot).max(now);
            *slot = scheduled + self.interval;
            scheduled.saturating_duration_since(now)
        };
        if !wait.is_zero() {
            tokio::time::sleep(wait).await;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn rate_limiter_caps_throughput() {
        let limiter = RateLimiter::new(50);
        let start = Instant::now();
        for _ in 0..50 {
            limiter.acquire().await;
        }
        let elapsed = start.elapsed();
        assert!(
            elapsed >= Duration::from_millis(900),
            "expected ~1s for 50 rps, got {elapsed:?}"
        );
    }
}
