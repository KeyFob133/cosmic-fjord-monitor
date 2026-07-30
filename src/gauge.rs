//! The ring gauges.
//!
//! Each gauge is a ring of discrete ticks rather than a continuous arc. Ticks are
//! easier to read at a glance from across a desk than a smooth sweep, and they
//! quantise the value, which stops a fluctuating CPU reading from looking like the
//! widget is vibrating.

use cosmic::iced::widget::canvas::{Frame, LineCap, Path, Stroke};
use cosmic::iced::{Color, Point};

use crate::draw::{centered, digits_font, label_font};
use crate::palette;

const TICKS: usize = 32;
const TAU: f32 = std::f32::consts::TAU;

pub struct Gauge<'a> {
    pub center: Point,
    pub radius: f32,
    /// `None` renders an unlit ring: the metric exists but could not be read.
    pub value: Option<f32>,
    pub accent: Color,
    /// Metric name, above the number.
    pub label: &'a str,
    /// The number itself, already formatted.
    pub value_text: &'a str,
    /// Secondary line under the number: temperature, or used-of-total.
    pub sub_text: &'a str,
}

impl Gauge<'_> {
    pub fn draw(&self, frame: &mut Frame) {
        let value = self.value.unwrap_or(0.0).clamp(0.0, 1.0);
        let lit = (value * TICKS as f32).round() as usize;

        let tick_len = self.radius * 0.24;
        let tick_width = (self.radius * 0.11).max(2.0);
        let outer = self.radius;
        let inner = self.radius - tick_len;

        for i in 0..TICKS {
            // Start at twelve o'clock and run clockwise. The half-tick offset
            // centres the first tick on the top axis instead of straddling it.
            let angle = -TAU / 4.0 + (i as f32 + 0.5) / TICKS as f32 * TAU;
            let (sin, cos) = angle.sin_cos();

            let from = Point::new(self.center.x + cos * inner, self.center.y + sin * inner);
            let to = Point::new(self.center.x + cos * outer, self.center.y + sin * outer);
            let segment = Path::line(from, to);

            let is_lit = i < lit;
            let color = if is_lit {
                self.accent
            } else {
                palette::GAUGE_TRACK
            };

            // Lit ticks get a wide, faint pass underneath to suggest bloom.
            if is_lit {
                frame.stroke(
                    &segment,
                    Stroke::default()
                        .with_color(palette::alpha(self.accent, 0.22))
                        .with_width(tick_width * 2.4)
                        .with_line_cap(LineCap::Round),
                );
            }

            frame.stroke(
                &segment,
                Stroke::default()
                    .with_color(color)
                    .with_width(tick_width)
                    .with_line_cap(LineCap::Round),
            );
        }

        // A hairline inside the ticks closes the shape and separates the readout
        // from whatever scenery is behind the gauge.
        frame.stroke(
            &Path::circle(self.center, inner - tick_width * 0.9),
            Stroke::default()
                .with_color(palette::alpha(palette::PANEL_EDGE, 0.5))
                .with_width(1.0),
        );

        frame.fill(
            &Path::circle(self.center, inner - tick_width),
            palette::alpha(palette::WATER_DEEP, 0.78),
        );

        let unread = self.value.is_none();

        centered(
            frame,
            self.label,
            Point::new(self.center.x, self.center.y - self.radius * 0.42),
            self.radius * 0.20,
            if unread {
                palette::alpha(self.accent, 0.35)
            } else {
                self.accent
            },
            label_font(),
        );

        centered(
            frame,
            self.value_text,
            Point::new(self.center.x, self.center.y + self.radius * 0.02),
            self.radius * 0.40,
            if unread {
                palette::TEXT_DIM
            } else {
                palette::SNOW
            },
            digits_font(),
        );

        if !self.sub_text.is_empty() {
            centered(
                frame,
                self.sub_text,
                Point::new(self.center.x, self.center.y + self.radius * 0.46),
                self.radius * 0.17,
                palette::TEXT_DIM,
                digits_font(),
            );
        }
    }
}

/// Format a temperature for the sub-line, or an em dash if the sensor is absent.
pub fn temp_text(celsius: Option<f32>) -> String {
    match celsius {
        Some(t) => format!("{t:.0}\u{00B0}C"),
        None => "\u{2014}".to_string(),
    }
}
