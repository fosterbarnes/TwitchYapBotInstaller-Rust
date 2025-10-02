//! Launch flags module for TwitchYapBot
//!
//! This module handles parsing and processing of command-line launch flags for testing and debugging purposes.

use std::env;

/// Launch flag configuration structure
#[derive(Debug, Default)]
pub struct LaunchFlags {
    pub force_current_version: bool,
    pub force_out_of_date_version: bool,
    pub force_unpublished_version: bool,
    pub settings_window: bool,
    pub force_gui: bool,
    pub first_launch: bool,
}

impl LaunchFlags {
    /// Parse command line arguments and return launch flags configuration
    pub fn from_args() -> Self {
        let args: Vec<String> = env::args().collect();
        
        Self {
            force_current_version: args.iter().any(|a| a == "--force-current-version" || a == "--current"),
            force_out_of_date_version: args.iter().any(|a| a == "--force-out-of-date-version" || a == "--old"),
            force_unpublished_version: args.iter().any(|a| a == "--force-unpublished-version" || a == "--new"),
            settings_window: args.iter().any(|a| a == "--settings-window" || a == "--settings" || a == "--s"),
            force_gui: args.iter().any(|a| a == "--force-gui" || a == "--gui"),
            first_launch: args.iter().any(|a| a == "--force-first-launch" || a == "--first-launch"),
        }
    }
    
    /// Apply the launch flags by setting appropriate environment variables
    pub fn apply(&self) {
        if self.force_current_version {
            env::set_var("YAPBOT_FORCE_CURRENT_VERSION", "true");
        }
        if self.force_out_of_date_version {
            env::set_var("YAPBOT_FORCE_OUT_OF_DATE_VERSION", "true");
        }
        if self.force_unpublished_version {
            env::set_var("YAPBOT_FORCE_UNPUBLISHED_VERSION", "true");
        }
        if self.first_launch {
            env::set_var("YAPBOT_FORCE_FIRST_LAUNCH", "true");
        }
    }
    
    /// Check if any version override flags are set
    pub fn has_version_overrides(&self) -> bool {
        self.force_current_version || self.force_out_of_date_version || self.force_unpublished_version
    }
    
    /// Check if settings window should be launched
    pub fn should_launch_settings_window(&self) -> bool {
        self.settings_window
    }
    
    /// Check if GUI should be forced (skip tray minimization check)
    pub fn should_force_gui(&self) -> bool {
        self.force_gui
    }
    
    /// Get a description of the active flags for debugging
    pub fn get_active_flags_description(&self) -> String {
        let mut flags = Vec::new();
        
        if self.force_current_version {
            flags.push("--force-current-version");
        }
        if self.force_out_of_date_version {
            flags.push("--force-out-of-date-version");
        }
        if self.force_unpublished_version {
            flags.push("--force-unpublished-version");
        }
        if self.settings_window {
            flags.push("--settings-window");
        }
        if self.force_gui {
            flags.push("--force-gui");
        }
        if self.first_launch {
            flags.push("--force-first-launch");
        }
        
        if flags.is_empty() {
            "No launch flags active".to_string()
        } else {
            format!("Active flags: {}", flags.join(", "))
        }
    }
}
