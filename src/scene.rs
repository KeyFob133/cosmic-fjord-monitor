//! Composition of the whole widget: one canvas, drawn back to front.
//!
//! The scene is split into a cached layer and a live layer. Sky, stars, cliffs and
//! their reflection never change, so they are drawn once into a `Cache` and reused
//! until the surface is resized. Aurora, water, wildlife and gauges are redrawn
//! every frame. On an idle machine that is a few hundred paths per frame, which is
//! why the frame rate is configurable and why nothing here allocates textures.
//!
//! Load is mapped onto the scene rather than merely reported by it:
//!
//! - Busy thread count decides how many halibut are out.
//! - Average CPU load decides how hard they beat their tails.
//! - Memory drives the swell on the water surface.
//! - CPU temperature drives the aurora, which brightens and reddens with it.
//!
//! So the widget is readable at a glance from motion alone, before you focus on
//! any of the numbers.

use cosmic::iced::widget::canvas::{self, Cache, Frame, LineCap, Path, Stroke};
use cosmic::iced::{mouse, Point, Rectangle, Size};

use crate::config::Config;
use crate::draw::{centered, label_font, rounded_rect, smoothstep, Rng};
use crate::gauge::{temp_text, Gauge};
use crate::halibut::Halibut;
use crate::palette;
use crate::penguin::{Floe, Penguin};
use crate::stats::Sample;

/// Upper bound on fish. The whole population is always simulated; load decides
/// how many of them are faded in, which keeps entrances and exits smooth.
const MAX_HALIBUT: usize = 6;

/// Busy threads per fish. Calibrated so a saturated 16-to-24-thread machine fills
/// the water without the fish overlapping into mush.
const THREADS_PER_HALIBUT: f32 = 3.5;

pub struct Scene<'a> {
    pub sample: Sample,
    /// Seconds since the widget started. Drives every animation.
    pub time: f32,
    pub config: &'a Config,
    pub terrain: &'a Cache,
}

impl<Message> canvas::Program<Message, cosmic::Theme> for Scene<'_> {
    type State = ();

    fn draw(
        &self,
        _state: &Self::State,
        renderer: &cosmic::Renderer,
        _theme: &cosmic::Theme,
        bounds: Rectangle,
        _cursor: mouse::Cursor,
    ) -> Vec<canvas::Geometry> {
        let size = bounds.size();
        let mut layers = Vec::with_capacity(2);

        if self.config.scene {
            layers.push(
                self.terrain
                    .draw(renderer, size, |frame| self.draw_terrain(frame, size)),
            );
        }

        let mut frame = Frame::new(renderer, size);
        if self.config.scene {
            self.draw_aurora(&mut frame, size);
            self.draw_water(&mut frame, size);
            self.draw_wildlife(&mut frame, size);
        } else {
            self.draw_panel(&mut frame, size);
        }
        self.draw_gauges(&mut frame, size);
        layers.push(frame.into_geometry());

        layers
    }
}

impl Scene<'_> {
    fn horizon(&self, size: Size) -> f32 {
        size.height * 0.52
    }

    /// Rounded slab and edge. In scene mode the sky doubles as the slab, so this is
    /// only called when the scene is off.
    fn draw_panel(&self, frame: &mut Frame, size: Size) {
        let slab = Path::new(|b| {
            rounded_rect(b, 0.0, 0.0, size.width, size.height, 20.0);
        });
        frame.fill(&slab, palette::PANEL);
        frame.stroke(
            &slab,
            Stroke::default()
                .with_color(palette::PANEL_EDGE)
                .with_width(1.0),
        );
    }

    // ---------------------------------------------------------------- cached

    fn draw_terrain(&self, frame: &mut Frame, size: Size) {
        let w = size.width;
        let h = size.height;
        let horizon = self.horizon(size);

        // Clip everything to the rounded slab by filling it first and drawing
        // inside it. The canvas has no clip operation, so the shapes below are
        // authored to stay inside the bounds instead.
        let slab = Path::new(|b| {
            rounded_rect(b, 0.0, 0.0, w, h, 20.0);
        });
        frame.fill(&slab, palette::alpha(palette::SKY_HIGH, 0.94));

        // Sky, as horizontal bands from zenith to horizon. Bands rather than a
        // gradient: it is one fill per band and it suits the flat, printed look of
        // the rest of the scene.
        let bands = 14;
        for i in 0..bands {
            let t = i as f32 / (bands - 1) as f32;
            let colour = if t < 0.6 {
                palette::mix(palette::SKY_HIGH, palette::SKY_MID, t / 0.6)
            } else {
                palette::mix(palette::SKY_MID, palette::SKY_LOW, (t - 0.6) / 0.4)
            };
            let y0 = horizon * (i as f32 / bands as f32);
            let y1 = horizon * ((i + 1) as f32 / bands as f32);
            let band = Path::new(|b| {
                b.move_to(Point::new(0.0, y0));
                b.line_to(Point::new(w, y0));
                b.line_to(Point::new(w, y1));
                b.line_to(Point::new(0.0, y1));
                b.close();
            });
            frame.fill(&band, palette::alpha(colour, 0.96));
        }

        self.draw_stars(frame, size, horizon);

        // Two ridgelines: a pale distant range, then the near cliffs that form the
        // walls of the fjord.
        self.draw_ridge(frame, size, horizon, 0.62, 0x5EED_1A17, palette::GRANITE_FAR, 0.85);
        self.draw_ridge(frame, size, horizon, 1.0, 0x91C2_7B03, palette::GRANITE_NEAR, 1.0);
    }

    fn draw_stars(&self, frame: &mut Frame, size: Size, horizon: f32) {
        let mut rng = Rng::new(0x51A7_C0DE);
        for _ in 0..70 {
            let x = rng.range(0.0, size.width);
            let y = rng.range(4.0, horizon * 0.86);
            // Fade stars out toward the horizon, where the sky is brightest.
            let depth = 1.0 - (y / (horizon * 0.86));
            let r = rng.range(0.4, 1.25);
            let a = rng.range(0.18, 0.72) * (0.35 + 0.65 * depth);
            frame.fill(
                &Path::circle(Point::new(x, y), r),
                palette::alpha(palette::SNOW, a),
            );
        }
    }

    /// One mountain ridge plus its reflection.
    ///
    /// `scale` sets peak height as a fraction of the sky, `seed` picks the profile,
    /// and `weight` mixes the fill toward full strength for nearer ranges.
    fn draw_ridge(
        &self,
        frame: &mut Frame,
        size: Size,
        horizon: f32,
        scale: f32,
        seed: u64,
        colour: cosmic::iced::Color,
        weight: f32,
    ) {
        let w = size.width;
        let steps = 26;
        let mut rng = Rng::new(seed);

        // Sample a height for each step, then smooth it so peaks have shoulders
        // instead of looking like noise.
        let raw: Vec<f32> = (0..=steps).map(|_| rng.range(0.15, 1.0)).collect();
        let heights: Vec<f32> = (0..=steps)
            .map(|i: usize| {
                let prev = raw[i.saturating_sub(1)];
                let next = raw[(i + 1).min(steps)];
                (prev + raw[i] * 2.0 + next) / 4.0
            })
            .collect();

        let peak = horizon * 0.55 * scale;
        let point_at = |i: usize| -> Point {
            let x = w * i as f32 / steps as f32;
            Point::new(x, horizon - heights[i] * peak)
        };

        let ridge = Path::new(|b| {
            b.move_to(Point::new(0.0, horizon));
            for i in 0..=steps {
                b.line_to(point_at(i));
            }
            b.line_to(Point::new(w, horizon));
            b.close();
        });
        frame.fill(&ridge, palette::mix(palette::SKY_LOW, colour, weight));

        // Snow on any peak above a threshold: a small cap following the two faces
        // down from the summit.
        for i in 1..steps {
            if heights[i] > heights[i - 1] && heights[i] > heights[i + 1] && heights[i] > 0.55 {
                let summit = point_at(i);
                let drop = peak * 0.14;
                let cap = Path::new(|b| {
                    b.move_to(summit);
                    b.line_to(Point::new(
                        summit.x + (point_at(i + 1).x - summit.x) * 0.42,
                        summit.y + drop,
                    ));
                    b.line_to(Point::new(summit.x + (point_at(i + 1).x - summit.x) * 0.1, summit.y + drop * 0.55));
                    b.line_to(Point::new(
                        summit.x - (summit.x - point_at(i - 1).x) * 0.38,
                        summit.y + drop * 0.9,
                    ));
                    b.close();
                });
                frame.fill(&cap, palette::alpha(palette::SNOW, 0.72 * weight));
            }
        }

        // Reflection: the same ridge mirrored in the water, heavily faded. Fjord
        // water is glassy near the walls, so this is worth the extra fill.
        let mirror = Path::new(|b| {
            b.move_to(Point::new(0.0, horizon));
            for i in 0..=steps {
                let p = point_at(i);
                b.line_to(Point::new(p.x, horizon + (horizon - p.y) * 0.55));
            }
            b.line_to(Point::new(w, horizon));
            b.close();
        });
        frame.fill(&mirror, palette::alpha(colour, 0.30 * weight));
    }

    // ----------------------------------------------------------------- live

    /// Aurora ribbons, driven by CPU temperature.
    ///
    /// Temperature is a better subject than GPU load on a hybrid laptop, where the
    /// discrete card sleeps through ordinary work and would leave the sky flat all
    /// day. Heat, by contrast, tracks whatever the machine is actually doing.
    ///
    /// A missing sensor holds the sky at the dim baseline rather than going dark,
    /// so a machine without `coretemp` does not look broken.
    fn draw_aurora(&self, frame: &mut Frame, size: Size) {
        let heat = match self.sample.cpu_temp_c {
            Some(celsius) => smoothstep(
                self.config.aurora_temp_low,
                self.config.aurora_temp_high,
                celsius,
            ),
            None => 0.05,
        };
        let intensity = 0.18 + heat * 0.82;

        let w = size.width;
        let horizon = self.horizon(size);

        for ribbon in 0..3 {
            let k = ribbon as f32;
            let base_y = horizon * (0.20 + 0.16 * k);
            let amp = horizon * (0.07 + 0.02 * k);
            let speed = 0.22 + 0.09 * k;
            let cool = palette::mix(palette::AURORA_A, palette::AURORA_B, k / 2.0);
            let colour = palette::mix(cool, palette::AURORA_HOT, heat * 0.6);

            // Three passes of decreasing width build a soft edge without a blur.
            for (pass, (width_mul, alpha_mul)) in
                [(3.4_f32, 0.10_f32), (1.8, 0.16), (0.7, 0.30)].into_iter().enumerate()
            {
                let path = Path::new(|b| {
                    let segments = 40;
                    for i in 0..=segments {
                        let t = i as f32 / segments as f32;
                        let x = t * w;
                        let phase = self.time * speed + k * 1.9;
                        let y = base_y
                            + (t * 5.0 + phase).sin() * amp
                            + (t * 11.0 - phase * 1.7).sin() * amp * 0.35
                            + pass as f32 * 0.6;
                        if i == 0 {
                            b.move_to(Point::new(x, y));
                        } else {
                            b.line_to(Point::new(x, y));
                        }
                    }
                });

                frame.stroke(
                    &path,
                    Stroke::default()
                        .with_color(palette::alpha(colour, alpha_mul * intensity))
                        .with_width(horizon * 0.10 * width_mul)
                        .with_line_cap(LineCap::Round),
                );
            }
        }
    }

    fn draw_water(&self, frame: &mut Frame, size: Size) {
        let w = size.width;
        let h = size.height;
        let horizon = self.horizon(size);
        let swell = 1.0 + self.sample.mem_load() * 2.2;

        // Depth bands. Each is a plain quad; the surface band gets the wave.
        let bands = 10;
        for i in 0..bands {
            let t = i as f32 / (bands - 1) as f32;
            let colour = if t < 0.45 {
                palette::mix(palette::WATER_SURFACE, palette::WATER_MID, t / 0.45)
            } else {
                palette::mix(palette::WATER_MID, palette::WATER_DEEP, (t - 0.45) / 0.55)
            };
            let y0 = horizon + (h - horizon) * (i as f32 / bands as f32);
            let y1 = horizon + (h - horizon) * ((i + 1) as f32 / bands as f32);

            let band = Path::new(|b| {
                if i == 0 {
                    // Wavy top edge, left to right.
                    let segments = 48;
                    for j in 0..=segments {
                        let t = j as f32 / segments as f32;
                        let x = t * w;
                        let y = y0 + self.wave(x, swell);
                        if j == 0 {
                            b.move_to(Point::new(x, y));
                        } else {
                            b.line_to(Point::new(x, y));
                        }
                    }
                } else {
                    b.move_to(Point::new(0.0, y0));
                    b.line_to(Point::new(w, y0));
                }
                b.line_to(Point::new(w, y1));
                b.line_to(Point::new(0.0, y1));
                b.close();
            });
            frame.fill(&band, palette::alpha(colour, if i == 0 { 0.92 } else { 0.96 }));
        }

        // Highlight riding the surface.
        let crest = Path::new(|b| {
            let segments = 48;
            for j in 0..=segments {
                let t = j as f32 / segments as f32;
                let x = t * w;
                let y = horizon + self.wave(x, swell);
                if j == 0 {
                    b.move_to(Point::new(x, y));
                } else {
                    b.line_to(Point::new(x, y));
                }
            }
        });
        frame.stroke(
            &crest,
            Stroke::default()
                .with_color(palette::alpha(palette::ICE, 0.30))
                .with_width(1.4),
        );

        // Seabed: a silt shelf the halibut work along.
        let floor_y = h * 0.94;
        let bed = Path::new(|b| {
            b.move_to(Point::new(0.0, h));
            b.line_to(Point::new(0.0, floor_y + 6.0));
            let segments = 20;
            for j in 0..=segments {
                let t = j as f32 / segments as f32;
                let x = t * w;
                let y = floor_y + (t * 7.0).sin() * 4.0;
                b.line_to(Point::new(x, y));
            }
            b.line_to(Point::new(w, h));
            b.close();
        });
        frame.fill(&bed, palette::alpha(palette::SILT, 0.85));
    }

    /// Surface displacement at `x`. Two components at different wavelengths so the
    /// water does not read as a single sine.
    fn wave(&self, x: f32, swell: f32) -> f32 {
        let k1 = 0.045;
        let k2 = 0.017;
        (x * k1 + self.time * 1.6).sin() * 1.6 * swell
            + (x * k2 - self.time * 0.9).sin() * 2.4 * swell
    }

    fn draw_wildlife(&self, frame: &mut Frame, size: Size) {
        let w = size.width;
        let h = size.height;
        let horizon = self.horizon(size);
        let cpu = self.sample.cpu_load;
        let swell = 1.0 + self.sample.mem_load() * 2.2;

        // Halibut, farthest first so nearer fish overlap correctly.
        //
        // Population follows the number of busy threads, not the average across
        // all of them. On twenty threads one saturated core is five per cent of
        // the average, which would show an empty fjord while a build pins a core;
        // counted this way that is one fish, and `make -j20` fills the water.
        //
        // Energy still comes from the average, which is the smoother of the two
        // signals and the better fit for a rate. The constant offset keeps one
        // fish in view at idle, because an empty scene reads as broken.
        let wanted = 0.6 + self.sample.busy_threads as f32 / THREADS_PER_HALIBUT;
        for i in 0..MAX_HALIBUT {
            let mut rng = Rng::new(0x1000 + i as u64 * 7919);
            let lane = rng.range(0.70, 0.90);
            let length = w * rng.range(0.115, 0.175);
            let rightward = i % 2 == 0;
            let offset = rng.range(0.0, 1.0);
            let drift = rng.range(0.85, 1.25);

            // Fade the slot in as load rises past its index.
            let opacity = smoothstep(i as f32 - 0.15, i as f32 + 0.85, wanted);
            if opacity <= 0.01 {
                continue;
            }

            let speed = (16.0 + cpu * 78.0) * drift;
            let span = w + length * 2.0;
            let travelled = offset * span + self.time * speed;
            let along = travelled % span;
            let x = if rightward {
                -length + along
            } else {
                w + length - along
            };

            // Slow vertical loiter, so fish do not run on rails.
            let y = h * lane + (self.time * 0.5 + i as f32 * 1.3).sin() * h * 0.018;

            Halibut {
                center: Point::new(x, y),
                length,
                heading: if rightward { 0.0 } else { std::f32::consts::PI },
                phase: self.time * (2.2 + cpu * 5.5) * drift + i as f32 * 1.7,
                vigour: 0.45 + cpu * 0.9,
                opacity,
                seed: 0xBEEF + i as u64 * 104_729,
            }
            .draw(frame);
        }

        // Penguins on floes along the waterline.
        let count = self.config.penguins.min(4) as usize;
        let jackets = [palette::POP_TEAL, palette::POP_ORANGE, palette::POP_MINT];

        for i in 0..count {
            let mut rng = Rng::new(0x2000 + i as u64 * 6151);
            // Spread the floes across the width with a little jitter, keeping them
            // clear of the rounded corners.
            let slot = (i as f32 + 0.5) / count as f32;
            let x = w * (0.10 + slot * 0.80) + rng.range(-w * 0.03, w * 0.03);
            let floe_w = h * rng.range(0.20, 0.26);
            let phase = self.time * 1.5 + i as f32 * 2.1;
            let bob = (self.time * 1.1 + i as f32 * 1.7).sin() * 2.0 * swell;
            let y = horizon + h * 0.012 + bob;

            Floe {
                center: Point::new(x, y),
                width: floe_w,
            }
            .draw(frame);

            Penguin {
                feet: Point::new(x + floe_w * 0.02, y - floe_w * 0.11),
                height: h * 0.165,
                jacket: jackets[i % jackets.len()],
                text: &self.config.jacket_text,
                phase,
                facing: if i % 2 == 0 { 1.0 } else { -1.0 },
            }
            .draw(frame);
        }
    }

    fn draw_gauges(&self, frame: &mut Frame, size: Size) {
        let w = size.width;
        let h = size.height;
        let radius = (w / 7.4).min(h * 0.135);
        let cy = if self.config.scene {
            h * 0.175
        } else {
            h * 0.5
        };

        let cpu_pct = format!("{:.0}%", self.sample.cpu_load * 100.0);
        let cpu_sub = temp_text(self.sample.cpu_temp_c);

        let mem_pct = format!("{:.0}%", self.sample.mem_load() * 100.0);
        let mem_sub = format!(
            "{:.1}/{:.1}G",
            self.sample.mem_used_gib, self.sample.mem_total_gib
        );

        let gpu_pct = match self.sample.gpu_load {
            Some(load) => format!("{:.0}%", load * 100.0),
            None => "n/a".to_string(),
        };
        let gpu_sub = temp_text(self.sample.gpu_temp_c);

        let gauges = [
            Gauge {
                center: Point::new(w * 0.20, cy),
                radius,
                value: Some(self.sample.cpu_load),
                accent: palette::POP_TEAL,
                label: "CPU",
                value_text: &cpu_pct,
                sub_text: &cpu_sub,
            },
            Gauge {
                center: Point::new(w * 0.50, cy),
                radius,
                value: Some(self.sample.mem_load()),
                accent: palette::POP_ORANGE,
                label: "RAM",
                value_text: &mem_pct,
                sub_text: &mem_sub,
            },
            Gauge {
                center: Point::new(w * 0.80, cy),
                radius,
                value: self.sample.gpu_load,
                accent: palette::POP_MINT,
                label: "GPU",
                value_text: &gpu_pct,
                sub_text: &gpu_sub,
            },
        ];

        for gauge in &gauges {
            gauge.draw(frame);
        }

        // Place name, bottom left, quiet.
        if self.config.scene {
            centered(
                frame,
                "sognefjord",
                Point::new(w * 0.5, h * 0.975),
                h * 0.026,
                palette::alpha(palette::TEXT_DIM, 0.55),
                label_font(),
            );
        }
    }
}
