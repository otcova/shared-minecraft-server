// #![cfg_attr(not(debug_assertions), windows_subsystem = "windows")] // hide console window on Windows in release

mod app;
mod local_storage;
mod scene;
mod server_files;
mod transitions;

use app::*;
use eframe::egui;
use egui::*;
use scene::*;

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
