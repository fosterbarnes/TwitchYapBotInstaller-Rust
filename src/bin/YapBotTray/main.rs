#![windows_subsystem = "windows"]

mod obs_monitor;
mod bot_manager;

use systray::Application;
use std::process::Command;
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::fs;
use std::env;
use chrono::Local;

#[cfg(windows)]
use std::os::windows::process::CommandExt;


// Import shared modules from the main crate
use yap_bot_installer::config;
use yap_bot_installer::log_and_print;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Load settings from the YapBotInstallerSettings.json file
fn load_settings() -> Option<serde_json::Value> {
    let settings_path = bot_manager::get_markovchain_path().join("YapBotInstallerSettings.json");
    if let Ok(contents) = fs::read_to_string(settings_path) {
        if let Ok(settings) = serde_json::from_str(&contents) {
            return Some(settings);
        }
    }
    None
}

/// Check if OBS monitoring is enabled in settings
fn is_obs_monitoring_enabled() -> bool {
    if let Some(settings) = load_settings() {
        if let Some(exit_when_monitored_app_closes) = settings.get("ExitWhenMonitoredAppCloses") {
            if let Some(enabled) = exit_when_monitored_app_closes.as_bool() {
                return enabled;
            }
        }
    }
    false
}

/// Update the tray tooltip with current bot status
fn update_tray_tooltip(app: &mut Application) {
    let status = if bot_manager::is_bot_running() { "Running" } else { "Stopped" };
    let tooltip = format!("YapBot Tray - Bot: {}", status);
    if let Err(e) = app.set_tooltip(&tooltip) {
        log_and_print!("ERROR: Failed to update tooltip: {}", e);
    }
}

/// Launch the full GUI version of YapBot
fn launch_gui() -> Result<(), Box<dyn std::error::Error>> {
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let gui_exe = format!("{}\\YapBot\\TwitchYapBot.exe", appdata);
    
    if !std::path::Path::new(&gui_exe).exists() {
        log_and_print!("ERROR: TwitchYapBot.exe not found at {}", gui_exe);
        return Err("TwitchYapBot.exe not found".into());
    }
    
    Command::new(&gui_exe)
        .arg("--force-gui")  // Bypass "start minimized to tray" check
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    
    log_and_print!("Launched TwitchYapBot GUI with --force-gui from: {}", gui_exe);
    
    Ok(())
}

fn main() {
    // Set up logging FIRST, before anything else
    if env::var("YAPBOT_LOG_PATH").is_err() {
        let log_dir = config::get_log_dir();
        if !log_dir.exists() {
            if let Err(e) = fs::create_dir_all(&log_dir) {
                eprintln!("YapBotTray: ERROR: Failed to create log directory: {}", e);
                return;
            }
        }
        let now = Local::now();
        let log_filename = now.format("%m-%d-%y_%H-%M-%S_tray.log").to_string();
        let log_path = log_dir.join(log_filename);
        env::set_var("YAPBOT_LOG_PATH", &log_path);
    }
    
    // Set up signal handler for cleanup on unexpected termination
    if let Err(e) = ctrlc::set_handler(|| {
        println!("YapBotTray: Received termination signal, cleaning up...");
        // Comprehensive cleanup: kill ALL Python and PowerShell processes
        bot_manager::kill_all_markovchain_processes();
        obs_monitor::kill_all_powershell_processes();
        // Clean up lock file
        let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        let lock_file = std::path::Path::new(&appdata).join("YapBot").join("YapBotTray.lock");
        if let Err(e) = std::fs::remove_file(&lock_file) {
            println!("YapBotTray: WARNING: Failed to remove lock file: {}", e);
        }
        std::process::exit(0);
    }) {
        eprintln!("YapBotTray: Failed to set signal handler: {}", e);
    }
    
    // Single-instance protection - check if another instance is already running
    // We'll use a simple file-based approach since named mutexes are complex in Rust
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let yapbot_dir = std::path::Path::new(&appdata).join("YapBot");
    let lock_file = yapbot_dir.join("YapBotTray.lock");
    
    // Ensure the YapBot directory exists
    if !yapbot_dir.exists() {
        if let Err(e) = std::fs::create_dir_all(&yapbot_dir) {
            log_and_print!("YapBotTray: ERROR: Failed to create YapBot directory: {}", e);
            return;
        }
    }
    
    // Try to create the lock file
    match std::fs::File::create(&lock_file) {
        Ok(_) => {
            // Successfully created lock file, this is the first instance
            log_and_print!("YapBotTray: First instance, proceeding");
        }
        Err(e) => {
            // Lock file creation failed - could be another instance or permission issue
            log_and_print!("YapBotTray: Failed to create lock file: {}", e);
            log_and_print!("YapBotTray: Another instance may be running, exiting");
            return;
        }
    }
    
    log_and_print!("YapBotTray started");
    
    // Create a shared flag to indicate if the application should exit
    let should_exit = Arc::new(Mutex::new(false));
    
    // Check for handover file from GUI app cleanup
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let handover_path = std::path::Path::new(&appdata).join("YapBot").join("handover.txt");
    
    // Check if handover file exists (indicating we're being launched from GUI "to tray" button)
    if handover_path.exists() {
        log_and_print!("Handover file detected - waiting for GUI cleanup to complete...");
        
        // Poll handover file every 10ms for "READY" signal
        let mut ready = false;
        for attempt in 1..=6000 { // Max 1 minute (6000 * 10ms)
            if let Ok(contents) = std::fs::read_to_string(&handover_path) {
                if contents.trim() == "READY" {
                    log_and_print!("Received READY signal from GUI app after {} attempts", attempt);
                    ready = true;
                    break;
                }
            }
            std::thread::sleep(Duration::from_millis(10));
        }
        
        if !ready {
            log_and_print!("WARNING: Never received READY signal from GUI app, proceeding anyway");
        }
        
        // Clear the handover file
        if let Err(e) = std::fs::remove_file(&handover_path) {
            log_and_print!("WARNING: Failed to remove handover file: {}", e);
        } else {
            log_and_print!("Handover file cleared");
        }
    } else {
        log_and_print!("No handover file detected - normal tray startup");
    }
    
    // Auto-start the bot if it's not already running
    if !bot_manager::is_bot_running() {
        log_and_print!("Auto-starting bot...");
        if let Err(e) = bot_manager::start_bot() {
            log_and_print!("ERROR: Failed to auto-start bot: {}", e);
        } else {
            log_and_print!("Bot auto-started successfully");
        }
    } else {
        log_and_print!("Bot is already running, skipping auto-start");
    }
    
    // Initialize OBS monitoring if enabled
    if is_obs_monitoring_enabled() {
        log_and_print!("[OBS_MONITOR] OBS monitoring enabled in settings, starting monitor");
        obs_monitor::start_obs_monitoring_with_direct_exit(|| bot_manager::stop_bot());
    } else {
        log_and_print!("[OBS_MONITOR] OBS monitoring disabled in settings");
    }
    
    // Initialize the system tray application
    let mut app = match Application::new() {
        Ok(app) => app,
        Err(e) => {
            log_and_print!("YapBotTray: ERROR: Failed to initialize system tray: {}", e);
            return;
        }
    };
    
    // Set the tray icon - use hardcoded path like AdobeProcessMonitor
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let icon_path = format!("{}\\YapBot\\icons\\yap_icon_purple.ico", appdata);
    log_and_print!("Looking for icon at: {}", icon_path);
    
    if std::path::Path::new(&icon_path).exists() {
        log_and_print!("Found icon at: {}", icon_path);
        app.set_icon_from_file(&icon_path).unwrap();
    } else {
        log_and_print!("WARNING: No icon found at {}", icon_path);
        log_and_print!("Tray will run without an icon");
    }
    
    // Set the initial tray tooltip with bot status
    update_tray_tooltip(&mut app);
    
    log_and_print!("System tray initialized successfully");
    
    // Clone the shared flag to pass to the menu item closures
    let should_exit_clone = Arc::clone(&should_exit);
    
        // Add menu items
    app.add_menu_item("Launch Full App", move |app: &mut Application| {
        // STEP 2: Bot is stopped (using same logic as "Stop Bot" button that works)
        log_and_print!("Stopping bot before launching GUI...");
        if let Err(e) = bot_manager::stop_bot() {
            log_and_print!("ERROR: Failed to stop bot before launching GUI: {}", e);
        } else {
            log_and_print!("Bot stopped successfully");
        }
        
        // STEP 3: GUI app is launched with force gui launch flag
        if let Err(e) = launch_gui() {
            log_and_print!("ERROR: Failed to launch GUI: {}", e);
        } else {
            log_and_print!("Closing tray app and launching GUI");
            app.quit(); // Stop the systray event loop
        }
        Ok::<(), std::io::Error>(())
    }).unwrap();
    
    app.add_menu_item("Restart Bot", move |app: &mut Application| {
        log_and_print!("Restarting bot...");
        // Stop the bot first
        if let Err(e) = bot_manager::stop_bot() {
            log_and_print!("ERROR: Failed to stop bot during restart: {}", e);
        } else {
            // Wait a moment for the process to fully stop
            std::thread::sleep(Duration::from_millis(1000));
            // Start the bot again
            if let Err(e) = bot_manager::start_bot() {
                log_and_print!("ERROR: Failed to start bot during restart: {}", e);
            } else {
                log_and_print!("Bot restarted successfully");
            }
        }
        update_tray_tooltip(app);
        Ok::<(), std::io::Error>(())
    }).unwrap();
    
    app.add_menu_item("Stop Bot", move |app: &mut Application| {
        if let Err(e) = bot_manager::stop_bot() {
            log_and_print!("ERROR: Failed to stop bot: {}", e);
        } else {
            update_tray_tooltip(app);
        }
        Ok::<(), std::io::Error>(())
    }).unwrap();
    
    app.add_menu_item("Show Logs", move |_app: &mut Application| {
        // Open the log file in the default text editor
        if let Ok(log_path) = env::var("YAPBOT_LOG_PATH") {
            if let Err(e) = Command::new("notepad").arg(&log_path).spawn() {
                log_and_print!("ERROR: Failed to open log file: {}", e);
            } else {
                log_and_print!("Opened log file: {}", log_path);
            }
        } else {
            log_and_print!("ERROR: No log path found");
        }
        Ok::<(), std::io::Error>(())
    }).unwrap();
    
    app.add_menu_item("Exit", move |app: &mut Application| {
        let mut exit_flag = should_exit_clone.lock().unwrap();
        *exit_flag = true;
        
        // Comprehensive cleanup: kill ALL Python and PowerShell processes
        bot_manager::kill_all_markovchain_processes();
        obs_monitor::kill_all_powershell_processes();
        
        app.quit(); // Stop the systray event loop
        Ok::<(), std::io::Error>(())
    }).unwrap();
    
    // Bot status monitoring loop
    let should_exit_clone_for_thread = Arc::clone(&should_exit);
    spawn(move || {
        loop {
            {
                let exit_flag = should_exit_clone_for_thread.lock().unwrap();
                if *exit_flag {
                    break; // Exit the loop if the flag is set
                }
            }
            
            // Check bot status every 30 seconds
            sleep(Duration::from_secs(30));
        }
    });
    
    // Start the systray event loop
    app.wait_for_message().unwrap();
    
    // Clean up lock file on exit
    if let Err(e) = std::fs::remove_file(&lock_file) {
        log_and_print!("WARNING: Failed to remove lock file: {}", e);
    } else {
        log_and_print!("Lock file cleaned up");
    }
    
    // Exiting gracefully
    log_and_print!("YapBotTray exiting");
}
