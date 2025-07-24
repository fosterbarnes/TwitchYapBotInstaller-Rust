//! Cross-platform window centering utility for YapBot applications
//!
//! This module provides a function to calculate the centered window position for egui/eframe apps on all supported platforms.
//!
//! Used by YapBotInstaller, TwitchYapBot, and YapBotUpdater to ensure consistent window centering.

#[cfg(windows)]
#[allow(dead_code)]
pub fn calculate_window_position(window_size: [f32; 2]) -> eframe::egui::Pos2 {
    use windows::Win32::Foundation::POINT;
    use windows::Win32::Graphics::Gdi::{MonitorFromPoint, GetMonitorInfoW, MONITORINFO, MONITOR_DEFAULTTONEAREST};
    use windows::Win32::UI::WindowsAndMessaging::GetCursorPos;
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
                return eframe::egui::Pos2::new(x, y);
            } else {
                return eframe::egui::Pos2::new(100.0, 100.0);
            }
        } else {
            return eframe::egui::Pos2::new(100.0, 100.0);
        }
    }
}

#[cfg(not(windows))]
pub fn calculate_window_position(_window_size: [f32; 2]) -> eframe::egui::Pos2 {
    eframe::egui::Pos2::new(100.0, 100.0)
} 