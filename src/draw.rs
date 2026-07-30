//! Small drawing primitives shared by the scene modules.

use cosmic::iced::widget::canvas::path::Builder;
use cosmic::iced::widget::canvas::{Frame, Text};
use cosmic::iced::{Color, Font, Pixels, Point};

/// Every font used by the widget goes through here, so a rename upstream is a
/// one-line fix rather than a sweep through the drawing code.
pub fn digits_font() -> Font {
    cosmic::font::mono()
}

pub fn label_font() -> Font {
    cosmic::font::semibold()
}

/// Approximate advance width per character, as a fraction of the font size.
/// The canvas API cannot measure text, so centring is done from these constants;
/// they are close enough for two- and three-glyph readouts.
const MONO_ADVANCE: f32 = 0.60;
const LABEL_ADVANCE: f32 = 0.53;

/// Draw text centred on `at` horizontally, with `at.y` as the vertical centre.
pub fn centered(frame: &mut Frame, content: &str, at: Point, size: f32, color: Color, font: Font) {
    let advance = if font == digits_font() {
        MONO_ADVANCE
    } else {
        LABEL_ADVANCE
    };
    let width = content.chars().count() as f32 * size * advance;

    frame.fill_text(Text {
        content: content.to_string(),
        position: Point::new(at.x - width / 2.0, at.y - size * 0.62),
        color,
        size: Pixels(size),
        font,
        ..Text::default()
    });
}

/// Trace a rounded rectangle. Written out with explicit curves rather than using
/// the builder's rectangle helper so the corner radius can exceed half the
/// shorter side without surprises: it is clamped here.
pub fn rounded_rect(b: &mut Builder, x: f32, y: f32, w: f32, h: f32, r: f32) {
    let r = r.min(w / 2.0).min(h / 2.0).max(0.0);

    b.move_to(Point::new(x + r, y));
    b.line_to(Point::new(x + w - r, y));
    b.quadratic_curve_to(Point::new(x + w, y), Point::new(x + w, y + r));
    b.line_to(Point::new(x + w, y + h - r));
    b.quadratic_curve_to(Point::new(x + w, y + h), Point::new(x + w - r, y + h));
    b.line_to(Point::new(x + r, y + h));
    b.quadratic_curve_to(Point::new(x, y + h), Point::new(x, y + h - r));
    b.line_to(Point::new(x, y + r));
    b.quadratic_curve_to(Point::new(x, y), Point::new(x + r, y));
    b.close();
}

/// Deterministic generator for scenery placement.
///
/// The scene must look identical on every frame and on every launch, so nothing
/// visual may come from a real random source: stars, speckles and mountain
/// profiles are all derived from a fixed seed. This is a 64-bit xorshift, chosen
/// because it is four lines and has more than enough quality for placing dots.
pub struct Rng(u64);

impl Rng {
    pub fn new(seed: u64) -> Self {
        // A zero state would be a fixed point, so force it away from zero.
        Self(seed | 1)
    }

    fn next_u64(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }

    /// Uniform in `0.0..1.0`.
    pub fn unit(&mut self) -> f32 {
        (self.next_u64() >> 40) as f32 / (1u64 << 24) as f32
    }

    pub fn range(&mut self, lo: f32, hi: f32) -> f32 {
        lo + self.unit() * (hi - lo)
    }
}

/// Smooth 0-to-1 ramp, used for fading things in and out of the scene.
pub fn smoothstep(edge0: f32, edge1: f32, x: f32) -> f32 {
    if (edge1 - edge0).abs() < f32::EPSILON {
        return if x < edge0 { 0.0 } else { 1.0 };
    }
    let t = ((x - edge0) / (edge1 - edge0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}
