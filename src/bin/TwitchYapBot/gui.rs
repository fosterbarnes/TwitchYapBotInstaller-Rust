//! Main GUI logic for TwitchYapBot
//!
//! This module contains the main GUI logic, state management, and event handling for the TwitchYapBot executable.

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{self, Receiver};
use crate::settings::SettingsDialog;
use crate::log_and_print;
use eframe::{App, egui};
use std::sync::atomic::AtomicBool;
use std::collections::VecDeque;

use crate::update::{GithubRelease, spawn_github_release_fetch};
use crate::bot_manager::{stop_bot, restart_bot, run_markov_chain_bot};
use crate::ipc::start_ipc_server;
use crate::toolbar::render_toolbar;
use crate::output::render_output_log;
use crate::traymond::{launch_traymond, is_traymond_ready, minimize_twitch_yap_bot_to_tray, exit_traymond};
use crate::obs_monitor;
pub use yap_bot_installer::center_window::calculate_window_position;

/// Returns true if sound is enabled in the settings file.
pub fn is_sound_enabled() -> bool {
    use std::fs;
    use std::path::PathBuf;
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let appdata_settings_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\YapBotInstallerSettings.json", appdata));
    if let Ok(content) = fs::read_to_string(&appdata_settings_path) {
        if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
            return json.get("SoundEnabled").and_then(|v| v.as_bool()).unwrap_or(true);
        }
    }
    true
}

/// Returns the current app version from version.txt.
pub fn get_version() -> &'static str {
    include_str!("../../version.txt").trim()
}

/// Loads the app icon for the window.
pub fn load_app_icon() -> Option<egui::IconData> {
    if let Ok(image) = image::load_from_memory(include_bytes!("../../../resources/icon/yap_icon_purple.ico")) {
        let rgba = image.to_rgba8();
        let size = [rgba.width() as u32, rgba.height() as u32];
        Some(egui::IconData {
            rgba: rgba.into_raw(),
            width: size[0],
            height: size[1],
        })
    } else {
        None
    }
}

/// Sets up fonts and the Dracula theme for the egui context.
pub fn setup_fonts_and_theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "consolas".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../resources/font/Consolas_Regular.ttf")),
    );
    fonts.font_data.insert(
        "murder_font".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../resources/font/MurderFont.ttf")),
    );
    fonts.families.insert(
        egui::FontFamily::Name("consolas".into()),
        vec!["consolas".to_owned()]
    );
    fonts.families.insert(
        egui::FontFamily::Name("consolas_titles".into()),
        vec!["consolas".to_owned()]
    );
    fonts.families.insert(
        egui::FontFamily::Name("murder_font".into()),
        vec!["murder_font".to_owned()]
    );
    ctx.set_fonts(fonts);
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));         // #f8f8f2
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(189, 147, 249);            // #9591f9
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(139, 233, 253);           // #87e9fd
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(68, 71, 90);             // #44475a
    visuals.selection.bg_fill = egui::Color32::from_rgb(189, 147, 249);                 // #9591f9
    visuals.hyperlink_color = egui::Color32::from_rgb(139, 233, 253);                   // #87e9fd
    visuals.warn_fg_color = egui::Color32::from_rgb(255, 184, 108);                     // #ffb870
    visuals.error_fg_color = egui::Color32::from_rgb(255, 85, 85);                      // #ff5555
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(68, 71, 90);       // #44475a
    visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(248, 248, 242);    // #f8f8f2
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(40, 42, 54);      // #282a36
    ctx.set_visuals(visuals);
}

pub struct TwitchYapBotApp {
    pub output_lines: Arc<Mutex<VecDeque<String>>>,
    pub rx: Option<Receiver<String>>,
    pub marker_index: Option<usize>,
    pub auto_scroll: bool,
    pub last_num_displayed: usize,
    pub child: Option<Arc<Mutex<std::process::Child>>>,
    pub child_pid: Option<u32>,
    pub settings_dialog: SettingsDialog,
    pub github_release: GithubRelease,
    pub github_rx: Option<Receiver<GithubRelease>>,
    pub ipc_restart_flag: Arc<AtomicBool>,
    pub installing_python: bool,
    pub installing_dependencies: bool,
    pub step4_action_running: bool,
    pub updating: bool,
    pub show_output_log: bool, // controls custom collapsible output section
    pub previous_window_height: Option<f32>, // for restoring window height
    pub is_window_minimized: bool, // track minimized state
    // Animation state for output log arrow
    pub output_log_arrow_anim: f32, // 0.0 = right, 1.0 = down
    pub output_log_arrow_target: bool, // true = down, false = right
    pub output_log_arrow_animating: bool, // Animation state for output log fade
    pub output_log_fade_anim: f32, // 0.0 = fully hidden, 1.0 = fully shown
    pub output_log_fade_target: bool, // true = shown, false = hidden
    pub output_log_fade_animating: bool, // Animation state for output log fade
    // Traymond IPC integration
    pub traymond_child: Option<std::process::Child>,
    pub traymond_launched: bool,
    pub window_ready_for_minimize: bool,
    pub traymond_initialized: bool,
    pub traymond_init_rx: Option<Receiver<(bool, Option<std::process::Child>)>>,
    pub traymond_waiting_start_time: Option<std::time::Instant>,
    pub traymond_last_check_time: Option<std::time::Instant>,
    pub first_frame: bool,
    pub startup_minimization_handled: bool,
    // First launch tracking
    pub first_launch_handled: bool,
    // First launch popup
    pub show_first_launch_popup: bool,
    // OBS monitoring
    pub obs_monitor_rx: Option<mpsc::Receiver<()>>,
}

impl TwitchYapBotApp {
    pub fn new() -> Self {
        // Log the start of the Twitch Yap Bot run
        log_and_print!("--- Twitch Yap Bot Run ---");
        
        let output_lines = Arc::new(Mutex::new(VecDeque::with_capacity(200)));
        let (bot_tx, bot_rx) = mpsc::channel();
        let output_lines_clone = output_lines.clone();
        
        // Initialize traymond in background thread (always launch regardless of settings)
        let traymond_child = None;
        let traymond_launched = false;
        #[allow(unused_assignments)]
        let mut traymond_init_rx = None;
        let window_ready_for_minimize = false;
        
        // Always launch traymond for the minimize to tray button functionality
        log_and_print!("[TRAYMOND] Launching traymond-tcp for minimize to tray functionality");
        
        // Start traymond initialization in background thread
        let (traymond_tx, traymond_rx) = mpsc::channel::<(bool, Option<std::process::Child>)>();
        traymond_init_rx = Some(traymond_rx);
        
        std::thread::spawn(move || {
            // Launch traymond immediately without checking if it's running
            log_and_print!("[TRAYMOND] Launching traymond-tcp immediately...");
            match launch_traymond() {
                Ok(child) => {
                    log_and_print!("[TRAYMOND] Successfully launched traymond-tcp immediately");
                    // Send the child process back to the main thread
                    let _ = traymond_tx.send((true, Some(child)));
                }
                Err(e) => {
                    log_and_print!("[TRAYMOND] ERROR: Failed to launch traymond-tcp: {}", e);
                    let _ = traymond_tx.send((false, None));
                }
            }
        });
        
        // Note: We always launch traymond for the minimize to tray button functionality,
        // regardless of the "start minimized to tray" setting. The setting only controls
        // whether the window is automatically minimized on startup.
        
        // Now start the Python bot AFTER traymond is initialized
        let (child_sender, child_receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let (child_arc, pid) = run_markov_chain_bot(bot_tx, output_lines_clone);
            let _ = child_sender.send((child_arc, pid));
        });
        let (child, child_pid) = child_receiver.recv().unwrap_or((None, None));
        let (github_tx, github_rx) = mpsc::channel();
        spawn_github_release_fetch(github_tx);
        let ipc_restart_flag = Arc::new(AtomicBool::new(false));
        start_ipc_server(ipc_restart_flag.clone());
        
        // Initialize OBS monitoring if enabled
        let obs_monitor_rx = {
            let settings = SettingsDialog::new();
            if settings.settings.exit_when_monitored_app_closes {
                log_and_print!("[OBS_MONITOR] OBS monitoring enabled in settings, starting monitor");
                
                // If "start minimized" is enabled, use direct exit to avoid tray issues
                if settings.settings.start_minimized_to_tray {
                    log_and_print!("[OBS_MONITOR] Using direct exit monitoring due to start minimized setting");
                    obs_monitor::start_obs_monitoring_with_direct_exit();
                    None // No receiver needed for direct exit
                } else {
                    Some(obs_monitor::start_obs_monitoring())
                }
            } else {
                log_and_print!("[OBS_MONITOR] OBS monitoring disabled in settings");
                None
            }
        };
        
        Self {
            output_lines,
            rx: Some(bot_rx),
            marker_index: None,
            auto_scroll: true,
            last_num_displayed: 0,
            child,
            child_pid,
            settings_dialog: SettingsDialog::new(),
            github_release: GithubRelease::default(),
            github_rx: Some(github_rx),
            ipc_restart_flag,
            installing_python: false,
            installing_dependencies: false,
            step4_action_running: false,
            updating: false,
            show_output_log: true,
            previous_window_height: None,
            is_window_minimized: false,
            output_log_arrow_anim: 1.0, // start as down (expanded)
            output_log_arrow_target: true,
            output_log_arrow_animating: false,
            output_log_fade_anim: 1.0, // start as fully shown
            output_log_fade_target: true,
            output_log_fade_animating: false,
            // Traymond IPC integration
            traymond_child,
            traymond_launched,
            window_ready_for_minimize,
            traymond_initialized: false,
            traymond_init_rx,
            traymond_waiting_start_time: None,
            traymond_last_check_time: None,
            first_frame: true,
            startup_minimization_handled: false,
            // First launch tracking
            first_launch_handled: false,
            // First launch popup
            show_first_launch_popup: false,
            // OBS monitoring
            obs_monitor_rx,
        }
    }
    

}

impl Default for TwitchYapBotApp {
    fn default() -> Self {
        Self::new()
    }
}

impl App for TwitchYapBotApp {
    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Handle first frame - minimize immediately if traymond is ready and setting is enabled
        if self.first_frame {
            self.first_frame = false;
            
            // Check for first launch and show test message
            if !self.first_launch_handled && self.settings_dialog.settings.first_launch {
                log_and_print!("[FIRST_LAUNCH] First launch detected, showing test message");
                // Show first launch popup
                self.show_first_launch_popup = true;
                
                // Mark first launch as handled
                self.first_launch_handled = true;
                
                // Set first_launch to false in settings for next time
                self.settings_dialog.settings.first_launch = false;
                self.settings_dialog.temp_settings.first_launch = false;
                
                // Update only the first_launch field in the installer settings file
                self.settings_dialog.update_first_launch_only(false);
            }
            
            // If "start minimized" is disabled on startup, mark startup as handled to prevent future minimization
            if !self.settings_dialog.settings.start_minimized_to_tray {
                self.startup_minimization_handled = true;
                log_and_print!("[TRAYMOND] First frame - 'Start minimized' disabled, marking startup as handled");
            }
            
            if self.traymond_launched && self.window_ready_for_minimize && !self.is_window_minimized && self.settings_dialog.settings.start_minimized_to_tray {
                log_and_print!("[TRAYMOND] First frame - minimizing window immediately (start minimized enabled)");
                if let Err(e) = minimize_twitch_yap_bot_to_tray() {
                    log_and_print!("[TRAYMOND] ERROR: Failed to minimize on first frame: {}", e);
                } else {
                    log_and_print!("[TRAYMOND] Successfully minimized on first frame");
                    self.is_window_minimized = true;
                    self.startup_minimization_handled = true;
                    self.cleanup_traymond_state();
                }
            }
        }
        
        // Handle traymond initialization result
        if let Some(rx) = &self.traymond_init_rx {
            if let Ok((success, child)) = rx.try_recv() {
                if success {
                    self.traymond_launched = true;
                    self.traymond_child = child;
                    self.window_ready_for_minimize = true;
                    log_and_print!("[TRAYMOND] traymond initialization completed successfully");
                    
                    // Only minimize immediately if "start minimized to tray" setting is enabled
                    if self.settings_dialog.settings.start_minimized_to_tray && !self.is_window_minimized {
                        log_and_print!("[TRAYMOND] 'Start minimized to tray' enabled - minimizing window immediately");
                        if let Err(e) = minimize_twitch_yap_bot_to_tray() {
                            log_and_print!("[TRAYMOND] ERROR: Failed to minimize after initialization: {}", e);
                        } else {
                            log_and_print!("[TRAYMOND] Successfully minimized after initialization");
                            self.is_window_minimized = true;
                            self.cleanup_traymond_state();
                        }
                    } else {
                        log_and_print!("[TRAYMOND] traymond ready but not minimizing (start minimized setting disabled)");
                    }
                } else {
                    log_and_print!("[TRAYMOND] traymond initialization failed");
                }
                self.traymond_init_rx = None;
            }
        }
        
        // Mark traymond as initialized
        if !self.traymond_initialized {
            self.traymond_initialized = true;
        }
        
        // Poll for GitHub release info
        if let Some(rx) = &self.github_rx {
            if let Ok(release) = rx.try_recv() {
                self.github_release = release;
                self.github_rx = None;
                ctx.request_repaint();
            }
        }
        
        // Check OBS monitoring
        if let Some(rx) = &self.obs_monitor_rx {
            if let Ok(()) = rx.try_recv() {
                log_and_print!("[OBS_MONITOR] Received shutdown signal, exiting gracefully");
                
                // Clean up PowerShell processes used for OBS monitoring
                log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes due to OBS shutdown");
                crate::obs_monitor::cleanup_powershell_processes();
                
                // Clean up traymond before exiting
                log_and_print!("[TRAYMOND] Closing traymond-tcp due to OBS shutdown");
                if let Err(e) = exit_traymond() {
                    log_and_print!("[TRAYMOND] ERROR: Failed to exit traymond-tcp: {}", e);
                }
                
                // Also try to terminate the child process if we launched it
                if let Some(mut child) = self.traymond_child.take() {
                    if let Err(e) = child.kill() {
                        log_and_print!("[TRAYMOND] ERROR: Failed to kill traymond-tcp process: {}", e);
                    } else {
                        log_and_print!("[TRAYMOND] Successfully terminated traymond-tcp process");
                    }
                }
                
                // Stop the bot
                stop_bot(self);
                
                // Only set first_launch to false when closing the app if not updating
                // When updating, we want to keep first_launch as true so the popup shows after update
                if !self.updating {
                    // First reload and save all settings (like clicking Save button)
                    self.settings_dialog.reload_and_save_settings();
                    // Then set first_launch to false
                    self.settings_dialog.settings.first_launch = false;
                    self.settings_dialog.temp_settings.first_launch = false;
                    self.settings_dialog.update_first_launch_only(false);
                }
                
                // Shutdown logger
                crate::log_util::shutdown_logger();
                
                // Now exit
                std::process::exit(0);
            }
        }
        // Set a short tooltip delay for all tooltips
        let mut style = (*ctx.style()).clone();
        style.interaction.tooltip_delay = 0.25; // 250ms
        ctx.set_style(style);
        // Dracula theme and font setup (copied from main installer)
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));         // #f8f8f2
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(189, 147, 249);            // #9591f9
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(139, 233, 253);           // #87e9fd
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(68, 71, 90);             // #44475a
        visuals.selection.bg_fill = egui::Color32::from_rgb(189, 147, 249);                 // #9591f9
        visuals.hyperlink_color = egui::Color32::from_rgb(139, 233, 253);                   // #87e9fd
        visuals.warn_fg_color = egui::Color32::from_rgb(255, 184, 108);                     // #ffb870
        visuals.error_fg_color = egui::Color32::from_rgb(255, 85, 85);                      // #ff5555
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(68, 71, 90);       // #44475a
        visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(248, 248, 242);    // #f8f8f2
        visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(40, 42, 54);      // #282a36
        ctx.set_visuals(visuals);
        // Set global icon width to 24.0 for all egui icons (arrows, dropdowns, etc.)
        render_toolbar(self, ctx, frame);
        render_output_log(self, ctx, frame);
        
        // Show first launch popup if needed
        if self.show_first_launch_popup {
            let window_size = ctx.available_rect();
            let popup_size = [420.0, 200.0]; // Increased size to accommodate larger text
            let popup_pos = [
                window_size.center().x - popup_size[0] / 2.0,
                window_size.center().y - popup_size[1] / 2.0 - 40.0,
            ];
            
            egui::Window::new(&format!("Updated to v{}", crate::config::app_version()))
                .collapsible(false)
                .resizable(false)
                .fixed_pos(popup_pos)
                .fixed_size(popup_size)
                .show(ctx, |ui| {
                    ui.vertical_centered(|ui| {
                        // Create a custom text style with larger font
                        let mut rich_text = egui::RichText::new("Two new app settings were added:
                        
                        - Start minimized to tray
                        - Automatically exit when OBS or Streamlabs close
                        
                        (Make sure to save your settings after changing them)");
                        
                        // Set the font size (you can adjust this value)
                        rich_text = rich_text.size(14.0); // Increase from default ~14.0
                        
                        ui.label(rich_text);
                        ui.add_space(10.0);
                        if ui.button("OK").clicked() {
                            self.show_first_launch_popup = false;
                        }
                    });
                });
        }
        // Poll for new output
        if let Some(rx) = &self.rx {
            let websocket_marker = "[TwitchWebsocket.TwitchWebsocket] [INFO    ] - Attempting to initialize websocket connection.";
            while let Ok(line) = rx.try_recv() {
                push_log_line(self.output_lines.clone(), line);
                if self.marker_index.is_none() {
                    let lines = self.output_lines.lock().unwrap();
                    self.marker_index = lines.iter().position(|line| line.contains(websocket_marker));
                }
                ctx.request_repaint();
            }
        }
        if self.settings_dialog.needs_restart {
            restart_bot(self, "Reviving Yap Bot from the depths of hell...");
            self.settings_dialog.needs_restart = false;
            let _ = self.settings_dialog.load_settings();
        }
        if self.ipc_restart_flag.load(std::sync::atomic::Ordering::SeqCst) {
            restart_bot(self, "Reviving Yap Bot from the depths of hell...");
            self.ipc_restart_flag.store(false, std::sync::atomic::Ordering::SeqCst);
            let _ = self.settings_dialog.load_settings();
        }
        
        // Handle traymond minimize logic - only run if not already minimized
        if !self.is_window_minimized && self.traymond_launched {
            // If traymond was just launched, check if it's ready (non-blocking with timeout)
            if self.traymond_child.is_some() && !self.window_ready_for_minimize {
                // Start waiting timer if not already started
                if self.traymond_waiting_start_time.is_none() {
                    self.traymond_waiting_start_time = Some(std::time::Instant::now());
                    log_and_print!("[TRAYMOND] Starting to wait for traymond-tcp to be ready");
                }
                
                // Check if traymond is ready (only every 100ms to reduce frequency)
                let should_check = self.traymond_last_check_time
                    .map(|last| last.elapsed() > std::time::Duration::from_millis(100))
                    .unwrap_or(true);
                
                if should_check {
                    self.traymond_last_check_time = Some(std::time::Instant::now());
                    
                    if is_traymond_ready() {
                        log_and_print!("[TRAYMOND] traymond-tcp is ready");
                        self.window_ready_for_minimize = true;
                        self.traymond_waiting_start_time = None;
                        
                        // Only minimize if "start minimized to tray" setting is enabled AND we haven't handled startup yet AND this is not a restart
                        if self.settings_dialog.settings.start_minimized_to_tray && !self.startup_minimization_handled && !self.settings_dialog.needs_restart {
                            log_and_print!("[TRAYMOND] 'Start minimized to tray' enabled - minimizing window on startup");
                            if let Err(e) = minimize_twitch_yap_bot_to_tray() {
                                log_and_print!("[TRAYMOND] ERROR: Failed to minimize after ready check: {}", e);
                            } else {
                                log_and_print!("[TRAYMOND] Successfully minimized after ready check");
                                self.is_window_minimized = true;
                                self.startup_minimization_handled = true;
                                self.cleanup_traymond_state();
                            }
                        } else {
                            log_and_print!("[TRAYMOND] traymond ready but not minimizing (start minimized setting disabled or already handled)");
                        }
                    } else {
                        // Check for timeout (10 seconds)
                        if let Some(start_time) = self.traymond_waiting_start_time {
                            if start_time.elapsed() > std::time::Duration::from_secs(10) {
                                log_and_print!("[TRAYMOND] ERROR: Timeout waiting for traymond-tcp to be ready");
                                self.traymond_waiting_start_time = None;
                            }
                        }
                    }
                }
            } else if self.window_ready_for_minimize && self.settings_dialog.settings.start_minimized_to_tray && !self.startup_minimization_handled && !self.settings_dialog.needs_restart {
                // traymond is ready and we want to minimize on startup - do it once
                log_and_print!("[TRAYMOND] Window is ready, minimizing TwitchYapBot window to tray on startup");
                match minimize_twitch_yap_bot_to_tray() {
                    Ok(_) => {
                        log_and_print!("[TRAYMOND] Successfully minimized TwitchYapBot window to tray");
                        self.is_window_minimized = true;
                        self.startup_minimization_handled = true;
                        self.cleanup_traymond_state();
                    }
                    Err(e) => {
                        log_and_print!("[TRAYMOND] ERROR: Failed to minimize TwitchYapBot window to tray: {}", e);
                    }
                }
            }
        }
        // Efficient repaint: only animate at high FPS when needed
        if self.installing_python || self.installing_dependencies || self.step4_action_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS for spinner/animation
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250)); // 4 FPS idle
        }
        if self.updating {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Only set first_launch to false when closing the app if not updating
        // When updating, we want to keep first_launch as true so the popup shows after update
        if !self.updating {
            // First reload and save all settings (like clicking Save button)
            self.settings_dialog.reload_and_save_settings();
            // Then set first_launch to false
            self.settings_dialog.settings.first_launch = false;
            self.settings_dialog.temp_settings.first_launch = false;
            self.settings_dialog.update_first_launch_only(false);
        }
        
        stop_bot(self);
        
        // Clean up PowerShell processes used for OBS monitoring
        log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes on application exit");
        crate::obs_monitor::cleanup_powershell_processes();
        
        // Always close traymond on exit (we always launch it now)
        log_and_print!("[TRAYMOND] Closing traymond-tcp on application exit");
        if let Err(e) = exit_traymond() {
            log_and_print!("[TRAYMOND] ERROR: Failed to exit traymond-tcp: {}", e);
        }
        
        // Also try to terminate the child process if we launched it
        if let Some(mut child) = self.traymond_child.take() {
            if let Err(e) = child.kill() {
                log_and_print!("[TRAYMOND] ERROR: Failed to kill traymond-tcp process: {}", e);
            } else {
                log_and_print!("[TRAYMOND] Successfully terminated traymond-tcp process");
            }
        }
        
        log_and_print!("[GUI] Main window closed (x button in windows)");
        crate::log_util::shutdown_logger();
    }
}

impl TwitchYapBotApp {
    /// Clean up traymond state after successful minimization
    fn cleanup_traymond_state(&mut self) {
        self.traymond_waiting_start_time = None;
        self.traymond_last_check_time = None;
        // Note: We keep traymond_launched, traymond_child, and window_ready_for_minimize
        // as they might be needed for other operations (like showing the window later)
    }
}

// Helper function to add a log line to the ring buffer
pub fn push_log_line(buffer: Arc<Mutex<VecDeque<String>>>, line: String) {
    let mut buf = buffer.lock().unwrap();
    if buf.len() == buf.capacity() {
        buf.pop_front();
    }
    buf.push_back(line);
}
