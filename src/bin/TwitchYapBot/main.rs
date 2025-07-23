//! Twitch Yap Bot Runner
//! GUI wrapper for running MarkovChainBot.py with live output

mod gui;
mod update;
mod bot_manager;
mod ipc;
mod toolbar;
mod output;
mod settings;
mod buttons;

use eframe::egui;
use egui::ViewportBuilder;
use std::env;
use crate::gui::{get_version, calculate_window_position, load_app_icon, setup_fonts_and_theme};

// Window size and min size constants
const WINDOW_SIZE: [f32; 2] = [800.0, 517.0];
const MIN_WINDOW_SIZE: [f32; 2] = [730.0, 200.0];

fn main() {
    let args: Vec<String> = env::args().collect();
    if args.iter().any(|a| a == "--settings-window") {
        // Only run the settings window
        let mut dialog = settings::SettingsDialog::new();
        let _ = dialog.load_settings();
        let center_pos = calculate_window_position([666.0, 600.0]);
        let viewport_builder = ViewportBuilder::default()
            .with_inner_size([666.0, 600.0])
            .with_min_inner_size([400.0, 200.0])
            .with_position(center_pos);
        let native_options = eframe::NativeOptions {
            viewport: viewport_builder,
            ..Default::default()
        };
        eframe::run_native(
            "Yap Bot Settings",
            native_options,
            Box::new(|cc| {
                setup_fonts_and_theme(&cc.egui_ctx);
                struct SettingsWindowApp {
                    dialog: settings::SettingsDialog,
                }
                impl eframe::App for SettingsWindowApp {
                    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
                        self.dialog.show(ctx);
                    }
                }
                Ok(Box::new(SettingsWindowApp { dialog }))
            }),
        ).unwrap();
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
    let version = get_version();
    eframe::run_native(
        &format!("Twitch Yap Bot v{}", version),
        native_options,
        Box::new(move |cc| {
            setup_fonts_and_theme(&cc.egui_ctx);
            Ok(Box::new(gui::TwitchYapBotApp::default()))
        }),
    ).unwrap();
}