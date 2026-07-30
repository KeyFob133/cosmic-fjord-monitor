//! Penguins, dressed for the weather.
//!
//! Everything is drawn in a local frame whose origin is between the feet, with
//! `-y` pointing up, and scaled from a single `height` value. That means a penguin
//! can be placed anywhere at any size and all proportions follow.
//!
//! The jacket is a quilted puffer: it sits slightly wider than the body, carries
//! horizontal seams, and leaves the white belly showing below the hem, which is
//! what makes it read as clothing rather than as markings.

use cosmic::iced::widget::canvas::{Frame, LineCap, Path, Stroke};
use cosmic::iced::{Color, Point, Vector};

use crate::draw::{centered, label_font, rounded_rect};
use crate::palette;

pub struct Penguin<'a> {
    /// Point between the feet.
    pub feet: Point,
    /// Total height from feet to crown, in pixels.
    pub height: f32,
    pub jacket: Color,
    /// Text across the chest.
    pub text: &'a str,
    /// Animation phase in radians.
    pub phase: f32,
    /// `-1.0` faces left, `1.0` faces right.
    pub facing: f32,
}

impl Penguin<'_> {
    pub fn draw(&self, frame: &mut Frame) {
        let h = self.height;
        let bob = (self.phase).sin() * h * 0.018;
        let sway = (self.phase * 0.5).sin() * 0.035;

        frame.with_save(|frame| {
            frame.translate(Vector::new(self.feet.x, self.feet.y + bob));
            frame.rotate(sway);

            self.feet_and_shadow(frame, h);
            self.body(frame, h);
            self.jacket_body(frame, h);
            self.sleeves(frame, h);
            self.head(frame, h);
            self.chest_text(frame, h);
        });
    }

    fn feet_and_shadow(&self, frame: &mut Frame, h: f32) {
        let shadow = Path::new(|b| {
            b.move_to(Point::new(-h * 0.24, 0.0));
            b.quadratic_curve_to(
                Point::new(0.0, h * 0.06),
                Point::new(h * 0.24, 0.0),
            );
            b.quadratic_curve_to(Point::new(0.0, -h * 0.03), Point::new(-h * 0.24, 0.0));
        });
        frame.fill(&shadow, palette::alpha(palette::ICE_SHADOW, 0.35));

        for side in [-1.0_f32, 1.0] {
            let x = side * h * 0.09;
            let toe = Path::new(|b| {
                b.move_to(Point::new(x, -h * 0.04));
                b.line_to(Point::new(x + self.facing * h * 0.11, h * 0.01));
                b.quadratic_curve_to(
                    Point::new(x + self.facing * h * 0.06, h * 0.035),
                    Point::new(x - self.facing * h * 0.02, h * 0.02),
                );
                b.close();
            });
            frame.fill(&toe, palette::BEAK);
        }
    }

    /// Black body: a pear shape, wider at the hips than the shoulders.
    fn body(&self, frame: &mut Frame, h: f32) {
        let outline = Path::new(|b| {
            b.move_to(Point::new(0.0, -h * 0.80));
            b.quadratic_curve_to(
                Point::new(h * 0.24, -h * 0.66),
                Point::new(h * 0.23, -h * 0.30),
            );
            b.quadratic_curve_to(
                Point::new(h * 0.21, -h * 0.02),
                Point::new(0.0, -h * 0.01),
            );
            b.quadratic_curve_to(
                Point::new(-h * 0.21, -h * 0.02),
                Point::new(-h * 0.23, -h * 0.30),
            );
            b.quadratic_curve_to(
                Point::new(-h * 0.24, -h * 0.66),
                Point::new(0.0, -h * 0.80),
            );
            b.close();
        });
        frame.fill(&outline, palette::PENGUIN_BODY);

        // Belly, most of which the jacket will cover. The part left showing below
        // the hem is what sells the jacket as a separate garment.
        let belly = Path::new(|b| {
            b.move_to(Point::new(0.0, -h * 0.70));
            b.quadratic_curve_to(
                Point::new(h * 0.17, -h * 0.52),
                Point::new(h * 0.15, -h * 0.16),
            );
            b.quadratic_curve_to(
                Point::new(h * 0.09, -h * 0.03),
                Point::new(0.0, -h * 0.03),
            );
            b.quadratic_curve_to(
                Point::new(-h * 0.09, -h * 0.03),
                Point::new(-h * 0.15, -h * 0.16),
            );
            b.quadratic_curve_to(
                Point::new(-h * 0.17, -h * 0.52),
                Point::new(0.0, -h * 0.70),
            );
            b.close();
        });
        frame.fill(&belly, palette::PENGUIN_BELLY);
    }

    fn jacket_body(&self, frame: &mut Frame, h: f32) {
        let top = -h * 0.70;
        let bottom = -h * 0.22;
        let half = h * 0.27;

        let shell = Path::new(|b| {
            rounded_rect(
                b,
                -half,
                top,
                half * 2.0,
                bottom - top,
                h * 0.09,
            );
        });
        frame.fill(&shell, self.jacket);

        // Quilting. Seams are drawn as shallow arcs so the panels look inflated.
        let panels = 4;
        for i in 1..panels {
            let y = top + (bottom - top) * i as f32 / panels as f32;
            let seam = Path::new(|b| {
                b.move_to(Point::new(-half * 0.92, y));
                b.quadratic_curve_to(Point::new(0.0, y + h * 0.012), Point::new(half * 0.92, y));
            });
            frame.stroke(
                &seam,
                Stroke::default()
                    .with_color(palette::alpha(Color::BLACK, 0.18))
                    .with_width(h * 0.014)
                    .with_line_cap(LineCap::Round),
            );
        }

        // Highlight along the left edge of each panel row, so the puffer catches
        // the same light as the snow.
        let highlight = Path::new(|b| {
            b.move_to(Point::new(-half * 0.86, top + h * 0.06));
            b.quadratic_curve_to(
                Point::new(-half * 1.0, (top + bottom) / 2.0),
                Point::new(-half * 0.84, bottom - h * 0.04),
            );
        });
        frame.stroke(
            &highlight,
            Stroke::default()
                .with_color(palette::alpha(palette::SNOW, 0.22))
                .with_width(h * 0.02)
                .with_line_cap(LineCap::Round),
        );

        // Collar: a fleece roll at the neck.
        let collar = Path::new(|b| {
            rounded_rect(b, -h * 0.20, top - h * 0.05, h * 0.40, h * 0.09, h * 0.045);
        });
        frame.fill(&collar, palette::mix(self.jacket, palette::SNOW, 0.35));

        // Hem band.
        let hem = Path::new(|b| {
            rounded_rect(b, -half, bottom - h * 0.05, half * 2.0, h * 0.06, h * 0.03);
        });
        frame.fill(&hem, palette::mix(self.jacket, Color::BLACK, 0.35));
    }

    fn sleeves(&self, frame: &mut Frame, h: f32) {
        let swing = (self.phase * 1.3).sin() * 0.22;

        for side in [-1.0_f32, 1.0] {
            let shoulder = Point::new(side * h * 0.24, -h * 0.63);
            let angle = swing * side;
            let cuff = Point::new(
                shoulder.x + side * h * 0.07 + angle * h * 0.10,
                shoulder.y + h * 0.34,
            );

            let sleeve = Path::new(|b| {
                b.move_to(Point::new(shoulder.x - side * h * 0.05, shoulder.y));
                b.quadratic_curve_to(
                    Point::new(shoulder.x + side * h * 0.09, shoulder.y + h * 0.16),
                    cuff,
                );
                b.quadratic_curve_to(
                    Point::new(cuff.x - side * h * 0.10, cuff.y - h * 0.02),
                    Point::new(shoulder.x - side * h * 0.10, shoulder.y + h * 0.04),
                );
                b.close();
            });
            frame.fill(&sleeve, palette::mix(self.jacket, Color::BLACK, 0.12));

            // Cuff, then the flipper tip poking out of it.
            frame.fill(
                &Path::circle(cuff, h * 0.035),
                palette::mix(self.jacket, palette::SNOW, 0.35),
            );
            frame.fill(
                &Path::circle(
                    Point::new(cuff.x + side * h * 0.01, cuff.y + h * 0.045),
                    h * 0.028,
                ),
                palette::PENGUIN_BODY,
            );
        }
    }

    fn head(&self, frame: &mut Frame, h: f32) {
        let center = Point::new(self.facing * h * 0.015, -h * 0.85);
        let r = h * 0.155;

        frame.fill(&Path::circle(center, r), palette::PENGUIN_BODY);

        // Face patch, offset toward the direction of gaze.
        let face = Path::new(|b| {
            b.move_to(Point::new(center.x + self.facing * r * 0.15, center.y - r * 0.55));
            b.quadratic_curve_to(
                Point::new(center.x + self.facing * r * 0.95, center.y - r * 0.1),
                Point::new(center.x + self.facing * r * 0.55, center.y + r * 0.72),
            );
            b.quadratic_curve_to(
                Point::new(center.x - self.facing * r * 0.35, center.y + r * 0.9),
                Point::new(center.x - self.facing * r * 0.5, center.y + r * 0.2),
            );
            b.quadratic_curve_to(
                Point::new(center.x - self.facing * r * 0.35, center.y - r * 0.5),
                Point::new(center.x + self.facing * r * 0.15, center.y - r * 0.55),
            );
            b.close();
        });
        frame.fill(&face, palette::PENGUIN_BELLY);

        // Beak.
        let beak_root = Point::new(center.x + self.facing * r * 0.55, center.y + r * 0.12);
        let beak = Path::new(|b| {
            b.move_to(Point::new(beak_root.x, beak_root.y - r * 0.18));
            b.line_to(Point::new(beak_root.x + self.facing * r * 0.72, beak_root.y + r * 0.05));
            b.line_to(Point::new(beak_root.x, beak_root.y + r * 0.26));
            b.close();
        });
        frame.fill(&beak, palette::BEAK);

        // Eye, with a blink derived from the animation phase: closed for a short
        // slice of each cycle rather than on a timer of its own.
        let eye = Point::new(center.x + self.facing * r * 0.3, center.y - r * 0.12);
        let blink = ((self.phase * 0.31).sin() > 0.985) as u8;
        if blink == 1 {
            frame.stroke(
                &Path::line(
                    Point::new(eye.x - r * 0.16, eye.y),
                    Point::new(eye.x + r * 0.16, eye.y),
                ),
                Stroke::default()
                    .with_color(palette::PENGUIN_BODY)
                    .with_width(h * 0.012),
            );
        } else {
            frame.fill(&Path::circle(eye, r * 0.17), palette::PENGUIN_BODY);
            frame.fill(
                &Path::circle(Point::new(eye.x + r * 0.06, eye.y - r * 0.06), r * 0.06),
                palette::SNOW,
            );
        }

        // Knitted hat: brim, crown and bobble.
        let brim = Path::new(|b| {
            rounded_rect(
                b,
                center.x - r * 0.95,
                center.y - r * 1.02,
                r * 1.9,
                r * 0.34,
                r * 0.17,
            );
        });
        frame.fill(&brim, palette::mix(self.jacket, palette::SNOW, 0.25));

        let crown = Path::new(|b| {
            b.move_to(Point::new(center.x - r * 0.85, center.y - r * 0.95));
            b.quadratic_curve_to(
                Point::new(center.x - self.facing * r * 0.2, center.y - r * 1.9),
                Point::new(center.x + r * 0.85, center.y - r * 0.95),
            );
            b.close();
        });
        frame.fill(&crown, self.jacket);

        frame.fill(
            &Path::circle(
                Point::new(center.x - self.facing * r * 0.28, center.y - r * 1.62),
                r * 0.22,
            ),
            palette::SNOW,
        );
    }

    fn chest_text(&self, frame: &mut Frame, h: f32) {
        if self.text.is_empty() {
            return;
        }
        centered(
            frame,
            self.text,
            Point::new(0.0, -h * 0.44),
            h * 0.115,
            palette::alpha(palette::SNOW, 0.92),
            label_font(),
        );
    }
}

/// The ice floe a penguin stands on. Drawn separately so the scene can bob the
/// floe and the passenger together.
pub struct Floe {
    /// Centre of the waterline of the floe.
    pub center: Point,
    pub width: f32,
}

impl Floe {
    pub fn draw(&self, frame: &mut Frame) {
        let w = self.width;
        let c = self.center;

        // Submerged mass, hinted at below the waterline.
        let under = Path::new(|b| {
            b.move_to(Point::new(c.x - w * 0.42, c.y));
            b.quadratic_curve_to(
                Point::new(c.x, c.y + w * 0.24),
                Point::new(c.x + w * 0.40, c.y),
            );
            b.close();
        });
        frame.fill(&under, palette::alpha(palette::ICE_SHADOW, 0.40));

        // Deck: an irregular slab, flat on top.
        let top = Path::new(|b| {
            b.move_to(Point::new(c.x - w * 0.5, c.y));
            b.line_to(Point::new(c.x - w * 0.40, c.y - w * 0.12));
            b.line_to(Point::new(c.x + w * 0.34, c.y - w * 0.10));
            b.line_to(Point::new(c.x + w * 0.5, c.y));
            b.close();
        });
        frame.fill(&top, palette::ICE);

        let snow = Path::new(|b| {
            b.move_to(Point::new(c.x - w * 0.40, c.y - w * 0.12));
            b.line_to(Point::new(c.x - w * 0.16, c.y - w * 0.17));
            b.line_to(Point::new(c.x + w * 0.20, c.y - w * 0.15));
            b.line_to(Point::new(c.x + w * 0.34, c.y - w * 0.10));
            b.close();
        });
        frame.fill(&snow, palette::SNOW);
    }
}
