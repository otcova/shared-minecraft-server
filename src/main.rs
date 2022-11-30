#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app;
mod autoupdate;
mod ddns;
mod error;
mod fetch;
mod git;
mod process;
mod public_ip;
mod verify_signature;
mod pull_channel;

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

    eframe::run_native("MC Hoster", options, Box::new(|cc| Box::new(App::new(cc))));
}
