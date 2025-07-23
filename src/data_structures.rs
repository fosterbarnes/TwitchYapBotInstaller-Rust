//! Data structures and types for the YapBot Installer
//! 
//! This module contains the core data structures including application state
//! and shared data types used throughout the application.

use std::sync::{Arc, Mutex};
use std::sync::mpsc::{Sender, Receiver, channel};
use serde::{Serialize, Deserialize};
use include_dir::{include_dir, Dir};
use std::path::Path;
use once_cell::sync::Lazy;
pub const TWITCH_MARKOVCHAIN_DIR: Dir = include_dir!("$CARGO_MANIFEST_DIR/TwitchMarkovChain");

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct YapBotInstallerSettings {
    pub oauth: String,
    pub main_channel_name: String,
    pub bot_channel_name: String,
    pub denied_users: String,
    pub cooldown: String,
    pub generate_command: String,
    pub step4_db_prompt_answered_yes: Option<bool>, // None = not answered, Some(true) = Yes, Some(false) = No
    pub twitch_token_client_id: Option<String>,
}

impl YapBotInstallerSettings {
    pub fn is_complete(&self) -> bool {
        !self.oauth.is_empty()
            && !self.main_channel_name.is_empty()
            && !self.bot_channel_name.is_empty()
            && !self.denied_users.is_empty()
            && !self.cooldown.is_empty()
            && !self.generate_command.is_empty()
    }
}

/// Main application state for the YapBot installer
pub struct YapBotInstaller {
    // Application state
    pub status: String,
    
    // Python installation state
    pub python_installed: bool,
    pub python_version: Option<String>,
    pub installing_python: bool,
    pub python_install_result: Arc<Mutex<Option<Result<(), String>>>>,
    
    // Dependencies installation state
    pub installing_dependencies: bool,
    pub dependencies_installed: bool,
    pub dependencies_install_result: Arc<Mutex<Option<Result<(), String>>>>,
    
    // Step 2: Bot authentication state
    pub step2_visible: bool,
    pub step2_start_time: Option<f32>,
    pub bot_oauth_token: Option<String>,
    
    // UI state
    pub last_refresh_time: std::time::Instant,
    pub refresh_interval: std::time::Duration,
    pub show_paste_token_btn: bool,
    pub last_token_btn_width: f32,
    pub step3_visible: bool,
    pub step1_open: bool,
    pub step2_open: bool,
    pub step3_open: bool,
    pub step1_just_changed: bool,
    pub step2_just_changed: bool,
    pub step3_just_changed: bool,
    pub advance_step: bool,
    pub pending_step3: bool,
    pub token_just_pasted: bool,
    // Step 3: Configuration fields
    pub main_channel_name: String,
    pub bot_channel_name: String,
    pub denied_users: String,
    pub denied_users_list: Vec<String>,
    pub temp_denied_user_input: String,
    pub cooldown: String,
    pub generate_command: String,
    pub temp_generate_command_input: String,
    pub generate_command_list: Vec<String>,
    // For Step 3 focus/blur logic
    pub prev_main_channel_name: String,
    pub prev_bot_channel_name: String,
    // Step 4 state
    pub step4_visible: bool,
    pub step4_open: bool,
    pub step4_just_changed: bool,
    // Step 4 progress
    pub step4_action_index: usize,
    pub step4_action_progress: f32,
    pub step4_action_running: bool,
    pub step4_action_tx: Sender<usize>,
    pub step4_action_rx: Receiver<usize>,
    // Step 4 DB migration prompt
    pub step4_db_prompt_visible: bool,
    pub step4_db_prompt_answered: bool,
    pub step4_db_prompt_answered_yes: Option<bool>,
    pub step4_db_prompt_running: bool,
    pub step4_db_migrating_file: Option<String>,
    pub step4_db_migrated_file: Option<String>,
    pub step4_db_file_tx: Option<Sender<(String, String)>>,
    pub step4_db_file_rx: Option<Receiver<(String, String)>>,
    pub step4_db_copied_files: Vec<(String, String)>,
    pub loaded_settings: Option<YapBotInstallerSettings>,
    pub step4_skipped_to_from_settings: bool,
    // Step 5 state
    pub step5_visible: bool,
    pub step5_open: bool,
    pub step5_just_changed: bool,
    // Version check state
    pub latest_version: Option<String>,
    pub version_check_error: Option<String>,
    pub version_checked: bool,
    pub twitch_token_username_warning: Option<String>,
    pub twitch_token_checked_username: Option<String>,
    pub twitch_token_client_id: Option<String>,
}

impl Default for YapBotInstaller {
    fn default() -> Self {
        // Check Python on startup like rustitles does
        use crate::python_manager::PythonManager;
        let python_version = PythonManager::get_version();
        let python_installed = python_version.is_some();
        
        let (tx, rx) = channel();
        let mut app = Self {
            status: "Ready".to_string(),
            python_installed,
            python_version,
            installing_python: false,
            python_install_result: Arc::new(Mutex::new(None)),
            installing_dependencies: false,
            dependencies_installed: false,
            dependencies_install_result: Arc::new(Mutex::new(None)),
            step2_visible: false,
            step2_start_time: None,
            bot_oauth_token: None,
            last_refresh_time: std::time::Instant::now(),
            refresh_interval: std::time::Duration::from_secs(1),
            show_paste_token_btn: false,
            last_token_btn_width: 0.0,
            step3_visible: false,
            step1_open: true,
            step2_open: false,
            step3_open: false,
            step1_just_changed: false,
            step2_just_changed: false,
            step3_just_changed: false,
            advance_step: false,
            pending_step3: false,
            token_just_pasted: false,
            // Step 3 config defaults
            main_channel_name: String::new(),
            bot_channel_name: String::new(),
            denied_users: "StreamElements, Nightbot, Moobot, Marbiebot, LumiaStream".to_string(),
            denied_users_list: vec![
                String::from("StreamElements"),
                String::from("Nightbot"),
                String::from("Moobot"),
                String::from("Marbiebot"),
                String::from("LumiaStream"),
            ],
            temp_denied_user_input: String::new(),
            cooldown: "0".to_string(),
            generate_command: "!yap".to_string(),
            temp_generate_command_input: String::new(),
            generate_command_list: vec!["!yap".to_string()],
            // For Step 3 focus/blur logic
            prev_main_channel_name: String::new(),
            prev_bot_channel_name: String::new(),
            // Step 4 state
            step4_visible: false,
            step4_open: false,
            step4_just_changed: false,
            // Step 4 progress
            step4_action_index: 0,
            step4_action_progress: 0.0,
            step4_action_running: false,
            // Step 4 threading
            step4_action_tx: tx,
            step4_action_rx: rx,
            // Step 4 DB migration prompt
            step4_db_prompt_visible: false,
            step4_db_prompt_answered: false,
            step4_db_prompt_answered_yes: None,
            step4_db_prompt_running: false,
            step4_db_migrating_file: None,
            step4_db_migrated_file: None,
            step4_db_file_tx: None,
            step4_db_file_rx: None,
            step4_db_copied_files: Vec::new(),
            loaded_settings: None,
            step4_skipped_to_from_settings: false,
            // Step 5 state
            step5_visible: false,
            step5_open: false,
            step5_just_changed: false,
            // Version check state
            latest_version: None,
            version_check_error: None,
            version_checked: false,
            twitch_token_username_warning: None,
            twitch_token_checked_username: None,
            twitch_token_client_id: None,
        };
        
        // If Python is already installed, automatically start dependencies installation
        if python_installed {
            println!("Python already installed, starting dependencies installation automatically");
            app.start_dependencies_install();
        }
        
        app
    }
}

impl YapBotInstaller {
    /// Show Step 2 after dependencies are installed
    pub fn show_step2(&mut self) {
        self.step2_visible = true;
        println!("DEBUG: Step 2 is now visible");
        self.step1_open = false;
        self.step1_just_changed = true;
        self.step2_open = true;
        self.step2_just_changed = true;
    }
    
    /// Open the Twitch token generator in the default browser
    pub fn open_token_generator(&self) -> Result<(), String> {
        let url = "https://twitchtokengenerator.com";
        webbrowser::open(url)
            .map_err(|e| format!("Failed to open browser: {}", e))
    }

    /// After a valid token is pasted, show Step 3 and collapse Step 2
    pub fn show_step3(&mut self) {
        self.step3_visible = true;
        self.step2_open = false;
        self.step2_just_changed = true;
        self.step3_open = true;
        self.step3_just_changed = true;
    }

    /// Show Step 4 after Save & Continue
    pub fn show_step4(&mut self) {
        self.step4_visible = true;
        self.step3_open = false;
        self.step3_just_changed = true;
        self.step4_open = true;
        self.step4_just_changed = true;
    }
}

impl YapBotInstaller {
    /// Check if Python is installed
    pub fn is_python_installed(&self) -> bool {
        self.python_installed
    }
    
    /// Get Python version
    pub fn get_python_version(&self) -> Option<&String> {
        self.python_version.as_ref()
    }
    
    /// Start Python installation
    pub fn start_python_install(&mut self) {
        self.installing_python = true;
        self.status = "Installing Python...".to_string();
        
        // Clone the result Arc for the thread
        let result_arc = Arc::clone(&self.python_install_result);
        
        std::thread::spawn(move || {
            #[cfg(windows)]
            {
                use crate::python_manager::PythonManager;
                
                let result = (|| -> Result<(), String> {
                    // Download installer
                    let installer_path = PythonManager::download_installer()
                        .map_err(|e| format!("Failed to download installer: {}", e))?;
                    
                    // Install Python
                    let success = PythonManager::install_silent(&installer_path)
                        .map_err(|e| format!("Failed to install Python: {}", e))?;
                    
                    if !success {
                        return Err("Python installation failed".to_string());
                    }
                    
                    // Add scripts to PATH
                    PythonManager::add_scripts_to_path()?;
                    
                    Ok(())
                })();
                
                // Store the result
                if let Ok(mut guard) = result_arc.lock() {
                    *guard = Some(result);
                }
            }
            
            #[cfg(not(windows))]
            {
                // On Linux, we can't install Python automatically
                if let Ok(mut guard) = result_arc.lock() {
                    *guard = Some(Err("Automatic Python installation is not supported on Linux. Please install Python 3 manually.".to_string()));
                }
            }
        });
    }
    
    /// Update Python installation status
    pub fn update_python_status(&mut self) {
        // Only check if installation completed
        if self.installing_python {
            if let Ok(guard) = self.python_install_result.lock() {
                if let Some(result) = guard.as_ref() {
                    self.installing_python = false;
                    match result {
                        Ok(_) => {
                            self.status = "Python installed successfully!".to_string();
                            // Assume Python is installed after successful installation (like rustitles does)
                            self.python_installed = true;
                            self.python_version = Some("Python 3.x (installed)".to_string());
                        }
                        Err(e) => {
                            self.status = format!("Python installation failed: {}", e);
                        }
                    }
                }
            }
        }
    }

    /// Start dependencies installation
    pub fn start_dependencies_install(&mut self) {
        println!("GUI: start_dependencies_install called");
        self.installing_dependencies = true;
        self.status = "Installing YapBot Dependencies...".to_string();

        // Clone the result Arc for the thread
        let result_arc = Arc::clone(&self.dependencies_install_result);

        std::thread::spawn(move || {
            use std::panic;
            println!("THREAD: Dependencies install thread started.");
            let result = panic::catch_unwind(|| {
                println!("THREAD: Calling PythonManager::install_dependencies...");
                let result = crate::python_manager::PythonManager::install_dependencies();
                println!("THREAD: install_dependencies returned: {:?}", result);
                result
            });
            match result {
                Ok(res) => {
                    println!("THREAD: Dependencies install thread finished. Result: {:?}", res);
                    if let Ok(mut guard) = result_arc.lock() {
                        *guard = Some(res);
                    }
                }
                Err(e) => {
                    println!("THREAD: Dependencies install thread panicked: {:?}", e);
                    if let Ok(mut guard) = result_arc.lock() {
                        *guard = Some(Err("Dependencies install thread panicked".to_string()));
                    }
                }
            }
        });
    }
    
    /// Handle Python installation states (copied from rustitles)
    pub fn handle_installation_states(&mut self) {
        let mut should_start_dependencies = false;
        let mut should_show_step2 = false;
        
        if self.installing_python {
            if let Some(result) = self.python_install_result.lock().unwrap().take() {
                self.installing_python = false;
                match result {
                    Ok(_) => {
                        // Refresh environment to pick up new Python installation
                        #[cfg(windows)]
                        {
                            use crate::python_manager::PythonManager;
                            let _ = PythonManager::refresh_environment();
                        }
                        self.python_version = crate::python_manager::PythonManager::get_version();
                        self.python_installed = self.python_version.is_some();
                        self.status = "✅ Python installed successfully!".to_string();
                        
                        // Mark that we should start dependencies installation
                        should_start_dependencies = true;
                    }
                    Err(e) => {
                        self.status = format!("❌ Python install failed: {}", e);
                    }
                }
            }
        }
        
        if self.installing_dependencies {
            if let Some(result) = self.dependencies_install_result.lock().unwrap().take() {
                self.installing_dependencies = false;
                match result {
                    Ok(_) => {
                        self.status = "✅ Dependencies installed successfully!".to_string();
                        // Mark dependencies as installed
                        self.dependencies_installed = true;
                        // Mark that we should show Step 2
                        should_show_step2 = true;
                    }
                    Err(e) => {
                        self.status = format!("❌ Dependencies install failed: {}", e);
                    }
                }
            }
        }
        
        // Start dependencies installation after Python installation completes
        if should_start_dependencies {
            self.start_dependencies_install();
        }
        
        // Show Step 2 after dependencies installation completes
        if should_show_step2 {
            self.show_step2();
        }
    }
} 

impl YapBotInstaller {
    /// Ensure denied_users always includes main and bot channel names, but only the full name and only once, and append on blur
    pub fn update_denied_users_on_blur(&mut self, _prev_main: &str, prev_bot: &str) {
        let mut users: Vec<String> = self.denied_users
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
        let mut changed = false;
        // Remove any previous bot name
        if !prev_bot.is_empty() {
            users.retain(|u| !u.eq_ignore_ascii_case(prev_bot));
        }
        // Append current bot name if not empty and not already present
        let bot_name = &self.bot_channel_name;
        if !bot_name.is_empty() && !users.iter().any(|u| u.eq_ignore_ascii_case(bot_name)) {
            users.push(bot_name.clone());
            changed = true;
        }
        if changed {
            self.denied_users = users.join(", ");
        }
    }

    /// Sync denied_users_list from denied_users string
    pub fn sync_denied_users_list(&mut self) {
        self.denied_users_list = self.denied_users
            .split(',')
            .map(|s| s.trim().to_string())
            .filter(|s| !s.is_empty())
            .collect();
    }
    /// Sync denied_users string from denied_users_list
    pub fn sync_denied_users_string(&mut self) {
        self.denied_users = self.denied_users_list
            .iter()
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect::<Vec<_>>()
            .join(", ");
    }
} 

impl YapBotInstaller {
    pub fn load_settings_from_file() -> Option<YapBotInstallerSettings> {
        let path = std::path::Path::new("YapBotInstallerSettings.json");
        if let Ok(data) = std::fs::read_to_string(path) {
            if let Ok(settings) = serde_json::from_str::<YapBotInstallerSettings>(&data) {
                if settings.is_complete() {
                    return Some(settings);
                }
            }
        }
        None
    }

    pub fn save_settings_to_file(&self) {
        let path = std::path::Path::new("YapBotInstallerSettings.json");
        if let Some(settings) = self.get_current_settings() {
            if let Ok(json) = serde_json::to_string_pretty(&settings) {
                let _ = std::fs::write(path, json);
            }
        }
    }

    pub fn get_current_settings(&self) -> Option<YapBotInstallerSettings> {
        let oauth = self.bot_oauth_token.clone().unwrap_or_default();
        let main_channel_name = self.main_channel_name.clone();
        let bot_channel_name = self.bot_channel_name.clone();
        let denied_users = self.denied_users.clone();
        let cooldown = self.cooldown.clone();
        let generate_command = self.generate_command.clone();
        let step4_db_prompt_answered_yes = self.step4_db_prompt_answered_yes;
        if oauth.is_empty() || main_channel_name.is_empty() || bot_channel_name.is_empty()
            || denied_users.is_empty() || cooldown.is_empty() || generate_command.is_empty() {
            return None;
        }
        Some(YapBotInstallerSettings {
            oauth,
            main_channel_name,
            bot_channel_name,
            denied_users,
            cooldown,
            generate_command,
            step4_db_prompt_answered_yes,
            twitch_token_client_id: self.twitch_token_client_id.clone(),
        })
    }

    pub fn get_latest_version(&self) -> Option<&String> { self.latest_version.as_ref() }
    pub fn get_version_check_error(&self) -> Option<&String> { self.version_check_error.as_ref() }
    pub fn is_version_checked(&self) -> bool { self.version_checked }

    /// Poll for version check results
    pub fn poll_version_check(&mut self) {
        if self.version_checked { return; }
        let lock = VERSION_PTR.lock().unwrap();
        if lock.2 {
            self.latest_version = lock.0.clone();
            self.version_check_error = lock.1.clone();
            self.version_checked = true;
        }
    }
}

impl YapBotInstaller {
    pub fn sync_generate_command_string(&mut self) {
        self.generate_command = self.generate_command_list.join(", ");
    }
}

// Version check state pointer for background thread
pub static VERSION_PTR: Lazy<Arc<Mutex<(Option<String>, Option<String>, bool)>>> = Lazy::new(|| {
    Arc::new(Mutex::new((None, None, false)))
});

// Utility: Recursively copy a directory
pub fn copy_dir_all(src: &std::path::Path, dst: &std::path::Path) -> std::io::Result<()> {
    std::fs::create_dir_all(dst)?;
    for entry in std::fs::read_dir(src)? {
        let entry = entry?;
        let ty = entry.file_type()?;
        let dest_file = dst.join(entry.file_name());
        if ty.is_dir() {
            copy_dir_all(&entry.path(), &dest_file)?;
        } else {
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if (file_name_str == "Database.py" || file_name_str.ends_with(".db")) && dest_file.exists() {
                println!("[YapBotInstaller] Skipped existing {} at {}", file_name_str, dest_file.display());
                continue;
            }
            std::fs::copy(entry.path(), &dest_file)?;
            println!("[YapBotInstaller] Copied file: {} -> {}", entry.path().display(), dest_file.display());
        }
    }
    Ok(())
}

// Utility: Edit Settings.py with user info
pub fn edit_settings_py(
    settings_path: &std::path::Path,
    host: &str,
    port: i32,
    channel: &str,
    nickname: &str,
    authentication: &str,
    denied_users: &[String],
    allowed_users: &[String],
    cooldown: i32,
    key_length: i32,
    max_sentence_word_amount: i32,
    min_sentence_word_amount: i32,
    help_message_timer: i32,
    automatic_generation_timer: i32,
    whisper_cooldown: bool,
    enable_generate_command: bool,
    sentence_separator: &str,
    allow_generate_params: bool,
    generate_commands: &[String],
) -> std::io::Result<()> {
    use std::io::{Read, Write};
    let mut contents = String::new();
    {
        let mut file = std::fs::File::open(settings_path)?;
        file.read_to_string(&mut contents)?;
    }
    // Find the DEFAULTS block and replace the entire dict
    let lines: Vec<String> = contents.lines().map(|l| l.to_string()).collect();
    let mut start = None;
    let mut end = None;
    let mut brace_count;
    for (i, line) in lines.iter().enumerate() {
        if line.contains("DEFAULTS: SettingsData = {") {
            start = Some(i);
            brace_count = line.chars().filter(|&c| c == '{').count();
            for j in i+1..lines.len() {
                brace_count += lines[j].chars().filter(|&c| c == '{').count();
                brace_count -= lines[j].chars().filter(|&c| c == '}').count();
                if brace_count == 0 {
                    end = Some(j);
                    break;
            }
            }
                break;
            }
        }
    if let (Some(start), Some(end)) = (start, end) {
        let dict_str = generate_python_settings_dict(
            host, port, channel, nickname, authentication, denied_users, allowed_users, cooldown, key_length,
            max_sentence_word_amount, min_sentence_word_amount, help_message_timer, automatic_generation_timer,
            whisper_cooldown, enable_generate_command, sentence_separator, allow_generate_params, generate_commands
        );
        let mut new_lines = lines[..start+1].to_vec();
        new_lines.push(dict_str);
        new_lines.push(lines[end].clone());
        new_lines.extend(lines[end+1..].iter().cloned());
    let mut file = std::fs::File::create(settings_path)?;
        file.write_all(new_lines.join("\n").as_bytes())?;
    println!("[YapBotInstaller] Edited file: {}", settings_path.display());
    }
    Ok(())
} 

// Utility: Copy any .db files from old Yap Bot folder to new AppData location
pub fn migrate_db_files_with_callback_and_channel(
    mut on_copy: impl FnMut(&str, &str),
    tx: Option<&Sender<(String, String)>>,
) -> std::io::Result<()> {
    use std::env;
    use std::fs;
    let user = env::var("USERNAME").unwrap_or_else(|_| "User".to_string());
    let old_dir = format!("C:/Users/{}/Documents/Applications/Yap Bot/TwitchMarkovChain", user);
    let new_dir = format!("C:/Users/{}/AppData/Roaming/YapBot/TwitchMarkovChain", user);
    let old_path = std::path::Path::new(&old_dir);
    let new_path = std::path::Path::new(&new_dir);
    if old_path.exists() && new_path.exists() {
        for entry in fs::read_dir(old_path)? {
            let entry = entry?;
            let file_name = entry.file_name();
            let file_name_str = file_name.to_string_lossy();
            if file_name_str.ends_with(".db") {
                let src = entry.path();
                let dest = new_path.join(&file_name);
                on_copy(&src.display().to_string(), &dest.display().to_string());
                if let Some(tx) = tx {
                    let _ = tx.send((src.display().to_string(), dest.display().to_string()));
                }
                fs::copy(&src, &dest)?;
                println!("[YapBotInstaller] Migrated DB file: {} -> {}", src.display(), dest.display());
            }
        }
    }
    Ok(())
} 

pub fn copy_embedded_twitch_markovchain_to(dst: &Path) -> std::io::Result<()> {
    use crate::data_structures::TWITCH_MARKOVCHAIN_DIR;
    use include_dir::DirEntry;
    use std::fs;
    fn copy_dir(dir: &Dir, dst: &Path) -> std::io::Result<()> {
        for entry in dir.entries() {
            match entry {
                DirEntry::Dir(subdir) => {
                    let sub_dst = dst.join(subdir.path());
                    fs::create_dir_all(&sub_dst)?;
                    copy_dir(subdir, &sub_dst)?;
                }
                DirEntry::File(file) => {
                    let rel_path = file.path();
                    let dest_path = dst.join(rel_path);
                    let file_name = rel_path.file_name().unwrap().to_string_lossy();
                    if (file_name == "Database.py" || file_name.ends_with(".db")) && dest_path.exists() {
                        println!("[YapBotInstaller] Skipped existing {} at {}", file_name, dest_path.display());
                        continue;
                    }
                    if let Some(parent) = dest_path.parent() {
                        fs::create_dir_all(parent)?;
                    }
                    fs::write(&dest_path, file.contents())?;
                    println!("[YapBotInstaller] Copied embedded file: {} -> {}", rel_path.display(), dest_path.display());
                }
            }
        }
        Ok(())
    }
    copy_dir(&TWITCH_MARKOVCHAIN_DIR, dst)
} 

/// Generate the Python dict string for the DEFAULTS block in Settings.py
pub fn generate_python_settings_dict(
    host: &str,
    port: i32,
    channel: &str,
    nickname: &str,
    authentication: &str,
    denied_users: &[String],
    allowed_users: &[String],
    cooldown: i32,
    key_length: i32,
    max_sentence_word_amount: i32,
    min_sentence_word_amount: i32,
    help_message_timer: i32,
    automatic_generation_timer: i32,
    whisper_cooldown: bool,
    enable_generate_command: bool,
    sentence_separator: &str,
    allow_generate_params: bool,
    generate_commands: &[String],
) -> String {
    // Helper for formatting Python lists
    fn py_list(strings: &[String]) -> String {
        if strings.is_empty() {
            "[]".to_string()
        } else {
            let mut out = String::from("[\n");
            for s in strings {
                out.push_str(&format!("            \"{}\",\n", s));
            }
            out.push_str("        ]");
            out
        }
    }
    let denied_users_str = py_list(denied_users);
    let allowed_users_str = py_list(allowed_users);
    let generate_commands_str = py_list(generate_commands);
    format!(
        "        \"Host\": \"{}\",\n        \"Port\": {},\n        \"Channel\": \"{}\",\n        \"Nickname\": \"{}\",\n        \"Authentication\": \"{}\",\n        \"DeniedUsers\": {},\n        \"AllowedUsers\": {},\n        \"Cooldown\": {},\n        \"KeyLength\": {},\n        \"MaxSentenceWordAmount\": {},\n        \"MinSentenceWordAmount\": {},\n        \"HelpMessageTimer\": {},\n        \"AutomaticGenerationTimer\": {},\n        \"WhisperCooldown\": {},\n        \"EnableGenerateCommand\": {},\n        \"SentenceSeparator\": \"{}\",\n        \"AllowGenerateParams\": {},\n        \"GenerateCommands\": {}\n    ",
        host,
        port,
        channel,
        nickname,
        authentication,
        denied_users_str,
        allowed_users_str,
        cooldown,
        key_length,
        max_sentence_word_amount,
        min_sentence_word_amount,
        help_message_timer,
        automatic_generation_timer,
        if whisper_cooldown { "True" } else { "False" },
        if enable_generate_command { "True" } else { "False" },
        sentence_separator,
        if allow_generate_params { "True" } else { "False" },
        generate_commands_str
    )
}

/// Generate pretty-printed JSON for settings.json with 2-space indentation and pretty arrays
pub fn generate_settings_json(
    allow_generate_params: bool,
    allowed_users: &[String],
    authentication: &str,
    automatic_generation_timer: i32,
    channel: &str,
    cooldown: i32,
    denied_users: &[String],
    enable_generate_command: bool,
    generate_commands: &[String],
    help_message_timer: i32,
    host: &str,
    key_length: i32,
    max_sentence_word_amount: i32,
    min_sentence_word_amount: i32,
    nickname: &str,
    port: i32,
    sentence_separator: &str,
    whisper_cooldown: bool,
) -> String {
    use serde_json::json;
    let value = json!({
        "AllowGenerateParams": allow_generate_params,
        "AllowedUsers": allowed_users,
        "Authentication": authentication,
        "AutomaticGenerationTimer": automatic_generation_timer,
        "Channel": channel,
        "Cooldown": cooldown,
        "DeniedUsers": denied_users,
        "EnableGenerateCommand": enable_generate_command,
        "GenerateCommands": generate_commands,
        "HelpMessageTimer": help_message_timer,
        "Host": host,
        "KeyLength": key_length,
        "MaxSentenceWordAmount": max_sentence_word_amount,
        "MinSentenceWordAmount": min_sentence_word_amount,
        "Nickname": nickname,
        "Port": port,
        "SentenceSeparator": sentence_separator,
        "WhisperCooldown": whisper_cooldown,
    });
    // Custom pretty-printer for 2-space indentation and pretty arrays
    let mut s = serde_json::to_string_pretty(&value).unwrap();
    // Replace 4-space with 2-space indentation
    s = s.replace("    ", "  ");
    s
} 