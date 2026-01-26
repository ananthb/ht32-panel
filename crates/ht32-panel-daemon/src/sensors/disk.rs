//! Disk I/O sensor.

use super::Sensor;
use std::fs;
use std::time::Instant;

/// Disk I/O sensor that reads from /proc/diskstats.
pub struct DiskSensor {
    name: String,
    device: String,
    last_read_sectors: u64,
    last_write_sectors: u64,
    last_time: Option<Instant>,
    last_read_rate: f64,
    last_write_rate: f64,
}

impl DiskSensor {
    /// Creates a new disk sensor for a specific device (e.g., "sda", "nvme0n1").
    pub fn new(device: &str) -> Self {
        Self {
            name: format!("disk_{}", device),
            device: device.to_string(),
            last_read_sectors: 0,
            last_write_sectors: 0,
            last_time: None,
            last_read_rate: 0.0,
            last_write_rate: 0.0,
        }
    }

    /// Creates a disk sensor that auto-detects the primary disk.
    pub fn auto() -> Self {
        // Try to find the primary disk
        let device = Self::detect_primary_disk().unwrap_or_else(|| "sda".to_string());
        Self::new(&device)
    }

    /// Detects the primary disk device.
    fn detect_primary_disk() -> Option<String> {
        // Try common disk device names in order of preference
        let candidates = ["nvme0n1", "sda", "vda", "xvda", "mmcblk0"];

        for candidate in candidates {
            let path = format!("/sys/block/{}", candidate);
            if std::path::Path::new(&path).exists() {
                return Some(candidate.to_string());
            }
        }

        None
    }

    /// Reads disk stats from /proc/diskstats.
    ///
    /// Format: https://www.kernel.org/doc/Documentation/ABI/testing/procfs-diskstats
    /// Fields: major minor name reads_completed reads_merged sectors_read time_reading
    ///         writes_completed writes_merged sectors_written time_writing
    ///         ios_in_progress time_doing_io weighted_time_doing_io
    fn read_stats(&self) -> Option<(u64, u64)> {
        let content = fs::read_to_string("/proc/diskstats").ok()?;

        for line in content.lines() {
            let parts: Vec<&str> = line.split_whitespace().collect();
            if parts.len() >= 10 && parts[2] == self.device {
                // sectors_read is field 5 (0-indexed)
                // sectors_written is field 9 (0-indexed)
                let read_sectors: u64 = parts[5].parse().ok()?;
                let write_sectors: u64 = parts[9].parse().ok()?;
                return Some((read_sectors, write_sectors));
            }
        }

        None
    }

    /// Returns the current read rate in bytes/second.
    pub fn read_rate(&self) -> f64 {
        self.last_read_rate
    }

    /// Returns the current write rate in bytes/second.
    pub fn write_rate(&self) -> f64 {
        self.last_write_rate
    }
}

impl Sensor for DiskSensor {
    fn name(&self) -> &str {
        &self.name
    }

    fn sample(&mut self) -> f64 {
        if let Some((read_sectors, write_sectors)) = self.read_stats() {
            if let Some(last_time) = self.last_time {
                let elapsed = last_time.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    let read_delta = read_sectors.saturating_sub(self.last_read_sectors);
                    let write_delta = write_sectors.saturating_sub(self.last_write_sectors);

                    // Sectors are typically 512 bytes
                    const SECTOR_SIZE: f64 = 512.0;
                    self.last_read_rate = (read_delta as f64 * SECTOR_SIZE) / elapsed;
                    self.last_write_rate = (write_delta as f64 * SECTOR_SIZE) / elapsed;
                }
            }

            self.last_read_sectors = read_sectors;
            self.last_write_sectors = write_sectors;
            self.last_time = Some(Instant::now());
        }

        // Return combined rate in KB/s
        (self.last_read_rate + self.last_write_rate) / 1024.0
    }

    fn min(&self) -> f64 {
        0.0
    }

    fn max(&self) -> f64 {
        1000000.0 // 1 GB/s max
    }

    fn unit(&self) -> &str {
        "KB/s"
    }
}
