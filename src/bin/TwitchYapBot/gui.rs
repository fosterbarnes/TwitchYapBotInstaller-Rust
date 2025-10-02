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
    pub is_window_in_tray: bool, // track tray minimization state
    pub is_in_tray_shared: Arc<AtomicBool>, // shared tray state for background thread
    pub repaint_control_shared: Arc<AtomicBool>, // shared repaint control state for background thread
    pub previous_window_state: Option<bool>, // track previous window state for minimization detection

    pub window_state_rx: Option<std::sync::mpsc::Receiver<(bool, bool)>>, // receive window state updates from background thread
    // Animation state for output log arrow
    pub output_log_arrow_anim: f32, // 0.0 = right, 1.0 = down
    pub output_log_arrow_target: bool, // true = down, false = right
    pub output_log_arrow_animating: bool, // Animation state for output log fade
    pub output_log_fade_anim: f32, // 0.0 = fully hidden, 1.0 = fully shown
    pub output_log_fade_target: bool, // true = shown, false = hidden
    pub output_log_fade_animating: bool, // Animation state for output log fade

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
        
        // Start the Python bot
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
                Some(obs_monitor::start_obs_monitoring())
            } else {
                log_and_print!("[OBS_MONITOR] OBS monitoring disabled in settings");
                None
            }
        };
        
        // Start optimized background window state monitoring with tray awareness
        let (window_tx, window_rx) = std::sync::mpsc::channel::<(bool, bool)>(); // (minimized_state, was_tray_restore)
        let is_in_tray_shared = Arc::new(AtomicBool::new(false));
        let repaint_control_shared = Arc::new(AtomicBool::new(false));
        let is_in_tray_thread = is_in_tray_shared.clone();
        let repaint_control_thread = repaint_control_shared.clone();
        
        std::thread::spawn(move || {
            let mut previous_state: Option<bool> = None;
            let mut check_count = 0u32;
            
            loop {
                // Adaptive polling: faster when active, slower when stable
                let sleep_duration = if previous_state.is_none() || check_count < 10 {
                    std::time::Duration::from_millis(250) // Fast initial detection
                } else {
                    std::time::Duration::from_millis(1000) // Slower stable monitoring
                };
                
                std::thread::sleep(sleep_duration);
                
                // Optimized window state check
                let current_minimized = check_window_minimized_optimized();
                let is_in_tray = is_in_tray_thread.load(std::sync::atomic::Ordering::Relaxed);
                

                
                // Only process and notify on actual state changes
                if let Some(prev_state) = previous_state {
                    if prev_state != current_minimized {
                        if current_minimized {
                            // Window became minimized/hidden
                            if is_in_tray {
                                // We already know it's in tray, don't log again
                            } else {
                                log_and_print!("[WINDOW_STATE] Window minimized to taskbar");
                            }
                            // Immediately trigger repaint control change for minimized state
                            repaint_control_thread.store(true, std::sync::atomic::Ordering::Relaxed);
                            log_and_print!("[REPAINT] Switching to 5 second repaint intervals");
                            // Send state change to main thread
                            let _ = window_tx.send((current_minimized, false));
                        } else {
                            // Window became visible/restored
                            if is_in_tray {
                                // Reset the tray flag
                                is_in_tray_thread.store(false, std::sync::atomic::Ordering::Relaxed);
                                // Immediately trigger repaint control change for visible state
                                repaint_control_thread.store(false, std::sync::atomic::Ordering::Relaxed);
                                log_and_print!("[REPAINT] Switching to 250ms repaint intervals");
                                // Send state change to main thread with tray restoration info
                                let _ = window_tx.send((current_minimized, true));
                            } else {
                                log_and_print!("[WINDOW_STATE] Window unminimized from taskbar");
                                // Immediately trigger repaint control change for visible state
                                repaint_control_thread.store(false, std::sync::atomic::Ordering::Relaxed);
                                log_and_print!("[REPAINT] Switching to 250ms repaint intervals");
                                // Send normal state change to main thread
                                let _ = window_tx.send((current_minimized, false));
                            }
                        }
                        
                        check_count = 0; // Reset for faster detection after state change
                    } else {
                        check_count = check_count.saturating_add(1);
                    }
                } else {
                    check_count = check_count.saturating_add(1);
                }
                
                previous_state = Some(current_minimized);
            }
        });
        
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
            is_window_in_tray: false,
            is_in_tray_shared: is_in_tray_shared,
            repaint_control_shared: repaint_control_shared,
            previous_window_state: None,
            window_state_rx: Some(window_rx),
            output_log_arrow_anim: 1.0, // start as down (expanded)
            output_log_arrow_target: true,
            output_log_arrow_animating: false,
            output_log_fade_anim: 1.0, // start as fully shown
            output_log_fade_target: true,
            output_log_fade_animating: false,

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
        
        // Efficiently check for window state updates
        if let Some(rx) = &self.window_state_rx {
            // Process all pending state changes at once
            while let Ok((new_state, was_tray_restore)) = rx.try_recv() {
                // Check if we have a previous state to compare against
                if let Some(prev_state) = self.previous_window_state {
                    if prev_state != new_state {
                        // State changed - sync local tray state with shared state
                        if !self.is_in_tray_shared.load(std::sync::atomic::Ordering::Relaxed) {
                            self.is_window_in_tray = false;
                        }
                        
                        // If this was a tray restoration, mark startup minimization as handled
                        if was_tray_restore {
                            self.startup_minimization_handled = true;
                        }
                        // Also mark startup as handled for any manual window state change
                        // This prevents auto-minimization after any manual user interaction
                        else if !self.startup_minimization_handled {
                            self.startup_minimization_handled = true;
                        }
                        
                        // Note: All window state logging is now handled by the background thread
                        
                        self.previous_window_state = Some(new_state);
                        ctx.request_repaint();
                    }
                } else {
                    // First state update - just store it
                    self.previous_window_state = Some(new_state);
                    ctx.request_repaint();
                }
            }
        }
        

        
        // Handle first frame
        if self.first_frame {
            self.first_frame = false;
            
            // Check for first launch and show test message
            // Check if --first-launch flag is set or if settings indicate first launch
            let force_first_launch = std::env::var("YAPBOT_FORCE_FIRST_LAUNCH").is_ok();
            let should_show_first_launch = force_first_launch || self.settings_dialog.settings.first_launch;
            
            if !self.first_launch_handled && should_show_first_launch {
                if force_first_launch {
                    log_and_print!("[FIRST_LAUNCH] First launch forced by --first-launch flag, showing test message");
                } else {
                    log_and_print!("[FIRST_LAUNCH] First launch detected, showing test message");
                }
                // Show first launch popup
                self.show_first_launch_popup = true;
                
                // Mark first launch as handled
                self.first_launch_handled = true;
                
                // Only update settings if not forced by flag
                if !force_first_launch {
                    // Set first_launch to false in settings for next time
                    self.settings_dialog.settings.first_launch = false;
                    self.settings_dialog.temp_settings.first_launch = false;
                    
                    // Update only the first_launch field in the installer settings file
                    self.settings_dialog.update_first_launch_only(false);
                }
            }
            

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
            let popup_size = [500.0, 200.0]; // Increased size to accommodate larger text
            let popup_pos = [
                window_size.center().x - popup_size[0] / 2.0,
                window_size.center().y - popup_size[1] / 2.0 - 80.0,
            ];
            
            egui::Window::new(&format!("Updated to v{}", crate::config::app_version()))
                .collapsible(false)
                .resizable(false)
                .fixed_pos(popup_pos)
                .fixed_size(popup_size)
                .show(ctx, |ui| {
                    ui.vertical(|ui| {
                        // Read the first launch message from embedded file
                        let message_text = include_str!("firstLaunchMessage.txt");
                        
                        // Parse and display the message with clickable URLs
                        self.render_message_with_links(ui, message_text);
                        
                        ui.add_space(10.0);
                        ui.with_layout(egui::Layout::top_down(egui::Align::Center), |ui| {
                            // Adjust button size here - [width, height]
                            let button_size = [100.0, 30.0]; // Change these values to adjust size
                            if ui.add_sized(button_size, egui::Button::new("OK")).clicked() {
                                self.show_first_launch_popup = false;
                            }
                        });
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

        // Window state-aware repaint scheduling: use background thread control
        let is_minimized = self.repaint_control_shared.load(std::sync::atomic::Ordering::Relaxed);
        
        if self.installing_python || self.installing_dependencies || self.step4_action_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS for spinner/animation
        } else if self.updating {
            ctx.request_repaint_after(std::time::Duration::from_millis(16)); // 60 FPS for update progress
        } else if is_minimized {
            ctx.request_repaint_after(std::time::Duration::from_secs(5)); // 0.2 FPS when minimized (5 second intervals)
        } else {
            ctx.request_repaint_after(std::time::Duration::from_millis(250)); // 4 FPS when visible
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
        

        

        
        log_and_print!("[GUI] Main window closed (x button in windows)");
        crate::log_util::shutdown_logger();
    }
}

impl TwitchYapBotApp {







}

// Helper function to add a log line to the ring buffer
pub fn push_log_line(buffer: Arc<Mutex<VecDeque<String>>>, line: String) {
    let mut buf = buffer.lock().unwrap();
    if buf.len() == buf.capacity() {
        buf.pop_front();
    }
    buf.push_back(line);
}

/// Highly optimized window state check with minimal allocations
/// Returns true if window is minimized OR hidden (for tray detection)
fn check_window_minimized_optimized() -> bool {
    use std::sync::OnceLock;
    
    // Cache the title CString to avoid repeated allocations
    static CACHED_TITLE: OnceLock<std::ffi::CString> = OnceLock::new();
    
    unsafe {
        use windows::Win32::UI::WindowsAndMessaging::{FindWindowA, IsIconic, IsWindowVisible};
        
        let title_cstring = CACHED_TITLE.get_or_init(|| {
            let expected_title = format!("Twitch Yap Bot v{}", crate::config::app_version());
            std::ffi::CString::new(expected_title).unwrap_or_else(|_| std::ffi::CString::new("").unwrap())
        });
        
        // Fast window lookup with cached title
        let hwnd = FindWindowA(None, windows::core::PCSTR(title_cstring.as_ptr() as *const u8));
        
        if hwnd.0 != 0 {
            let is_iconic = IsIconic(hwnd).as_bool();
            let is_visible = IsWindowVisible(hwnd).as_bool();
            

            
            // Window is considered "minimized" if it's either iconic (taskbar) or not visible (tray)
            is_iconic || !is_visible
        } else {
            // Window not found - consider it minimized
            true // If we can't find the window, consider it "minimized"
        }
    }
}

impl TwitchYapBotApp {
    /// Render message text with clickable URLs
    fn render_message_with_links(&self, ui: &mut egui::Ui, message_text: &str) {
        let lines: Vec<&str> = message_text.lines().collect();
        
        for line in lines {
            if line.trim().is_empty() {
                ui.add_space(8.0);
                continue;
            }
            
            // Check if line contains markdown-style links [URL](text)
            if line.contains('[') && line.contains(']') && line.contains('(') && line.contains(')') {
                self.render_line_with_markdown_links(ui, line);
            } else {
                // Regular text line
                let rich_text = egui::RichText::new(line)
                    .size(14.0);
                ui.label(rich_text);
            }
        }
    }
    
    /// Render a line that may contain markdown-style links [URL](text)
    fn render_line_with_markdown_links(&self, ui: &mut egui::Ui, line: &str) {
        // Use horizontal layout to keep everything on the same line
        ui.horizontal(|ui| {
            let mut remaining = line;
            
            while !remaining.is_empty() {
                // Look for markdown link pattern [URL](text)
                if let Some(bracket_start) = remaining.find('[') {
                    // Display text before the link
                    if bracket_start > 0 {
                        let before_link = &remaining[..bracket_start];
                        let before_text = egui::RichText::new(before_link)
                            .size(14.0);
                        ui.label(before_text);
                    }
                    
                    // Find the end of the URL part
                    if let Some(bracket_end) = remaining[bracket_start..].find(']') {
                        let url_start = bracket_start + 1; // Skip the '['
                        let url_end = bracket_start + bracket_end;
                        let url = &remaining[url_start..url_end];
                        
                        // Find the display text in parentheses
                        let after_bracket = &remaining[url_end + 1..]; // Skip the ']'
                        if let Some(paren_start) = after_bracket.find('(') {
                            if let Some(paren_end) = after_bracket[paren_start..].find(')') {
                                let text_start = paren_start + 1; // Skip the '('
                                let text_end = paren_start + paren_end;
                                let display_text = &after_bracket[text_start..text_end];
                                
                                // Create clickable link with custom display text
                                let link_text = egui::RichText::new(display_text)
                                    .size(14.0)
                                    .color(egui::Color32::from_rgb(81, 169, 236)) // #51a9ec - same as donation link
                                    .underline();
                                
                                ui.hyperlink_to(link_text, url);
                                
                                // Update remaining text to continue parsing
                                let total_consumed = url_end + 1 + paren_start + paren_end + 1; // +1 for each ')'
                                remaining = &remaining[total_consumed..];
                                continue;
                            }
                        }
                    }
                }
                
                // If we get here, no more links found - display remaining text
                if !remaining.is_empty() {
                    let rich_text = egui::RichText::new(remaining)
                        .size(14.0);
                    ui.label(rich_text);
                }
                break;
            }
        });
    }
}


