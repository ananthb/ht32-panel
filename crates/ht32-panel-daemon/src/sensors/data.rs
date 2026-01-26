//! System data aggregation for faces and widgets.

/// Aggregated system data from all sensors.
#[derive(Debug, Clone, Default)]
pub struct SystemData {
    /// Hostname of the system
    pub hostname: String,
    /// Current time formatted as "HH:MM"
    pub time: String,
    /// Uptime formatted as "Xd Yh Zm"
    pub uptime: String,
    /// CPU usage percentage (0-100)
    pub cpu_percent: f64,
    /// RAM usage percentage (0-100)
    pub ram_percent: f64,
    /// Disk read rate in bytes/second
    pub disk_read_rate: f64,
    /// Disk write rate in bytes/second
    pub disk_write_rate: f64,
    /// Network interface name
    pub net_interface: String,
    /// Network receive rate in bytes/second
    pub net_rx_rate: f64,
    /// Network transmit rate in bytes/second
    pub net_tx_rate: f64,
    /// IPv6 address (if available)
    pub ipv6_address: Option<String>,
    /// IPv4 address (if available)
    pub ipv4_address: Option<String>,
}

impl SystemData {
    /// Formats a byte rate as a human-readable string (e.g., "1.2 MB/s")
    pub fn format_rate(bytes_per_sec: f64) -> String {
        if bytes_per_sec >= 1_000_000_000.0 {
            format!("{:.1} GB/s", bytes_per_sec / 1_000_000_000.0)
        } else if bytes_per_sec >= 1_000_000.0 {
            format!("{:.1} MB/s", bytes_per_sec / 1_000_000.0)
        } else if bytes_per_sec >= 1_000.0 {
            format!("{:.1} KB/s", bytes_per_sec / 1_000.0)
        } else {
            format!("{:.0} B/s", bytes_per_sec)
        }
    }

    /// Formats a byte rate compactly (e.g., "1.2M")
    pub fn format_rate_compact(bytes_per_sec: f64) -> String {
        if bytes_per_sec >= 1_000_000_000.0 {
            format!("{:.1}G", bytes_per_sec / 1_000_000_000.0)
        } else if bytes_per_sec >= 1_000_000.0 {
            format!("{:.1}M", bytes_per_sec / 1_000_000.0)
        } else if bytes_per_sec >= 1_000.0 {
            format!("{:.1}K", bytes_per_sec / 1_000.0)
        } else {
            format!("{:.0}B", bytes_per_sec)
        }
    }
}
