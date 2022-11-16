#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
#![allow(dead_code, unused_variables)]

mod app;
mod fetch;
mod autoupdate;
mod error;
mod git;
mod process;
mod public_ip;

use app::*;
use eframe::egui;
use egui::*;

fn main() {
    autoupdate::update();
    public_ip::fetch();

    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(vec2(300., 0.));
    options.resizable = false;
    options.follow_system_theme = true;

    eframe::run_native(
        "Shared Server",
        options,
        Box::new(|cc| Box::new(App::new(cc))),
    );
}
