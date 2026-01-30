//! Arcs face using only circles and arcs for data display.
//!
//! All metrics are shown as circular arc gauges rather than
//! traditional bars or graphs.

use std::f32::consts::PI;

use super::{
    complication_options, complications, date_formats, complication_names, time_formats,
    Complication, EnabledComplications, Face, Theme,
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

/// Derive colors from theme for the arcs face.
struct FaceColors {
    /// Primary arc color (CPU)
    primary: u32,
    /// Secondary arc color (RAM)
    secondary: u32,
    /// Arc background (unfilled portion)
    arc_bg: u32,
    /// Text color
    text: u32,
    /// Dimmed text color
    dim: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            primary: theme.primary,
            secondary: theme.secondary,
            arc_bg: dim_color(theme.primary, theme.background, 0.25),
            text: theme.text,
            dim: dim_color(theme.text, theme.background, 0.7), // Higher factor for better contrast
        }
    }
}

/// Font sizes.
const FONT_LARGE: f32 = 18.0;
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;
const FONT_TINY: f32 = 11.0;

/// Formats a byte rate compactly with max 4 characters (e.g., "1.2M", "12M", "999K").
fn format_rate_short(bytes_per_sec: f64) -> String {
    if bytes_per_sec >= 1_000_000_000.0 {
        let val = bytes_per_sec / 1_000_000_000.0;
        if val >= 10.0 {
            format!("{:.0}G", val)
        } else {
            format!("{:.1}G", val)
        }
    } else if bytes_per_sec >= 1_000_000.0 {
        let val = bytes_per_sec / 1_000_000.0;
        if val >= 10.0 {
            format!("{:.0}M", val)
        } else {
            format!("{:.1}M", val)
        }
    } else if bytes_per_sec >= 1_000.0 {
        let val = bytes_per_sec / 1_000.0;
        if val >= 10.0 {
            format!("{:.0}K", val)
        } else {
            format!("{:.1}K", val)
        }
    } else {
        format!("{:.0}", bytes_per_sec)
    }
}

/// A face using circular arc gauges for all metrics.
pub struct ArcsFace;

impl ArcsFace {
    /// Creates a new arcs face.
    pub fn new() -> Self {
        Self
    }

    /// Draws a circular arc gauge.
    ///
    /// The gauge spans from start_angle to end_angle, with the filled
    /// portion determined by the percent value.
    #[allow(clippy::too_many_arguments)]
    fn draw_arc_gauge(
        canvas: &mut Canvas,
        cx: i32,
        cy: i32,
        radius: u32,
        stroke_width: f32,
        percent: f64,
        fg_color: u32,
        bg_color: u32,
    ) {
        // Draw arc from 135° to 405° (270° sweep, starting bottom-left)
        let start_angle = 135.0 * PI / 180.0;
        let end_angle = 405.0 * PI / 180.0;
        let sweep = end_angle - start_angle;

        // Background arc (full sweep)
        canvas.draw_arc(
            cx,
            cy,
            radius,
            start_angle,
            end_angle,
            stroke_width,
            bg_color,
        );

        // Foreground arc (partial, based on percent)
        if percent > 0.0 {
            let fill_angle = start_angle + sweep * (percent.min(100.0) / 100.0) as f32;
            canvas.draw_arc(
                cx,
                cy,
                radius,
                start_angle,
                fill_angle,
                stroke_width,
                fg_color,
            );
        }
    }

    /// Draws a small activity indicator arc.
    /// Uses logarithmic scaling for better visualization of varying rates.
    #[allow(clippy::too_many_arguments)]
    fn draw_activity_arc(
        canvas: &mut Canvas,
        cx: i32,
        cy: i32,
        radius: u32,
        stroke_width: f32,
        value: f64,
        max_value: f64,
        fg_color: u32,
        bg_color: u32,
    ) {
        let start_angle = 135.0 * PI / 180.0;
        let end_angle = 405.0 * PI / 180.0;
        let sweep = end_angle - start_angle;

        // Background arc
        canvas.draw_arc(
            cx,
            cy,
            radius,
            start_angle,
            end_angle,
            stroke_width,
            bg_color,
        );

        // Use logarithmic scaling for activity
        if value > 0.0 && max_value > 0.0 {
            let log_value = (1.0 + value).ln();
            let log_max = (1.0 + max_value).ln();
            let normalized = (log_value / log_max).min(1.0);
            let fill_angle = start_angle + sweep * normalized as f32;
            canvas.draw_arc(
                cx,
                cy,
                radius,
                start_angle,
                fill_angle,
                stroke_width,
                fg_color,
            );
        }
    }
}

impl Default for ArcsFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for ArcsFace {
    fn name(&self) -> &str {
        "arcs"
    }

    fn available_complications(&self) -> Vec<Complication> {
        vec![
            complications::time(true),
            complications::date(true, date_formats::ISO),
            complications::ip_address(true),
            complications::network(true),
            complications::disk_io(true),
            complications::cpu_temp(false),
        ]
    }

    fn render(
        &self,
        canvas: &mut Canvas,
        data: &SystemData,
        theme: &Theme,
        comp: &EnabledComplications,
    ) {
        let colors = FaceColors::from_theme(theme);
        let (width, height) = canvas.dimensions();
        let portrait = width < 200;

        let is_on = |id: &str| comp.is_enabled(self.name(), id, true);

        // Get time format option
        let time_format = comp
            .get_option(
                self.name(),
                complication_names::TIME,
                complication_options::TIME_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(time_formats::DIGITAL_24H);

        // Get date format option
        let date_format = comp
            .get_option(
                self.name(),
                complication_names::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::ISO);

        if portrait {
            let margin = 8;
            let gauge_radius = 28_u32;
            let stroke = 6.0;
            let small_radius = 18_u32;
            let small_stroke = 4.0;

            let mut top_y = margin;

            // Complication: Time at top
            if is_on(complication_names::TIME) && time_format != time_formats::ANALOGUE {
                let time_str = data.format_time(time_format);
                let time_width = canvas.text_width(&time_str, FONT_LARGE);
                let time_x = (width as i32 - time_width) / 2;
                canvas.draw_text(time_x, top_y, &time_str, FONT_LARGE, colors.text);
                top_y += canvas.line_height(FONT_LARGE) + 2;
            }

            // Complication: Date (centered)
            if is_on(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_SMALL);
                    let date_x = (width as i32 - date_width) / 2;
                    canvas.draw_text(date_x, top_y, &date_str, FONT_SMALL, colors.dim);
                    top_y += canvas.line_height(FONT_SMALL) + 4;
                }
            }

            // Position gauges below time/date section
            let row1_y = top_y.max(margin + 8);
            let cpu_cx = margin + gauge_radius as i32 + 4;
            let cpu_cy = row1_y + gauge_radius as i32;

            // Base element: CPU gauge (always shown)
            Self::draw_arc_gauge(
                canvas,
                cpu_cx,
                cpu_cy,
                gauge_radius,
                stroke,
                data.cpu_percent,
                colors.primary,
                colors.arc_bg,
            );
            canvas.draw_text(cpu_cx - 10, cpu_cy - 6, "CPU", FONT_TINY, colors.dim);
            let cpu_text = format!("{:.0}", data.cpu_percent);
            let cpu_w = canvas.text_width(&cpu_text, FONT_SMALL);
            canvas.draw_text(
                cpu_cx - cpu_w / 2,
                cpu_cy + 4,
                &cpu_text,
                FONT_SMALL,
                colors.text,
            );

            // Base element: RAM gauge (always shown)
            let ram_cx = width as i32 - margin - gauge_radius as i32 - 4;
            let ram_cy = cpu_cy;
            Self::draw_arc_gauge(
                canvas,
                ram_cx,
                ram_cy,
                gauge_radius,
                stroke,
                data.ram_percent,
                colors.secondary,
                colors.arc_bg,
            );
            canvas.draw_text(ram_cx - 10, ram_cy - 6, "RAM", FONT_TINY, colors.dim);
            let ram_text = format!("{:.0}", data.ram_percent);
            let ram_w = canvas.text_width(&ram_text, FONT_SMALL);
            canvas.draw_text(
                ram_cx - ram_w / 2,
                ram_cy + 4,
                &ram_text,
                FONT_SMALL,
                colors.text,
            );

            let row2_y = row1_y + gauge_radius as i32 * 2 + 16;
            let io_max = 100_000_000.0;
            let disk_r_cx = margin + small_radius as i32 + 4;
            let disk_r_cy = row2_y + small_radius as i32;

            // Complication: Disk gauges
            if is_on(complication_names::DISK_IO) {
                Self::draw_activity_arc(
                    canvas,
                    disk_r_cx,
                    disk_r_cy,
                    small_radius,
                    small_stroke,
                    data.disk_read_rate,
                    io_max,
                    colors.primary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let disk_r_text = format_rate_short(data.disk_read_rate);
                let disk_r_w = canvas.text_width(&disk_r_text, FONT_TINY);
                canvas.draw_text(
                    disk_r_cx - disk_r_w / 2,
                    disk_r_cy - 5,
                    &disk_r_text,
                    FONT_TINY,
                    colors.text,
                );
                // Letter in bottom open space
                canvas.draw_text(
                    disk_r_cx - 3,
                    disk_r_cy + small_radius as i32 - 2,
                    "R",
                    FONT_TINY,
                    colors.dim,
                );

                let disk_w_cx = disk_r_cx + small_radius as i32 * 2 + 8;
                Self::draw_activity_arc(
                    canvas,
                    disk_w_cx,
                    disk_r_cy,
                    small_radius,
                    small_stroke,
                    data.disk_write_rate,
                    io_max,
                    colors.primary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let disk_w_text = format_rate_short(data.disk_write_rate);
                let disk_w_w = canvas.text_width(&disk_w_text, FONT_TINY);
                canvas.draw_text(
                    disk_w_cx - disk_w_w / 2,
                    disk_r_cy - 5,
                    &disk_w_text,
                    FONT_TINY,
                    colors.text,
                );
                // Letter in bottom open space
                canvas.draw_text(
                    disk_w_cx - 4,
                    disk_r_cy + small_radius as i32 - 2,
                    "W",
                    FONT_TINY,
                    colors.dim,
                );
            }

            // Complication: Network gauges
            if is_on(complication_names::NETWORK) {
                let net_rx_cx = width as i32 - margin - small_radius as i32 * 4 - 12;
                Self::draw_activity_arc(
                    canvas,
                    net_rx_cx,
                    disk_r_cy,
                    small_radius,
                    small_stroke,
                    data.net_rx_rate,
                    io_max,
                    colors.secondary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let net_rx_text = format_rate_short(data.net_rx_rate);
                let net_rx_w = canvas.text_width(&net_rx_text, FONT_TINY);
                canvas.draw_text(
                    net_rx_cx - net_rx_w / 2,
                    disk_r_cy - 5,
                    &net_rx_text,
                    FONT_TINY,
                    colors.text,
                );
                // Arrow in bottom open space
                canvas.draw_text(
                    net_rx_cx - 4,
                    disk_r_cy + small_radius as i32 - 2,
                    "\u{2193}",
                    FONT_TINY,
                    colors.dim,
                );

                let net_tx_cx = width as i32 - margin - small_radius as i32 - 4;
                Self::draw_activity_arc(
                    canvas,
                    net_tx_cx,
                    disk_r_cy,
                    small_radius,
                    small_stroke,
                    data.net_tx_rate,
                    io_max,
                    colors.secondary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let net_tx_text = format_rate_short(data.net_tx_rate);
                let net_tx_w = canvas.text_width(&net_tx_text, FONT_TINY);
                canvas.draw_text(
                    net_tx_cx - net_tx_w / 2,
                    disk_r_cy - 5,
                    &net_tx_text,
                    FONT_TINY,
                    colors.text,
                );
                // Arrow in bottom open space
                canvas.draw_text(
                    net_tx_cx - 4,
                    disk_r_cy + small_radius as i32 - 2,
                    "\u{2191}",
                    FONT_TINY,
                    colors.dim,
                );
            }

            // Base elements: Hostname and uptime at bottom (always shown)
            let bottom_y = height as i32 - margin - 42;
            canvas.draw_text(margin, bottom_y, &data.hostname, FONT_SMALL, colors.dim);
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, bottom_y + 14, &uptime_text, FONT_TINY, colors.dim);

            // Complication: IP address
            if is_on(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    let ip_width = canvas.text_width(ip, FONT_TINY);
                    canvas.draw_text(
                        (width as i32 - ip_width) / 2,
                        bottom_y + 28,
                        ip,
                        FONT_TINY,
                        colors.dim,
                    );
                }
            }
        } else {
            // Landscape layout
            let margin = 10;
            let gauge_radius = 36_u32;
            let stroke = 8.0;
            let small_radius = 22_u32;
            let small_stroke = 5.0;

            let top_y = margin;

            // Complication: Time
            if is_on(complication_names::TIME) && time_format != time_formats::ANALOGUE {
                let time_str = data.format_time(time_format);
                canvas.draw_text(margin, top_y, &time_str, FONT_LARGE, colors.text);
            }

            // Hostname at top right (always shown)
            let host_width = canvas.text_width(&data.hostname, FONT_SMALL);
            canvas.draw_text(
                width as i32 - margin - host_width,
                top_y,
                &data.hostname,
                FONT_SMALL,
                colors.dim,
            );

            // Complication: Date (below hostname if shown)
            if is_on(complication_names::DATE) {
                if let Some(date_str) = data.format_date(date_format) {
                    let date_width = canvas.text_width(&date_str, FONT_TINY);
                    canvas.draw_text(
                        width as i32 - margin - date_width,
                        top_y + 14,
                        &date_str,
                        FONT_TINY,
                        colors.dim,
                    );
                }
            }

            let gauge_y = margin + 28 + gauge_radius as i32;
            let cpu_cx = margin + gauge_radius as i32 + 10;

            // Base element: CPU gauge (always shown)
            Self::draw_arc_gauge(
                canvas,
                cpu_cx,
                gauge_y,
                gauge_radius,
                stroke,
                data.cpu_percent,
                colors.primary,
                colors.arc_bg,
            );
            canvas.draw_text(cpu_cx - 10, gauge_y - 8, "CPU", FONT_TINY, colors.dim);
            let cpu_text = format!("{:.0}%", data.cpu_percent);
            let cpu_w = canvas.text_width(&cpu_text, FONT_NORMAL);
            canvas.draw_text(
                cpu_cx - cpu_w / 2,
                gauge_y + 2,
                &cpu_text,
                FONT_NORMAL,
                colors.text,
            );

            // Base element: RAM gauge (always shown)
            let ram_cx = cpu_cx + gauge_radius as i32 * 2 + 30;
            Self::draw_arc_gauge(
                canvas,
                ram_cx,
                gauge_y,
                gauge_radius,
                stroke,
                data.ram_percent,
                colors.secondary,
                colors.arc_bg,
            );
            canvas.draw_text(ram_cx - 12, gauge_y - 8, "RAM", FONT_TINY, colors.dim);
            let ram_text = format!("{:.0}%", data.ram_percent);
            let ram_w = canvas.text_width(&ram_text, FONT_NORMAL);
            canvas.draw_text(
                ram_cx - ram_w / 2,
                gauge_y + 2,
                &ram_text,
                FONT_NORMAL,
                colors.text,
            );

            let io_x = ram_cx + gauge_radius as i32 + 40;
            let io_max = 100_000_000.0;
            let disk_r_cx = io_x;
            let disk_cy = margin + 28 + small_radius as i32;

            // Complication: Disk gauges
            if is_on(complication_names::DISK_IO) {
                Self::draw_activity_arc(
                    canvas,
                    disk_r_cx,
                    disk_cy,
                    small_radius,
                    small_stroke,
                    data.disk_read_rate,
                    io_max,
                    colors.primary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let disk_r_text = format_rate_short(data.disk_read_rate);
                let disk_r_w = canvas.text_width(&disk_r_text, FONT_TINY);
                canvas.draw_text(
                    disk_r_cx - disk_r_w / 2,
                    disk_cy - 5,
                    &disk_r_text,
                    FONT_TINY,
                    colors.text,
                );
                // Letter in bottom open space
                canvas.draw_text(
                    disk_r_cx - 3,
                    disk_cy + small_radius as i32 - 2,
                    "R",
                    FONT_TINY,
                    colors.dim,
                );

                let disk_w_cx = disk_r_cx + small_radius as i32 * 2 + 12;
                Self::draw_activity_arc(
                    canvas,
                    disk_w_cx,
                    disk_cy,
                    small_radius,
                    small_stroke,
                    data.disk_write_rate,
                    io_max,
                    colors.primary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let disk_w_text = format_rate_short(data.disk_write_rate);
                let disk_w_w = canvas.text_width(&disk_w_text, FONT_TINY);
                canvas.draw_text(
                    disk_w_cx - disk_w_w / 2,
                    disk_cy - 5,
                    &disk_w_text,
                    FONT_TINY,
                    colors.text,
                );
                // Letter in bottom open space
                canvas.draw_text(
                    disk_w_cx - 4,
                    disk_cy + small_radius as i32 - 2,
                    "W",
                    FONT_TINY,
                    colors.dim,
                );
            }

            // Complication: Network gauges
            let net_cy = disk_cy + small_radius as i32 * 2 + 12;
            if is_on(complication_names::NETWORK) {
                Self::draw_activity_arc(
                    canvas,
                    disk_r_cx,
                    net_cy,
                    small_radius,
                    small_stroke,
                    data.net_rx_rate,
                    io_max,
                    colors.secondary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let net_rx_text = format_rate_short(data.net_rx_rate);
                let net_rx_w = canvas.text_width(&net_rx_text, FONT_TINY);
                canvas.draw_text(
                    disk_r_cx - net_rx_w / 2,
                    net_cy - 5,
                    &net_rx_text,
                    FONT_TINY,
                    colors.text,
                );
                // Arrow in bottom open space
                canvas.draw_text(
                    disk_r_cx - 4,
                    net_cy + small_radius as i32 - 2,
                    "\u{2193}",
                    FONT_TINY,
                    colors.dim,
                );

                let net_w_cx = disk_r_cx + small_radius as i32 * 2 + 12;
                Self::draw_activity_arc(
                    canvas,
                    net_w_cx,
                    net_cy,
                    small_radius,
                    small_stroke,
                    data.net_tx_rate,
                    io_max,
                    colors.secondary,
                    colors.arc_bg,
                );
                // Number centered in dial
                let net_tx_text = format_rate_short(data.net_tx_rate);
                let net_tx_w = canvas.text_width(&net_tx_text, FONT_TINY);
                canvas.draw_text(
                    net_w_cx - net_tx_w / 2,
                    net_cy - 5,
                    &net_tx_text,
                    FONT_TINY,
                    colors.text,
                );
                // Arrow in bottom open space
                canvas.draw_text(
                    net_w_cx - 4,
                    net_cy + small_radius as i32 - 2,
                    "\u{2191}",
                    FONT_TINY,
                    colors.dim,
                );
            }

            // Base element: Uptime at bottom (always shown)
            let bottom_y = height as i32 - margin - 14;
            let uptime_text = format!("Up: {}", data.uptime);
            canvas.draw_text(margin, bottom_y, &uptime_text, FONT_TINY, colors.dim);

            // Complication: IP address
            if is_on(complication_names::IP_ADDRESS) {
                if let Some(ref ip) = data.display_ip {
                    let ip_width = canvas.text_width(ip, FONT_TINY);
                    canvas.draw_text(
                        width as i32 - margin - ip_width,
                        bottom_y,
                        ip,
                        FONT_TINY,
                        colors.dim,
                    );
                }
            }
        }
    }
}
