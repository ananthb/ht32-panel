//! Professional face with graphical progress bars.
//!
//! Landscape layout (320x170):
//! ```text
//! endeavour               18:45
//! Up: 5d 12h 34m          2025-01-31
//! IP:                  192.168.1.100
//! Temp:                        45°C
//! CPU: 45%
//! [████████████░░░░░░░░░░░░░░░░░░]
//! RAM: 67%
//! [██████████████████░░░░░░░░░░░░]
//! DSK:                   R:12M W:5M
//! [▁▁▂▃▄▅▆▇███▇▆▅▄▃▂▁▁▁▁▁▁▁▁▁▁▁▁]
//! NET:                 ↓:1.2M ↑:0.8M
//! [▁▁▁▂▂▃▃▄▄▅▅▆▆▇▇████▇▇▆▆▅▅▄▄▃▃]
//! ```

use super::{
    complication_names, complication_options, complications, date_formats, draw_mini_analog_clock,
    time_formats, Complication, EnabledComplications, Face, Theme,
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
            complications::time(true),
            complications::date(true, date_formats::ISO),
            complications::ip_address(true),
            complications::network(true),
            complications::disk_io(true),
            complications::cpu_temp(true),
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
                complication_names::TIME,
                complication_options::TIME_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(time_formats::DIGITAL_24H);

        // Get date format option
        let date_format = complications
            .get_option(
                self.name(),
                complication_names::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::ISO);

        if portrait {
            // Portrait layout - full width bars on own lines, stacked text
            let bar_width = (width - (margin * 2) as u32).min(200);
            let tall_bar_height = 14_u32; // Taller bars for CPU/RAM
            let section_spacing = 6; // Extra spacing between sections
            let line_height = canvas.line_height(FONT_SMALL);

            // Hostname (always shown)
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);

            // Complication: Time (right-aligned)
            if is_enabled(complication_names::TIME) {
                if time_format == time_formats::ANALOGUE {
                    // Draw small analog clock on the right
                    let clock_radius = 10_u32;
                    let clock_cx = width as i32 - margin - clock_radius as i32;
                    let clock_cy = y + clock_radius as i32;
                    draw_mini_analog_clock(
                        canvas,
                        clock_cx,
                        clock_cy,
                        clock_radius,
                        data.hour,
                        data.minute,
                        colors.highlight,
                        colors.text,
                    );
                } else {
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
            }
            y += canvas.line_height(FONT_LARGE) + 2;

            // Complication: Date (right-aligned, under time)
            if is_enabled(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y,
                        &date_str,
                        FONT_SMALL,
                        colors.dim,
                    );
                    y += line_height;
                }
            }

            // Two lines lower before Uptime
            y += line_height * 2;

            // Base element: Uptime (always shown)
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_SMALL, colors.dim);
            y += line_height + section_spacing;

            // Complication: IP address with label on its own line
            if is_enabled(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    // IP label on its own line
                    canvas.draw_text(margin, y, "IP:", FONT_SMALL, colors.dim);
                    y += line_height;
                    // IP address on next line
                    let max_width = width as i32 - margin * 2;
                    let ip_width = canvas.text_width(ip, FONT_SMALL);
                    if ip_width > max_width && ip.contains(':') {
                        let mid = ip.len() / 2;
                        let split_pos = ip[..mid].rfind(':').map(|p| p + 1).unwrap_or(mid);
                        let (first, second) = ip.split_at(split_pos);
                        canvas.draw_text(margin, y, first, FONT_SMALL, colors.text);
                        y += line_height;
                        canvas.draw_text(margin, y, second, FONT_SMALL, colors.text);
                    } else {
                        canvas.draw_text(margin, y, ip, FONT_SMALL, colors.text);
                    }
                    y += line_height + section_spacing * 2;
                }
            }

            // Complication: CPU temperature
            if is_enabled(complication_names::CPU_TEMP) {
                if let Some(temp) = data.cpu_temp {
                    canvas.draw_text(margin, y, "Temp:", FONT_SMALL, colors.dim);
                    let temp_val = format!("{:.0}°C", temp);
                    let temp_w = canvas.text_width(&temp_val, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - temp_w,
                        y,
                        &temp_val,
                        FONT_SMALL,
                        colors.text,
                    );
                    y += line_height + section_spacing;
                }
            }

            // Base element: CPU label on its own line, then bar below
            let cpu_label = format!("CPU: {:2.0}%", data.cpu_percent);
            canvas.draw_text(margin, y, &cpu_label, FONT_SMALL, colors.dim);
            y += line_height;
            Self::draw_progress_bar(
                canvas,
                margin,
                y,
                bar_width,
                tall_bar_height,
                data.cpu_percent,
                colors.bar_cpu,
                colors.bar_bg,
            );
            y += tall_bar_height as i32 + section_spacing;

            // Base element: RAM label on its own line, then bar below
            let ram_label = format!("RAM: {:2.0}%", data.ram_percent);
            canvas.draw_text(margin, y, &ram_label, FONT_SMALL, colors.dim);
            y += line_height;
            Self::draw_progress_bar(
                canvas,
                margin,
                y,
                bar_width,
                tall_bar_height,
                data.ram_percent,
                colors.bar_ram,
                colors.bar_bg,
            );
            y += tall_bar_height as i32 + section_spacing;

            // Complication: Disk I/O graph
            if is_enabled(complication_names::DISK_IO) {
                let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
                let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
                canvas.draw_text(margin, y, "DSK:", FONT_SMALL, colors.dim);
                let disk_rates = format!("R:{} W:{}", disk_r, disk_w);
                let disk_rates_w = canvas.text_width(&disk_rates, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - disk_rates_w,
                    y,
                    &disk_rates,
                    FONT_SMALL,
                    colors.text,
                );
                y += line_height;
                canvas.draw_graph(
                    margin,
                    y,
                    bar_width,
                    GRAPH_HEIGHT,
                    &data.disk_history,
                    SystemData::compute_graph_scale(&data.disk_history),
                    colors.bar_disk,
                    colors.bar_bg,
                );
                y += GRAPH_HEIGHT as i32 + section_spacing;
            }

            // Complication: Network I/O graph
            if is_enabled(complication_names::NETWORK) {
                let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
                let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
                canvas.draw_text(margin, y, "NET:", FONT_SMALL, colors.dim);
                let net_rates = format!("\u{2193}:{} \u{2191}:{}", net_rx, net_tx);
                let net_rates_w = canvas.text_width(&net_rates, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - net_rates_w,
                    y,
                    &net_rates,
                    FONT_SMALL,
                    colors.text,
                );
                y += line_height;
                canvas.draw_graph(
                    margin,
                    y,
                    bar_width,
                    GRAPH_HEIGHT,
                    &data.net_history,
                    SystemData::compute_graph_scale(&data.net_history),
                    colors.bar_net,
                    colors.bar_bg,
                );
            }
        } else {
            // Landscape layout - compact with bars on same line as labels
            let line_height = canvas.line_height(FONT_SMALL);
            let label_width = 70_i32; // Space for "CPU: 99%" or "RAM: 99%"
            let bar_x = margin + label_width;
            let bar_width = (width as i32 - bar_x - margin - 40) as u32; // Leave room for temp

            // Hostname (always shown)
            y = 1;
            canvas.draw_text(margin, y, &data.hostname, FONT_LARGE, colors.highlight);

            // Complication: Time (right-aligned)
            if is_enabled(complication_names::TIME) {
                if time_format == time_formats::ANALOGUE {
                    // Draw small analog clock on the right
                    let clock_radius = 10_u32;
                    let clock_cx = width as i32 - margin - clock_radius as i32;
                    let clock_cy = y + clock_radius as i32;
                    draw_mini_analog_clock(
                        canvas,
                        clock_cx,
                        clock_cy,
                        clock_radius,
                        data.hour,
                        data.minute,
                        colors.highlight,
                        colors.text,
                    );
                } else {
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
            }
            y += canvas.line_height(FONT_LARGE) + 1;

            // Complication: Date (right-aligned)
            if is_enabled(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        y,
                        &date_str,
                        FONT_SMALL,
                        colors.dim,
                    );
                }
            }

            // Up: on left side
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, y, &uptime_text, FONT_SMALL, colors.dim);
            y += line_height + 1;

            // IP: label and address on same line, left aligned
            if is_enabled(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    let ip_text = format!("IP: {}", ip);
                    canvas.draw_text(margin, y, &ip_text, FONT_SMALL, colors.dim);
                    y += line_height + 2;
                }
            }

            // CPU: label, bar, and temp all on same line
            let cpu_label = format!("CPU: {:2.0}%", data.cpu_percent);
            canvas.draw_text(margin, y, &cpu_label, FONT_SMALL, colors.dim);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 2,
                bar_width,
                BAR_HEIGHT,
                data.cpu_percent,
                colors.bar_cpu,
                colors.bar_bg,
            );
            // CPU temp on same line (no label)
            if is_enabled(complication_names::CPU_TEMP) {
                if let Some(temp) = data.cpu_temp {
                    let temp_val = format!("{:.0}°C", temp);
                    let temp_w = canvas.text_width(&temp_val, FONT_SMALL);
                    canvas.draw_text(
                        width as i32 - margin - temp_w,
                        y,
                        &temp_val,
                        FONT_SMALL,
                        colors.text,
                    );
                }
            }
            y += line_height + 2;

            // RAM: label and bar on same line
            let ram_label = format!("RAM: {:2.0}%", data.ram_percent);
            canvas.draw_text(margin, y, &ram_label, FONT_SMALL, colors.dim);
            Self::draw_progress_bar(
                canvas,
                bar_x,
                y + 2,
                bar_width,
                BAR_HEIGHT,
                data.ram_percent,
                colors.bar_ram,
                colors.bar_bg,
            );
            y += line_height + 8;

            // DSK: label line, then graph on next line
            if is_enabled(complication_names::DISK_IO) {
                let disk_r = SystemData::format_rate_compact(data.disk_read_rate);
                let disk_w = SystemData::format_rate_compact(data.disk_write_rate);
                canvas.draw_text(margin, y, "DSK:", FONT_SMALL, colors.dim);
                let disk_rates = format!("R:{} W:{}", disk_r, disk_w);
                let disk_rates_w = canvas.text_width(&disk_rates, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - disk_rates_w,
                    y,
                    &disk_rates,
                    FONT_SMALL,
                    colors.text,
                );
                y += line_height;
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

            // NET: label line, then graph on next line
            if is_enabled(complication_names::NETWORK) {
                let net_rx = SystemData::format_rate_compact(data.net_rx_rate);
                let net_tx = SystemData::format_rate_compact(data.net_tx_rate);
                canvas.draw_text(margin, y, "NET:", FONT_SMALL, colors.dim);
                let net_rates = format!("\u{2193}:{} \u{2191}:{}", net_rx, net_tx);
                let net_rates_w = canvas.text_width(&net_rates, FONT_SMALL);
                canvas.draw_text(
                    width as i32 - margin - net_rates_w,
                    y,
                    &net_rates,
                    FONT_SMALL,
                    colors.text,
                );
                y += line_height;
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
