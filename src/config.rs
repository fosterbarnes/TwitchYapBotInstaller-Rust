//! Configuration constants and settings for the YapBot Installer
//! 
//! This module contains application-wide configuration values including
//! UI settings and window dimensions.

use std::path::PathBuf;

/// The current application version, embedded at compile time from version.txt
pub fn app_version() -> &'static str {
    include_str!("version.txt").trim()
}

/// Default window size
pub static WINDOW_SIZE: [f32; 2] = [800.0, 580.0];

/// Minimum window size
pub static MIN_WINDOW_SIZE: [f32; 2] = [600.0, 461.0];

/// Maximum number of log files to keep
pub const MAX_LOG_FILES: usize = 10;

/// Returns the path to the YapBot log directory (e.g., C:\Users\User\AppData\Roaming\YapBot\logs)
pub fn get_log_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    PathBuf::from(format!("{}\\YapBot\\logs", appdata))
} 