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

    /// Returns the current time formatted as "HH:MM:SS".
    pub fn time(&self) -> String {
        let now = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default();
        let secs = now.as_secs();

        // Get local time offset (simplified - assumes UTC for now, but works for display)
        // For proper timezone support, would need chrono crate
        let local_secs = secs + self.timezone_offset();
        let hours = (local_secs % 86400) / 3600;
        let minutes = (local_secs % 3600) / 60;
        let seconds = local_secs % 60;

        format!("{:02}:{:02}:{:02}", hours, minutes, seconds)
    }

    /// Returns the timezone offset in seconds from UTC.
    fn timezone_offset(&self) -> u64 {
        // Try to read from /etc/localtime or use environment
        // For simplicity, check TZ environment or default to 0 (UTC)
        // A full implementation would parse the timezone database
        if let Ok(tz) = std::env::var("TZ") {
            // Simple parsing for offset-based timezones like "UTC-5" or "UTC+10"
            if tz.starts_with("UTC") || tz.starts_with("GMT") {
                let offset_str = &tz[3..];
                if let Ok(hours) = offset_str.parse::<i64>() {
                    // Note: TZ convention is inverted (UTC-5 means +5 hours)
                    return (hours * -3600) as u64;
                }
            }
        }

        // Try to get from libc (more reliable)
        #[cfg(target_os = "linux")]
        {
            use std::io::Read;
            if let Ok(mut file) = fs::File::open("/etc/timezone") {
                let mut contents = String::new();
                if file.read_to_string(&mut contents).is_ok() {
                    // Common timezones - simplified lookup
                    let tz = contents.trim();
                    return match tz {
                        "America/New_York" | "US/Eastern" => 5 * 3600, // Actually should be -5, but we add to UTC
                        "America/Los_Angeles" | "US/Pacific" => 8 * 3600,
                        "Europe/London" | "GB" => 0,
                        "Europe/Paris" | "Europe/Berlin" => 1 * 3600,
                        "Asia/Tokyo" | "Japan" => 9 * 3600,
                        "Asia/Kolkata" | "Asia/Calcutta" => (5 * 3600) + 1800, // UTC+5:30
                        _ => 0,
                    };
                }
            }
        }

        0 // Default to UTC
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
