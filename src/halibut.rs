//! Halibut.
//!
//! Drawn from above, which is the view that makes a halibut a halibut: a wide flat
//! diamond, a fringe of fin running almost the whole way round, and both eyes
//! crowded onto one side of the head. The body is not a fixed outline. It is
//! sampled along a spine that carries a travelling wave, so the whole fish
//! genuinely flexes as it swims instead of sliding rigidly across the screen.
//!
//! Halibut are ambush predators that hold station on the bottom, so the scene
//! keeps them in the lower band of the water, near the silt.

use cosmic::iced::widget::canvas::{Frame, LineCap, Path, Stroke};
use cosmic::iced::{Point, Vector};

use crate::palette;

/// Samples along the spine. Twenty-eight is enough that the outline reads as a
/// curve at this size without the segment count showing.
const SAMPLES: usize = 28;

pub struct Halibut {
    /// Centre of the body, in scene coordinates.
    pub center: Point,
    /// Nose-to-tail-root length in pixels.
    pub length: f32,
    /// 0.0 swims right, `PI` swims left.
    pub heading: f32,
    /// Position in the tail-beat cycle, in radians.
    pub phase: f32,
    /// Beat amplitude as a fraction of length. Rises with CPU load.
    pub vigour: f32,
    /// Fade, so fish can enter and leave without popping.
    pub opacity: f32,
    /// Per-fish seed for the speckle pattern.
    pub seed: u64,
}

impl Halibut {
    pub fn draw(&self, frame: &mut Frame) {
        if self.opacity <= 0.01 {
            return;
        }

        let l = self.length;
        let a = self.opacity;

        // The cast shadow sits in scene space, unrotated and offset down, so it
        // stays on the seabed rather than swinging around with the fish.
        let shadow = Path::new(|b| {
            b.move_to(Point::new(self.center.x - l * 0.5, self.center.y + l * 0.30));
            b.quadratic_curve_to(
                Point::new(self.center.x, self.center.y + l * 0.42),
                Point::new(self.center.x + l * 0.5, self.center.y + l * 0.30),
            );
            b.quadratic_curve_to(
                Point::new(self.center.x, self.center.y + l * 0.20),
                Point::new(self.center.x - l * 0.5, self.center.y + l * 0.30),
            );
        });
        frame.fill(&shadow, palette::alpha(palette::WATER_DEEP, 0.35 * a));

        frame.with_save(|frame| {
            frame.translate(Vector::new(self.center.x, self.center.y));
            frame.rotate(self.heading);

            self.fringe(frame);
            self.tail(frame);
            self.body(frame);
            self.speckles(frame);
            self.face(frame);
        });
    }

    /// Half-width of the body at spine position `s`, where `s` runs from `-0.5` at
    /// the tail root to `+0.5` at the snout.
    fn half_width(&self, s: f32) -> f32 {
        // Shift the parameter so the widest point lands forward of centre, as it
        // does on a real flatfish, then take a rounded lens profile.
        let skewed = ((s - 0.06) / 0.56).clamp(-1.0, 1.0);
        let lens = (1.0 - skewed * skewed).max(0.0).powf(0.58);
        // Taper hard into the tail root so the caudal fin has something to attach to.
        let peduncle = 0.30 + 0.70 * ((s + 0.5) / 0.35).clamp(0.0, 1.0);
        self.length * 0.30 * lens * peduncle
    }

    /// Lateral displacement of the spine: a wave that grows toward the tail.
    fn spine(&self, s: f32) -> f32 {
        let from_head = 0.5 - s; // 0 at the snout, 1 at the tail root
        let amplitude = self.length * 0.075 * self.vigour * from_head.powf(1.9);
        amplitude * (self.phase - from_head * 4.2).sin()
    }

    /// Outline points along one side. `side` is `-1.0` for the upper edge.
    fn edge(&self, side: f32, swell: f32) -> Vec<Point> {
        (0..=SAMPLES)
            .map(|i| {
                let s = -0.5 + i as f32 / SAMPLES as f32;
                let w = self.half_width(s) + swell;
                Point::new(s * self.length, self.spine(s) + side * w)
            })
            .collect()
    }

    fn body(&self, frame: &mut Frame) {
        let upper = self.edge(-1.0, 0.0);
        let lower = self.edge(1.0, 0.0);

        let outline = Path::new(|b| {
            b.move_to(upper[0]);
            for p in &upper[1..] {
                b.line_to(*p);
            }
            for p in lower.iter().rev() {
                b.line_to(*p);
            }
            b.close();
        });

        frame.fill(&outline, palette::alpha(palette::HALIBUT_TOP, self.opacity));

        // A pale rim along the lower edge reads as the light-coloured flank
        // catching light from above and gives the flat body some volume.
        let rim = Path::new(|b| {
            b.move_to(lower[0]);
            for p in &lower[1..] {
                b.line_to(*p);
            }
        });
        frame.stroke(
            &rim,
            Stroke::default()
                .with_color(palette::alpha(palette::HALIBUT_BELLY, 0.30 * self.opacity))
                .with_width(self.length * 0.035)
                .with_line_cap(LineCap::Round),
        );

        // Lateral line.
        let lateral = Path::new(|b| {
            let mut first = true;
            for i in 0..=SAMPLES {
                let s = -0.45 + i as f32 / SAMPLES as f32 * 0.85;
                let p = Point::new(s * self.length, self.spine(s) - self.half_width(s) * 0.18);
                if first {
                    b.move_to(p);
                    first = false;
                } else {
                    b.line_to(p);
                }
            }
        });
        frame.stroke(
            &lateral,
            Stroke::default()
                .with_color(palette::alpha(palette::HALIBUT_SPECK, 0.45 * self.opacity))
                .with_width(1.0),
        );
    }

    /// The continuous dorsal and anal fin, drawn as a rippling skirt just outside
    /// the body outline. The ripple runs head-to-tail slightly out of step with
    /// the body wave, which is what makes the fin look like a separate membrane.
    fn fringe(&self, frame: &mut Frame) {
        for side in [-1.0_f32, 1.0] {
            let points: Vec<Point> = (0..=SAMPLES)
                .map(|i| {
                    let s = -0.5 + i as f32 / SAMPLES as f32;
                    let from_head = 0.5 - s;
                    let ripple = 1.0
                        + 0.42
                            * (self.phase * 2.1 - from_head * 9.0 + if side < 0.0 { 0.0 } else { 1.7 })
                                .sin();
                    // Fin depth tapers off at both ends of the body.
                    let taper = (1.0 - (s / 0.5).abs().powf(2.4)).max(0.0);
                    let swell = self.length * 0.055 * ripple * taper;
                    let w = self.half_width(s) + swell;
                    Point::new(s * self.length, self.spine(s) + side * w)
                })
                .collect();

            let inner: Vec<Point> = (0..=SAMPLES)
                .map(|i| {
                    let s = -0.5 + i as f32 / SAMPLES as f32;
                    Point::new(
                        s * self.length,
                        self.spine(s) + side * self.half_width(s) * 0.9,
                    )
                })
                .collect();

            let skirt = Path::new(|b| {
                b.move_to(inner[0]);
                for p in &points[1..] {
                    b.line_to(*p);
                }
                for p in inner.iter().rev() {
                    b.line_to(*p);
                }
                b.close();
            });

            frame.fill(&skirt, palette::alpha(palette::HALIBUT_FIN, 0.62 * self.opacity));
        }
    }

    fn tail(&self, frame: &mut Frame) {
        let root_s = -0.5;
        let root = Point::new(root_s * self.length, self.spine(root_s));
        // The fin trails the body wave by a quarter cycle.
        let sweep = (self.phase - 1.4).sin() * 0.35;
        let span = self.length * 0.26;
        let reach = self.length * 0.22;

        let fin = Path::new(|b| {
            b.move_to(root);
            b.quadratic_curve_to(
                Point::new(root.x - reach * 0.5, root.y - span * 0.4 + sweep * span),
                Point::new(root.x - reach, root.y - span * 0.75 + sweep * span),
            );
            // Shallow fork in the trailing edge.
            b.quadratic_curve_to(
                Point::new(root.x - reach * 0.72, root.y + sweep * span),
                Point::new(root.x - reach, root.y + span * 0.75 + sweep * span),
            );
            b.quadratic_curve_to(
                Point::new(root.x - reach * 0.5, root.y + span * 0.4 + sweep * span),
                root,
            );
            b.close();
        });

        frame.fill(&fin, palette::alpha(palette::HALIBUT_FIN, 0.85 * self.opacity));
    }

    fn speckles(&self, frame: &mut Frame) {
        let mut rng = crate::draw::Rng::new(self.seed);
        for _ in 0..22 {
            let s = rng.range(-0.44, 0.44);
            let across = rng.range(-0.82, 0.82);
            let p = Point::new(
                s * self.length,
                self.spine(s) + across * self.half_width(s),
            );
            let r = self.length * rng.range(0.008, 0.022);
            frame.fill(
                &Path::circle(p, r),
                palette::alpha(palette::HALIBUT_SPECK, rng.range(0.25, 0.55) * self.opacity),
            );
        }
    }

    /// Both eyes on one side of the head. This is the whole point of a flatfish:
    /// one eye migrates across the skull during development, so an adult lies on
    /// its blind flank with both eyes looking up.
    fn face(&self, frame: &mut Frame) {
        let a = self.opacity;
        let eye_side = -1.0_f32; // ocular side

        for (s, scale) in [(0.34_f32, 1.0_f32), (0.25, 0.88)] {
            let base = Point::new(
                s * self.length,
                self.spine(s) + eye_side * self.half_width(s) * 0.34,
            );
            let r = self.length * 0.045 * scale;

            frame.fill(
                &Path::circle(base, r * 1.5),
                palette::alpha(palette::HALIBUT_BELLY, 0.35 * a),
            );
            frame.fill(&Path::circle(base, r), palette::alpha(palette::HALIBUT_SPECK, a));
            frame.fill(
                &Path::circle(Point::new(base.x + r * 0.3, base.y - r * 0.3), r * 0.34),
                palette::alpha(palette::SNOW, 0.85 * a),
            );
        }

        // Mouth: a short hook at the snout, on the ocular side.
        let s = 0.47;
        let snout = Point::new(s * self.length, self.spine(s));
        let mouth = Path::new(|b| {
            b.move_to(Point::new(snout.x + self.length * 0.02, snout.y - self.length * 0.01));
            b.quadratic_curve_to(
                Point::new(snout.x - self.length * 0.03, snout.y + self.length * 0.03),
                Point::new(snout.x - self.length * 0.07, snout.y + self.length * 0.02),
            );
        });
        frame.stroke(
            &mouth,
            Stroke::default()
                .with_color(palette::alpha(palette::HALIBUT_SPECK, 0.8 * a))
                .with_width(self.length * 0.022)
                .with_line_cap(LineCap::Round),
        );
    }
}
