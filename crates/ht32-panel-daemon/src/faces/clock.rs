//! Clock face displaying a clean analog clock.
//!
//! A minimalist watch face focused on time display with optional
//! date and hostname complications.

use std::f32::consts::PI;

use super::{
    complication_options, complications, date_formats, Complication, ComplicationChoice,
    ComplicationOption, EnabledComplications, Face, Theme,
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

/// Derive colors from theme for the clock face.
struct FaceColors {
    /// Clock face outline and markers
    outline: u32,
    /// Hour hand color
    hour_hand: u32,
    /// Minute hand color
    minute_hand: u32,
    /// Center dot color
    center: u32,
    /// Text color (for complications)
    text: u32,
    /// Dimmed text color
    dim: u32,
}

impl FaceColors {
    fn from_theme(theme: &Theme) -> Self {
        Self {
            outline: theme.primary,
            hour_hand: theme.text,
            minute_hand: theme.text,
            center: theme.primary,
            text: theme.text,
            dim: dim_color(theme.text, theme.background, 0.7),
        }
    }
}

/// Font sizes.
const FONT_NORMAL: f32 = 14.0;
const FONT_SMALL: f32 = 12.0;

/// A minimalist analog clock face.
pub struct ClockFace;

impl ClockFace {
    /// Creates a new clock face.
    pub fn new() -> Self {
        Self
    }

    /// Draws an analog clock.
    fn draw_analog_clock(
        canvas: &mut Canvas,
        cx: i32,
        cy: i32,
        radius: u32,
        hour: u8,
        minute: u8,
        colors: &FaceColors,
    ) {
        let radius_f = radius as f32;

        // Draw clock face outline
        let start_angle = 0.0;
        let end_angle = 2.0 * PI;
        canvas.draw_arc(cx, cy, radius, start_angle, end_angle, 2.0, colors.outline);

        // Draw hour markers
        for i in 0..12 {
            let angle = (i as f32) * PI / 6.0 - PI / 2.0; // Start from 12 o'clock
            let inner_r = radius_f * 0.85;
            let outer_r = radius_f * 0.95;

            let x1 = cx as f32 + inner_r * angle.cos();
            let y1 = cy as f32 + inner_r * angle.sin();
            let x2 = cx as f32 + outer_r * angle.cos();
            let y2 = cy as f32 + outer_r * angle.sin();

            // Thicker markers at 12, 3, 6, 9
            let stroke = if i % 3 == 0 { 3.0 } else { 1.5 };
            canvas.draw_line(
                x1 as i32,
                y1 as i32,
                x2 as i32,
                y2 as i32,
                stroke,
                colors.outline,
            );
        }

        // Calculate hand angles (12 o'clock = -PI/2)
        let minute_angle = (minute as f32) * PI / 30.0 - PI / 2.0;
        let hour_angle = ((hour % 12) as f32 + minute as f32 / 60.0) * PI / 6.0 - PI / 2.0;

        // Draw hour hand (shorter, thicker)
        let hour_length = radius_f * 0.5;
        let hour_x = cx as f32 + hour_length * hour_angle.cos();
        let hour_y = cy as f32 + hour_length * hour_angle.sin();
        canvas.draw_line(cx, cy, hour_x as i32, hour_y as i32, 4.0, colors.hour_hand);

        // Draw minute hand (longer, thinner)
        let minute_length = radius_f * 0.75;
        let minute_x = cx as f32 + minute_length * minute_angle.cos();
        let minute_y = cy as f32 + minute_length * minute_angle.sin();
        canvas.draw_line(
            cx,
            cy,
            minute_x as i32,
            minute_y as i32,
            2.5,
            colors.minute_hand,
        );

        // Draw center dot
        canvas.fill_circle(cx, cy, 4, colors.center);
    }
}

impl Default for ClockFace {
    fn default() -> Self {
        Self::new()
    }
}

impl Face for ClockFace {
    fn name(&self) -> &str {
        "clock"
    }

    fn available_complications(&self) -> Vec<Complication> {
        vec![
            Complication::with_options(
                complications::DATE,
                "Date",
                "Display the current date",
                false,
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
                    date_formats::SHORT,
                )],
            ),
            Complication::new("hostname", "Hostname", "Display the system hostname", false),
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

        let is_on = |id: &str| comp.is_enabled(self.name(), id, false);

        // Get date format option
        let date_format = comp
            .get_option(
                self.name(),
                complications::DATE,
                complication_options::DATE_FORMAT,
            )
            .map(|s| s.as_str())
            .unwrap_or(date_formats::SHORT);

        // Calculate clock size and position
        // Use the smaller dimension to ensure the clock fits
        let margin = 10;
        let max_radius = ((width.min(height) as i32 - margin * 2) / 2) as u32;

        // Adjust for complications
        let show_date = is_on(complications::DATE);
        let show_hostname = is_on("hostname");
        let complication_space = if show_date || show_hostname { 20 } else { 0 };

        let radius = max_radius.saturating_sub(complication_space as u32 / 2);
        let cx = width as i32 / 2;
        let cy = if show_date || show_hostname {
            height as i32 / 2 - complication_space / 2
        } else {
            height as i32 / 2
        };

        // Draw the analog clock
        Self::draw_analog_clock(canvas, cx, cy, radius, data.hour, data.minute, &colors);

        // Draw complications below the clock
        let mut bottom_y = cy + radius as i32 + 8;

        if show_date {
            if let Some(date_str) = data.format_date(date_format) {
                let date_width = canvas.text_width(&date_str, FONT_NORMAL);
                let date_x = (width as i32 - date_width) / 2;
                canvas.draw_text(date_x, bottom_y, &date_str, FONT_NORMAL, colors.text);
                bottom_y += canvas.line_height(FONT_NORMAL) + 2;
            }
        }

        if show_hostname {
            let host_width = canvas.text_width(&data.hostname, FONT_SMALL);
            let host_x = (width as i32 - host_width) / 2;
            canvas.draw_text(host_x, bottom_y, &data.hostname, FONT_SMALL, colors.dim);
        }
    }
}
