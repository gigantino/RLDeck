#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![forbid(unsafe_code)]

mod app;
mod atomic;
mod catalog;
mod config;
mod fetch;
mod files;
mod gallery;
mod game;
mod hash;
mod http;
mod icon;
mod install;
mod library;
mod matching;
mod model;
mod progress;
#[cfg(test)]
mod testing;
mod theme;
mod thumbs;
mod ui;

use app::RlDeck;

fn window_icon() -> Option<iced::window::Icon> {
    let logo = image::load_from_memory_with_format(include_bytes!("../assets/icon.png"), image::ImageFormat::Png).ok()?.into_rgba8();
    let (width, height) = logo.dimensions();

    iced::window::icon::from_rgba(logo.into_raw(), width, height).ok()
}

pub fn main() -> iced::Result {
    iced::application(RlDeck::boot, RlDeck::update, RlDeck::view)
        .title("RLDeck")
        .default_font(iced::Font::with_name("Fira Sans"))
        .antialiasing(true)
        .theme(RlDeck::theme)
        .subscription(RlDeck::subscription)
        .window(iced::window::Settings {
            size: app::WINDOW,
            min_size: Some(app::MIN_WINDOW),
            icon: window_icon(),
            ..iced::window::Settings::default()
        })
        .run()
}
