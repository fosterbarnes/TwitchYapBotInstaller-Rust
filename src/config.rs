//! Configuration constants and settings for the YapBot Installer
//! 
//! This module contains application-wide configuration values including
//! UI settings and window dimensions.

/// The current application version, embedded at compile time from version.txt
pub fn app_version() -> &'static str {
    include_str!("version.txt").trim()
}

/// Default window size
pub static WINDOW_SIZE: [f32; 2] = [800.0, 580.0];

/// Minimum window size
pub static MIN_WINDOW_SIZE: [f32; 2] = [600.0, 461.0]; 