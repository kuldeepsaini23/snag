use std::sync::Mutex;
use std::sync::atomic::{AtomicU64, Ordering};
use tokio::time::{Duration, Instant};

pub struct RateLimiter {
    rate: AtomicU64,
    bucket: Mutex<Bucket>,
}

struct Bucket {
    tokens: f64,
    last: Instant,
}

impl RateLimiter {
    /// `bytes_per_sec == 0` means unlimited. Burst capacity is one second of traffic.
    pub fn new(bytes_per_sec: u64) -> Self {
        Self {
            rate: AtomicU64::new(bytes_per_sec),
            bucket: Mutex::new(Bucket { tokens: bytes_per_sec as f64, last: Instant::now() }),
        }
    }

    pub fn set_limit(&self, bytes_per_sec: u64) {
        self.rate.store(bytes_per_sec, Ordering::Relaxed);
    }

    pub fn limit(&self) -> u64 {
        self.rate.load(Ordering::Relaxed)
    }

    /// Takes `bytes` from the bucket, sleeping off any deficit. Tokens may go
    /// negative so concurrent callers queue up fairly behind each other.
    pub async fn acquire(&self, bytes: u64) {
        let rate = self.limit();
        if rate == 0 {
            return;
        }
        let wait = {
            let rate = rate as f64;
            let mut b = self.bucket.lock().unwrap();
            let now = Instant::now();
            b.tokens = (b.tokens + now.duration_since(b.last).as_secs_f64() * rate).min(rate);
            b.last = now;
            b.tokens -= bytes as f64;
            if b.tokens >= 0.0 {
                return;
            }
            Duration::from_secs_f64(-b.tokens / rate)
        };
        tokio::time::sleep(wait).await;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test(start_paused = true)]
    async fn unlimited_never_waits() {
        let l = RateLimiter::new(0);
        let t = Instant::now();
        l.acquire(100 * 1024 * 1024).await;
        assert_eq!(t.elapsed(), Duration::ZERO);
    }

    #[tokio::test(start_paused = true)]
    async fn allows_one_second_burst_then_throttles() {
        let l = RateLimiter::new(1000);
        let t = Instant::now();
        l.acquire(1000).await;
        assert_eq!(t.elapsed(), Duration::ZERO);
        l.acquire(2000).await;
        let e = t.elapsed().as_secs_f64();
        assert!((1.99..=2.01).contains(&e), "elapsed {e}");
    }

    #[tokio::test(start_paused = true)]
    async fn concurrent_callers_share_the_budget() {
        let l = std::sync::Arc::new(RateLimiter::new(1000));
        l.acquire(1000).await; // drain burst
        let t = Instant::now();
        let a = { let l = l.clone(); tokio::spawn(async move { l.acquire(1000).await }) };
        let b = { let l = l.clone(); tokio::spawn(async move { l.acquire(1000).await }) };
        a.await.unwrap();
        b.await.unwrap();
        let e = t.elapsed().as_secs_f64();
        assert!((1.99..=2.01).contains(&e), "elapsed {e}");
    }

    #[tokio::test(start_paused = true)]
    async fn set_limit_zero_disables() {
        let l = RateLimiter::new(10);
        l.set_limit(0);
        assert_eq!(l.limit(), 0);
        let t = Instant::now();
        l.acquire(1_000_000).await;
        assert_eq!(t.elapsed(), Duration::ZERO);
    }
}
