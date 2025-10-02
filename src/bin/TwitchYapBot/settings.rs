//! Settings dialog logic and persistent bot settings for TwitchYapBot
//!
//! This module contains the settings dialog UI, persistent bot settings, and related logic for the TwitchYapBot executable.
//!
//! Settings dialog logic split from main.rs

use std::path::PathBuf;
use std::io::Write;
use serde::{Deserialize, Serialize};
use yap_bot_installer::bubbles::bubble_list_ui;
use yap_bot_installer::data_structures::edit_settings_py;
use eframe::egui;
use std::fs;
use crate::log_and_print;
use crate::config::{INSTALLER_SETTINGS_FILENAME, SETTINGS_PY_FILENAME, SETTINGS_JSON_FILENAME, SETTINGS_WINDOW_SIZE, SETTINGS_MIN_WINDOW_SIZE};
use open;

// Default values for settings
const DEFAULT_GENERATE_COMMANDS: &[&str] = &["!generate", "!g"];
const DEFAULT_DENIED_USERS: &[&str] = &["StreamElements", "Nightbot", "Moobot", "Marbiebot"];
const DEFAULT_COOLDOWN: i32 = 20;
const DEFAULT_KEY_LENGTH: i32 = 2;
const DEFAULT_MAX_SENTENCE_WORDS: i32 = 40;
const DEFAULT_MIN_SENTENCE_WORDS: i32 = -1;
const DEFAULT_AUTO_GEN_TIMER: i32 = -1;
const DEFAULT_RANDOMIZED_TIMER_MIN: i32 = 30;
const DEFAULT_RANDOMIZED_TIMER_MAX: i32 = 100;
const DEFAULT_HELP_MESSAGE_TIMER: i32 = 216000;
const DEFAULT_SENTENCE_SEPARATOR: &str = " - ";

// UI constants
const UI_SPACING_SMALL: f32 = 2.0;
const UI_SPACING_MEDIUM: f32 = 4.0;
const UI_SPACING_LARGE: f32 = 16.0;
const UI_BUTTON_HEIGHT: f32 = 24.0;
const UI_BUTTON_WIDTH: f32 = 200.0;
const UI_ICON_SIZE: f32 = 20.0;
const UI_TEXT_WIDTH: f32 = 250.0;
const UI_BUBBLE_HEIGHT: f32 = 18.0;
const UI_FONT_SIZE: f32 = 13.0;
const UI_SCROLL_HEIGHT: f32 = 80.0;

// Token validation constants
const TWITCH_TOKEN_LENGTH: usize = 30;
const MIN_AUTO_GEN_TIMER: i32 = 5;
const MIN_RANDOMIZED_TIMER: i32 = 5;

// File paths
const TWITCH_API_URL: &str = "https://api.twitch.tv/helix/users";
const TOKEN_GENERATOR_URL: &str = "https://twitchtokengenerator.com";
const TCP_RESTART_MESSAGE: &[u8] = b"RESTART_BOT";
const TCP_ADDRESS: &str = "127.0.0.1:9876";

fn default_generate_commands() -> Vec<String> {
    DEFAULT_GENERATE_COMMANDS.iter().map(|s| s.to_string()).collect()
}

fn default_denied_users() -> Vec<String> {
    DEFAULT_DENIED_USERS.iter().map(|s| s.to_string()).collect()
}

fn default_allowed_users() -> Vec<String> {
    vec![]
}

fn default_start_minimized_to_tray() -> bool {
    false
}

fn default_exit_when_monitored_app_closes() -> bool {
    false
}

fn default_randomized_generation_timer_enabled() -> bool {
    false
}

fn default_randomized_generation_timer_min() -> i32 {
    DEFAULT_RANDOMIZED_TIMER_MIN
}

fn default_randomized_generation_timer_max() -> i32 {
    DEFAULT_RANDOMIZED_TIMER_MAX
}

fn default_first_launch() -> bool {
    true
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BotSettings {
    pub channel: String,
    pub nickname: String,
    pub authentication: String,
    #[serde(default = "default_denied_users")]
    pub denied_users: Vec<String>,
    #[serde(default = "default_allowed_users")]
    pub allowed_users: Vec<String>,
    pub cooldown: i32,
    pub key_length: i32,
    pub max_sentence_word_amount: i32,
    pub min_sentence_word_amount: i32,
    pub automatic_generation_timer: i32,
    #[serde(default = "default_generate_commands")]
    pub generate_commands: Vec<String>,
    pub sound_enabled: bool,
    #[serde(default = "default_start_minimized_to_tray")]
    pub start_minimized_to_tray: bool,
    #[serde(default = "default_exit_when_monitored_app_closes")]
    pub exit_when_monitored_app_closes: bool,
    // New fields for randomized timer
    #[serde(default = "default_randomized_generation_timer_enabled")]
    pub randomized_generation_timer_enabled: bool,
    #[serde(default = "default_randomized_generation_timer_min")]
    pub randomized_generation_timer_min: i32,
    #[serde(default = "default_randomized_generation_timer_max")]
    pub randomized_generation_timer_max: i32,
    #[serde(default = "default_first_launch")]
    pub first_launch: bool,
}

impl Default for BotSettings {
    fn default() -> Self {
        Self {
            channel: "#<channel>".to_string(),
            nickname: "<name>".to_string(),
            authentication: "oauth:<auth>".to_string(),
            denied_users: default_denied_users(),
            allowed_users: default_allowed_users(),
            cooldown: DEFAULT_COOLDOWN,
            key_length: DEFAULT_KEY_LENGTH,
            max_sentence_word_amount: DEFAULT_MAX_SENTENCE_WORDS,
            min_sentence_word_amount: DEFAULT_MIN_SENTENCE_WORDS,
            automatic_generation_timer: DEFAULT_AUTO_GEN_TIMER,
            generate_commands: default_generate_commands(),
            sound_enabled: true,
            start_minimized_to_tray: default_start_minimized_to_tray(),
            exit_when_monitored_app_closes: default_exit_when_monitored_app_closes(),
            randomized_generation_timer_enabled: default_randomized_generation_timer_enabled(),
            randomized_generation_timer_min: default_randomized_generation_timer_min(),
            randomized_generation_timer_max: default_randomized_generation_timer_max(),
            first_launch: default_first_launch(),
        }
    }
}

pub struct SettingsDialog {
    // Core dialog state
    pub is_open: bool,
    pub needs_restart: bool,
    
    // Settings data
    pub settings: BotSettings,
    pub temp_settings: BotSettings,
    
    // Input fields for UI
    pub channel_input: String,
    pub denied_input: String,
    pub allowed_input: String,
    pub generate_command_input: String,
    
    // UI layout adjustments
    pub denied_left_spacing: f32,
    pub allowed_left_spacing: f32,
    pub commands_left_spacing: f32,
    
    // Track last committed values for change detection
    last_committed_channel: String,
    last_committed_nickname: String,
    last_committed_auth: String,
    last_committed_cooldown: i32,
    last_committed_key_length: i32,
    last_committed_max_sent: i32,
    last_committed_min_sent: i32,
    last_committed_auto_gen: i32,
    
    // Randomized timer UI state
    randomized_timer_enabled: Option<bool>,
    randomized_timer_min: Option<i32>,
    randomized_timer_max: Option<i32>,
    
    // Token management UI state
    show_access_token: bool,
    show_paste_token_button: bool,
    show_paste_client_id_button: bool,
    show_token_success: bool,
    token_paste_warning: Option<String>,
    client_id_warning: Option<String>,
    pasted_client_id: Option<String>,
    
    // UI assets
    eye_on_texture: Option<egui::TextureHandle>,
    eye_off_texture: Option<egui::TextureHandle>,
}

impl SettingsDialog {
    pub fn new() -> Self {
        let appdata_settings_path = Self::get_appdata_settings_path();
        let mut default = BotSettings::default();
        
        // Check for first_launch field in the installer settings file
        let should_show_first_launch = Self::check_first_launch_field(&appdata_settings_path);
        default.first_launch = should_show_first_launch;
        
        if appdata_settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&appdata_settings_path) {
                if let Ok(settings) = serde_json::from_str::<BotSettings>(&content) {
                    default = settings;
                    default.first_launch = should_show_first_launch;
                }
            }
        }
        
        // Set channel input based on loaded or default channel
        let loaded_channel = default.channel.trim_start_matches('#');
        let channel_input = if loaded_channel.is_empty() || loaded_channel == "<channel>" {
            String::new()
        } else {
            loaded_channel.to_string()
        };
        Self {
            is_open: true,
            settings: default.clone(),
            temp_settings: default.clone(),
            needs_restart: false,
            denied_input: String::new(),
            allowed_input: String::new(),
            generate_command_input: String::new(),
            denied_left_spacing: 0.0,
            allowed_left_spacing: -5.0,
            commands_left_spacing: 11.0,
            last_committed_channel: default.channel.clone(),
            last_committed_nickname: default.nickname.clone(),
            last_committed_auth: default.authentication.clone(),
            last_committed_cooldown: default.cooldown,
            last_committed_key_length: default.key_length,
            last_committed_max_sent: default.max_sentence_word_amount,
            last_committed_min_sent: default.min_sentence_word_amount,
            last_committed_auto_gen: default.automatic_generation_timer,
            randomized_timer_enabled: None,
            randomized_timer_min: None,
            randomized_timer_max: None,
            show_paste_token_button: false,
            token_paste_warning: None,
            show_paste_client_id_button: false,
            pasted_client_id: None,
            client_id_warning: None,
            show_token_success: false,
            channel_input,
            show_access_token: false,
            eye_on_texture: None,
            eye_off_texture: None,
        }
    }

    pub fn load_settings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let appdata_settings_path = Self::get_appdata_settings_path();
        log_and_print!("[DEBUG] Loading GUI settings from: {}", appdata_settings_path.display());
        log_and_print!("[DEBUG] File exists: {}", appdata_settings_path.exists());
        
        // Check for first_launch field in the installer settings file
        let should_show_first_launch = Self::check_first_launch_field(&appdata_settings_path);
        log_and_print!("[DEBUG] First launch field value: {}", should_show_first_launch);
        
        // Set the first_launch value based on what we found
        self.settings.first_launch = should_show_first_launch;
        self.temp_settings.first_launch = should_show_first_launch;
        
        if appdata_settings_path.exists() {
            match fs::read_to_string(&appdata_settings_path) {
                Ok(content) => {
                    match serde_json::from_str::<BotSettings>(&content) {
                        Ok(settings) => {
                            self.settings = settings.clone();
                            self.temp_settings = settings.clone();
                            
                            // Set channel input
                            let loaded_channel = self.temp_settings.channel.trim_start_matches('#');
                            self.channel_input = if loaded_channel.is_empty() || loaded_channel == "<channel>" {
                                String::new()
                            } else {
                                loaded_channel.to_string()
                            };
                            
                            // Sync dialog fields for randomized timer from loaded settings
                            self.randomized_timer_enabled = Some(self.temp_settings.randomized_generation_timer_enabled);
                            self.randomized_timer_min = Some(self.temp_settings.randomized_generation_timer_min);
                            self.randomized_timer_max = Some(self.temp_settings.randomized_generation_timer_max);
                            log_and_print!("[DEBUG] Successfully loaded GUI settings from file.");
                        }
                        Err(e) => {
                            println!("[DEBUG] Failed to parse {} as BotSettings: {}", INSTALLER_SETTINGS_FILENAME, e);
                            if !self.migrate_old_settings(&content) {
                                self.settings = BotSettings::default();
                                self.temp_settings = self.settings.clone();
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("[DEBUG] Failed to read {}: {}", INSTALLER_SETTINGS_FILENAME, e);
                    log_and_print!("[DEBUG] Failed to read {}: {}", INSTALLER_SETTINGS_FILENAME, e);
                    self.settings = BotSettings::default();
                    self.temp_settings = self.settings.clone();
                }
            }
        } else {
            println!("[DEBUG] {} does not exist, using defaults.", INSTALLER_SETTINGS_FILENAME);
            log_and_print!("[DEBUG] {} does not exist, using defaults.", INSTALLER_SETTINGS_FILENAME);
            self.settings = BotSettings::default();
            self.temp_settings = self.settings.clone();
        }
        Ok(())
    }

    pub fn save_settings(&mut self) {
        self.settings = self.temp_settings.clone();
        // Copy randomized timer values from dialog fields to settings
        self.settings.randomized_generation_timer_enabled = self.randomized_timer_enabled.unwrap_or(false);
        self.settings.randomized_generation_timer_min = self.randomized_timer_min.unwrap_or(30);
        self.settings.randomized_generation_timer_max = self.randomized_timer_max.unwrap_or(100);
        // Before saving, ensure all generate_commands are prefixed with a single '!'
        self.settings.generate_commands = self.settings.generate_commands.iter()
            .map(|cmd| {
                let trimmed = cmd.trim_start_matches('!');
                format!("!{}", trimmed)
            })
            .collect();
        
        // Save installer settings file in the GUI format (same as old working version)
        let appdata_settings_path = Self::get_appdata_settings_path();
        
        // Save in the GUI format (BotSettings format) - same as the old working version
        let installer_settings = serde_json::json!({
            "Channel": self.settings.channel,
            "Nickname": self.settings.nickname,
            "Authentication": self.settings.authentication,
            "DeniedUsers": self.settings.denied_users,
            "AllowedUsers": self.settings.allowed_users,
            "Cooldown": self.settings.cooldown,
            "KeyLength": self.settings.key_length,
            "MaxSentenceWordAmount": self.settings.max_sentence_word_amount,
            "MinSentenceWordAmount": self.settings.min_sentence_word_amount,
            "AutomaticGenerationTimer": self.settings.automatic_generation_timer,
            "GenerateCommands": self.settings.generate_commands,
            "SoundEnabled": self.settings.sound_enabled,
            "StartMinimizedToTray": self.settings.start_minimized_to_tray,
            "ExitWhenMonitoredAppCloses": self.settings.exit_when_monitored_app_closes,
            "RandomizedGenerationTimerEnabled": self.settings.randomized_generation_timer_enabled,
            "RandomizedGenerationTimerMin": self.settings.randomized_generation_timer_min,
            "RandomizedGenerationTimerMax": self.settings.randomized_generation_timer_max
        });
        
        if let Ok(json) = serde_json::to_string_pretty(&installer_settings) {
            let _ = std::fs::create_dir_all(appdata_settings_path.parent().unwrap());
            let _ = std::fs::write(&appdata_settings_path, json);
            log_and_print!("[DEBUG] Saved GUI settings to {}", INSTALLER_SETTINGS_FILENAME);
        }
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        let settings_py_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\{}", appdata, SETTINGS_PY_FILENAME));
        let settings_json_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\{}", appdata, SETTINGS_JSON_FILENAME));
        let channel_with_hash = if self.settings.channel.starts_with('#') {
            self.settings.channel.clone()
        } else {
            format!("#{}", self.settings.channel)
        };
        let python_bot_settings = serde_json::json!({
            "Host": "irc.chat.twitch.tv",
            "Port": 6667,
            "Channel": channel_with_hash,
            "Nickname": self.settings.nickname,
            "Authentication": self.settings.authentication,
            "DeniedUsers": self.settings.denied_users,
            "AllowedUsers": self.settings.allowed_users,
            "Cooldown": self.settings.cooldown,
            "KeyLength": self.settings.key_length,
            "MaxSentenceWordAmount": self.settings.max_sentence_word_amount,
            "MinSentenceWordAmount": self.settings.min_sentence_word_amount,
            "HelpMessageTimer": DEFAULT_HELP_MESSAGE_TIMER,
            "AutomaticGenerationTimer": self.settings.automatic_generation_timer,
            "RandomizedGenerationTimerEnabled": self.settings.randomized_generation_timer_enabled,
            "RandomizedGenerationTimerMin": self.settings.randomized_generation_timer_min,
            "RandomizedGenerationTimerMax": self.settings.randomized_generation_timer_max,
            "WhisperCooldown": true,
            "EnableGenerateCommand": true,
            "SentenceSeparator": DEFAULT_SENTENCE_SEPARATOR,
            "AllowGenerateParams": true,
            "GenerateCommands": self.settings.generate_commands
        });
        if let Ok(json) = serde_json::to_string_pretty(&python_bot_settings) {
            let _ = std::fs::write(&settings_json_path, json);
            log_and_print!("[DEBUG] Saved {} for Python bot", SETTINGS_JSON_FILENAME);
        }
        // TODO: Update edit_settings_py to support new fields if needed
        let _ = edit_settings_py(
            &settings_py_path,
            "irc.chat.twitch.tv",
            6667,
            &channel_with_hash,
            &self.settings.nickname,
            &self.settings.authentication,
            &self.settings.denied_users,
            &self.settings.allowed_users,
            self.settings.cooldown,
            self.settings.key_length,
            self.settings.max_sentence_word_amount,
            self.settings.min_sentence_word_amount,
            DEFAULT_HELP_MESSAGE_TIMER,
            self.settings.automatic_generation_timer,
            true,
            true,
            DEFAULT_SENTENCE_SEPARATOR,
            true,
            &self.settings.generate_commands,
        );
        log_and_print!("[DEBUG] Saved {} for Python bot", SETTINGS_PY_FILENAME);
        self.needs_restart = true;
        // Send RESTART_BOT message to main GUI via TCP
        if let Ok(mut stream) = std::net::TcpStream::connect(TCP_ADDRESS) {
            let _ = stream.write_all(TCP_RESTART_MESSAGE);
        }
    }

    /// Reloads settings from all sources before saving
    /// This ensures we have the most up-to-date information when saving
    pub fn reload_and_save_settings(&mut self) {
        // First reload settings from all sources
        if let Err(e) = self.load_settings() {
            log_and_print!("[ERROR] Failed to reload settings before saving: {}", e);
        }
        
        // Then save the settings
        self.save_settings();
    }

    /// Updates only the first_launch field in the installer settings file
    /// This preserves the original installer settings format and prevents duplicates
    pub fn update_first_launch_only(&self, first_launch: bool) {
        let appdata_settings_path = Self::get_appdata_settings_path();
        
        if appdata_settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(&appdata_settings_path) {
                if let Ok(mut json) = serde_json::from_str::<serde_json::Value>(&content) {
                    // Check if first_launch field already exists
                    let field_exists = json.get("first_launch").is_some();
                    
                    // Update or add the first_launch field
                    json["first_launch"] = serde_json::Value::Bool(first_launch);
                    
                    if let Ok(updated_json) = serde_json::to_string_pretty(&json) {
                        if let Err(e) = std::fs::write(&appdata_settings_path, updated_json) {
                            log_and_print!("[ERROR] Failed to update first_launch in installer settings: {}", e);
                        } else {
                            if field_exists {
                                log_and_print!("[DEBUG] Updated existing first_launch field to {} in installer settings", first_launch);
                            } else {
                                log_and_print!("[DEBUG] Added first_launch field with value {} to installer settings", first_launch);
                            }
                        }
                    }
                } else {
                    log_and_print!("[ERROR] Failed to parse installer settings file as JSON");
                }
            } else {
                log_and_print!("[ERROR] Failed to read installer settings file");
            }
        } else {
            log_and_print!("[WARNING] Installer settings file does not exist, cannot update first_launch field");
        }
    }

    /// Commits any pending field changes before saving
    fn commit_pending_changes(&mut self) {
        let new_channel = if self.channel_input.is_empty() {
            String::new()
        } else {
            format!("#{}", self.channel_input.trim_start_matches('#'))
        };
        if self.last_committed_channel != new_channel {
            log_and_print!("[SETTINGS] Committing pending channel change: '{}' -> '{}'", self.last_committed_channel, new_channel);
            self.last_committed_channel = new_channel.clone();
            self.temp_settings.channel = new_channel;
        }
        if self.last_committed_nickname != self.temp_settings.nickname {
            log_and_print!("[SETTINGS] Committing pending nickname change: '{}' -> '{}'", self.last_committed_nickname, self.temp_settings.nickname);
            self.last_committed_nickname = self.temp_settings.nickname.clone();
        }
        if self.last_committed_auth != self.temp_settings.authentication {
            log_and_print!("[SETTINGS] Committing pending authentication change: '{}' -> '{}'", self.last_committed_auth, self.temp_settings.authentication);
            self.last_committed_auth = self.temp_settings.authentication.clone();
        }
        if self.last_committed_cooldown != self.temp_settings.cooldown {
            log_and_print!("[SETTINGS] Committing pending cooldown change: {} -> {}", self.last_committed_cooldown, self.temp_settings.cooldown);
            self.last_committed_cooldown = self.temp_settings.cooldown;
        }
        if self.last_committed_key_length != self.temp_settings.key_length {
            log_and_print!("[SETTINGS] Committing pending key length change: {} -> {}", self.last_committed_key_length, self.temp_settings.key_length);
            self.last_committed_key_length = self.temp_settings.key_length;
        }
        if self.last_committed_max_sent != self.temp_settings.max_sentence_word_amount {
            log_and_print!("[SETTINGS] Committing pending max sentence change: {} -> {}", self.last_committed_max_sent, self.temp_settings.max_sentence_word_amount);
            self.last_committed_max_sent = self.temp_settings.max_sentence_word_amount;
        }
        if self.last_committed_min_sent != self.temp_settings.min_sentence_word_amount {
            log_and_print!("[SETTINGS] Committing pending min sentence change: {} -> {}", self.last_committed_min_sent, self.temp_settings.min_sentence_word_amount);
            self.last_committed_min_sent = self.temp_settings.min_sentence_word_amount;
        }
        if self.last_committed_auto_gen != self.temp_settings.automatic_generation_timer {
            log_and_print!("[SETTINGS] Committing pending auto gen timer change: {} -> {}", self.last_committed_auto_gen, self.temp_settings.automatic_generation_timer);
            self.last_committed_auto_gen = self.temp_settings.automatic_generation_timer;
        }
    }

    /// Closes the settings window and logs the action
    fn close_window(&mut self, reason: &str) {
        self.is_open = false;
        log_and_print!("[GUI] Settings window closed ({} button clicked)", reason);
        log_and_print!("[GUI] {} button clicked in settings", reason);
    }

    /// Initializes the eye icon textures if not already loaded
    fn init_eye_icons(&mut self, ctx: &egui::Context) {
        if self.eye_on_texture.is_none() {
            self.eye_on_texture = load_eye_icon(ctx, "eye_on", include_bytes!("../../../resources/buttons/eyeOn.png"));
        }
        if self.eye_off_texture.is_none() {
            self.eye_off_texture = load_eye_icon(ctx, "eye_off", include_bytes!("../../../resources/buttons/eyeOff.png"));
        }
    }

    /// Checks and loads the first_launch field from the installer settings file
    fn check_first_launch_field(appdata_settings_path: &PathBuf) -> bool {
        if appdata_settings_path.exists() {
            if let Ok(content) = std::fs::read_to_string(appdata_settings_path) {
                if let Ok(json) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(first_launch_value) = json.get("first_launch") {
                        if let Some(first_launch_bool) = first_launch_value.as_bool() {
                            return first_launch_bool;
                        }
                    }
                }
            }
        }
        true // Default to true if file doesn't exist or field not found
    }

    /// Gets the appdata settings path
    fn get_appdata_settings_path() -> PathBuf {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\{}", appdata, INSTALLER_SETTINGS_FILENAME))
    }

    /// Migrates old installer settings format to new BotSettings format
    fn migrate_old_settings(&mut self, content: &str) -> bool {
        #[derive(serde::Deserialize)]
        struct OldInstallerSettings {
            oauth: Option<String>,
            main_channel_name: Option<String>,
            bot_channel_name: Option<String>,
            denied_users: Option<String>,
            cooldown: Option<String>,
            generate_command: Option<String>,
        }

        match serde_json::from_str::<OldInstallerSettings>(content) {
            Ok(old) => {
                log_and_print!("[DEBUG] Migrating old installer settings to BotSettings format");
                log_and_print!("[DEBUG] Old settings - oauth: {:?}, main_channel: {:?}, bot_channel: {:?}, denied_users: {:?}, cooldown: {:?}, generate_command: {:?}", 
                    old.oauth, old.main_channel_name, old.bot_channel_name, old.denied_users, old.cooldown, old.generate_command);
                
                let denied_users = old.denied_users
                    .as_deref()
                    .unwrap_or("")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();
                
                let generate_commands = old.generate_command
                    .as_deref()
                    .unwrap_or("")
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect::<Vec<_>>();
                
                let settings = BotSettings {
                    channel: format!("#{}", old.main_channel_name.as_deref().unwrap_or("<channel>").trim()),
                    nickname: old.bot_channel_name.unwrap_or_else(|| "<name>".to_string()),
                    authentication: old.oauth.unwrap_or_else(|| "oauth:<auth>".to_string()),
                    denied_users,
                    allowed_users: vec![],
                    cooldown: old.cooldown.and_then(|c| c.parse().ok()).unwrap_or(0),
                    key_length: DEFAULT_KEY_LENGTH,
                    max_sentence_word_amount: DEFAULT_MAX_SENTENCE_WORDS,
                    min_sentence_word_amount: DEFAULT_MIN_SENTENCE_WORDS,
                    automatic_generation_timer: DEFAULT_AUTO_GEN_TIMER,
                    generate_commands,
                    sound_enabled: true,
                    start_minimized_to_tray: false,
                    exit_when_monitored_app_closes: false,
                    randomized_generation_timer_enabled: false,
                    randomized_generation_timer_min: DEFAULT_RANDOMIZED_TIMER_MIN,
                    randomized_generation_timer_max: DEFAULT_RANDOMIZED_TIMER_MAX,
                    first_launch: false, // Don't show popup on migration
                };
                
                self.settings = settings.clone();
                self.temp_settings = settings.clone();
                
                let loaded_channel = settings.channel.trim_start_matches('#');
                self.channel_input = if loaded_channel.is_empty() || loaded_channel == "<channel>" {
                    String::new()
                } else {
                    loaded_channel.to_string()
                };
                
                log_and_print!("[DEBUG] Migrated settings from old installer format to BotSettings format.");
                log_and_print!("[DEBUG] Converted settings - channel: {}, nickname: {}, denied_users: {:?}, cooldown: {}, generate_commands: {:?}", 
                    settings.channel, settings.nickname, settings.denied_users, settings.cooldown, settings.generate_commands);
                true
            }
            Err(e2) => {
                println!("[DEBUG] Failed to parse as old installer settings: {}", e2);
                log_and_print!("[DEBUG] Failed to parse as old installer settings: {}", e2);
                false
            }
        }
    }

    /// Renders the app settings section
    fn render_app_settings(&mut self, ui: &mut egui::Ui) {
        ui.add_space(UI_SPACING_SMALL);
        ui.label(egui::RichText::new("App Settings").size(18.0).strong());
        ui.add_space(UI_SPACING_SMALL);
        ui.separator();
        
        ui.horizontal(|ui| {
            let prev = self.temp_settings.sound_enabled;
            if ui.checkbox(&mut self.temp_settings.sound_enabled, "Enable sound").changed() {
                log_and_print!("[SETTINGS] Changed: Enable sound: {} -> {}", prev, self.temp_settings.sound_enabled);
            }
        });
        
        ui.horizontal(|ui| {
            let prev = self.temp_settings.start_minimized_to_tray;
            if ui.checkbox(&mut self.temp_settings.start_minimized_to_tray, "Start minimized to tray").changed() {
                log_and_print!("[SETTINGS] Changed: Start minimized to tray: {} -> {}", prev, self.temp_settings.start_minimized_to_tray);
            }
        });
        
        ui.horizontal(|ui| {
            let prev = self.temp_settings.exit_when_monitored_app_closes;
            if ui.checkbox(&mut self.temp_settings.exit_when_monitored_app_closes, "Automatically exit when OBS or Streamlabs close").changed() {
                log_and_print!("[SETTINGS] Changed: Automatically exit when OBS or Streamlabs close: {} -> {}", prev, self.temp_settings.exit_when_monitored_app_closes);
            }
        });
    }
}

/// Loads the settings window icon (settings_cog.ico) for the window bar.
pub fn load_settings_icon() -> Option<egui::IconData> {
    if let Ok(image) = image::load_from_memory(include_bytes!("../../../resources/icon/settings_cog.ico")) {
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

/// Loads an eye icon from PNG data and returns an egui texture
fn load_eye_icon(ctx: &egui::Context, name: &str, png_data: &[u8]) -> Option<egui::TextureHandle> {
    if let Ok(image) = image::load_from_memory(png_data) {
        let rgba = image.to_rgba8();
        let size = [rgba.width() as usize, rgba.height() as usize];
        let color_image = egui::ColorImage::from_rgba_unmultiplied(size, rgba.as_raw());
        Some(ctx.load_texture(name, color_image, egui::TextureOptions::default()))
    } else {
        None
    }
}

/// Launches the settings window as a standalone app (used by --settings-window argument).
pub fn run_settings_window() {
    use eframe::egui;
    use egui::ViewportBuilder;
    use crate::gui::setup_fonts_and_theme;
    let mut dialog = SettingsDialog::new();
    let _ = dialog.load_settings();
    let center_pos = crate::gui::calculate_window_position(SETTINGS_WINDOW_SIZE);
    let icon_data = load_settings_icon();
    let mut viewport_builder = ViewportBuilder::default()
        .with_inner_size(SETTINGS_WINDOW_SIZE)
        .with_min_inner_size(SETTINGS_MIN_WINDOW_SIZE)
        .with_position(center_pos);
    if let Some(icon) = icon_data {
        viewport_builder = viewport_builder.with_icon(icon);
    }
    let native_options = eframe::NativeOptions {
        viewport: viewport_builder,
        ..Default::default()
    };
    eframe::run_native(
        "Yap Bot Settings",
        native_options,
        Box::new(|cc| {
            setup_fonts_and_theme(&cc.egui_ctx);
            struct SettingsWindowApp {
                dialog: SettingsDialog,
            }
            impl eframe::App for SettingsWindowApp {
                fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
                    self.dialog.show(ctx);
                    if !self.dialog.is_open {
                        std::process::exit(0);
                    }
                }
                fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
                    // Commit any pending changes and save settings when window is closed with X button
                    self.dialog.commit_pending_changes();
                    self.dialog.save_settings();
                    log_and_print!("[GUI] Settings window closed (x button in windows) - settings saved");
                    crate::log_util::shutdown_logger();
                }
            }
            Ok(Box::new(SettingsWindowApp { dialog }))
        }),
    ).unwrap();
}

// Helper to render a labeled input row with a plus button and associated bubble list for Vec<String>.
fn input_with_bubbles(
    ui: &mut egui::Ui,
    label: &str,
    input: &mut String,
    items: &mut Vec<String>,
    font_id: egui::FontId,
    bubble_height: f32,
    id_prefix: &str,
    left_spacing: f32, // new param
    tooltip_text: &str,
) {
    egui::Grid::new(format!("{}_grid", id_prefix)).num_columns(2).spacing([30.0, 8.0]).show(ui, |ui| {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.add_space(left_spacing); // independently tweakable
            let input_id = ui.make_persistent_id(format!("{}_input", id_prefix));
            let input_widget = ui.add(egui::TextEdit::singleline(input).desired_width(250.0).id(input_id)).on_hover_text(tooltip_text);
            let plus_btn = ui.add(egui::Button::new("+").min_size([20.0, 20.0].into()));
            let mut should_refocus = false;
            if (plus_btn.clicked() || (input_widget.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && !input.trim().is_empty() {
                let new_item = input.trim().to_string();
                if !items.iter().any(|u| u.eq_ignore_ascii_case(&new_item)) {
                    items.push(new_item);
                }
                input.clear();
                should_refocus = true;
            }
            if should_refocus {
                ui.memory_mut(|mem| mem.request_focus(input_id));
            }
        });
        ui.end_row();
    });
    ui.add_space(2.0);
    egui::ScrollArea::vertical()
        .id_source(format!("{}_scroll", id_prefix))
        .max_height(UI_SCROLL_HEIGHT)
        .show(ui, |ui| {
            if let Some(idx) = bubble_list_ui(
                ui,
                items,
                None,
                bubble_height,
                font_id,
                egui::Color32::from_rgb(224, 224, 224),
                egui::Color32::BLACK,
            ) {
                items.remove(idx);
            }
        });
    ui.add_space(5.0);
}

impl SettingsDialog {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }
        
        // Initialize eye icons if needed
        self.init_eye_icons(ctx);
        
        // Check if the window is being closed (e.g., by clicking the X button)
        let window_closing = ctx.input(|i| i.viewport().close_requested());
        
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        let mut reset_clicked = false;
        let _prev_settings = self.temp_settings.clone();
        let _prev_denied = self.temp_settings.denied_users.clone();
        let _prev_allowed = self.temp_settings.allowed_users.clone();
        let _prev_commands = self.temp_settings.generate_commands.clone();
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                self.render_app_settings(ui);
                ui.add_space(UI_SPACING_LARGE);
                ui.label(egui::RichText::new("Twitch Account Info").size(18.0).strong());
                ui.add_space(UI_SPACING_SMALL);
                ui.separator();
                egui::Grid::new("twitch_account_info_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Channel:");
                        let channel_id = ui.make_persistent_id("channel_input");
                        let channel_edit = ui.add(egui::TextEdit::singleline(&mut self.channel_input).desired_width(UI_TEXT_WIDTH).id(channel_id));
                        let lost_focus = channel_edit.lost_focus() && !channel_edit.has_focus();
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        // Always save with a single '#'
                        let new_channel = if self.channel_input.is_empty() {
                            String::new()
                        } else {
                            format!("#{}", self.channel_input.trim_start_matches('#'))
                        };
                        if (lost_focus || enter_pressed) && self.last_committed_channel != new_channel {
                            log_and_print!("[SETTINGS] Changed: Channel name: '{}' -> '{}'", self.last_committed_channel, new_channel);
                            self.last_committed_channel = new_channel.clone();
                            self.temp_settings.channel = new_channel;
                        }
                        ui.end_row();
                        ui.label("Bot Account:");
                        let nick_id = ui.make_persistent_id("nickname_input");
                        let nick_edit = ui.add(egui::TextEdit::singleline(&mut self.temp_settings.nickname).desired_width(UI_TEXT_WIDTH).id(nick_id));
                        let lost_focus = nick_edit.lost_focus() && !nick_edit.has_focus();
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        if (lost_focus || enter_pressed) && self.last_committed_nickname != self.temp_settings.nickname {
                            log_and_print!("[SETTINGS] Changed: Bot account: '{}' -> '{}'", self.last_committed_nickname, self.temp_settings.nickname);
                            self.last_committed_nickname = self.temp_settings.nickname.clone();
                        }
                        ui.end_row();
                        ui.label("Access Token:");
                        let auth_id = ui.make_persistent_id("auth_input");
                        
                        ui.horizontal(|ui| {
                            if self.show_access_token {
                                // Show the actual token and allow editing
                                let auth_edit = ui.add(egui::TextEdit::singleline(&mut self.temp_settings.authentication).desired_width(UI_TEXT_WIDTH).id(auth_id));
                                let lost_focus = auth_edit.lost_focus() && !auth_edit.has_focus();
                                let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                                if (lost_focus || enter_pressed) && self.last_committed_auth != self.temp_settings.authentication {
                                    log_and_print!("[SETTINGS] Changed: Authentication: '{}' -> '{}'", self.last_committed_auth, self.temp_settings.authentication);
                                    self.last_committed_auth = self.temp_settings.authentication.clone();
                                }
                            } else {
                                // Show obscured text that's not editable
                                let mut obscured_text = "•".repeat(self.temp_settings.authentication.len().max(20));
                                let _auth_edit = ui.add(egui::TextEdit::singleline(&mut obscured_text).desired_width(UI_TEXT_WIDTH).id(auth_id).interactive(false));
                            }
                            
                            // Toggle button to show/hide token with strict 20x20 sizing
                            let button_response = ui.allocate_ui([UI_ICON_SIZE, UI_ICON_SIZE].into(), |ui| {
                                let button = egui::Button::new("").min_size([UI_ICON_SIZE, UI_ICON_SIZE].into());
                                let response = ui.add(button);
                                
                                // Draw the appropriate eye icon centered in the button
                                if let Some(ref texture) = if self.show_access_token { &self.eye_off_texture } else { &self.eye_on_texture } {
                                    let texture_size = texture.size_vec2();
                                    let button_size = response.rect.size();
                                    
                                    // Calculate scale to fit the icon within the button while maintaining aspect ratio
                                    let scale = (button_size.x / texture_size.x).min(button_size.y / texture_size.y);
                                    let scaled_size = texture_size * scale;
                                    let offset = (button_size - scaled_size) / 2.0;
                                    
                                    let image_rect = egui::Rect::from_min_size(
                                        response.rect.min + offset,
                                        scaled_size
                                    );
                                    
                                    ui.painter().image(
                                        texture.id(), 
                                        image_rect, 
                                        egui::Rect::from_min_max(egui::pos2(0.0, 0.0), egui::pos2(1.0, 1.0)), 
                                        egui::Color32::WHITE
                                    );
                                } else {
                                    // Fallback to text if icon fails to load
                                    let button_text = if self.show_access_token { "Hide" } else { "Show" };
                                    ui.painter().text(
                                        response.rect.center(),
                                        egui::Align2::CENTER_CENTER,
                                        button_text,
                                        egui::FontId::default(),
                                        egui::Color32::WHITE
                                    );
                                }
                                
                                response
                            });
                            
                            if button_response.inner.clicked() {
                                self.show_access_token = !self.show_access_token;
                            }
                            button_response.inner.on_hover_text(if self.show_access_token { "Hide token" } else { "Show token" });
                        });
                }); // end of egui::Grid
                // Place token buttons below the grid
                ui.add_space(UI_SPACING_MEDIUM);
                ui.horizontal(|ui| {
                    if ui.add_sized([UI_BUTTON_WIDTH, UI_BUTTON_HEIGHT], egui::Button::new("Generate New Bot Token")).on_hover_text("Login to your bot account on twitch before generating a new token").clicked() {
                        let _ = open::that(TOKEN_GENERATOR_URL);
                        self.show_paste_token_button = true;
                    }
                    if self.show_token_success {
                        ui.colored_label(egui::Color32::from_rgb(80, 250, 123), "Success!");
                    }
                });
                if self.show_paste_token_button {
                    ui.add_space(4.0);
                    if ui.add_sized([UI_BUTTON_WIDTH, UI_BUTTON_HEIGHT], egui::Button::new("Paste Access Token")).on_hover_text("Paste a the access token from your clipboard").clicked() {
                        if let Ok(token) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                            let token = token.trim();
                            if token.len() == TWITCH_TOKEN_LENGTH && token.chars().all(|c| c.is_ascii_alphanumeric()) {
                                let new_auth = format!("oauth:{}", token);
                                self.temp_settings.authentication = new_auth.clone();
                                self.last_committed_auth = new_auth.clone();
                                // Save to YapBotInstallerSettings.json
                                if let Ok(appdata) = std::env::var("APPDATA") {
                                    let path = std::path::Path::new(&appdata).join("YapBot").join("TwitchMarkovChain").join("YapBotInstallerSettings.json");
                                    if let Ok(json) = std::fs::read_to_string(&path) {
                                        if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&json) {
                                            v["Authentication"] = serde_json::Value::String(new_auth);
                                            if let Ok(new_json) = serde_json::to_string_pretty(&v) {
                                                let _ = std::fs::write(&path, new_json);
                                            }
                                        }
                                    }
                                }
                                self.token_paste_warning = None;
                                self.show_paste_client_id_button = true;
                                self.pasted_client_id = None;
                                self.client_id_warning = None;
                                self.show_token_success = false;
                            } else {
                                self.token_paste_warning = Some("Clipboard does not contain a valid 30-character Twitch token.".to_string());
                                self.show_paste_client_id_button = false;
                                self.show_token_success = false;
                            }
                        } else {
                            self.token_paste_warning = Some("Could not read clipboard.".to_string());
                            self.show_paste_client_id_button = false;
                            self.show_token_success = false;
                        }
                    }
                    if let Some(ref warn) = self.token_paste_warning {
                        ui.colored_label(egui::Color32::RED, warn);
                    }
                    if self.show_paste_client_id_button {
                        ui.add_space(4.0);
                        if ui.add_sized([UI_BUTTON_WIDTH, UI_BUTTON_HEIGHT], egui::Button::new("Paste Client ID")).on_hover_text("Paste the client ID from your clipboard").clicked() {
                            if let Ok(client_id) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                                let client_id = client_id.trim();
                                if !client_id.is_empty() {
                                    // Call Twitch API to get bot account name
                                    let token = self.temp_settings.authentication.trim_start_matches("oauth:");
                                    let client_id_str = client_id;
                                    let user_name = get_twitch_bot_username(token, client_id_str);
                                    if let Some(name) = user_name {
                                        self.temp_settings.nickname = name.clone();
                                        self.last_committed_nickname = name.clone();
                                        // Save to YapBotInstallerSettings.json
                                        if let Ok(appdata) = std::env::var("APPDATA") {
                                            let path = std::path::Path::new(&appdata).join("YapBot").join("TwitchMarkovChain").join("YapBotInstallerSettings.json");
                                            if let Ok(json) = std::fs::read_to_string(&path) {
                                                if let Ok(mut v) = serde_json::from_str::<serde_json::Value>(&json) {
                                                    v["Nickname"] = serde_json::Value::String(name.clone());
                                                    if let Ok(new_json) = serde_json::to_string_pretty(&v) {
                                                        let _ = std::fs::write(&path, new_json);
                                                    }
                                                }
                                            }
                                        }
                                        self.show_paste_token_button = false;
                                        self.show_paste_client_id_button = false;
                                        self.show_token_success = true;
                                        self.client_id_warning = None;
                                    } else {
                                        self.client_id_warning = Some("Failed to fetch bot account name from Twitch. Check your token and client ID.".to_string());
                                    }
                                } else {
                                    self.client_id_warning = Some("Clipboard does not contain a valid client ID.".to_string());
                                }
                            } else {
                                self.client_id_warning = Some("Could not read clipboard.".to_string());
                            }
                        }
                        if let Some(ref warn) = self.client_id_warning {
                            ui.colored_label(egui::Color32::RED, warn);
                        }
                    }
                }
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Bot Settings").size(18.0).strong());
                ui.add_space(2.0);
                ui.separator();
                egui::Grid::new("bot_settings_grid")
                    .num_columns(4)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Cooldown (seconds):");
                        let cooldown_edit = ui.push_id("cooldown_drag", |ui| ui.add(egui::DragValue::new(&mut self.temp_settings.cooldown).speed(1))).inner.on_hover_text("Minimum time (in seconds) between allowed !generate commands from regular users");
                        let lost_focus = cooldown_edit.lost_focus() && !cooldown_edit.has_focus();
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let drag_released = cooldown_edit.drag_stopped();
                        if (lost_focus || enter_pressed || drag_released) && self.last_committed_cooldown != self.temp_settings.cooldown {
                            log_and_print!("[SETTINGS] Changed: Cooldown: {} -> {}", self.last_committed_cooldown, self.temp_settings.cooldown);
                            self.last_committed_cooldown = self.temp_settings.cooldown;
                        }
                        ui.label("Max Sentence Word Amount:");
                        let max_sent_edit = ui.push_id("max_sent_drag", |ui| ui.add(egui::DragValue::new(&mut self.temp_settings.max_sentence_word_amount).speed(1))).inner.on_hover_text("The maximum number of words in a generated message");
                        let lost_focus = max_sent_edit.lost_focus() && !max_sent_edit.has_focus();
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let drag_released = max_sent_edit.drag_stopped();
                        if (lost_focus || enter_pressed || drag_released) && self.last_committed_max_sent != self.temp_settings.max_sentence_word_amount {
                            log_and_print!("[SETTINGS] Changed: Max sentence word amount: {} -> {}", self.last_committed_max_sent, self.temp_settings.max_sentence_word_amount);
                            self.last_committed_max_sent = self.temp_settings.max_sentence_word_amount;
                        }
                        ui.end_row();
                        ui.label("Key Length:");
                        let key_length_edit = ui.push_id("key_length_drag", |ui| ui.add(egui::DragValue::new(&mut self.temp_settings.key_length).speed(1))).inner.on_hover_text("How many words are used as context for generating the next word. Higher values = more coherent, but less random");
                        let lost_focus = key_length_edit.lost_focus() && !key_length_edit.has_focus();
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let drag_released = key_length_edit.drag_stopped();
                        if (lost_focus || enter_pressed || drag_released) && self.last_committed_key_length != self.temp_settings.key_length {
                            log_and_print!("[SETTINGS] Changed: Key length: {} -> {}", self.last_committed_key_length, self.temp_settings.key_length);
                            self.last_committed_key_length = self.temp_settings.key_length;
                        }
                        ui.label("Min Sentence Word Amount:");
                        let min_sent_edit = ui.push_id("min_sent_drag", |ui| ui.add(egui::DragValue::new(&mut self.temp_settings.min_sentence_word_amount).speed(1))).inner.on_hover_text("The minimum number of words in a generated message. Set to -1 for no minimum");
                        let lost_focus = min_sent_edit.lost_focus() && !min_sent_edit.has_focus();
                        let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                        let drag_released = min_sent_edit.drag_stopped();
                        if (lost_focus || enter_pressed || drag_released) && self.last_committed_min_sent != self.temp_settings.min_sentence_word_amount {
                            log_and_print!("[SETTINGS] Changed: Min sentence word amount: {} -> {}", self.last_committed_min_sent, self.temp_settings.min_sentence_word_amount);
                            self.last_committed_min_sent = self.temp_settings.min_sentence_word_amount;
                        }
                        ui.end_row();
                        // Automatic Generation Timer 
                        ui.label("Automatic Generation Timer (seconds):");
                        let mut auto_gen_enabled = self.temp_settings.automatic_generation_timer >= 0;
                        let checkbox_response = ui.checkbox(&mut auto_gen_enabled, "Enable").on_hover_text("Generates a message after the given number of seconds");
                        // If enabling auto, disable randomized if needed
                        if checkbox_response.changed() {
                            if auto_gen_enabled {
                                // Disable randomized if it was enabled
                                if self.randomized_timer_enabled.get_or_insert(false).clone() {
                                    *self.randomized_timer_enabled.get_or_insert(false) = false;
                                    // Optionally reset min/max or leave as is
                                }
                                self.temp_settings.automatic_generation_timer = 5;
                                log_and_print!("[SETTINGS] Enabled automatic generation timer, set to 5");
                            } else {
                                self.temp_settings.automatic_generation_timer = -1;
                                log_and_print!("[SETTINGS] Disabled automatic generation timer");
                            }
                        }
                        if auto_gen_enabled {
                            let timer = &mut self.temp_settings.automatic_generation_timer;
                            if *timer < MIN_AUTO_GEN_TIMER {
                                *timer = MIN_AUTO_GEN_TIMER;
                            }
                            let auto_gen_edit = ui.push_id("auto_gen_drag", |ui| {
                                ui.add(egui::DragValue::new(timer).speed(1).range(MIN_AUTO_GEN_TIMER..=i32::MAX))
                            }).inner;
                            let lost_focus = auto_gen_edit.lost_focus() && !auto_gen_edit.has_focus();
                            let enter_pressed = ui.input(|i| i.key_pressed(egui::Key::Enter));
                            let drag_released = auto_gen_edit.drag_stopped();
                            if (lost_focus || enter_pressed || drag_released) && self.last_committed_auto_gen != *timer {
                                log_and_print!("[SETTINGS] Changed: Automatic generation timer: {} -> {}", self.last_committed_auto_gen, *timer);
                                self.last_committed_auto_gen = *timer;
                            }
                        } else {
                            ui.label(""); // empty cell to preserve grid alignment
                        }
                        ui.end_row();
                        // Randomized Generation Timer
                        ui.label("Randomized Generation Timer (seconds):");
                        // Always use temp_settings for UI state
                        if self.randomized_timer_enabled.is_none() {
                            self.randomized_timer_enabled = Some(self.temp_settings.randomized_generation_timer_enabled);
                        }
                        if self.randomized_timer_min.is_none() {
                            self.randomized_timer_min = Some(self.temp_settings.randomized_generation_timer_min);
                        }
                        if self.randomized_timer_max.is_none() {
                            self.randomized_timer_max = Some(self.temp_settings.randomized_generation_timer_max);
                        }
                        let enabled = self.randomized_timer_enabled.get_or_insert(self.temp_settings.randomized_generation_timer_enabled);
                        let min_val = self.randomized_timer_min.get_or_insert(self.temp_settings.randomized_generation_timer_min);
                        let max_val = self.randomized_timer_max.get_or_insert(self.temp_settings.randomized_generation_timer_max);
                        let rand_checkbox_response = ui.checkbox(enabled, "Enable").on_hover_text("Generates a message after randomly picking a number of seconds between your minimum & maximum. A new number is picked after each message");
                        // If enabling randomized, disable auto if needed
                        if rand_checkbox_response.changed() {
                            if *enabled {
                                // Disable auto if it was enabled
                                if auto_gen_enabled {
                                    self.temp_settings.automatic_generation_timer = -1;
                                    log_and_print!("[SETTINGS] Disabled automatic generation timer (randomized enabled)");
                                }
                            }
                        }
                        if *enabled {
                            if *min_val < MIN_RANDOMIZED_TIMER { *min_val = MIN_RANDOMIZED_TIMER; }
                            ui.horizontal(|ui| {
                                ui.push_id("rand_gen_min", |ui| {
                                    ui.add(egui::DragValue::new(min_val).speed(1).range(MIN_RANDOMIZED_TIMER..=i32::MAX))
                                });
                                ui.label("-");
                                ui.push_id("rand_gen_max", |ui| {
                                    ui.add(egui::DragValue::new(max_val).speed(1).range(*min_val..=i32::MAX))
                                });
                            });
                        } else {
                            ui.label(""); // empty cell for grid alignment
                        }
                        ui.end_row();
                    });
                ui.add_space(5.0);

                // User Lists and commands
                let bubble_height = UI_BUBBLE_HEIGHT;
                let font_id = egui::FontId::new(UI_FONT_SIZE, egui::FontFamily::Proportional);
                let font_id2 = font_id.clone();
                let font_id3 = font_id.clone();
                // Denied Users
                let before = self.temp_settings.denied_users.clone();
                input_with_bubbles(ui, "Denied Users:", &mut self.denied_input, &mut self.temp_settings.denied_users, font_id.clone(), bubble_height, "denied_users", self.denied_left_spacing, "Users in this list cannot use the bot's commands");
                if self.temp_settings.denied_users != before {
                    let added: Vec<_> = self.temp_settings.denied_users.iter().filter(|u| !before.contains(u)).cloned().collect();
                    let removed: Vec<_> = before.iter().filter(|u| !self.temp_settings.denied_users.contains(u)).cloned().collect();
                    for a in added {
                        log_and_print!("[SETTINGS] Added to denied users: {}", a);
                    }
                    for r in removed {
                        log_and_print!("[SETTINGS] Removed from denied users: {}", r);
                    }
                }
                // Allowed Users
                let before = self.temp_settings.allowed_users.clone();
                input_with_bubbles(ui, "Allowed Users:", &mut self.allowed_input, &mut self.temp_settings.allowed_users, font_id2.clone(), bubble_height, "allowed_users", self.allowed_left_spacing, "Users in this list can always use the bot's commands, even if denied or on cooldown");
                if self.temp_settings.allowed_users != before {
                    let added: Vec<_> = self.temp_settings.allowed_users.iter().filter(|u| !before.contains(u)).cloned().collect();
                    let removed: Vec<_> = before.iter().filter(|u| !self.temp_settings.allowed_users.contains(u)).cloned().collect();
                    for a in added {
                        log_and_print!("[SETTINGS] Added to allowed users: {}", a);
                    }
                    for r in removed {
                        log_and_print!("[SETTINGS] Removed from allowed users: {}", r);
                    }
                }
                // Commands
                let before = self.temp_settings.generate_commands.clone();
                // Ensure all commands in the bubble list are shown with a single '!' prefix
                self.temp_settings.generate_commands = self.temp_settings.generate_commands.iter()
                    .map(|cmd| {
                        let trimmed = cmd.trim_start_matches('!');
                        format!("!{}", trimmed)
                    })
                    .collect();
                input_with_bubbles(ui, "Commands:", &mut self.generate_command_input, &mut self.temp_settings.generate_commands, font_id3.clone(), bubble_height, "commands", self.commands_left_spacing, "Chat commands that will trigger the bot to generate a message");
                if self.temp_settings.generate_commands != before {
                    let added: Vec<_> = self.temp_settings.generate_commands.iter().filter(|u| !before.contains(u)).cloned().collect();
                    let removed: Vec<_> = before.iter().filter(|u| !self.temp_settings.generate_commands.contains(u)).cloned().collect();
                    for a in added {
                        log_and_print!("[SETTINGS] Added to commands: {}", a);
                    }
                    for r in removed {
                        log_and_print!("[SETTINGS] Removed from commands: {}", r);
                    }
                }
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save_clicked = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.temp_settings = self.settings.clone();
                        cancel_clicked = true;
                    }
                    if ui.button("Reset to Defaults").clicked() {
                        self.temp_settings = BotSettings::default();
                        reset_clicked = true;
                    }
                });
            });
        });
        if cancel_clicked {
            self.close_window("Cancel");
        }
        if save_clicked || window_closing {
            // If window is closing, commit any pending field changes first
            if window_closing {
                self.commit_pending_changes();
            }
            self.save_settings();
            self.close_window(if window_closing { "X button" } else { "Save" });
        }
        if reset_clicked {
            log_and_print!("[GUI] Reset to Defaults button clicked in settings");
        }
    }
}

fn get_twitch_bot_username(token: &str, client_id: &str) -> Option<String> {
    let url = TWITCH_API_URL;
    let client = reqwest::blocking::Client::new();
    let resp = client
        .get(url)
        .bearer_auth(token)
        .header("Client-Id", client_id)
        .send()
        .ok()?;
    let json: serde_json::Value = resp.json().ok()?;
    if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
        if let Some(user) = arr.get(0) {
            if let Some(login) = user.get("login").and_then(|l| l.as_str()) {
                return Some(login.to_string());
            }
        }
    }
    None
} 