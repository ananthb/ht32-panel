//! Professional face with graphical progress bars.
//!
//! Layout (320x170):
//! ```text
//! endeavour               18:45
//! Uptime: 5d 12h 34m
//!
//! CPU [████████░░░░░░░░] 45%
//! RAM [██████████░░░░░░] 67%
//! Disk[████░░░░░░░░░░░░] R: 12 MB/s  W: 5 MB/s
//! Net [██████░░░░░░░░░░] ↓: 1.2 MB/s ↑: 0.8 MB/s
//!
//! enp2s0: 192.168.1.100
//! ```

use super::{
    complication_options, complications, date_formats, time_formats, Complication,
    ComplicationChoice, ComplicationOption, EnabledComplications, Face, Theme,
};
use crate::rendering::Canvas;
use crate::sensors::data::SystemData;

/// Dim a color by mixing it toward the background.
fn dim_color(color: u32, background: u32, factor: f32) -> u32 {
    let r1 = ((color >> 16) & 0xFF) as f32;
    let g1 = ((color >> 8) & 0xFF) as f32;
    let b1 = (color & 0xFF) as f32;
    let r2 = ((background >> 16) & 0xFF) as f32;
    let g2 = ((background >> 8) & 0xFF) as f32;
    let b2 = (background & 0xFF) as f32;

    let r = (r1 * factor + r2 * (1.0 - factor)) as u32;
    let g = (g1 * factor + g2 * (1.0 - factor)) as u32;
    let b = (b1 * factor + b2 * (1.0 - factor)) as u32;

    (r << 16) | (g << 8) | b
}

/// Derive colors from theme for the professional face.
struct FaceColors {
    /// Primary highlight color (hostname, interface name)
    highlight: u32,
    /// Main text color
    text: u32,
    /// Dimmed text color (uptime, IPs)
    dim: u32,
    /// Progress bar background
    bar_bg: u32,
    /// CPU bar fill color
    bar_cpu: u32,
    /// RAM bar fill color
    bar_ram: u32,
    /// Disk bar fill color
    bar_disk: u32,
    /// Network bar fill color
    bar_net: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            highlight: theme.primary,
            text: theme.text,
            dim: dim_color(theme.text, theme.background, 0.7), // Higher for better contrast
            bar_bg: dim_color(theme.primary, theme.background, 0.2),
            bar_cpu: theme.primary,
            bar_ram: theme.secondary,
            bar_disk: dim_color(theme.primary, theme.secondary, 0.5), // Blend of primary/secondary
            bar_net: theme.secondary,
        }
    }
}

/// Font sizes.
const FONT_LARGE: f32 = 16.0;
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;

/// Progress bar dimensions.
const BAR_WIDTH: u32 = 120;
const BAR_HEIGHT: u32 = 10;

/// Graph dimensions.
const GRAPH_HEIGHT: u32 = 16;

/// A professional face with graphical progress bars.
pub struct ProfessionalFace;

impl ProfessionalFace {
    /// Creates a new professional face.
    pub fn new() -> Self {
        Self
    }

    /// Draws a progress bar.
    #[allow(clippy::too_many_arguments)]
    fn draw_progress_bar(
        canvas: &mut Canvas,
        x: i32,
        y: i32,
        width: u32,
        height: u32,
        percent: f64,
        fill_color: u32,
        bg_color: u32,
    ) {
        // Draw background
        canvas.fill_rect(x, y, width, height, bg_color);

        // Draw filled portion
        let fill_width = ((width as f64 * (percent / 100.0)) as u32).min(width);
        if fill_width > 0 {
            canvas.fill_rect(x, y, fill_width, height, fill_color);
        }
    }
}

impl Default for ProfessionalFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for ProfessionalFace {
    fn name(&self) -> &str {
        "professional"
    }

    fn available_complications(&self) -> Vec<Complication> {
        vec![
            Complication::with_options(
                complications::TIME,
                "Time",
                "Display the current time",
                true,
                vec![ComplicationOption::choice(
                    complication_options::TIME_FORMAT,
                    "Format",
                    "Time display format",
                    vec![
                        ComplicationChoice::new(time_formats::DIGITAL_24H, "Digital (24h)"),
                        ComplicationChoice::new(time_formats::DIGITAL_12H, "Digital (12h)"),
                        ComplicationChoice::new(time_formats::ANALOGUE, "Analogue"),
                    ],
                    time_formats::DIGITAL_24H,
                )],
            ),
            Complication::with_options(
                complications::DATE,
                "Date",
                "Display the current date",
                true,
                vec![ComplicationOption::choice(
                    complication_options::DATE_FORMAT,
                    "Format",
                    "Date display format",
                    vec![
                        ComplicationChoice::new(date_formats::ISO, "ISO (2024-01-15)"),
                        ComplicationChoice::new(date_formats::US, "US (01/15/2024)"),
                        ComplicationChoice::new(date_formats::EU, "EU (15/01/2024)"),
                        ComplicationChoice::new(date_formats::SHORT, "Short (Jan 15)"),
                        ComplicationChoice::new(date_formats::LONG, "Long (January 15, 2024)"),
                        ComplicationChoice::new(date_formats::WEEKDAY, "Weekday (Mon, Jan 15)"),
                    ],
                    date_formats::ISO,
                )],
            ),
            Complication::with_options(
                complications::IP_ADDRESS,
                "IP Address",
                "Display network IP address",
                true,
                vec![ComplicationOption::choice(
                    complication_options::IP_TYPE,
                    "IP Type",
                    "Type of IP address to display",
                    vec![
                        ComplicationChoice::new("ipv6-gua", "IPv6 Global"),
                        ComplicationChoice::new("ipv6-lla", "IPv6 Link-Local"),
                        ComplicationChoice::new("ipv6-ula", "IPv6 ULA"),
                        ComplicationChoice::new("ipv4", "IPv4"),
                    ],
                    "ipv6-gua",
                )],
            ),
            Complication::with_options(
                complications::NETWORK,
                "Network",
                "Display network activity graph",
                true,
                vec![ComplicationOption::choice(
                    complication_options::INTERFACE,
                    "Interface",
                    "Network interface to monitor",
                    vec![ComplicationChoice::new("auto", "Auto-detect")],
                    "auto",
                )],
            ),
            Complication::new(
                complications::DISK_IO,
                "Disk I/O",
                "Display disk read/write activity graph",
                true,
            ),
            Complication::new(
                complications::CPU_TEMP,
                "CPU Temperature",
                "Display CPU temperature",
                true,
            ),
        ]
    }

    fn render(
        &self,
        canvas: &mut Canvas,
        data: &SystemData,
        theme: &Theme,
        complications: &EnabledComplications,
    ) {
        let colors = FaceColors::from_theme(theme);
        let (width, _height) = canvas.dimensions();
        let portrait = width < 200;
        let margin = 8;
        let mut y = margin;

        // Helper to check if a complication is enabled
        let is_enabled = |id: &str| -> bool { complications.is_enabled(self.name(), id, true) };

        // Get time format option
        let time_format = complications
            .get_option(
                self.name(),
                complications::TIME,
                complication_options::TIME_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(time_formats::DIGITAL_24H);

        // Get date format option
        let date_format = complications
            .get_option(
                self.name(),
                complications::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::ISO);

        if portrait {
            // Portrait layout - narrower bars, stacked text
            let bar_width = (width as i32 - margin * 2 - 60) as u32;
            let bar_x = margin + 32;

            // Hostname (always shown)
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);

            // Complication: Time (right-aligned)
            if is_enabled(complications::TIME) && time_format != time_formats::ANALOGUE {
                let time_str = data.format_time(time_format);
                let time_width = canvas.text_width(&time_str, FONT_LARGE);
                canvas.draw_text(
                    width as i32 - margin - time_width,
                    y,
                    &time_str,
                    FONT_LARGE,
                    colors.text,
                );
            }
            y += canvas.line_height(FONT_LARGE) + 2;

            // Complication: Date (right-aligned, under time)
            if is_enabled(complications::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y,
                        &date_str,
                        FONT_SMALL,
                        colors.dim,
                    );
                    y += canvas.line_height(FONT_SMALL) + 2;
                }
            }

            // Base element: Uptime (always shown)
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_SMALL, colors.dim);
            y += canvas.line_height(FONT_SMALL) + 2;

            // Complication: IP address
            if is_enabled(complications::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    let max_width = width as i32 - margin * 2;
                    let ip_width = canvas.text_width(ip, FONT_SMALL);
                    if ip_width > max_width && ip.contains(':') {
                        let mid = ip.len() / 2;
                        let split_pos = ip[..mid].rfind(':').map(|p| p + 1).unwrap_or(mid);
                        let (first, second) = ip.split_at(split_pos);
                        canvas.draw_text(margin, y, first, FONT_SMALL, colors.dim);
                        y += canvas.line_height(FONT_SMALL);
                        canvas.draw_text(margin, y, second, FONT_SMALL, colors.dim);
                        y += canvas.line_height(FONT_SMALL) + 2;
                    } else {
                        canvas.draw_text(margin, y, ip, FONT_SMALL, colors.dim);
                        y += canvas.line_height(FONT_SMALL) + 2;
                    }
                }
            }

            // Complication: CPU temperature
            if is_enabled(complications::CPU_TEMP) {
                if let Some(temp) = data.cpu_temp {
                    let temp_text = format!("Temp: {:.0}°C", temp);
                    canvas.draw_text(margin, y, &temp_text, FONT_SMALL, colors.dim);
                    y += canvas.line_height(FONT_SMALL) + 4;
                } else {
                    y += 4;
                }
            }

            // Base element: CPU bar (always shown)
            canvas.draw_text(margin, y, "CPU", FONT_SMALL, colors.text);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                bar_width,
                BAR_HEIGHT,
                data.cpu_percent,
                colors.bar_cpu,
                colors.bar_bg,
            );
            let cpu_pct = format!("{:2.0}%", data.cpu_percent);
            canvas.draw_text(
                bar_x + bar_width as i32 + 4,
                y,
                &cpu_pct,
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 3;

            // Base element: RAM bar (always shown)
            canvas.draw_text(margin, y, "RAM", FONT_SMALL, colors.text);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                bar_width,
                BAR_HEIGHT,
                data.ram_percent,
                colors.bar_ram,
                colors.bar_bg,
            );
            let ram_pct = format!("{:2.0}%", data.ram_percent);
            canvas.draw_text(
                bar_x + bar_width as i32 + 4,
                y,
                &ram_pct,
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 3;

            // Complication: Disk I/O graph
            if is_enabled(complications::DISK_IO) {
                let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
                let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
                canvas.draw_text(
                    margin,
                    y,
                    &format!("DSK R:{} W:{}", disk_r, disk_w),
                    FONT_SMALL,
                    colors.text,
                );
                y += canvas.line_height(FONT_SMALL) + 1;
                canvas.draw_graph(
                    margin,
                    y,
                    width - (margin * 2) as u32,
                    GRAPH_HEIGHT,
                    &data.disk_history,
                    SystemData::compute_graph_scale(&data.disk_history),
                    colors.bar_disk,
                    colors.bar_bg,
                );
                y += GRAPH_HEIGHT as i32 + 4;
            }

            // Complication: Network I/O graph
            if is_enabled(complications::NETWORK) {
                let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
                let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
                canvas.draw_text(
                    margin,
                    y,
                    &format!("NET \u{2193}:{} \u{2191}:{}", net_rx, net_tx),
                    FONT_SMALL,
                    colors.text,
                );
                y += canvas.line_height(FONT_SMALL) + 1;
                canvas.draw_graph(
                    margin,
                    y,
                    width - (margin * 2) as u32,
                    GRAPH_HEIGHT,
                    &data.net_history,
                    SystemData::compute_graph_scale(&data.net_history),
                    colors.bar_net,
                    colors.bar_bg,
                );
            }
        } else {
            // Landscape layout
            // Hostname (always shown)
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);

            // Complication: Time (right-aligned)
            if is_enabled(complications::TIME) && time_format != time_formats::ANALOGUE {
                let time_str = data.format_time(time_format);
                let time_width = canvas.text_width(&time_str, FONT_LARGE);
                canvas.draw_text(
                    width as i32 - margin - time_width,
                    y,
                    &time_str,
                    FONT_LARGE,
                    colors.text,
                );
            }
            y += canvas.line_height(FONT_LARGE) + 2;

            // Complication: Date (right-aligned, under time)
            if is_enabled(complications::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_NORMAL);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y,
                        &date_str,
                        FONT_NORMAL,
                        colors.dim,
                    );
                    y += canvas.line_height(FONT_NORMAL) + 2;
                }
            }

            // Base element: Uptime (always shown)
            let uptime_text = format!("Uptime: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_NORMAL, colors.dim);
            y += canvas.line_height(FONT_NORMAL) + 2;

            // Complication: IP address
            if is_enabled(complications::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    canvas.draw_text(margin, y, ip, FONT_SMALL, colors.dim);
                    y += canvas.line_height(FONT_SMALL) + 4;
                } else {
                    y += 4;
                }
            }

            let bar_x = margin + 35;

            // Base element: CPU bar with optional temperature (always shown)
            canvas.draw_text(margin, y, "CPU", FONT_SMALL, colors.text);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                BAR_WIDTH,
                BAR_HEIGHT,
                data.cpu_percent,
                colors.bar_cpu,
                colors.bar_bg,
            );
            let cpu_percent = format!("{:3.0}%", data.cpu_percent);
            canvas.draw_text(
                bar_x + BAR_WIDTH as i32 + 6,
                y,
                &cpu_percent,
                FONT_SMALL,
                colors.text,
            );
            // Complication: Temperature on same line (landscape)
            if is_enabled(complications::CPU_TEMP) {
                if let Some(temp) = data.cpu_temp {
                    let temp_text = format!("{:.0}°C", temp);
                    canvas.draw_text(
                        bar_x + BAR_WIDTH as i32 + 50,
                        y,
                        &temp_text,
                        FONT_SMALL,
                        colors.dim,
                    );
                }
            }
            y += canvas.line_height(FONT_SMALL) + 3;

            // Base element: RAM bar (always shown)
            canvas.draw_text(margin, y, "RAM", FONT_SMALL, colors.text);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 1,
                BAR_WIDTH,
                BAR_HEIGHT,
                data.ram_percent,
                colors.bar_ram,
                colors.bar_bg,
            );
            let ram_percent = format!("{:3.0}%", data.ram_percent);
            canvas.draw_text(
                bar_x + BAR_WIDTH as i32 + 6,
                y,
                &ram_percent,
                FONT_SMALL,
                colors.text,
            );
            y += canvas.line_height(FONT_SMALL) + 3;

            // Complication: Disk I/O graph
            if is_enabled(complications::DISK_IO) {
                let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
                let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
                canvas.draw_text(margin, y, "DSK", FONT_SMALL, colors.text);
                canvas.draw_text(
                    bar_x + BAR_WIDTH as i32 + 6,
                    y,
                    &format!("R:{} W:{}", disk_r, disk_w),
                    FONT_SMALL,
                    colors.dim,
                );
                y += canvas.line_height(FONT_SMALL) + 1;
                canvas.draw_graph(
                    margin,
                    y,
                    width - (margin * 2) as u32,
                    GRAPH_HEIGHT,
                    &data.disk_history,
                    SystemData::compute_graph_scale(&data.disk_history),
                    colors.bar_disk,
                    colors.bar_bg,
                );
                y += GRAPH_HEIGHT as i32 + 3;
            }

            // Complication: Network I/O graph
            if is_enabled(complications::NETWORK) {
                let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
                let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
                canvas.draw_text(margin, y, "NET", FONT_SMALL, colors.text);
                canvas.draw_text(
                    bar_x + BAR_WIDTH as i32 + 6,
                    y,
                    &format!("\u{2193}:{} \u{2191}:{}", net_rx, net_tx),
                    FONT_SMALL,
                    colors.dim,
                );
                y += canvas.line_height(FONT_SMALL) + 1;
                canvas.draw_graph(
                    margin,
                    y,
                    width - (margin * 2) as u32,
                    GRAPH_HEIGHT,
                    &data.net_history,
                    SystemData::compute_graph_scale(&data.net_history),
                    colors.bar_net,
                    colors.bar_bg,
                );
            }
        }
        // Suppress unused variable warning when all complications are disabled
        let _ = y;
    }
}
