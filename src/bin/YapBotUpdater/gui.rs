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

pub fn calculate_window_position(window_size: [f32; 2]) -> egui::Pos2 {
    #[cfg(windows)]
    {
        use windows::Win32::Foundation::POINT;
        use windows::Win32::Graphics::Gdi::{MonitorFromPoint, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};
        unsafe {
            let mut point = POINT { x: 0, y: 0 };
            if windows::Win32::UI::WindowsAndMessaging::GetCursorPos(&mut point).is_ok() {
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
                    return egui::Pos2::new(x, y);
                } else {
                    return egui::Pos2::new(100.0, 100.0);
                }
            } else {
                return egui::Pos2::new(100.0, 100.0);
            }
        }
    }
    #[cfg(not(windows))]
    {
        egui::Pos2::new(100.0, 100.0)
    }
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
    #[cfg(windows)]
    {
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
    #[cfg(not(windows))]
    {
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
                            let _ = std::process::Command::new(exe_path)
                                .spawn();
                        }
                        std::process::exit(0);
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
