//! Configuration constants and settings for TwitchYapBot
//!
//! This module contains application-wide configuration values including
//! UI settings and window dimensions for the TwitchYapBot executable.

use std::path::PathBuf;

/// The current application version, embedded at compile time from version.txt
pub fn app_version() -> &'static str {
    include_str!("../../version.txt").trim()
}

/// Default window size
pub static WINDOW_SIZE: [f32; 2] = [800.0, 517.0];

/// Minimum window size
pub static MIN_WINDOW_SIZE: [f32; 2] = [730.0, 200.0];

/// Maximum number of log files to keep
pub const MAX_LOG_FILES: usize = 10;

/// Returns the path to the YapBot log directory (e.g., C:\Users\User\AppData\Roaming\YapBot\logs)
pub fn get_log_dir() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    PathBuf::from(format!("{}\\YapBot\\logs", appdata))
}

/// Settings window default size
pub static SETTINGS_WINDOW_SIZE: [f32; 2] = [666.0, 600.0];

/// Minimum window size for settings window
pub static SETTINGS_MIN_WINDOW_SIZE: [f32; 2] = [400.0, 200.0];

// Common file names/paths
pub const INSTALLER_SETTINGS_FILENAME: &str = "YapBotInstallerSettings.json";
pub const SETTINGS_PY_FILENAME: &str = "Settings.py";
pub const SETTINGS_JSON_FILENAME: &str = "settings.json";
pub const TWITCH_MARKOVCHAIN_DIR: &str = "TwitchMarkovChain"; 