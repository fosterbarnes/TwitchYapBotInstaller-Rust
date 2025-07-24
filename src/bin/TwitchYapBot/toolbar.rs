//! Toolbar (top panel) logic for TwitchYapBot
//!
//! This module is responsible for rendering the title, version, update check, and main toolbar buttons for the TwitchYapBot GUI.

use eframe::egui;
use crate::gui::{TwitchYapBotApp, is_sound_enabled};
use crate::buttons;
use crate::bot_manager;
use std::io::Read;
use rand::Rng;
use std::sync::mpsc::{self, Sender, Receiver};
use std::panic;
use std::sync::{Mutex, OnceLock};
use crate::log_and_print;

static UPDATE_RESULT_TX: OnceLock<Mutex<Option<Sender<Result<(), String>>>>> = OnceLock::new();
static UPDATE_RESULT_RX: OnceLock<Mutex<Option<Receiver<Result<(), String>>>>> = OnceLock::new();

// Spinner drawing function (copied from installer)
fn draw_spinner(ui: &mut egui::Ui, color: egui::Color32) {
    let time = ui.ctx().input(|i| i.time) as f32;
    let rotation_speed = 4.0;
    let angle = (time * rotation_speed) % (2.0 * std::f32::consts::PI);
    let center = ui.cursor().min + egui::vec2(8.0, 8.0);
    let radius = 6.0;
    let painter = ui.painter();
    let start_angle = angle;
    let end_angle = angle + std::f32::consts::PI * 1.5;
    let segments = 20;
    let angle_step = (end_angle - start_angle) / segments as f32;
    for i in 0..segments {
        let angle1 = start_angle + i as f32 * angle_step;
        let angle2 = start_angle + (i + 1) as f32 * angle_step;
        let p1 = center + egui::vec2(radius * angle1.cos(), radius * angle1.sin());
        let p2 = center + egui::vec2(radius * angle2.cos(), radius * angle2.sin());
        painter.line_segment([p1, p2], egui::Stroke::new(2.0, color));
    }
}

pub fn render_toolbar(app: &mut TwitchYapBotApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    // Poll for update completion
    let update_result = {
        let rx_mutex = UPDATE_RESULT_RX.get_or_init(|| Mutex::new(None));
        let rx_opt = rx_mutex.lock().unwrap();
        if let Some(rx) = &*rx_opt {
            match rx.try_recv() {
                Ok(res) => Some(res),
                Err(mpsc::TryRecvError::Empty) => None,
                Err(_) => None,
            }
        } else {
            None
        }
    };
    if let Some(result) = update_result {
        app.updating = false;
        // Debug log: result received
        app.output_lines.lock().unwrap().push_back(format!("[DEBUG] Update result received: {:?}", result));
        match result {
            Ok(()) => {
                // Success: launch updater and exit
                bot_manager::stop_bot(app);
                if let Ok(appdata) = std::env::var("APPDATA") {
                    let exe_path = std::path::Path::new(&appdata)
                        .join("YapBot")
                        .join("YapBotUpdater.exe");
                    let _ = std::process::Command::new(exe_path)
                        .spawn();
                }
                std::process::exit(0);
            }
            Err(err) => {
                use chrono::Local;
                let now = Local::now();
                let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
                app.output_lines.lock().unwrap().push_back(format!("{} ERROR: {}", timestamp, err));
            }
        }
        // Clear the channel after use
        let rx_mutex = UPDATE_RESULT_RX.get_or_init(|| Mutex::new(None));
        rx_mutex.lock().unwrap().take();
        let tx_mutex = UPDATE_RESULT_TX.get_or_init(|| Mutex::new(None));
        tx_mutex.lock().unwrap().take();
    }
    egui::TopBottomPanel::top("title").show(ctx, |ui| {
        let mut update_section_shown = false;
        ui.horizontal(|ui| {
            // Title (left-aligned)
            ui.vertical(|ui| {
                ui.add_space(8.0);
                let title_url = "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust";
                let title_text = egui::RichText::new(format!("Twitch Yap Bot v{}", crate::get_version()))
                    .font(egui::FontId::new(17.0, egui::FontFamily::Name("consolas_titles".into())))
                    .color(egui::Color32::from_rgb(189, 147, 249));
                ui.hyperlink_to(title_text, title_url);
                // New update check logic
                if let Some(tag) = app.github_release.tag_name.as_ref() {
                    let current = format!("v{}", crate::get_version());
                    let current_trim = current.trim_start_matches('v');
                    let tag_trim = tag.trim_start_matches('v');
                    if is_outdated(current_trim, tag_trim) {
                        // If current version is greater than tag, show 'Newest public release:'
                        if current_trim > tag_trim {
                            update_section_shown = true;
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Newest public release:")
                                        .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas".into())))
                                        .color(egui::Color32::from_rgb(255, 184, 108)) // #ffb870
                                        .size(13.0)
                                );
                                if let Some(tag) = app.github_release.tag_name.as_ref() {
                                    let url = app.github_release.html_url.as_deref().unwrap_or("https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases");
                                    let link_text = format!("({})", tag);
                                    let link_rich = egui::RichText::new(link_text)
                                        .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas".into())))
                                        .color(egui::Color32::from_rgb(80, 160, 255))
                                        .size(13.0);
                                    ui.hyperlink_to(link_rich, url);
                                }
                            });
                            // Only add extra space below the buttons, not above
                            ui.add_space(0.0 + 20.0 + 5.0);
                        } else {
                            update_section_shown = true;
                            ui.horizontal(|ui| {
                                ui.label(
                                    egui::RichText::new("Yap Bot's out of date")
                                        .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas".into())))
                                        .color(egui::Color32::from_rgb(255, 85, 85)) // #ff5555
                                        .size(13.0)
                                );
                                if let Some(tag) = app.github_release.tag_name.as_ref() {
                                    let url = app.github_release.html_url.as_deref().unwrap_or("https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases");
                                    let link_text = format!("({})", tag);
                                    let link_rich = egui::RichText::new(link_text)
                                        .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas".into())))
                                        .color(egui::Color32::from_rgb(80, 160, 255))
                                        .size(13.0);
                                    ui.hyperlink_to(link_rich, url);
                                }
                            });
                            ui.add_space(5.0);
                            // Replace the Update Now button and spinner section with a horizontal layout
                            ui.horizontal(|ui| {
                                let button = ui.add_sized([
                                    190.0,
                                    20.0
                                ], egui::Button::new("Update Now"));
                                if app.updating {
                                    draw_spinner(ui, egui::Color32::from_rgb(189, 147, 249)); // #bd93f9
                                }
                                if button.clicked() && !app.updating {
                                    app.updating = true;
                                    // Create a channel for update completion
                                    let (tx, rx) = mpsc::channel();
                                    let tx_mutex = UPDATE_RESULT_TX.get_or_init(|| Mutex::new(None));
                                    *tx_mutex.lock().unwrap() = Some(tx);
                                    let rx_mutex = UPDATE_RESULT_RX.get_or_init(|| Mutex::new(None));
                                    *rx_mutex.lock().unwrap() = Some(rx);
                                    std::thread::spawn(move || {
                                        let thread_result = panic::catch_unwind(|| {
                                            let updater_url = "https://raw.githubusercontent.com/fosterbarnes/TwitchYapBotInstaller-Rust/main/resources/updater/YapBotUpdater.exe";
                                            let mut download_error: Option<String> = None;
                                            match reqwest::blocking::get(updater_url) {
                                                Ok(resp) => {
                                                    if resp.status().is_success() {
                                                        let bytes = resp.bytes().map(|b| b.to_vec()).unwrap_or_else(|e| {
                                                            download_error = Some(format!("Failed to read updater bytes: {}", e));
                                                            Vec::new()
                                                        });
                                                        if download_error.is_none() {
                                                            if let Ok(tmp) = std::env::temp_dir().join("YapBotUpdater.exe").into_os_string().into_string() {
                                                                match std::fs::write(&tmp, &bytes) {
                                                                    Ok(_) => {
                                                                        if let Ok(appdata) = std::env::var("APPDATA") {
                                                                            let dest = std::path::Path::new(&appdata).join("YapBot").join("YapBotUpdater.exe");
                                                                            if let Err(e) = std::fs::copy(&tmp, &dest) {
                                                                                download_error = Some(format!("Failed to copy updater to AppData: {}", e));
                                                                            }
                                                                        } else {
                                                                            download_error = Some("Could not get APPDATA path".to_string());
                                                                        }
                                                                    }
                                                                    Err(e) => {
                                                                        download_error = Some(format!("Failed to write temp updater: {}", e));
                                                                    }
                                                                }
                                                            } else {
                                                                download_error = Some("Could not get temp file path".to_string());
                                                            }
                                                        }
                                                    } else {
                                                        download_error = Some(format!("Failed to download updater: HTTP {}", resp.status()));
                                                    }
                                                }
                                                Err(e) => {
                                                    download_error = Some(format!("Failed to download updater: {}", e));
                                                }
                                            }
                                            let tx_mutex = UPDATE_RESULT_TX.get_or_init(|| Mutex::new(None));
                                            if let Some(err) = download_error {
                                                if let Some(tx) = &*tx_mutex.lock().unwrap() {
                                                    let _ = tx.send(Err(err));
                                                }
                                            } else {
                                                if let Some(tx) = &*tx_mutex.lock().unwrap() {
                                                    let _ = tx.send(Ok(()));
                                                }
                                            }
                                        });
                                        if thread_result.is_err() {
                                            let tx_mutex = UPDATE_RESULT_TX.get_or_init(|| Mutex::new(None));
                                            if let Some(tx) = &*tx_mutex.lock().unwrap() {
                                                let _ = tx.send(Err("Update thread panicked".to_string()));
                                            }
                                        }
                                    });
                                }
                            });
                            ui.add_space(8.0);
                        }
                    }
                }
                if !update_section_shown {
                    // Add vertical space to match the height of the update section when not shown
                    ui.add_space(5.0 + 20.0 + 5.0); // 5.0 (space) + 20.0 (button height) + 8.0 (space)
                }
                // (Do not show any up-to-date or newer-version message)
            });
            ui.add_space(16.0);
            ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                // Settings cog (rightmost button)
                let icon_size = 45.0;
                let icon_resp = ui.add_sized([icon_size, icon_size], buttons::settings_cog_button(ctx, icon_size)).on_hover_text("Settings");
                if icon_resp.clicked() {
                    let exe = std::env::current_exe().unwrap();
                    let mut cmd = std::process::Command::new(exe);
                    cmd.arg("--settings-window");
                    if let Ok(log_path) = std::env::var("YAPBOT_LOG_PATH") {
                        cmd.env("YAPBOT_LOG_PATH", log_path);
                    }
                    let _ = cmd.spawn();
                }
                ui.add_space(12.0);
                // Revive button
                let revive_resp = ui.add_sized([121.0, 45.0], buttons::revive_button(ctx)).on_hover_text("Restart Yap Bot");
                if revive_resp.clicked() {
                    log_and_print!("[GUI] Revive button pressed");
                    if is_sound_enabled() {
                        buttons::play_random_sound(&buttons::ANGELIC_SOUNDS);
                    }
                    bot_manager::restart_bot(app, "Reviving Yap Bot from the depths of hell...");
                }
                ui.add_space(8.0);
                // Murder button
                let murder_resp = ui.add_sized([121.0, 45.0], buttons::murder_button(ctx)).on_hover_text("Stop Yap Bot");
                if murder_resp.clicked() {
                    log_and_print!("[GUI] Murder button pressed");
                    if is_sound_enabled() {
                        buttons::play_random_sound(&buttons::DEATH_SCREAMS);
                    }
                    bot_manager::stop_bot(app);
                }
                ui.add_space(8.0);
                // Yap button
                let yap_resp = ui.add_sized([121.0, 45.0], buttons::yap_button(ctx)).on_hover_text("Manually trigger a response");
                if yap_resp.clicked() {
                    log_and_print!("[GUI] Yap button pressed");
                    let output_lines = app.output_lines.clone();
                    std::thread::spawn(move || {
                        let mut connected = false;
                        let mut last_err = None;
                        for _attempt in 0..40 {
                            match std::net::TcpStream::connect("127.0.0.1:8765") {
                                Ok(mut stream) => {
                                    use std::io::Write;
                                    let _ = stream.write_all(b"YAP");
                                    let _ = stream.flush();
                                    // Wait for a short response or delay to ensure server processes the request
                                    let mut buf = [0u8; 8];
                                    let _ = stream.set_read_timeout(Some(std::time::Duration::from_millis(100)));
                                    let _ = stream.read(&mut buf); // ignore result, just wait
                                    connected = true;
                                    break;
                                }
                                Err(e) => {
                                    last_err = Some(e);
                                    std::thread::sleep(std::time::Duration::from_millis(50));
                                }
                            }
                        }
                        use chrono::Local;
                        let now = Local::now();
                        let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
                        if !connected {
                            let err_msg = match last_err {
                                Some(e) => format!("{} ERROR: Could not connect to Python bot on 127.0.0.1:8765 after 2 seconds: {}", timestamp, e),
                                None => format!("{} ERROR: Could not connect to Python bot on 127.0.0.1:8765 after 2 seconds (unknown error)", timestamp),
                            };
                            output_lines.lock().unwrap().push_back(err_msg);
                        }
                        let trigger_messages = [
                            "(manual trigger) YAP YAP YAP YAP YAP",
                            "(manual trigger) that felt kinda good :)",
                            "(manual trigger) stop pressing my button you dirty freak",
                            "(manual trigger) wtf that hurt",
                            "(manual trigger) please stop poking me",
                            "(manual trigger) you think you can just come to MY house and press MY button? smh",
                            "(manual trigger) it's nice to feel the touch of a human",
                            "(manual trigger) who up pressing they buttons",
                            "(manual trigger) AHHHHHHHHHHH",
                            "(manual trigger) I was asleep and you woke me up :("
                        ];
                        let mut rng = rand::thread_rng();
                        let msg = trigger_messages[rng.gen_range(0..trigger_messages.len())];
                        output_lines.lock().unwrap().push_back(format!("{} {}", timestamp, msg));
                    });
                }
            });
        });
        if !update_section_shown {
            ui.add_space(6.0); // Add vertical space below the toolbar only if version is up to date
        }
    });
}

// Helper function for version comparison (same as in installer)
fn is_outdated(current: &str, latest: &str) -> bool {
    // If the tags are not equal, show the update link
    current != latest
} 