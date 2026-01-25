//! Memory usage sensor.

use super::Sensor;
use std::fs;

/// Memory usage sensor.
pub struct MemorySensor {
    name: String,
    total_kb: u64,
}

impl MemorySensor {
    /// Creates a new memory sensor.
    pub fn new() -> Self {
        let total_kb = Self::read_total_memory().unwrap_or(0);
        Self {
            name: "memory".to_string(),
            total_kb,
        }
    }

    fn read_total_memory() -> Option<u64> {
        let content = fs::read_to_string("/proc/meminfo").ok()?;
        for line in content.lines() {
            if line.starts_with("MemTotal:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }
        None
    }

    fn read_available_memory() -> Option<u64> {
        let content = fs::read_to_string("/proc/meminfo").ok()?;
        for line in content.lines() {
            if line.starts_with("MemAvailable:") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if parts.len() >= 2 {
                    return parts[1].parse().ok();
                }
            }
        }
        None
    }
}

impl Default for MemorySensor {
    fn default() -> Self {
        Self::new()
    }
}

impl Sensor for MemorySensor {
    fn name(&self) -> &str {
        &self.name
    }

    fn sample(&mut self) -> f64 {
        if let Some(available) = Self::read_available_memory() {
            if self.total_kb > 0 {
                let used = self.total_kb.saturating_sub(available);
                return 100.0 * (used as f64 / self.total_kb as f64);
            }
        }
        0.0
    }

    fn min(&self) -> f64 {
        0.0
    }

    fn max(&self) -> f64 {
        100.0
    }

    fn unit(&self) -> &str {
        "%"
    }
}
