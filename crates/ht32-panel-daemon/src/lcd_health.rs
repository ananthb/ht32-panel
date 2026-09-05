//! Hardware-free LCD write-health state machine.
//!
//! Decides when a streak of USB write failures means the device handle should be
//! dropped and reopened (`Demote`), or that recovery has failed long enough that
//! the process should exit for systemd to relaunch it. Also rate-limits error
//! logging. Time is passed in by the caller (no `Instant::now()` inside), so the
//! logic is fully deterministic and unit-testable.

use std::time::{Duration, Instant};

/// Action the caller should take after recording a write result.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WriteAction {
    /// Nothing to do; keep going.
    None,
    /// Failure streak hit the threshold: drop the handle and reconnect.
    Demote,
}

/// Tracks consecutive LCD write failures and recovery timing.
pub struct LcdHealth {
    failure_threshold: u32,
    exit_after: Duration,
    log_interval: Duration,
    consecutive_failures: u32,
    /// `None` until the first successful write since (re)start.
    last_success: Option<Instant>,
    last_error_log: Option<Instant>,
}

impl LcdHealth {
    pub fn new(failure_threshold: u32, exit_after: Duration, log_interval: Duration) -> Self {
        Self {
            failure_threshold,
            exit_after,
            log_interval,
            consecutive_failures: 0,
            last_success: None,
            last_error_log: None,
        }
    }

    pub fn consecutive_failures(&self) -> u32 {
        self.consecutive_failures
    }

    /// Record a successful device write.
    pub fn record_success(&mut self, now: Instant) {
        self.consecutive_failures = 0;
        self.last_success = Some(now);
        self.last_error_log = None;
    }

    /// Record a failed device write; returns whether to demote.
    pub fn record_failure(&mut self) -> WriteAction {
        self.consecutive_failures += 1;
        if self.consecutive_failures == self.failure_threshold {
            WriteAction::Demote
        } else {
            WriteAction::None
        }
    }

    /// True when recovery has failed for `exit_after`. Only arms after at least
    /// one successful write, so a never-present device never triggers a restart loop.
    pub fn should_exit(&self, now: Instant) -> bool {
        if self.exit_after.is_zero() {
            return false;
        }
        matches!(self.last_success, Some(t) if now.duration_since(t) >= self.exit_after)
    }

    /// Throttled error logging: returns the failure count when a log line is due
    /// (first failure of a streak, then at most once per `log_interval`).
    pub fn should_log(&mut self, now: Instant) -> Option<u32> {
        let due = match self.last_error_log {
            None => true,
            Some(t) => now.duration_since(t) >= self.log_interval,
        };
        if due {
            self.last_error_log = Some(now);
            Some(self.consecutive_failures)
        } else {
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn health() -> LcdHealth {
        LcdHealth::new(3, Duration::from_secs(300), Duration::from_secs(60))
    }

    #[test]
    fn transient_failures_below_threshold_do_not_demote() {
        let mut h = health();
        assert_eq!(h.record_failure(), WriteAction::None);
        assert_eq!(h.record_failure(), WriteAction::None);
    }

    #[test]
    fn threshold_failures_demote() {
        let mut h = health();
        h.record_failure();
        h.record_failure();
        assert_eq!(h.record_failure(), WriteAction::Demote);
        assert_eq!(h.consecutive_failures(), 3);
    }

    #[test]
    fn success_resets_failure_streak() {
        let t = Instant::now();
        let mut h = health();
        h.record_failure();
        h.record_failure();
        h.record_success(t);
        assert_eq!(h.consecutive_failures(), 0);
        assert_eq!(h.record_failure(), WriteAction::None);
    }

    #[test]
    fn exit_only_arms_after_a_success_then_goes_dark() {
        let t = Instant::now();
        let mut h = health();
        // Never connected: should never exit.
        assert!(!h.should_exit(t + Duration::from_secs(10_000)));
        // After a success, going dark past exit_after triggers exit.
        h.record_success(t);
        assert!(!h.should_exit(t + Duration::from_secs(299)));
        assert!(h.should_exit(t + Duration::from_secs(300)));
    }

    #[test]
    fn exit_after_zero_disables_escalation() {
        let t = Instant::now();
        let mut h = LcdHealth::new(3, Duration::ZERO, Duration::from_secs(60));
        h.record_success(t);
        assert!(!h.should_exit(t + Duration::from_secs(100_000)));
    }

    #[test]
    fn log_throttle_logs_first_then_once_per_interval() {
        let t = Instant::now();
        let mut h = health();
        h.record_failure();
        assert_eq!(h.should_log(t), Some(1)); // first failure logs
        h.record_failure();
        assert_eq!(h.should_log(t + Duration::from_secs(1)), None); // suppressed
        h.record_failure();
        assert_eq!(h.should_log(t + Duration::from_secs(60)), Some(3)); // interval elapsed
    }

    #[test]
    fn success_reenables_immediate_logging() {
        let t = Instant::now();
        let mut h = health();
        h.record_failure();
        h.should_log(t);
        h.record_success(t + Duration::from_secs(1));
        h.record_failure();
        assert_eq!(h.should_log(t + Duration::from_secs(2)), Some(1));
    }
}
