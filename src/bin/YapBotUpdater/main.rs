pub mod updater;
mod gui;

use eframe::egui;
use egui::ViewportBuilder;
use crate::gui::{load_app_icon, setup_fonts_and_theme, calculate_window_position};

const WINDOW_SIZE: [f32; 2] = [400.0, 82.0];
const MIN_WINDOW_SIZE: [f32; 2] = [400.0, 82.0];

#[tokio::main]
async fn main() {
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
        "Yap Bot Updater",
        native_options,
        Box::new(|cc| {
            setup_fonts_and_theme(&cc.egui_ctx);
            Ok(Box::new(gui::YapUpdaterApp::default()))
        }),
    ).unwrap();
}
