//! The libcosmic application.
//!
//! The widget has no ordinary window. It runs with `no_main_window(true)` and
//! creates a single wlr-layer-shell surface on the `Bottom` layer, which puts it
//! above the wallpaper and below normal windows: it decorates the desktop and never
//! gets in the way of what you are actually doing. Pointer interactivity is off, so
//! clicks land on whatever is underneath.
//!
//! Change `Layer::Bottom` to `Layer::Overlay` below if you want it pinned above
//! everything, including fullscreen windows.

use std::time::Instant;

use cosmic::app::{Core, Settings, Task};
use cosmic::iced::platform_specific::shell::commands::layer_surface::{
    destroy_layer_surface, get_layer_surface, Anchor, KeyboardInteractivity, Layer,
};
use cosmic::iced::widget::canvas::{Cache, Canvas};
use cosmic::iced::{window, Length, Subscription};
use cosmic::iced::platform_specific::runtime::wayland::layer_surface::{
    IcedMargin, IcedOutput, SctkLayerSurfaceSettings,
};
use cosmic::{Application, Element};

use crate::config::{Config, Corner};
use crate::scene::Scene;
use crate::stats::{Monitor, Sample};

pub const APP_ID: &str = "com.system76.CosmicFjordMonitor";

#[derive(Clone, Debug)]
pub enum Message {
    /// Animation tick.
    Frame(Instant),
    /// Time to re-read the system counters.
    Sample(Instant),
    /// Settings changed on disk.
    ConfigChanged(Config),
}

pub struct FjordMonitor {
    core: Core,
    config: Config,
    config_handle: Option<cosmic::cosmic_config::Config>,
    monitor: Monitor,
    sample: Sample,
    surface: window::Id,
    started: Instant,
    elapsed: f32,
    /// Cached static scenery. Cleared when the surface geometry changes.
    terrain: Cache,
}

impl Application for FjordMonitor {
    type Executor = cosmic::executor::Default;
    type Flags = ();
    type Message = Message;

    const APP_ID: &'static str = APP_ID;

    fn core(&self) -> &Core {
        &self.core
    }

    fn core_mut(&mut self) -> &mut Core {
        &mut self.core
    }

    fn init(core: Core, _flags: Self::Flags) -> (Self, Task<Self::Message>) {
        let (config_handle, config) = Config::load();
        let mut monitor = Monitor::new(config.gpu_poll_interval());
        // Take one reading immediately. CPU load will read as zero on this first
        // call because there is no previous sample to difference against; the
        // gauges fill in properly one interval later.
        let sample = monitor.sample();

        let surface = window::Id::unique();
        let task = create_surface(surface, &config);

        let app = Self {
            core,
            config,
            config_handle,
            monitor,
            sample,
            surface,
            started: Instant::now(),
            elapsed: 0.0,
            terrain: Cache::new(),
        };

        (app, task)
    }

    fn update(&mut self, message: Self::Message) -> Task<Self::Message> {
        match message {
            Message::Frame(now) => {
                self.elapsed = now.duration_since(self.started).as_secs_f32();
            }
            Message::Sample(_) => {
                self.sample = self.monitor.sample();
            }
            Message::ConfigChanged(config) => {
                let geometry_changed = config.width != self.config.width
                    || config.height != self.config.height
                    || config.margin != self.config.margin
                    || config.corner != self.config.corner;

                self.config = config;
                self.terrain.clear();

                if geometry_changed {
                    // A layer surface cannot be resized in place, so replace it.
                    let old = self.surface;
                    self.surface = window::Id::unique();
                    return Task::batch([
                        destroy_layer_surface(old),
                        create_surface(self.surface, &self.config),
                    ]);
                }
            }
        }

        Task::none()
    }

    /// Never called: there is no main window.
    fn view(&self) -> Element<Self::Message> {
        cosmic::widget::text("").into()
    }

    fn view_window(&self, _id: window::Id) -> Element<Self::Message> {
        Canvas::new(Scene {
            sample: self.sample,
            time: self.elapsed,
            config: &self.config,
            terrain: &self.terrain,
        })
        .width(Length::Fill)
        .height(Length::Fill)
        .into()
    }

    fn subscription(&self) -> Subscription<Self::Message> {
        Subscription::batch([
            cosmic::iced::time::every(self.config.frame_interval()).map(Message::Frame),
            cosmic::iced::time::every(self.config.sample_interval()).map(Message::Sample),
            self.config_subscription(),
        ])
    }

}

impl FjordMonitor {
    /// Watch the config directory so edits apply without a restart.
    fn config_subscription(&self) -> Subscription<Message> {
        struct ConfigWatcher;

        cosmic::cosmic_config::config_subscription::<_, Config>(
            std::any::TypeId::of::<ConfigWatcher>(),
            APP_ID.into(),
            <Config as cosmic::cosmic_config::CosmicConfigEntry>::VERSION,
        )
        .map(|update| Message::ConfigChanged(update.config))
    }
}

fn create_surface(id: window::Id, config: &Config) -> Task<Message> {
    let (anchor, margin) = match config.corner {
        Corner::TopRight => (
            Anchor::TOP | Anchor::RIGHT,
            IcedMargin {
                top: config.margin,
                right: config.margin,
                ..Default::default()
            },
        ),
        Corner::TopLeft => (
            Anchor::TOP | Anchor::LEFT,
            IcedMargin {
                top: config.margin,
                left: config.margin,
                ..Default::default()
            },
        ),
        Corner::BottomRight => (
            Anchor::BOTTOM | Anchor::RIGHT,
            IcedMargin {
                bottom: config.margin,
                right: config.margin,
                ..Default::default()
            },
        ),
        Corner::BottomLeft => (
            Anchor::BOTTOM | Anchor::LEFT,
            IcedMargin {
                bottom: config.margin,
                left: config.margin,
                ..Default::default()
            },
        ),
    };

    get_layer_surface(SctkLayerSurfaceSettings {
        id,
        // Bottom keeps the widget on the desktop rather than over your work.
        layer: Layer::Bottom,
        keyboard_interactivity: KeyboardInteractivity::None,
        anchor,
        output: IcedOutput::Active,
        namespace: "fjord-monitor".into(),
        margin,
        size: Some((Some(config.width), Some(config.height))),
        // Zero: do not reserve space, so maximised windows cover the whole screen.
        exclusive_zone: 0,
        ..Default::default()
    })
}

pub fn run() -> cosmic::iced::Result {
    let settings = Settings::default()
        .no_main_window(true)
        .exit_on_close(false);

    cosmic::app::run::<FjordMonitor>(settings, ())
}
