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
#[cfg(windows)]
use std::os::windows::process::CommandExt;

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
                
                // Show donation link when app is up to date or newer than public release
                // (This will be shown unless an update section is displayed)
                let mut show_donation_link = true;
                let mut should_show_donation_link = true;
                
                // New update check logic
                if let Some(tag) = app.github_release.tag_name.as_ref() {
                    let current = format!("v{}", crate::get_version());
                    let current_trim = current.trim_start_matches('v');
                    let tag_trim = tag.trim_start_matches('v');
                    
                    // Get version comparison result (respects override flags)
                    let (is_outdated, current_is_newer, donation_link_should_show) = get_version_comparison_result(current_trim, tag_trim);
                    should_show_donation_link = donation_link_should_show;
                    
                    if is_outdated {
                        // If current version is greater than tag, show 'Newest public release:'
                        if current_is_newer {
                            update_section_shown = true;
                            show_donation_link = false;
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
                            // Add negative space to counteract the height added by the "Newest public release" text
                            ui.add_space(-16.0);
                        } else {
                            update_section_shown = true;
                            show_donation_link = false;
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
                                    
                                    // Set first_launch to true when update is initiated
                                    app.settings_dialog.settings.first_launch = true;
                                    app.settings_dialog.temp_settings.first_launch = true;
                                    app.settings_dialog.update_first_launch_only(true);
                                    
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
                // Show donation link if no update section is displayed
                // Use negative spacing to counteract the height added by the link
                if show_donation_link && should_show_donation_link {
                    let donation_url = "https://buymeacoffee.com/FosterBarnes";
                    let donation_text = "buymeacoffee.com/FosterBarnes";
                    let donation_rich = egui::RichText::new(donation_text)
                        .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas".into())))
                        .color(egui::Color32::from_rgb(81, 169, 236)) // #51a9ec
                        .size(13.0);
                    
                    // Add the donation link
                    ui.hyperlink_to(donation_rich, donation_url);
                    
                    // Immediately add negative space to counteract the height
                    ui.add_space(-16.0); // Negative space to reduce toolbar height
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
                ui.add_space(8.0);
                // Minimize to tray button (left of settings button)
                let minimize_resp = ui.add_sized([icon_size, icon_size], buttons::minimize_to_tray_button(ctx, icon_size)).on_hover_text("Minimize to Tray");
                if minimize_resp.clicked() {
                    // Always launch tray app when user clicks the tray button, regardless of launch method
                    log_and_print!("[GUI] Minimize to tray button pressed - launching YapBotTray.exe");
                    
                    // Launch YapBotTray.exe from AppData\Roaming\YapBot
                    if let Ok(appdata) = std::env::var("APPDATA") {
                        let tray_exe_path = std::path::Path::new(&appdata)
                            .join("YapBot")
                            .join("YapBotTray.exe");
                        
                        log_and_print!("[GUI] APPDATA path: {}", appdata);
                        log_and_print!("[GUI] Full tray path: {}", tray_exe_path.display());
                        log_and_print!("[GUI] Tray exe exists: {}", tray_exe_path.exists());
                        
                        if tray_exe_path.exists() {
                            log_and_print!("[GUI] Launching YapBotTray.exe from: {}", tray_exe_path.display());
                            
                            // Launch the tray app
                            match std::process::Command::new(&tray_exe_path)
                                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                                .spawn() {
                                Ok(child) => {
                                    log_and_print!("[GUI] YapBotTray.exe launched successfully with PID: {}", child.id());
                                    
                                    // Handle first launch flag properly (same as on_exit)
                                    if !app.updating {
                                        // First reload and save all settings (like clicking Save button)
                                        app.settings_dialog.reload_and_save_settings();
                                        // Then set first_launch to false
                                        app.settings_dialog.settings.first_launch = false;
                                        app.settings_dialog.temp_settings.first_launch = false;
                                        app.settings_dialog.update_first_launch_only(false);
                                    }
                                    
                                    // Stop the bot
                                    bot_manager::stop_bot(app);
                                    
                                    // Clean up PowerShell processes
                                    crate::obs_monitor::cleanup_powershell_processes();
                                    
                                    log_and_print!("[GUI] Exiting main GUI app");
                                    std::process::exit(0);
                                }
                                Err(e) => {
                                    log_and_print!("[GUI] ERROR: Failed to launch YapBotTray.exe: {}", e);
                                }
                            }
                        } else {
                            log_and_print!("[GUI] ERROR: YapBotTray.exe not found at: {}", tray_exe_path.display());
                            // Try to list the directory contents to debug
                            if let Ok(entries) = std::fs::read_dir(std::path::Path::new(&appdata).join("YapBot")) {
                                log_and_print!("[GUI] Contents of YapBot directory:");
                                for entry in entries {
                                    if let Ok(entry) = entry {
                                        log_and_print!("[GUI]   - {}", entry.file_name().to_string_lossy());
                                    }
                                }
                            } else {
                                log_and_print!("[GUI] Could not read YapBot directory");
                            }
                        }
                    } else {
                        log_and_print!("[GUI] ERROR: Could not get APPDATA environment variable");
                    }
                }
                ui.add_space(8.0);
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

// Note: is_outdated function removed - now using get_version_comparison_result for flag support

// Helper function to check if version override flags are set
fn is_force_current_version() -> bool {
    std::env::var("YAPBOT_FORCE_CURRENT_VERSION").is_ok()
}

fn is_force_out_of_date_version() -> bool {
    std::env::var("YAPBOT_FORCE_OUT_OF_DATE_VERSION").is_ok()
}

fn is_force_unpublished_version() -> bool {
    std::env::var("YAPBOT_FORCE_UNPUBLISHED_VERSION").is_ok()
}

// Helper function to get modified version comparison result based on flags
fn get_version_comparison_result(current_trim: &str, tag_trim: &str) -> (bool, bool, bool) {
    // Check for override flags first
    if is_force_current_version() {
        // Force current version - act as if versions are equal (no update needed)
        return (false, false, true); // (is_outdated, current_is_newer, show_donation_link)
    }
    
    if is_force_out_of_date_version() {
        // Force out of date - act as if current is older than latest
        return (true, false, false); // (is_outdated, current_is_newer, show_donation_link)
    }
    
    if is_force_unpublished_version() {
        // Force unpublished - act as if current is newer than latest
        return (true, true, false); // (is_outdated, current_is_newer, show_donation_link)
    }
    
    // Normal comparison logic
    let is_outdated = current_trim != tag_trim;
    let current_is_newer = current_trim > tag_trim;
    let show_donation_link = !is_outdated;
    
    (is_outdated, current_is_newer, show_donation_link)
} 