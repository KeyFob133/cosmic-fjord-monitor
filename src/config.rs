//! Persisted settings, stored through `cosmic-config` so the widget participates
//! in the same configuration system as the rest of the desktop. Files land in
//! `~/.config/cosmic/<APP_ID>/v1/`, one file per field, and can be edited by hand.

use cosmic::cosmic_config::{self, cosmic_config_derive::CosmicConfigEntry, CosmicConfigEntry};
use serde::{Deserialize, Serialize};

use crate::app::APP_ID;

/// Which corner the surface sticks to.
#[derive(Clone, Copy, Debug, Default, Deserialize, Serialize, PartialEq, Eq)]
pub enum Corner {
    #[default]
    TopRight,
    TopLeft,
    BottomRight,
    BottomLeft,
}

#[derive(Clone, Debug, Deserialize, Serialize, PartialEq, CosmicConfigEntry)]
#[version = 1]
pub struct Config {
    /// Surface size in logical pixels.
    pub width: u32,
    pub height: u32,
    /// Gap between the surface and the screen edges it is anchored to.
    pub margin: i32,
    pub corner: Corner,
    /// Seconds between statistics samples. Values below 0.2 are clamped, because
    /// CPU load read faster than that is noise.
    pub sample_interval: f32,
    /// Seconds between `nvidia-smi` calls. Used only when no sysfs counter
    /// exists. Each call spawns a process and can keep a hybrid laptop's
    /// discrete GPU awake, so this is deliberately slower than `sample_interval`.
    pub gpu_poll_interval: f32,
    /// Package temperature in Celsius at which the aurora sits at its dim
    /// baseline, and the temperature at which it reaches full brightness.
    /// Tune these to your machine: a laptop that idles at 55 and peaks at
    /// 95 wants different bounds than a desktop idling at 30.
    pub aurora_temp_low: f32,
    pub aurora_temp_high: f32,
    /// Animation frames per second. Drop this to 15 on a laptop on battery.
    pub fps: u16,
    /// Draw the fjord scene. With this off only the gauges are rendered, which is
    /// the low-distraction mode.
    pub scene: bool,
    /// How many penguins stand on the floes. Capped at 4 by the layout.
    pub penguins: u8,
    /// Text printed on the jackets.
    pub jacket_text: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            width: 480,
            height: 400,
            margin: 16,
            corner: Corner::TopRight,
            sample_interval: 1.0,
            gpu_poll_interval: 2.0,
            aurora_temp_low: 50.0,
            aurora_temp_high: 90.0,
            fps: 30,
            scene: true,
            penguins: 3,
            jacket_text: "pop os".to_string(),
        }
    }
}

impl Config {
    /// Load the stored configuration, falling back to defaults field by field.
    ///
    /// A missing or malformed file is not worth refusing to start over, so errors
    /// are logged and the default value for that field is used.
    pub fn load() -> (Option<cosmic_config::Config>, Self) {
        let Ok(handle) = cosmic_config::Config::new(APP_ID, Self::VERSION) else {
            return (None, Self::default());
        };

        match Self::get_entry(&handle) {
            Ok(config) => (Some(handle), config),
            Err((errors, config)) => {
                for error in errors {
                    eprintln!("fjord-monitor: config: {error}");
                }
                (Some(handle), config)
            }
        }
    }

    pub fn frame_interval(&self) -> std::time::Duration {
        let fps = self.fps.clamp(5, 120) as f32;
        std::time::Duration::from_secs_f32(1.0 / fps)
    }

    pub fn sample_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f32(self.sample_interval.max(0.2))
    }

    pub fn gpu_poll_interval(&self) -> std::time::Duration {
        std::time::Duration::from_secs_f32(self.gpu_poll_interval.max(0.5))
    }
}
