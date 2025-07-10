//! Twitch Yap Bot Runner
//! GUI wrapper for running MarkovChainBot.py with live output

use eframe::{egui, App};
use egui::ViewportBuilder;
use std::process::{Command, Stdio, Child};
#[cfg(windows)]
use std::os::windows::process::CommandExt;
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
use std::io::{BufRead, BufReader};
use std::sync::{Arc, Mutex};
use std::thread;
use std::sync::mpsc::{self, Receiver};
use regex::Regex;
use std::collections::HashSet;

// Window size and min size constants (copy from main.rs)
const WINDOW_SIZE: [f32; 2] = [800.0, 580.0];
const MIN_WINDOW_SIZE: [f32; 2] = [600.0, 461.0];

// Centering logic (copy from main.rs)
#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};

fn calculate_window_position(window_size: [f32; 2]) -> egui::Pos2 {
    #[cfg(windows)]
    {
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            if GetCursorPos(&mut point).is_ok() {
                let monitor = MonitorFromPoint(point, MONITOR_DEFAULTTONEAREST);
                let mut info = MONITORINFO {
                    cbSize: std::mem::size_of::<MONITORINFO>() as u32,
                    ..Default::default()
                };
                if GetMonitorInfoW(monitor, &mut info).as_bool() {
                    let work_left = info.rcWork.left;
                    let work_top = info.rcWork.top;
                    let work_width = (info.rcWork.right - info.rcWork.left) as f32;
                    let work_height = (info.rcWork.bottom - info.rcWork.top) as f32;
                    let x = work_left as f32 + (work_width - window_size[0]) / 2.0;
                    let y = work_top as f32 + (work_height - window_size[1]) / 2.0;
                    egui::Pos2::new(x, y)
                } else {
                    egui::Pos2::new(100.0, 100.0)
                }
            } else {
                egui::Pos2::new(100.0, 100.0)
            }
        }
    }
    #[cfg(not(windows))]
    {
        egui::Pos2::new(100.0, 100.0)
    }
}

// Add this function to load the icon (copy from main.rs)
fn load_app_icon() -> Option<egui::IconData> {
    #[cfg(windows)]
    {
        if let Ok(image) = image::load_from_memory(include_bytes!("../../resources/icon/yap_icon_purple.ico")) {
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
    #[cfg(not(windows))]
    {
        if let Ok(image) = image::load_from_memory(include_bytes!("../../resources/icon/yap_icon_green.png")) {
            let rgba = image.to_rgba8();
            let size = [rgba.width() as u32, rgba.height() as u32];
            Some(egui::IconData {
                rgba: rgba.into_raw(),
                width: size[0],
                height: size[1],
            })
        } else if let Ok(image) = image::load_from_memory(include_bytes!("../../resources/icon/yap_icon_purple.ico")) {
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
}

const VERSION: &str = "5.0.0";

struct TwitchYapBotApp {
    output_lines: Arc<Mutex<Vec<String>>>,
    rx: Option<Receiver<String>>,
    marker_index: Option<usize>, // Track the index of the first marker line
    auto_scroll: bool, // Track if we should auto-scroll
    last_num_displayed: usize, // Track number of lines displayed last frame
    child: Option<Arc<Mutex<Child>>>, // Track the Python process
    child_pid: Option<u32>, // Track the Python process PID
    // (no settings fields)
}

impl Default for TwitchYapBotApp {
    fn default() -> Self {
        let output_lines = Arc::new(Mutex::new(Vec::new()));
        let (tx, rx) = mpsc::channel();
        let output_lines_clone = output_lines.clone();
        let (child_sender, child_receiver) = mpsc::channel();
        thread::spawn(move || {
            let (child_arc, pid) = run_markov_chain_bot(tx, output_lines_clone);
            let _ = child_sender.send((child_arc, pid));
        });
        let (child, child_pid) = child_receiver.recv().unwrap_or((None, None));
        Self {
            output_lines,
            rx: Some(rx),
            marker_index: None,
            auto_scroll: true, // Start with auto-scroll enabled
            last_num_displayed: 0,
            child,
            child_pid,
        }
    }
}

fn run_markov_chain_bot(
    tx: mpsc::Sender<String>,
    output_lines: Arc<Mutex<Vec<String>>>,
) -> (Option<Arc<Mutex<Child>>>, Option<u32>) {
    // Use %APPDATA%\YapBot\TwitchMarkovChain as the working directory for Python
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let workdir = std::path::PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain", appdata));
    // Try to find python in PATH
    let python = which::which("python").unwrap_or_else(|_| "python".into());
    let script = "MarkovChainBot.py";
    println!("\n--- Twitch Yap Bot Run ---\n");
    let mut cmd = Command::new(python);
    cmd.arg("-u")
        .arg(script)
        .current_dir(&workdir)
        .env("PYTHONUNBUFFERED", "1")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    let child = cmd.spawn().expect("Failed to start MarkovChainBot.py");
    let pid = Some(child.id());
    let child_arc = Arc::new(Mutex::new(child));
    let stdout = child_arc.lock().unwrap().stdout.take().unwrap();
    let stderr = child_arc.lock().unwrap().stderr.take().unwrap();
    // Read stdout
    let tx3 = tx.clone();
    let out_lines = output_lines.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("[MarkovChainBot.py] {}", line);
                let _ = tx3.send(line.clone());
                out_lines.lock().unwrap().push(line);
            }
        }
    });
    // Read stderr
    let tx2 = tx.clone();
    let out_lines = output_lines.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                println!("[MarkovChainBot.py][stderr] {}", line);
                let _ = tx2.send(line.clone());
                out_lines.lock().unwrap().push(line);
            }
        }
    });
    // Wait for process to exit in a background thread
    let child_arc_clone = child_arc.clone();
    thread::spawn(move || {
        let status = child_arc_clone.lock().unwrap().wait().expect("Failed to wait on child");
        println!("[MarkovChainBot.py] exited with status: {}", status);
        // Do NOT send to tx, so it doesn't show in the GUI
        // let _ = tx_exit.send(format!("[MarkovChainBot.py] exited with status: {}", status));
    });
    (Some(child_arc), pid)
}

impl TwitchYapBotApp {
    fn stop_bot(&mut self) {
        #[cfg(windows)]
        {
            if let Some(pid) = self.child_pid {
                use std::process::Command;
                let tk_result = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F", "/T"]).output();
                println!("[GUI] Ran: taskkill /PID {} /F /T", pid);
                if let Ok(ref out) = tk_result {
                    println!("[GUI] taskkill result: {}", String::from_utf8_lossy(&out.stdout).trim());
                }
            }
        }
        #[cfg(unix)]
        {
            if let Some(child_arc) = &self.child {
                let mut child = child_arc.lock().unwrap();
                let _ = child.kill();
            }
        }
        self.child = None;
        self.child_pid = None;
        use chrono::Local;
        let now = Local::now();
        let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
        self.output_lines.lock().unwrap().push(format!("{} Yap Bot has been destroyed by your own hands...", timestamp));
    }
}

impl App for TwitchYapBotApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Dracula theme and font setup (copied from main installer)
        let mut visuals = egui::Visuals::dark();
        visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));
        visuals.widgets.active.bg_fill = egui::Color32::from_rgb(189, 147, 249);
        visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(139, 233, 253);
        visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(68, 71, 90);
        visuals.selection.bg_fill = egui::Color32::from_rgb(189, 147, 249);
        visuals.hyperlink_color = egui::Color32::from_rgb(139, 233, 253);
        visuals.warn_fg_color = egui::Color32::from_rgb(255, 184, 108);
        visuals.error_fg_color = egui::Color32::from_rgb(255, 85, 85);
        visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(68, 71, 90);
        visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(248, 248, 242);
        visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(40, 42, 54);
        ctx.set_visuals(visuals);
        // Title
        egui::TopBottomPanel::top("title").show(ctx, |ui| {
            ui.add_space(10.0); // Increased padding above the title
            ui.label(egui::RichText::new(format!("Twitch Yap Bot v{}", VERSION))
                .font(egui::FontId::new(17.0, egui::FontFamily::Name("consolas_titles".into())))
                .color(egui::Color32::from_rgb(189, 147, 249)));
            ui.add_space(3.0);
        });
        egui::CentralPanel::default().show(ctx, |ui| {
            // Add padding below the title separator
            ui.add_space(8.0);
            // Two horizontally-aligned buttons
            ui.horizontal(|ui| {
                let button_size = egui::vec2(140.0, 36.0);
                let relaunch_color = egui::Color32::from_rgb(90, 80, 30); // deep olive
                let stop_color = egui::Color32::from_rgb(70, 20, 20); // very dark muted red
                let relaunch_clicked = ui.add_sized(
                    button_size,
                    egui::Button::new(
                        egui::RichText::new("Relaunch Bot").strong().size(13.0)
                    ).fill(relaunch_color)
                ).clicked();
                if relaunch_clicked {
                    // If running, stop first
                    if self.child.is_some() || self.child_pid.is_some() {
                        self.stop_bot();
                    }
                    // Relaunch MarkovChainBot.py
                    let output_lines = self.output_lines.clone();
                    let (tx, rx) = mpsc::channel();
                    let output_lines_clone = output_lines.clone();
                    let (child_sender, child_receiver) = mpsc::channel();
                    thread::spawn(move || {
                        let (child_arc, pid) = run_markov_chain_bot(tx, output_lines_clone);
                        let _ = child_sender.send((child_arc, pid));
                    });
                    let (child, child_pid) = child_receiver.recv().unwrap_or((None, None));
                    self.child = child;
                    self.child_pid = child_pid;
                    self.rx = Some(rx);
                    self.marker_index = None; // Reset output filtering for new run
                    use chrono::Local;
                    let now = Local::now();
                    let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
                    self.output_lines.lock().unwrap().push(format!("{} Reviving Yap Bot from the depths of hell...", timestamp));
                }
                ui.add_space(12.0);
                let stop_clicked = ui.add_sized(
                    button_size,
                    egui::Button::new(
                        egui::RichText::new("Stop Bot").strong().size(13.0)
                    ).fill(stop_color)
                ).clicked();
                if stop_clicked {
                    self.stop_bot();
                }
            });
            ui.add_space(10.0);
            ui.separator();
            ui.add_space(10.0);
            ui.label(
                egui::RichText::new("Yap Bot Output:")
                    .size(15.0)
                    .font(egui::FontId::new(15.0, egui::FontFamily::Name("consolas".into())))
            );
            ui.add_space(5.0);
            ui.separator();
            let scroll_id = ui.make_persistent_id("output_scroll_area");
            let mut num_displayed = 0;
            let mut end_rect = None;
            egui::ScrollArea::vertical().id_source(scroll_id).auto_shrink([false; 2]).show(ui, |ui| {
                let lines: Vec<String> = {
                    let guard = self.output_lines.lock().unwrap();
                    guard.clone()
                };
                let websocket_marker = "[TwitchWebsocket.TwitchWebsocket] [INFO    ] - Attempting to initialize websocket connection.";
                if self.marker_index.is_none() {
                    self.marker_index = lines.iter().position(|line| line.contains(websocket_marker));
                }
                let start = self.marker_index.unwrap_or(usize::MAX);
                let mut seen = HashSet::new();
                let display_lines = if start < lines.len() {
                    let after_marker = &lines[start..];
                    if after_marker.len() > 200 {
                        &after_marker[after_marker.len() - 200..]
                    } else {
                        after_marker
                    }
                } else {
                    &[]
                };
                let log_re = Regex::new(r"^\[(\d{4})-(\d{2})-(\d{2}) (\d{2}):(\d{2}):(\d{2}),\d+\].*- (.+)$").unwrap();
                for line in display_lines.iter() {
                    if !seen.insert(line) {
                        continue; // skip all duplicates
                    }
                    if line.contains("Fetching mod list...") ||
                       line.contains("Unrecognized command: /mods") ||
                       line.contains("Unrecognized command: /w") {
                        continue;
                    }
                    // Filter out the SyntaxWarning about invalid escape sequence
                    if line.contains("SyntaxWarning: invalid escape sequence '\\w'") && line.contains("MarkovChainBot.py") {
                        continue;
                    }
                    // Filter out the regex assignment line
                    if line.contains("self.link_regex = re.compile(\"\\w+\\.[a-z]{2,}\")") {
                        continue;
                    }
                    let rich = if let Some(caps) = log_re.captures(line) {
                        let formatted = format!(
                            "[{}/{}/{} - {}:{}:{}]: {}",
                            &caps[2], &caps[3], &caps[1], &caps[4], &caps[5], &caps[6], &caps[7]
                        );
                        egui::RichText::new(formatted)
                            .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                    } else {
                        egui::RichText::new(line)
                            .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                    };
                    if line.contains("error") || line.contains("Error") {
                        ui.colored_label(egui::Color32::from_rgb(255, 85, 85), rich);
                    } else {
                        ui.label(rich);
                    }
                    num_displayed += 1;
                }
                // Dummy label for autoscroll
                let end_resp = ui.label(egui::RichText::new("").font(egui::FontId::new(1.0, egui::FontFamily::Monospace)));
                if self.auto_scroll && num_displayed > self.last_num_displayed {
                    end_resp.scroll_to_me(Some(egui::Align::BOTTOM));
                }
                end_rect = Some(end_resp.rect);
            });
            // After rendering, check if the dummy label is visible in the clip rect
            let at_bottom = if let Some(rect) = end_rect {
                let clip_bottom = ctx.input(|i| i.screen_rect().bottom());
                (rect.bottom() - clip_bottom).abs() < 5.0 || rect.bottom() < clip_bottom
            } else {
                true
            };
            if at_bottom {
                self.auto_scroll = true;
            } else {
                self.auto_scroll = false;
            }
            self.last_num_displayed = num_displayed;
        });
        // Poll for new output
        if let Some(rx) = &self.rx {
            let websocket_marker = "[TwitchWebsocket.TwitchWebsocket] [INFO    ] - Attempting to initialize websocket connection.";
            while let Ok(line) = rx.try_recv() {
                self.output_lines.lock().unwrap().push(line);
                // Update marker_index if the marker is found in the new line
                if self.marker_index.is_none() {
                    let lines = self.output_lines.lock().unwrap();
                    self.marker_index = lines.iter().position(|line| line.contains(websocket_marker));
                }
                ctx.request_repaint();
            }
        }
        // Always repaint to ensure instant GUI updates
        ctx.request_repaint();
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        self.stop_bot();
    }
}

fn main() {
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
        &format!("Twitch Yap Bot v{}", VERSION),
        native_options,
        Box::new(|cc| {
            // Set up Consolas and consolas_titles font family (and consolas for regular text)
            let mut fonts = egui::FontDefinitions::default();
            fonts.font_data.insert(
                "consolas".to_owned(),
                egui::FontData::from_static(include_bytes!("../../resources/font/Consolas_Regular.ttf")),
            );
            // Map the 'consolas' font family name to the 'consolas' font data
            fonts.families.insert(
                egui::FontFamily::Name("consolas".into()),
                vec!["consolas".to_owned()]
            );
            // Add a custom font family for titles
            fonts.families.insert(
                egui::FontFamily::Name("consolas_titles".into()),
                vec!["consolas".to_owned()]
            );
            cc.egui_ctx.set_fonts(fonts);
            // Dracula theme (already present)
            let mut visuals = egui::Visuals::dark();
            visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242));
            visuals.widgets.active.bg_fill = egui::Color32::from_rgb(189, 147, 249);
            visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(139, 233, 253);
            visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(68, 71, 90);
            visuals.selection.bg_fill = egui::Color32::from_rgb(189, 147, 249);
            visuals.hyperlink_color = egui::Color32::from_rgb(139, 233, 253);
            visuals.warn_fg_color = egui::Color32::from_rgb(255, 184, 108);
            visuals.error_fg_color = egui::Color32::from_rgb(255, 85, 85);
            visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(68, 71, 90);
            visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(248, 248, 242);
            visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(40, 42, 54);
            cc.egui_ctx.set_visuals(visuals);
            Ok(Box::new(TwitchYapBotApp::default()))
        }),
    ).unwrap();
}