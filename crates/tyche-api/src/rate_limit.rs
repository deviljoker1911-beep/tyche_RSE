//! Per-tenant token-bucket rate limiter.
//!
//! # Scope
//!
//! This is the **single-process, in-memory** rate limiter for the spike.
//! Production (M2+) replaces it with a Redis-backed token bucket so the limit
//! is enforced across every API replica, not per-pod. The trait boundary is
//! deliberately small ([`RateLimiter::check`]) so that swap is a one-file
//! change.
//!
//! # Algorithm
//!
//! Classic token bucket. Each tenant key gets a bucket with `capacity` tokens
//! that refills at `refill_per_sec`. A request consumes one token. When the
//! bucket is empty the caller is told how many seconds until one token is
//! available again (rounded up), which becomes the `Retry-After` header.
//!
//! A background sweep evicts idle buckets so a flood of distinct tenant keys
//! can't grow the map without bound.

use std::collections::HashMap;
use std::sync::Mutex;
use std::time::{Duration, Instant};

/// Outcome of a rate-limit check.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Decision {
    /// Request may proceed.
    Allow,
    /// Request denied; retry after this many seconds.
    Deny {
        /// Seconds until at least one token is available.
        retry_after_secs: u64,
    },
}

struct Bucket {
    tokens: f64,
    last_refill: Instant,
    last_seen: Instant,
}

/// A per-tenant token-bucket rate limiter.
pub struct RateLimiter {
    capacity: f64,
    refill_per_sec: f64,
    buckets: Mutex<HashMap<String, Bucket>>,
}

impl RateLimiter {
    /// Construct a limiter with the given steady-state rate and burst capacity.
    ///
    /// `capacity` is the maximum burst (tokens available when fully topped up);
    /// `refill_per_sec` is the long-run sustained request rate per tenant.
    #[must_use]
    pub fn new(capacity: f64, refill_per_sec: f64) -> Self {
        Self {
            capacity,
            refill_per_sec,
            buckets: Mutex::new(HashMap::new()),
        }
    }

    /// Construct from environment, with sane defaults.
    ///
    /// - `TYCHE_RATE_CAPACITY` (default 60): burst size per tenant.
    /// - `TYCHE_RATE_PER_SEC` (default 20): sustained requests/sec per tenant.
    #[must_use]
    pub fn from_env() -> Self {
        let capacity = std::env::var("TYCHE_RATE_CAPACITY")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(60.0);
        let refill = std::env::var("TYCHE_RATE_PER_SEC")
            .ok()
            .and_then(|v| v.parse().ok())
            .unwrap_or(20.0);
        Self::new(capacity, refill)
    }

    /// Check (and consume) one token for `key`. Uses the supplied `now` so the
    /// logic is unit-testable without sleeping.
    fn check_at(&self, key: &str, now: Instant) -> Decision {
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let bucket = buckets.entry(key.to_string()).or_insert_with(|| Bucket {
            tokens: self.capacity,
            last_refill: now,
            last_seen: now,
        });
        // Refill based on elapsed wall-time since the last touch.
        let elapsed = now
            .saturating_duration_since(bucket.last_refill)
            .as_secs_f64();
        bucket.tokens = (bucket.tokens + elapsed * self.refill_per_sec).min(self.capacity);
        bucket.last_refill = now;
        bucket.last_seen = now;

        if bucket.tokens >= 1.0 {
            bucket.tokens -= 1.0;
            Decision::Allow
        } else {
            let deficit = 1.0 - bucket.tokens;
            let secs = (deficit / self.refill_per_sec).ceil().max(1.0);
            Decision::Deny {
                retry_after_secs: secs as u64,
            }
        }
    }

    /// Check (and consume) one token for `key`.
    #[must_use]
    pub fn check(&self, key: &str) -> Decision {
        self.check_at(key, Instant::now())
    }

    /// Evict buckets untouched for longer than `idle`. Call periodically from a
    /// background task to bound memory under high tenant cardinality.
    pub fn sweep_idle(&self, idle: Duration) {
        let now = Instant::now();
        let mut buckets = self
            .buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        buckets.retain(|_, b| now.saturating_duration_since(b.last_seen) < idle);
    }

    /// Number of tracked tenant buckets. For tests / introspection.
    #[must_use]
    pub fn tracked_tenants(&self) -> usize {
        self.buckets
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_up_to_capacity_then_denies() {
        let rl = RateLimiter::new(3.0, 1.0);
        let t0 = Instant::now();
        assert_eq!(rl.check_at("a", t0), Decision::Allow);
        assert_eq!(rl.check_at("a", t0), Decision::Allow);
        assert_eq!(rl.check_at("a", t0), Decision::Allow);
        // 4th in the same instant: bucket empty.
        match rl.check_at("a", t0) {
            Decision::Deny { retry_after_secs } => assert_eq!(retry_after_secs, 1),
            Decision::Allow => panic!("expected deny"),
        }
    }

    #[test]
    fn refills_over_time() {
        let rl = RateLimiter::new(1.0, 2.0); // 2 tokens/sec
        let t0 = Instant::now();
        assert_eq!(rl.check_at("a", t0), Decision::Allow);
        assert!(matches!(rl.check_at("a", t0), Decision::Deny { .. }));
        // 600ms later → 1.2 tokens refilled → allow again.
        let t1 = t0 + Duration::from_millis(600);
        assert_eq!(rl.check_at("a", t1), Decision::Allow);
    }

    #[test]
    fn tenants_are_isolated() {
        let rl = RateLimiter::new(1.0, 1.0);
        let t0 = Instant::now();
        assert_eq!(rl.check_at("a", t0), Decision::Allow);
        // Different tenant has its own full bucket.
        assert_eq!(rl.check_at("b", t0), Decision::Allow);
    }

    #[test]
    fn sweep_evicts_idle() {
        let rl = RateLimiter::new(1.0, 1.0);
        let _ = rl.check("a");
        assert_eq!(rl.tracked_tenants(), 1);
        rl.sweep_idle(Duration::from_secs(0)); // everything is "idle" at 0
        assert_eq!(rl.tracked_tenants(), 0);
    }
}
