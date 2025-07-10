//! YapBot Installer
//! 
//! A desktop application for installing and configuring YapBot for Twitch.
//! Built with Rust and egui for cross-platform (Windows & Linux)

// Import all modules
mod config;
mod data_structures;
mod gui;
mod python_manager;

// Re-export commonly used items
pub use config::*;
pub use data_structures::*;

// Third-party crate imports
use eframe::egui;

// Platform-specific imports
#[cfg(windows)]
use windows::Win32::Foundation::POINT;
#[cfg(windows)]
use windows::Win32::Graphics::Gdi::{MonitorFromPoint, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;

/// Load application icon from embedded resources
fn load_app_icon() -> Option<egui::IconData> {
    #[cfg(windows)]
    {
        if let Ok(image) = image::load_from_memory(include_bytes!("../resources/icon/yap_icon_green.ico")) {
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
        // Try PNG first on Linux, then fallback to ICO
        if let Ok(image) = image::load_from_memory(include_bytes!("../resources/icon/yap_icon_green.png")) {
            let rgba = image.to_rgba8();
            let size = [rgba.width() as u32, rgba.height() as u32];
            Some(egui::IconData {
                rgba: rgba.into_raw(),
                width: size[0],
                height: size[1],
            })
        } else if let Ok(image) = image::load_from_memory(include_bytes!("../resources/icon/yap_icon_green.ico")) {
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

/// Calculate window position to center on the currently used monitor
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
        // On Linux, just center the window on screen
        // We'll use a simple approach that works with most window managers
        egui::Pos2::new(100.0, 100.0)
    }
}

/// Configure the application window and visuals
fn configure_window(icon_data: Option<egui::IconData>) -> eframe::NativeOptions {
    let window_size = WINDOW_SIZE;
    let center_pos = calculate_window_position(window_size);

    let mut viewport_builder = egui::ViewportBuilder::default()
        .with_inner_size(window_size)
        .with_position(center_pos)
        .with_decorations(true)
        .with_resizable(true)
        .with_min_inner_size(MIN_WINDOW_SIZE); // Minimum window size to prevent UI elements from disappearing
    
    if let Some(icon) = icon_data {
        viewport_builder = viewport_builder.with_icon(icon);
    }

    eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    }
}

/// Apply Dracula theme and configure fonts
fn configure_visuals(ctx: &egui::Context) {
    let mut visuals = egui::Visuals::dark();
    
    // Dracula theme accent colors
    visuals.override_text_color = Some(egui::Color32::from_rgb(248, 248, 242)); // #f8f8f2 (light gray)
    visuals.widgets.active.bg_fill = egui::Color32::from_rgb(189, 147, 249); // #bd93f9 (purple)
    visuals.widgets.hovered.bg_fill = egui::Color32::from_rgb(139, 233, 253); // #8be9fd (cyan)
    visuals.widgets.inactive.bg_fill = egui::Color32::from_rgb(68, 71, 90); // #44475a (darker gray)
    visuals.selection.bg_fill = egui::Color32::from_rgb(189, 147, 249); // #bd93f9 (purple)
    visuals.hyperlink_color = egui::Color32::from_rgb(139, 233, 253); // #8be9fd (cyan)
    visuals.warn_fg_color = egui::Color32::from_rgb(255, 184, 108); // #ffb86c (orange)
    visuals.error_fg_color = egui::Color32::from_rgb(255, 85, 85); // #ff5555 (red)
    visuals.widgets.noninteractive.bg_fill = egui::Color32::from_rgb(68, 71, 90); // #44475a
    visuals.widgets.active.fg_stroke.color = egui::Color32::from_rgb(248, 248, 242); // #f8f8f2 (white text on purple)
    visuals.widgets.hovered.fg_stroke.color = egui::Color32::from_rgb(40, 42, 54); // #282a36 (dark text on cyan)
    
    ctx.set_visuals(visuals);
    
    // Configure fonts to use bundled Consolas for specific elements
    let mut fonts = egui::FontDefinitions::default();
    
    // Add Consolas as a bundled font
    fonts.font_data.insert(
        "consolas".to_owned(),
        egui::FontData::from_static(include_bytes!("../resources/font/Consolas_Regular.ttf"))
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
    // Set Consolas as the monospace font and for text_edit
    fonts.families.insert(
        egui::FontFamily::Name("text_edit".into()),
        vec!["consolas".to_owned()]
    );
    ctx.set_fonts(fonts);
}

fn main() {
    println!("App started - main function entered");
    
    // Load application icon
    let icon_data = load_app_icon();
    
    // Configure window
    let native_options = configure_window(icon_data);
    
    // Load settings if present
    let loaded_settings = YapBotInstaller::load_settings_from_file();
    
    // Run the application
    let result = eframe::run_native(
        "YapBot Installer",
        native_options,
        Box::new(move |cc| {
            println!("App GUI initialized");
            // Configure visuals
            configure_visuals(&cc.egui_ctx);
            let mut app = YapBotInstaller::default();
            app.loaded_settings = loaded_settings.clone();
            // If settings are loaded and complete, populate fields and skip to step 4 after step 1
            if let Some(settings) = loaded_settings {
                app.main_channel_name = settings.main_channel_name;
                app.bot_channel_name = settings.bot_channel_name;
                app.denied_users = settings.denied_users;
                app.cooldown = settings.cooldown;
                app.generate_command = settings.generate_command;
                app.bot_oauth_token = Some(settings.oauth);
                // Do not skip to step 4 yet; this will be handled after step 1 completes in the UI logic
            }
            Ok(Box::new(app))
        }),
    );
    
    result.expect("Failed to start eframe");
}
