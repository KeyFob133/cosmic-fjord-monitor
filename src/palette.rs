//! Colour vocabulary for the widget.
//!
//! The scene is a Norwegian fjord at dusk under aurora, so the palette is built
//! from three families: granite/snow for the land, two depths of cold water, and
//! the Pop!_OS accent pair (teal + orange) reserved *only* for things the user is
//! meant to read: gauge arcs and the penguins' jackets. Nothing decorative uses
//! an accent colour, which keeps the readable parts readable.

use cosmic::iced::Color;

/// `0xRRGGBB` literal to a linear-ish `Color`. `Color::from_rgb8` handles the
/// sRGB conversion for us.
const fn rgb(hex: u32) -> Color {
    Color {
        r: ((hex >> 16) & 0xFF) as f32 / 255.0,
        g: ((hex >> 8) & 0xFF) as f32 / 255.0,
        b: (hex & 0xFF) as f32 / 255.0,
        a: 1.0,
    }
}

// Sky, top to horizon.
pub const SKY_HIGH: Color = rgb(0x050B14);
pub const SKY_MID: Color = rgb(0x0A1A2B);
pub const SKY_LOW: Color = rgb(0x14364A);

// Land.
pub const GRANITE_FAR: Color = rgb(0x1B2A38);
pub const GRANITE_NEAR: Color = rgb(0x101A24);
pub const SNOW: Color = rgb(0xE3EEF4);
pub const ICE: Color = rgb(0xBBD9E5);
pub const ICE_SHADOW: Color = rgb(0x6E9DB3);

// Water, surface to floor.
pub const WATER_SURFACE: Color = rgb(0x11485C);
pub const WATER_MID: Color = rgb(0x0A2E3E);
pub const WATER_DEEP: Color = rgb(0x061C27);
pub const SILT: Color = rgb(0x2A3A3C);

// Aurora ribbons.
pub const AURORA_A: Color = rgb(0x4FE0B0);
pub const AURORA_B: Color = rgb(0x7BB0FF);
/// High-altitude red. The ribbons mix toward this as the CPU heats, so
/// temperature is legible from hue alone, without reading the number.
pub const AURORA_HOT: Color = rgb(0xE8615F);

// Fish.
pub const HALIBUT_TOP: Color = rgb(0x6B6A55);
pub const HALIBUT_SPECK: Color = rgb(0x3C4038);
pub const HALIBUT_BELLY: Color = rgb(0xD8D5C4);
pub const HALIBUT_FIN: Color = rgb(0x8A8770);

// Pop!_OS accents. Reserved for gauges and jackets.
pub const POP_TEAL: Color = rgb(0x48B9C7);
pub const POP_ORANGE: Color = rgb(0xFAA41A);
pub const POP_MINT: Color = rgb(0x5FD3A6);

// Penguin.
pub const PENGUIN_BODY: Color = rgb(0x14171C);
pub const PENGUIN_BELLY: Color = rgb(0xF2F5F7);
pub const BEAK: Color = rgb(0xE8892B);

/// Panel chrome: a dark translucent slab so the widget reads as one object
/// against any wallpaper.
pub const PANEL: Color = Color {
    r: 0.02,
    g: 0.05,
    b: 0.07,
    a: 0.72,
};
pub const PANEL_EDGE: Color = Color {
    r: 0.45,
    g: 0.72,
    b: 0.78,
    a: 0.28,
};
pub const GAUGE_TRACK: Color = Color {
    r: 0.62,
    g: 0.72,
    b: 0.76,
    a: 0.20,
};
pub const TEXT_DIM: Color = Color {
    r: 0.72,
    g: 0.82,
    b: 0.86,
    a: 0.70,
};

/// Same colour, different opacity.
pub const fn alpha(c: Color, a: f32) -> Color {
    Color { a, ..c }
}

/// Straight-line blend, `t` clamped to `0.0..=1.0` by the caller.
pub fn mix(a: Color, b: Color, t: f32) -> Color {
    Color {
        r: a.r + (b.r - a.r) * t,
        g: a.g + (b.g - a.g) * t,
        b: a.b + (b.b - a.b) * t,
        a: a.a + (b.a - a.a) * t,
    }
}
