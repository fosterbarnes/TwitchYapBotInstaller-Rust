//! YapBot Updater GUI Module
//!
//! Contains the egui-based GUI logic, theming, and progress bar rendering for the YapBot Updater application.
//!
//! Handles update progress display, status messages, and launching the updated TwitchYapBot.

use eframe::egui;
use std::sync::Arc;
use std::sync::Mutex as StdMutex;

/// Draw a full-width progress bar with percentage, matching rustitles style.
pub fn draw_progress_bar(ui: &mut egui::Ui, progress: f32) {
    let window_width = ui.ctx().screen_rect().width();
    let progress_bar = egui::ProgressBar::new(progress)
        .show_percentage()
        .fill(egui::Color32::from_rgb(124, 99, 160)) // #7c63a0
        .desired_width(window_width - 18.0)
        .desired_height(20.0);
    ui.add(progress_bar);
}

// Setup fonts and Dracula theme for egui context
pub fn setup_fonts_and_theme(ctx: &egui::Context) {
    let mut fonts = egui::FontDefinitions::default();
    fonts.font_data.insert(
        "consolas".to_owned(),
        egui::FontData::from_static(include_bytes!("../../../resources/font/Consolas_Regular.ttf")),
    );
    fonts.families.insert(
        egui::FontFamily::Name("consolas".into()),
        vec!["consolas".to_owned()]
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

/// Loads the app icon for the window.
pub fn load_app_icon() -> Option<egui::IconData> {
    if let Ok(image) = image::load_from_memory(include_bytes!("../../../resources/icon/yap_icon_blue.ico")) {
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

/// Returns the current app version from version.txt.
pub fn get_version() -> &'static str {
    include_str!("../../version.txt").trim()
}

#[allow(dead_code)]
pub enum UpdateState {
    Idle,
    Downloading(String), // file name
    Replacing(String),   // file name
    Done,
    Error(String),
}

pub struct YapUpdaterApp {
    pub state: UpdateState,
    pub progress: f32,
    pub status: String,
    update_task: Option<tokio::task::JoinHandle<()>>,
    shared_progress: Arc<StdMutex<(f32, String)>>,
}

impl Default for YapUpdaterApp {
    fn default() -> Self {
        Self {
            state: UpdateState::Idle,
            progress: 0.0,
            status: "Ready to update.".to_string(),
            update_task: None,
            shared_progress: Arc::new(StdMutex::new((0.0, "Starting...".to_string()))),
        }
    }
}

impl eframe::App for YapUpdaterApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        setup_fonts_and_theme(ctx);
        let version = get_version();
        // Start update task if in Idle state
        if let UpdateState::Idle = self.state {
            self.status = "Starting update...".to_string();
            self.state = UpdateState::Downloading("TwitchYapBot.exe".to_string());
            let shared_progress = self.shared_progress.clone();
            let app_handle = ctx.clone();
            self.update_task = Some(tokio::spawn(async move {
                // Insert a 2-second delay before starting the update to make sure twitchyapbot is closed before the update starts
                tokio::time::sleep(std::time::Duration::from_secs(2)).await;
                let _ = super::updater::perform_update(|progress: super::updater::UpdateProgress| {
                    let mut lock = shared_progress.lock().unwrap();
                    *lock = (progress.progress, progress.status.clone());
                    app_handle.request_repaint();
                })
                .await;
            }));
        }
        // Update progress and status from shared_progress
        let (progress, status) = {
            let lock = self.shared_progress.lock().unwrap();
            (lock.0, lock.1.clone())
        };
        self.progress = progress;
        self.status = status;
        // Only set Done state and status if not already Done
        if self.progress >= 1.0 && !matches!(self.state, UpdateState::Done) {
            self.state = UpdateState::Done;
            self.status = "Update complete!".to_string();
        }
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.label(
                egui::RichText::new(format!("Yap Bot Updater v{}", version))
                    .font(egui::FontId::new(17.0, egui::FontFamily::Name("consolas".into())))
                    .color(egui::Color32::from_rgb(189, 147, 249)) // #bd93f9
            );
            //ui.add_space(5.0);
            match &self.state {
                UpdateState::Downloading(_file) | UpdateState::Replacing(_file) => {
                    ui.add_space(5.0);
                }
                _ => {}
            }
            ui.label(&self.status);
            match &self.state {
                UpdateState::Idle => {}, // Should not be visible
                UpdateState::Downloading(_file) | UpdateState::Replacing(_file) => {
                    draw_progress_bar(ui, self.progress);
                }
                UpdateState::Done => {
                    let button = ui.add_sized([
                        ui.available_width(),
                        25.0
                    ], egui::Button::new("Launch YapBot"));
                    if button.clicked() {
                        // Launch the new TwitchYapBot.exe
                        if let Ok(appdata) = std::env::var("APPDATA") {
                            let exe_path = std::path::Path::new(&appdata)
                                .join("YapBot")
                                .join("TwitchYapBot.exe");
                            if let Ok(child) = std::process::Command::new(exe_path).spawn() {
                                let child_pid = child.id();
                                
                                // Spawn a thread to wait for TwitchYapBot window to be ready
                                // This prevents VM blue screen issue by ensuring proper initialization
                                std::thread::spawn(move || {
                                    wait_for_yapbot_window_ready(child_pid);
                                    std::process::exit(0);
                                });
                            } else {
                                // Fallback to immediate exit if launch failed
                                std::process::exit(0);
                            }
                        } else {
                            std::process::exit(0);
                        }
                    }
                }
                UpdateState::Error(msg) => {
                    ui.colored_label(egui::Color32::from_rgb(255, 85, 85), format!("Error: {}", msg)); // #ff5555
                    if ui.button("Retry").clicked() {
                        self.state = UpdateState::Idle;
                        self.status = "Ready to update.".to_string();
                    }
                }
            }
        });
    }
}

/// Wait for TwitchYapBot window to be properly initialized before allowing parent process to exit.
/// This prevents VM blue screen issues by ensuring the child process has fully initialized its graphics context.
#[cfg(windows)]
fn wait_for_yapbot_window_ready(child_pid: u32) {
    use std::time::{Duration, Instant};
    
    let timeout = Duration::from_secs(15); // Maximum wait time
    let start_time = Instant::now();
    let check_interval = Duration::from_millis(200);
    
    // First, wait for the process to be running and stable
    std::thread::sleep(Duration::from_millis(500));
    
    loop {
        // Check if we've exceeded the timeout
        if start_time.elapsed() > timeout {
            eprintln!("[UPDATER] Timeout waiting for TwitchYapBot window, proceeding with exit");
            break;
        }
        
        // Check if the child process is still running
        if !is_process_running(child_pid) {
            eprintln!("[UPDATER] TwitchYapBot process {} is no longer running", child_pid);
            break;
        }
        
        // Look for the TwitchYapBot window by title pattern
        if is_yapbot_window_ready(child_pid) {
            // Window is visible and belongs to our child process
            // Wait a bit more to ensure it's fully initialized
            std::thread::sleep(Duration::from_millis(1000));
            println!("[UPDATER] TwitchYapBot window is ready, safe to exit updater");
            break;
        }
        
        std::thread::sleep(check_interval);
    }
}

/// Check if TwitchYapBot window is ready and visible
#[cfg(windows)]
fn is_yapbot_window_ready(child_pid: u32) -> bool {
    use std::ffi::CString;
    
    // Try to get version from embedded version file
    let version = get_version();
    let expected_title = format!("Twitch Yap Bot v{}", version);
    
    if let Ok(title_cstring) = CString::new(expected_title) {
        unsafe {
            use windows::Win32::UI::WindowsAndMessaging::{FindWindowA, IsWindowVisible, GetWindowThreadProcessId};
            use windows::core::PCSTR;
            
            let hwnd = FindWindowA(None, PCSTR(title_cstring.as_ptr() as *const u8));
            if hwnd.0 != 0 {
                // Found the window, check if it's visible and belongs to our process
                if IsWindowVisible(hwnd).as_bool() {
                    let mut window_pid: u32 = 0;
                    GetWindowThreadProcessId(hwnd, Some(&mut window_pid));
                    
                    return window_pid == child_pid;
                }
            }
        }
    }
    false
}

/// Check if a process with the given PID is still running
#[cfg(windows)]
fn is_process_running(pid: u32) -> bool {
    unsafe {
        use windows::Win32::System::Diagnostics::ToolHelp::{
            CreateToolhelp32Snapshot, Process32FirstW, Process32NextW, 
            PROCESSENTRY32W, TH32CS_SNAPPROCESS
        };
        use windows::Win32::Foundation::CloseHandle;
        
        let snapshot = match CreateToolhelp32Snapshot(TH32CS_SNAPPROCESS, 0) {
            Ok(handle) => handle,
            Err(_) => return false,
        };
        
        if snapshot.is_invalid() {
            return false;
        }
        
        let mut process_entry = PROCESSENTRY32W {
            dwSize: std::mem::size_of::<PROCESSENTRY32W>() as u32,
            ..Default::default()
        };
        
        let mut found = false;
        if Process32FirstW(snapshot, &mut process_entry).is_ok() {
            loop {
                if process_entry.th32ProcessID == pid {
                    found = true;
                    break;
                }
                if Process32NextW(snapshot, &mut process_entry).is_err() {
                    break;
                }
            }
        }
        
        let _ = CloseHandle(snapshot);
        found
    }
}
