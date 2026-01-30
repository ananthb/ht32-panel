//! System information sensor for hostname, uptime, and time.

use std::fs;
use std::time::{SystemTime, UNIX_EPOCH};

/// System information provider.
pub struct SystemInfo;

impl SystemInfo {
    /// Creates a new system info provider.
    pub fn new() -> Self {
        Self
    }

    /// Returns the hostname of the system.
    pub fn hostname(&self) -> String {
        fs::read_to_string("/etc/hostname")
            .map(|s| s.trim().to_string())
            .or_else(|_| {
                fs::read_to_string("/proc/sys/kernel/hostname").map(|s| s.trim().to_string())
            })
            .unwrap_or_else(|_| "unknown".to_string())
    }

    /// Returns the current time formatted as "HH:MM".
    pub fn time(&self) -> String {
        let (hours, minutes, _, _, _, _, _) = self.time_components();
        format!("{:02}:{:02}", hours, minutes)
    }

    /// Returns individual time/date components: (hour, minute, day, month, year, day_of_week).
    /// Day of week: 0=Sunday, 1=Monday, ..., 6=Saturday.
    /// Uses the system's local timezone.
    pub fn time_components(&self) -> (u8, u8, u8, u8, u16, u8, u64) {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();

        // Use libc's localtime_r to get proper local time with timezone/DST handling
        let time_t = secs as libc::time_t;
        let mut tm: libc::tm = unsafe { std::mem::zeroed() };

        // SAFETY: localtime_r is thread-safe and we provide a valid pointer
        let result = unsafe { libc::localtime_r(&time_t, &mut tm) };

        if result.is_null() {
            // Fallback to UTC if localtime_r fails
            let hours = ((secs % 86400) / 3600) as u8;
            let minutes = ((secs % 3600) / 60) as u8;
            return (hours, minutes, 1, 1, 1970, 0, secs);
        }

        let hours = tm.tm_hour as u8;
        let minutes = tm.tm_min as u8;
        let day = tm.tm_mday as u8;
        let month = (tm.tm_mon + 1) as u8; // tm_mon is 0-11
        let year = (tm.tm_year + 1900) as u16; // tm_year is years since 1900
        let day_of_week = tm.tm_wday as u8; // 0=Sunday

        (hours, minutes, day, month, year, day_of_week, secs)
    }

    /// Returns the system uptime formatted as "Xd Yh Zm".
    pub fn uptime(&self) -> String {
        let uptime_secs = self.uptime_seconds();

        let days = uptime_secs / 86400;
        let hours = (uptime_secs % 86400) / 3600;
        let minutes = (uptime_secs % 3600) / 60;

        if days > 0 {
            format!("{}d {}h {}m", days, hours, minutes)
        } else if hours > 0 {
            format!("{}h {}m", hours, minutes)
        } else {
            format!("{}m", minutes)
        }
    }

    /// Returns the uptime in seconds.
    pub fn uptime_seconds(&self) -> u64 {
        fs::read_to_string("/proc/uptime")
            .ok()
            .and_then(|content| {
                content
                    .split_whitespace()
                    .next()
                    .and_then(|s| s.parse::<f64>().ok())
                    .map(|f| f as u64)
            })
            .unwrap_or(0)
    }
}

impl Default for SystemInfo {
    fn default() -> Self {
        Self::new()
    }
}
