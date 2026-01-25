//! CPU usage sensor.

use super::Sensor;
use std::fs;
use std::time::Instant;

/// CPU usage sensor.
pub struct CpuSensor {
    name: String,
    last_idle: u64,
    last_total: u64,
    last_sample: f64,
    last_time: Option<Instant>,
}

impl CpuSensor {
    /// Creates a new CPU sensor.
    pub fn new() -> Self {
        Self {
            name: "cpu_usage".to_string(),
            last_idle: 0,
            last_total: 0,
            last_sample: 0.0,
            last_time: None,
        }
    }

    fn read_cpu_stats(&self) -> Option<(u64, u64)> {
        let content = fs::read_to_string("/proc/stat").ok()?;
        let line = content.lines().next()?;
        let parts: Vec<u64> = line
            .split_whitespace()
            .skip(1)
            .filter_map(|s| s.parse().ok())
            .collect();

        if parts.len() >= 4 {
            let idle = parts[3];
            let total: u64 = parts.iter().sum();
            Some((idle, total))
        } else {
            None
        }
    }
}

impl Default for CpuSensor {
    fn default() -> Self {
        Self::new()
    }
}

impl Sensor for CpuSensor {
    fn name(&self) -> &str {
        &self.name
    }

    fn sample(&mut self) -> f64 {
        if let Some((idle, total)) = self.read_cpu_stats() {
            if self.last_total > 0 {
                let idle_delta = idle.saturating_sub(self.last_idle);
                let total_delta = total.saturating_sub(self.last_total);

                if total_delta > 0 {
                    self.last_sample = 100.0 * (1.0 - (idle_delta as f64 / total_delta as f64));
                }
            }

            self.last_idle = idle;
            self.last_total = total;
            self.last_time = Some(Instant::now());
        }

        self.last_sample
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
