//! Twitch Yap Bot Runner (main entry point)
//!
//! This is the main entry point for the TwitchYapBot executable, responsible for launching the GUI and managing the application lifecycle.
//! GUI wrapper for running MarkovChainBot.py with live output

mod gui;
mod update;
mod bot_manager;
mod ipc;
mod toolbar;
mod output;
mod settings;
mod buttons;
mod config;
mod log_util;
use eframe::egui;
use egui::ViewportBuilder;
use std::env;
use std::fs;
use crate::gui::{get_version, load_app_icon, setup_fonts_and_theme};
use crate::config::{WINDOW_SIZE, MIN_WINDOW_SIZE, app_version};
use yap_bot_installer::center_window::calculate_window_position;

fn main() {
    // Only generate a new log file path if YAPBOT_LOG_PATH is not already set
    if std::env::var("YAPBOT_LOG_PATH").is_err() {
        let log_dir = crate::config::get_log_dir();
        if !log_dir.exists() {
            let _ = fs::create_dir_all(&log_dir);
        }
        let now = chrono::Local::now();
        let log_filename = now.format("%m-%d-%y_%H-%M-%S.log").to_string();
        let log_path = log_dir.join(log_filename);
        std::env::set_var("YAPBOT_LOG_PATH", &log_path);
    }

    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--settings-window") {
        // Only run the settings window
        crate::settings::run_settings_window();
        return;
    }
    let center_pos = calculate_window_position(WINDOW_SIZE);
    let icon_data = load_app_icon();
    let mut viewport_builder = ViewportBuilder::default()
        .with_inner_size(WINDOW_SIZE)
        .with_min_inner_size(MIN_WINDOW_SIZE)
        .with_position(center_pos);
    if let Some(icon) = icon_data {
        viewport_builder = viewport_builder.with_icon(icon);
    }
    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };
    eframe::run_native(
        &format!("Twitch Yap Bot v{}", app_version()),
        native_options,
        Box::new(move |cc| {
            setup_fonts_and_theme(&cc.egui_ctx);
            Ok(Box::new(gui::TwitchYapBotApp::default()))
        }),
    ).unwrap();
}