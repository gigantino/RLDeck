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

pub fn main() -> iced::Result {
    iced::application(RlDeck::boot, RlDeck::update, RlDeck::view)
        .title("RLDeck")
        .default_font(iced::Font::with_name("Fira Sans"))
        .antialiasing(true)
        .theme(RlDeck::theme)
        .subscription(RlDeck::subscription)
        .window(iced::window::Settings { size: app::WINDOW, min_size: Some(app::MIN_WINDOW), ..iced::window::Settings::default() })
        .run()
}
