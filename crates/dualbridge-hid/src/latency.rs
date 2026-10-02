//! Lock-free latency meter for the input path.
//!
//! The input thread records, for every report, the time between `read`
//! returning and the virtual controller being updated. The UI reads the
//! statistics whenever it wants; nothing here blocks or allocates.

use std::sync::atomic::{AtomicU64, Ordering};

use serde::Serialize;

#[derive(Debug, Default)]
pub struct LatencyMeter {
    last_ns: AtomicU64,
    /// Exponential moving average, in nanoseconds.
    avg_ns: AtomicU64,
    max_ns: AtomicU64,
    reports: AtomicU64,
    /// Reports that failed to parse (bad CRC, unexpected ID...).
    errors: AtomicU64,
}

/// A snapshot of the meter, for display.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, Serialize)]
pub struct LatencyStats {
    pub last_ns: u64,
    pub avg_ns: u64,
    pub max_ns: u64,
    pub reports: u64,
    pub errors: u64,
}

impl LatencyMeter {
    /// Records one processed report. Only called from the input thread.
    #[inline]
    pub fn record(&self, ns: u64) {
        self.last_ns.store(ns, Ordering::Relaxed);
        let n = self.reports.fetch_add(1, Ordering::Relaxed);
        let avg = if n == 0 {
            ns
        } else {
            // avg += (ns - avg) / 64, in integer arithmetic.
            let old = self.avg_ns.load(Ordering::Relaxed);
            (old * 63 + ns) / 64
        };
        self.avg_ns.store(avg, Ordering::Relaxed);
        self.max_ns.fetch_max(ns, Ordering::Relaxed);
    }

    #[inline]
    pub fn record_error(&self) {
        self.errors.fetch_add(1, Ordering::Relaxed);
    }

    /// Resets the maximum (e.g. when the user presses "reset" in the UI).
    pub fn reset_max(&self) {
        self.max_ns.store(0, Ordering::Relaxed);
    }

    pub fn stats(&self) -> LatencyStats {
        LatencyStats {
            last_ns: self.last_ns.load(Ordering::Relaxed),
            avg_ns: self.avg_ns.load(Ordering::Relaxed),
            max_ns: self.max_ns.load(Ordering::Relaxed),
            reports: self.reports.load(Ordering::Relaxed),
            errors: self.errors.load(Ordering::Relaxed),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn records() {
        let m = LatencyMeter::default();
        m.record(6400);
        assert_eq!(m.stats().avg_ns, 6400);
        m.record(0);
        let s = m.stats();
        assert_eq!(s.last_ns, 0);
        assert_eq!(s.avg_ns, 6300);
        assert_eq!(s.max_ns, 6400);
        assert_eq!(s.reports, 2);
        m.record_error();
        m.reset_max();
        assert_eq!(m.stats().errors, 1);
        assert_eq!(m.stats().max_ns, 0);
    }
}
