// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod scene;
mod app;
mod local_storage;

use scene::*;
use app::*;
use eframe::egui;
use egui::*;

fn main() {
    let mut options = eframe::NativeOptions::default();
    options.initial_window_size = Some(vec2(300., 200.));
    options.resizable = false;
    options.follow_system_theme = true;

    eframe::run_native(
        "Shared Server",
        options,
        Box::new(|cc| Box::new(App::new(cc))),
    );
}