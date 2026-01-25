//! Network throughput sensor.

use super::Sensor;
use std::fs;
use std::time::Instant;

/// Network throughput sensor.
pub struct NetworkSensor {
    name: String,
    interface: String,
    last_rx: u64,
    last_tx: u64,
    last_time: Option<Instant>,
    last_rx_rate: f64,
    last_tx_rate: f64,
}

impl NetworkSensor {
    /// Creates a new network sensor for a specific interface.
    pub fn new(interface: &str) -> Self {
        Self {
            name: format!("network_{}", interface),
            interface: interface.to_string(),
            last_rx: 0,
            last_tx: 0,
            last_time: None,
            last_rx_rate: 0.0,
            last_tx_rate: 0.0,
        }
    }

    fn read_stats(&self) -> Option<(u64, u64)> {
        let rx_path = format!("/sys/class/net/{}/statistics/rx_bytes", self.interface);
        let tx_path = format!("/sys/class/net/{}/statistics/tx_bytes", self.interface);

        let rx = fs::read_to_string(&rx_path).ok()?.trim().parse().ok()?;
        let tx = fs::read_to_string(&tx_path).ok()?.trim().parse().ok()?;

        Some((rx, tx))
    }

    /// Returns the current RX rate in bytes/second.
    pub fn rx_rate(&self) -> f64 {
        self.last_rx_rate
    }

    /// Returns the current TX rate in bytes/second.
    pub fn tx_rate(&self) -> f64 {
        self.last_tx_rate
    }
}

impl Sensor for NetworkSensor {
    fn name(&self) -> &str {
        &self.name
    }

    fn sample(&mut self) -> f64 {
        if let Some((rx, tx)) = self.read_stats() {
            if let Some(last_time) = self.last_time {
                let elapsed = last_time.elapsed().as_secs_f64();
                if elapsed > 0.0 {
                    let rx_delta = rx.saturating_sub(self.last_rx);
                    let tx_delta = tx.saturating_sub(self.last_tx);
                    self.last_rx_rate = rx_delta as f64 / elapsed;
                    self.last_tx_rate = tx_delta as f64 / elapsed;
                }
            }

            self.last_rx = rx;
            self.last_tx = tx;
            self.last_time = Some(Instant::now());
        }

        // Return combined rate in KB/s
        (self.last_rx_rate + self.last_tx_rate) / 1024.0
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
