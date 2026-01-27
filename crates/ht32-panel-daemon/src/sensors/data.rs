//! System data aggregation for faces.

use std::collections::VecDeque;

/// Number of history samples to keep for graphs.
pub const HISTORY_SIZE: usize = 60;

/// IP address display preference.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub enum IpDisplayPreference {
    /// IPv6 Global Unicast Address (2000::/3)
    #[default]
    Ipv6Gua,
    /// IPv6 Link-Local Address (fe80::/10)
    Ipv6Lla,
    /// IPv6 Unique Local Address (fc00::/7)
    Ipv6Ula,
    /// IPv4 address
    Ipv4,
}

impl IpDisplayPreference {
    /// Returns all available preferences.
    pub fn all() -> &'static [IpDisplayPreference] {
        &[
            IpDisplayPreference::Ipv6Gua,
            IpDisplayPreference::Ipv6Lla,
            IpDisplayPreference::Ipv6Ula,
            IpDisplayPreference::Ipv4,
        ]
    }

    /// Returns the display name for this preference.
    pub fn display_name(&self) -> &'static str {
        match self {
            IpDisplayPreference::Ipv6Gua => "IPv6 GUA",
            IpDisplayPreference::Ipv6Lla => "IPv6 LLA",
            IpDisplayPreference::Ipv6Ula => "IPv6 ULA",
            IpDisplayPreference::Ipv4 => "IPv4",
        }
    }
}

impl std::fmt::Display for IpDisplayPreference {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            IpDisplayPreference::Ipv6Gua => write!(f, "ipv6-gua"),
            IpDisplayPreference::Ipv6Lla => write!(f, "ipv6-lla"),
            IpDisplayPreference::Ipv6Ula => write!(f, "ipv6-ula"),
            IpDisplayPreference::Ipv4 => write!(f, "ipv4"),
        }
    }
}

impl std::str::FromStr for IpDisplayPreference {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "ipv6-gua" | "ipv6_gua" | "ipv6gua" | "gua" => Ok(IpDisplayPreference::Ipv6Gua),
            "ipv6-lla" | "ipv6_lla" | "ipv6lla" | "lla" => Ok(IpDisplayPreference::Ipv6Lla),
            "ipv6-ula" | "ipv6_ula" | "ipv6ula" | "ula" => Ok(IpDisplayPreference::Ipv6Ula),
            "ipv4" | "v4" => Ok(IpDisplayPreference::Ipv4),
            _ => Err(format!("Unknown IP display preference: {}", s)),
        }
    }
}

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
    /// CPU temperature in Celsius (None if unavailable)
    pub cpu_temp: Option<f64>,
    /// RAM usage percentage (0-100)
    pub ram_percent: f64,
    /// Disk read rate in bytes/second
    pub disk_read_rate: f64,
    /// Disk write rate in bytes/second
    pub disk_write_rate: f64,
    /// Disk I/O history (combined read+write rates, newest last)
    pub disk_history: VecDeque<f64>,
    /// Network interface name
    pub net_interface: String,
    /// Network receive rate in bytes/second
    pub net_rx_rate: f64,
    /// Network transmit rate in bytes/second
    pub net_tx_rate: f64,
    /// Network I/O history (combined rx+tx rates, newest last)
    pub net_history: VecDeque<f64>,
    /// IP address to display (based on preference)
    pub display_ip: Option<String>,
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
