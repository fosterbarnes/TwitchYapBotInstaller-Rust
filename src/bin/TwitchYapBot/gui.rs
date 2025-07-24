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
}

impl TwitchYapBotApp {
    pub fn new() -> Self {
        let output_lines = Arc::new(Mutex::new(VecDeque::with_capacity(200)));
        let (tx, rx) = mpsc::channel();
        let output_lines_clone = output_lines.clone();
        let (child_sender, child_receiver) = mpsc::channel();
        std::thread::spawn(move || {
            let (child_arc, pid) = run_markov_chain_bot(tx, output_lines_clone);
            let _ = child_sender.send((child_arc, pid));
        });
        let (child, child_pid) = child_receiver.recv().unwrap_or((None, None));
        let (github_tx, github_rx) = mpsc::channel();
        spawn_github_release_fetch(github_tx);
        let ipc_restart_flag = Arc::new(AtomicBool::new(false));
        start_ipc_server(ipc_restart_flag.clone());
        Self {
            output_lines,
            rx: Some(rx),
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
        // Poll for GitHub release info
        if let Some(rx) = &self.github_rx {
            if let Ok(release) = rx.try_recv() {
                self.github_release = release;
                self.github_rx = None;
                ctx.request_repaint();
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
        stop_bot(self);
        log_and_print!("[GUI] Main window closed (x button in windows)");
        crate::log_util::shutdown_logger();
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
