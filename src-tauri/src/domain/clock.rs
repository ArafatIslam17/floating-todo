//! Injected time source so scheduling is testable without waiting.

use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::Arc;

pub const MINUTE_MS: i64 = 60_000;

pub trait Clock: Send + Sync + 'static {
    /// Current wall-clock time as UTC epoch milliseconds.
    fn now_ms(&self) -> i64;
}

#[derive(Debug, Default, Clone, Copy)]
pub struct SystemClock;

impl Clock for SystemClock {
    fn now_ms(&self) -> i64 {
        let d = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default();
        d.as_millis() as i64
    }
}

/// Manually advanced clock for tests.
#[derive(Debug, Clone, Default)]
pub struct FakeClock(Arc<AtomicI64>);

impl FakeClock {
    pub fn at(ms: i64) -> Self {
        FakeClock(Arc::new(AtomicI64::new(ms)))
    }
    pub fn set(&self, ms: i64) {
        self.0.store(ms, Ordering::SeqCst);
    }
    pub fn advance_min(&self, minutes: i64) {
        self.0.fetch_add(minutes * MINUTE_MS, Ordering::SeqCst);
    }
}

impl Clock for FakeClock {
    fn now_ms(&self) -> i64 {
        self.0.load(Ordering::SeqCst)
    }
}
