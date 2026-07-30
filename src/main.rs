//! A floating fjord for the COSMIC desktop: CPU, memory and GPU reported by
//! halibut, penguins and the northern lights.

mod app;
mod config;
mod draw;
mod gauge;
mod halibut;
mod palette;
mod penguin;
mod scene;
mod stats;

fn main() -> cosmic::iced::Result {
    app::run()
}
