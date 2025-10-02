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
mod launch_flags;

mod obs_monitor;
use eframe::egui;
use egui::ViewportBuilder;
use std::fs;
use crate::gui::{get_version, load_app_icon, setup_fonts_and_theme};
use crate::config::{WINDOW_SIZE, MIN_WINDOW_SIZE, app_version};
use yap_bot_installer::center_window::calculate_window_position;
#[cfg(windows)]
use std::os::windows::process::CommandExt;


fn main() {
    // Set up signal handler for cleanup on unexpected termination
    if let Err(e) = ctrlc::set_handler(|| {
        println!("Received termination signal, cleaning up...");
        // Stop the Python bot
        crate::bot_manager::stop_bot_direct();
        // Clean up PowerShell processes
        crate::obs_monitor::cleanup_powershell_processes();
        std::process::exit(0);
    }) {
        eprintln!("Failed to set signal handler: {}", e);
    }
    
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

    // Parse launch flags
    let launch_flags = launch_flags::LaunchFlags::from_args();
    
    // Handle settings window launch
    if launch_flags.should_launch_settings_window() {
        crate::settings::run_settings_window();
        return;
    }
    
    // Apply version override flags
    launch_flags.apply();
    
    // Log active flags for debugging
    if launch_flags.has_version_overrides() || launch_flags.should_force_gui() {
        println!("[LAUNCH_FLAGS] {}", launch_flags.get_active_flags_description());
    }
    
    // Check if "start minimized to tray" is enabled and launch tray app immediately
    // Skip this check if --force-gui is specified (used when launching from tray app)
    if !launch_flags.should_force_gui() {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        let appdata_settings_path = std::path::PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\{}", appdata, crate::config::INSTALLER_SETTINGS_FILENAME));
        
        if appdata_settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&appdata_settings_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(start_minimized) = json.get("StartMinimizedToTray") {
                        if let Some(start_minimized_bool) = start_minimized.as_bool() {
                            if start_minimized_bool {
                                // Launch YapBotTray.exe and exit immediately
                                let tray_exe_path = std::path::Path::new(&appdata)
                                    .join("YapBot")
                                    .join("YapBotTray.exe");
                                
                                if tray_exe_path.exists() {
                                    println!("[STARTUP] 'Start minimized to tray' enabled - launching YapBotTray.exe");
                                    if let Ok(_) = std::process::Command::new(&tray_exe_path)
                                        .creation_flags(0x08000000) // CREATE_NO_WINDOW
                                        .spawn() {
                                        println!("[STARTUP] YapBotTray.exe launched successfully");
                                        std::process::exit(0);
                                    } else {
                                        eprintln!("[STARTUP] ERROR: Failed to launch YapBotTray.exe");
                                    }
                                } else {
                                    eprintln!("[STARTUP] ERROR: YapBotTray.exe not found at: {}", tray_exe_path.display());
                                }
                            }
                        }
                    }
                }
            }
        }
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