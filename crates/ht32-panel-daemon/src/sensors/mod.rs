//! System sensors module.
//!
//! Provides system metrics like CPU usage, memory, temperature, and network.

#![allow(dead_code, unused_imports)]

mod cpu;
mod memory;
mod network;

pub use cpu::CpuSensor;
pub use memory::MemorySensor;
pub use network::NetworkSensor;

/// Trait for all sensors.
pub trait Sensor: Send + Sync {
    /// Returns the sensor name.
    fn name(&self) -> &str;

    /// Samples the current value.
    fn sample(&mut self) -> f64;

    /// Returns the minimum value.
    fn min(&self) -> f64;

    /// Returns the maximum value.
    fn max(&self) -> f64;

    /// Returns the unit of measurement.
    fn unit(&self) -> &str;
}
