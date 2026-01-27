//! Network throughput sensor.

use super::Sensor;
use std::ffi::CStr;
use std::fs;
use std::net::{Ipv4Addr, Ipv6Addr};
use std::time::Instant;
use tracing::info;

/// Network throughput sensor.
pub struct NetworkSensor {
    name: String,
    interface: String,
    last_rx: u64,
    last_tx: u64,
    last_time: Option<Instant>,
    last_rx_rate: f64,
    last_tx_rate: f64,
    cached_ipv4: Option<String>,
    cached_ipv6: Option<String>,
    last_ip_check: Option<Instant>,
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
            cached_ipv4: None,
            cached_ipv6: None,
            last_ip_check: None,
        }
    }

    /// Creates a new network sensor with auto-detected interface.
    /// Tries to find the default gateway interface, falls back to first active interface.
    pub fn auto() -> Self {
        let interface = Self::detect_interface().unwrap_or_else(|| "eth0".to_string());
        info!("Network sensor using interface: {}", interface);
        Self::new(&interface)
    }

    /// Changes the monitored network interface. Resets rate counters.
    pub fn set_interface(&mut self, interface: &str) {
        self.name = format!("network_{}", interface);
        self.interface = interface.to_string();
        self.last_rx = 0;
        self.last_tx = 0;
        self.last_time = None;
        self.last_rx_rate = 0.0;
        self.last_tx_rate = 0.0;
        self.cached_ipv4 = None;
        self.cached_ipv6 = None;
        self.last_ip_check = None;
        info!("Network sensor switched to interface: {}", interface);
    }

    /// Sets interface to auto-detected default.
    pub fn set_auto(&mut self) {
        let interface = Self::detect_interface().unwrap_or_else(|| "eth0".to_string());
        self.set_interface(&interface);
    }

    /// Lists all available network interfaces (excludes loopback and virtual interfaces).
    pub fn list_interfaces() -> Vec<String> {
        let mut interfaces = Vec::new();
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip loopback and virtual interfaces
                if name == "lo" || name.starts_with("veth") || name.starts_with("docker") {
                    continue;
                }
                // Check if interface has stats (indicates a real interface)
                let stats_path = format!("/sys/class/net/{}/statistics/rx_bytes", name);
                if fs::metadata(&stats_path).is_ok() {
                    interfaces.push(name);
                }
            }
        }
        interfaces.sort();
        interfaces
    }

    /// Detects the primary network interface.
    /// Checks /proc/net/route for the default gateway interface.
    pub fn detect_interface() -> Option<String> {
        // Try to find the default route interface from /proc/net/route
        if let Ok(content) = fs::read_to_string("/proc/net/route") {
            for line in content.lines().skip(1) {
                // Skip header
                let fields: Vec<&str> = line.split_whitespace().collect();
                if fields.len() >= 2 {
                    let iface = fields[0];
                    let destination = fields[1];
                    // Default route has destination 00000000
                    if destination == "00000000" {
                        return Some(iface.to_string());
                    }
                }
            }
        }

        // Fallback: find first non-loopback interface with statistics
        if let Ok(entries) = fs::read_dir("/sys/class/net") {
            for entry in entries.flatten() {
                let name = entry.file_name().to_string_lossy().to_string();
                // Skip loopback and virtual interfaces
                if name == "lo" || name.starts_with("veth") || name.starts_with("docker") {
                    continue;
                }
                // Check if interface has stats
                let stats_path = format!("/sys/class/net/{}/statistics/rx_bytes", name);
                if fs::metadata(&stats_path).is_ok() {
                    return Some(name);
                }
            }
        }

        None
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

    /// Returns the network interface name.
    pub fn interface_name(&self) -> &str {
        &self.interface
    }

    /// Returns the IPv4 address for this interface (cached, refreshed every 30s).
    pub fn ipv4_address(&mut self) -> Option<String> {
        self.refresh_ip_cache();
        self.cached_ipv4.clone()
    }

    /// Returns the IPv6 address for this interface (cached, refreshed every 30s).
    pub fn ipv6_address(&mut self) -> Option<String> {
        self.refresh_ip_cache();
        self.cached_ipv6.clone()
    }

    /// Refreshes the IP address cache if stale (older than 30 seconds).
    fn refresh_ip_cache(&mut self) {
        let should_refresh = self
            .last_ip_check
            .map(|t| t.elapsed().as_secs() > 30)
            .unwrap_or(true);

        if should_refresh {
            let (ipv4, ipv6) = Self::get_ip_addresses(&self.interface);
            self.cached_ipv4 = ipv4;
            self.cached_ipv6 = ipv6;
            self.last_ip_check = Some(Instant::now());
        }
    }

    /// Gets IPv4 and IPv6 addresses for an interface using getifaddrs.
    fn get_ip_addresses(interface: &str) -> (Option<String>, Option<String>) {
        let mut ipv4 = None;
        let mut ipv6 = None;

        // SAFETY: getifaddrs is a standard POSIX function. We properly free the
        // list with freeifaddrs when done.
        unsafe {
            let mut ifaddrs: *mut libc::ifaddrs = std::ptr::null_mut();
            if libc::getifaddrs(&mut ifaddrs) != 0 {
                return (None, None);
            }

            let mut current = ifaddrs;
            while !current.is_null() {
                let ifa = &*current;

                // Check if this is the interface we're looking for
                if !ifa.ifa_name.is_null() {
                    let name = CStr::from_ptr(ifa.ifa_name).to_string_lossy();
                    if name == interface && !ifa.ifa_addr.is_null() {
                        let family = (*ifa.ifa_addr).sa_family as i32;

                        if family == libc::AF_INET && ipv4.is_none() {
                            // IPv4 address
                            let sockaddr_in = ifa.ifa_addr as *const libc::sockaddr_in;
                            let addr_bytes = (*sockaddr_in).sin_addr.s_addr.to_ne_bytes();
                            let addr = Ipv4Addr::from(addr_bytes);
                            ipv4 = Some(addr.to_string());
                        } else if family == libc::AF_INET6 && ipv6.is_none() {
                            // IPv6 address
                            let sockaddr_in6 = ifa.ifa_addr as *const libc::sockaddr_in6;
                            let addr_bytes = (*sockaddr_in6).sin6_addr.s6_addr;
                            let addr = Ipv6Addr::from(addr_bytes);

                            // Skip link-local addresses (fe80::)
                            if !addr.to_string().starts_with("fe80:") {
                                ipv6 = Some(addr.to_string());
                            }
                        }
                    }
                }

                current = ifa.ifa_next;
            }

            libc::freeifaddrs(ifaddrs);
        }

        (ipv4, ipv6)
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
