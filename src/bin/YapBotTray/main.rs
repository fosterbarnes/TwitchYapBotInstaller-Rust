#![windows_subsystem = "windows"]

use systray::Application;
use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;
use std::path::PathBuf;
use std::fs;
use std::env;
use chrono::Local;

#[cfg(windows)]
use std::os::windows::process::CommandExt;


// Import shared modules from the main crate
use yap_bot_installer::config;
use yap_bot_installer::log_and_print;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const OBS_PROCESSES: [&str; 2] = ["obs64.exe", "Streamlabs OBS.exe"];
const FALLBACK_CHECK_INTERVAL_SECONDS: u64 = 5;

// Global state to track PowerShell process IDs for cleanup
static POWERSHELL_PIDS: once_cell::sync::Lazy<Arc<Mutex<Vec<u32>>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Clean up all tracked PowerShell processes
fn cleanup_powershell_processes() {
    let pids = {
        let mut pids = POWERSHELL_PIDS.lock().unwrap();
        pids.drain(..).collect::<Vec<u32>>()
    };
    
    for pid in pids {
        if let Err(e) = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .creation_flags(CREATE_NO_WINDOW)
            .output() {
            log_and_print!("[OBS_MONITOR] Failed to kill PowerShell process {}: {}", pid, e);
        } else {
            log_and_print!("[OBS_MONITOR] Successfully killed PowerShell process {}", pid);
        }
    }
}

/// Check if any OBS process is currently running
fn is_obs_running() -> bool {
    if let Ok(output) = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", OBS_PROCESSES[0]), "/FI", &format!("IMAGENAME eq {}", OBS_PROCESSES[1])])
        .creation_flags(CREATE_NO_WINDOW)
        .output() {
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        return OBS_PROCESSES.iter().any(|process| output_str.contains(process));
    }
    false
}

/// Start OBS monitoring that directly exits the process when OBS closes
fn start_obs_monitoring_with_direct_exit() {
    spawn(move || {
        log_and_print!("[OBS_MONITOR] Starting OBS/Streamlabs monitoring with direct exit");
        start_powershell_monitoring_direct_exit();
    });
}

/// Monitoring with direct exit using PowerShell WaitForProgram and Wait-Process
fn start_powershell_monitoring_direct_exit() {
    log_and_print!("[OBS_MONITOR] Starting monitoring with direct exit using PowerShell");
    
    // PowerShell script that uses WaitForProgram and Wait-Process
    let ps_script = r#"
        function WaitForProgram {
            param (
                [Parameter(Mandatory=$true, Position=0)]
                [string]$ProgramName
            )

            $animation = @("Waiting for $ProgramName to start.  ", "Waiting for $ProgramName to start . ", "Waiting for $ProgramName to start  .")
            $index = 0

            while (-not (Get-Process -Name $ProgramName -ErrorAction SilentlyContinue)) {
                Write-Host "`r$($animation[$index])" -NoNewline
                Start-Sleep -Seconds 5
                $index = ($index + 1) % $animation.Length
            }

            Write-Host "`r$ProgramName has started.            "
        }

        try {
            # Wait for either obs64 or Streamlabs OBS to start
            $obsProcesses = @("obs64", "Streamlabs OBS")
            $startedProcess = $null
            
            foreach ($processName in $obsProcesses) {
                if (Get-Process -Name $processName -ErrorAction SilentlyContinue) {
                    $startedProcess = $processName
                    Write-Host "OBS_PROCESS_FOUND:$processName"
                    break
                }
            }
            
            if (-not $startedProcess) {
                # Wait for the first process to start
                Write-Host "WAITING_FOR_OBS_START"
                WaitForProgram -ProgramName "obs64"
                $startedProcess = "obs64"
                Write-Host "OBS_PROCESS_STARTED:$startedProcess"
            }
            
            # Now wait for the process to exit
            Write-Host "WAITING_FOR_PROCESS:$startedProcess"
            Wait-Process -Name $startedProcess -ErrorAction Stop
            Write-Host "OBS_PROCESS_CLOSED:$startedProcess"
        } catch {
            Write-Host "WAIT_PROCESS_ERROR:$($_.Exception.Message)"
            exit 1
        }
    "#;

    // Start PowerShell process
    let mut child = match Command::new("powershell")
        .args(["-NoProfile", "-NonInteractive", "-ExecutionPolicy", "Bypass", "-Command", ps_script])
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .creation_flags(CREATE_NO_WINDOW)
        .spawn() {
        Ok(child) => {
            // Track the PowerShell process ID for cleanup
            let pid = child.id();
            {
                let mut pids = POWERSHELL_PIDS.lock().unwrap();
                pids.push(pid);
            }
            log_and_print!("[OBS_MONITOR] Started PowerShell monitoring process with PID: {}", pid);
            child
        }
        Err(e) => {
            log_and_print!("[OBS_MONITOR] Failed to start PowerShell monitoring: {}", e);
            // Fallback to polling method
            start_polling_monitoring_direct_exit();
            return;
        }
    };

    // Handle stdout in a separate thread
    let stdout = child.stdout.take().unwrap();
    spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.starts_with("OBS_PROCESS_FOUND:") {
                    let process_name = line.trim_start_matches("OBS_PROCESS_FOUND:");
                    log_and_print!("[OBS_MONITOR] OBS process already running: {}", process_name);
                } else if line.starts_with("WAITING_FOR_OBS_START") {
                    log_and_print!("[OBS_MONITOR] Waiting for OBS to start...");
                } else if line.starts_with("OBS_PROCESS_STARTED:") {
                    let process_name = line.trim_start_matches("OBS_PROCESS_STARTED:");
                    log_and_print!("[OBS_MONITOR] OBS process started: {}", process_name);
                } else if line.starts_with("WAITING_FOR_PROCESS:") {
                    let process_name = line.trim_start_matches("WAITING_FOR_PROCESS:");
                    log_and_print!("[OBS_MONITOR] Waiting for process to close: {}", process_name);
                } else if line.starts_with("OBS_PROCESS_CLOSED:") {
                    let process_name = line.trim_start_matches("OBS_PROCESS_CLOSED:");
                    log_and_print!("[OBS_MONITOR] OBS process closed: {}", process_name);
                    log_and_print!("[OBS_MONITOR] Direct exit triggered due to OBS shutdown");
                    
                    // Clean up PowerShell processes used for OBS monitoring
                    log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes due to OBS shutdown");
                    cleanup_powershell_processes();
                    
                    // Stop the Python bot
                    log_and_print!("[OBS_MONITOR] Stopping Python bot due to OBS shutdown");
                    if let Err(e) = stop_bot() {
                        log_and_print!("[OBS_MONITOR] Failed to stop bot: {}", e);
                    }
                    
                    // Exit directly
                    std::process::exit(0);
                } else if line.starts_with("WAIT_PROCESS_ERROR:") {
                    let error = line.trim_start_matches("WAIT_PROCESS_ERROR:");
                    log_and_print!("[OBS_MONITOR] PowerShell monitoring error: {}", error);
                    log_and_print!("[OBS_MONITOR] Falling back to polling method");
                    break;
                }
            }
        }
    });

    // Handle stderr in a separate thread
    let stderr = child.stderr.take().unwrap();
    spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.trim().is_empty() {
                    log_and_print!("[OBS_MONITOR] PowerShell stderr: {}", line);
                }
            }
        }
    });

    // Wait for the PowerShell process to complete
    spawn(move || {
        if let Ok(status) = child.wait() {
            log_and_print!("[OBS_MONITOR] PowerShell Wait-Process exited with status: {}", status);
        }
    });
}

/// Fallback monitoring with direct exit using polling
fn start_polling_monitoring_direct_exit() {
    log_and_print!("[OBS_MONITOR] Starting fallback monitoring with direct exit using polling");
    
    // Phase 1: Wait for OBS to start
    log_and_print!("[OBS_MONITOR] Waiting for OBS/Streamlabs to start...");
    
    while !is_obs_running() {
        sleep(Duration::from_secs(5));
    }
    
    log_and_print!("[OBS_MONITOR] OBS/Streamlabs process detected, monitoring for shutdown");
    
    // Phase 2: Monitor for OBS closing
    loop {
        sleep(Duration::from_secs(FALLBACK_CHECK_INTERVAL_SECONDS));
        
        if !is_obs_running() {
            log_and_print!("[OBS_MONITOR] OBS/Streamlabs process closed, triggering direct exit");
            
            // Clean up PowerShell processes used for OBS monitoring
            log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes due to OBS shutdown");
            cleanup_powershell_processes();
            
            // Stop the Python bot
            log_and_print!("[OBS_MONITOR] Stopping Python bot due to OBS shutdown");
            if let Err(e) = stop_bot() {
                log_and_print!("[OBS_MONITOR] Failed to stop bot: {}", e);
            }
            
            // Exit directly
            std::process::exit(0);
        }
    }
}

/// Get the path to the MarkovChainBot directory
fn get_markovchain_path() -> PathBuf {
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain", appdata))
}

/// Load settings from the YapBotInstallerSettings.json file
fn load_settings() -> Option<serde_json::Value> {
    let settings_path = get_markovchain_path().join("YapBotInstallerSettings.json");
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

/// Check if the MarkovChainBot Python process is running
fn is_bot_running() -> bool {
    if let Ok(output) = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq python.exe", "/FO", "CSV"])
        .creation_flags(CREATE_NO_WINDOW)
        .output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("python.exe") {
                // Extract PID from CSV format
                if let Some(pid_str) = line.split(',').nth(1) {
                    if let Ok(pid) = pid_str.trim_matches('"').parse::<u32>() {
                        // Check if this Python process is running MarkovChainBot.py
                        if let Ok(wmic_output) = Command::new("wmic")
                            .args(["process", "where", &format!("ProcessId={}", pid), "get", "CommandLine", "/format:csv"])
                            .creation_flags(CREATE_NO_WINDOW)
                            .output() {
                            
                            let wmic_str = String::from_utf8_lossy(&wmic_output.stdout);
                            if wmic_str.contains("MarkovChainBot.py") {
                                log_and_print!("Found running bot process with PID: {}", pid);
                                return true;
                            }
                        }
                    }
                }
            }
        }
    }
    log_and_print!("No running bot process found");
    false
}

/// Start the MarkovChainBot Python process
fn start_bot() -> Result<(), Box<dyn std::error::Error>> {
    let markovchain_path = get_markovchain_path();
    let bot_script = markovchain_path.join("MarkovChainBot.py");
    
    if !bot_script.exists() {
        log_and_print!("ERROR: MarkovChainBot.py not found at {}", bot_script.display());
        return Err("MarkovChainBot.py not found".into());
    }
    
    log_and_print!("Starting bot from directory: {}", markovchain_path.display());
    
    // Check if bot is already running
    if is_bot_running() {
        log_and_print!("Bot is already running");
        return Ok(());
    }
    
    // Start the Python bot process with proper output handling
    let child = Command::new("python")
        .arg("MarkovChainBot.py")
        .current_dir(&markovchain_path)
        .stdout(Stdio::null())  // Don't capture stdout to avoid blocking
        .stderr(Stdio::null())  // Don't capture stderr to avoid blocking
        .creation_flags(CREATE_NO_WINDOW)
        .spawn()?;
    
    log_and_print!("Yap Bot has been started (PID: {})", child.id());
    
    // Give the process a moment to start up
    std::thread::sleep(Duration::from_millis(500));
    
    // Verify the bot started successfully
    if is_bot_running() {
        log_and_print!("Bot started successfully and is running");
    } else {
        log_and_print!("WARNING: Bot process started but may not be running properly");
    }
    
    Ok(())
}

/// Stop the MarkovChainBot Python process
fn stop_bot() -> Result<(), Box<dyn std::error::Error>> {
    // Find and kill any Python processes running MarkovChainBot.py
    if let Ok(output) = Command::new("tasklist")
        .args(["/FI", "IMAGENAME eq python.exe", "/FO", "CSV"])
        .creation_flags(CREATE_NO_WINDOW)
        .output() {
        let output_str = String::from_utf8_lossy(&output.stdout);
        for line in output_str.lines() {
            if line.contains("python.exe") {
                // Extract PID from CSV format
                if let Some(pid_str) = line.split(',').nth(1) {
                    if let Ok(pid) = pid_str.trim_matches('"').parse::<u32>() {
                        // Check if this Python process is running MarkovChainBot.py
                        if let Ok(wmic_output) = Command::new("wmic")
                            .args(["process", "where", &format!("ProcessId={}", pid), "get", "CommandLine", "/format:csv"])
                            .creation_flags(CREATE_NO_WINDOW)
                            .output() {
                            
                            let wmic_str = String::from_utf8_lossy(&wmic_output.stdout);
                            if wmic_str.contains("MarkovChainBot.py") {
                                let tk_result = Command::new("taskkill")
                                    .args(["/PID", &pid.to_string(), "/F", "/T"])
                                    .creation_flags(CREATE_NO_WINDOW)
                                    .output();
                                
                                if let Ok(ref out) = tk_result {
                                    let result_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                                    for line in result_str.lines() {
                                        log_and_print!("taskkill output: {}", line.trim());
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
    
    log_and_print!("Yap Bot has been destroyed by your own hands...");
    
    Ok(())
}

/// Update the tray tooltip with current bot status
fn update_tray_tooltip(app: &mut Application) {
    let status = if is_bot_running() { "Running" } else { "Stopped" };
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
        // Stop the Python bot
        if let Err(e) = stop_bot() {
            println!("YapBotTray: ERROR: Failed to stop bot on signal: {}", e);
        }
        // Clean up PowerShell processes used for OBS monitoring
        cleanup_powershell_processes();
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
    
    // Auto-start the bot if it's not already running
    if !is_bot_running() {
        log_and_print!("Auto-starting bot...");
        if let Err(e) = start_bot() {
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
        start_obs_monitoring_with_direct_exit();
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
        // Stop the bot before launching the GUI
        if let Err(e) = stop_bot() {
            log_and_print!("ERROR: Failed to stop bot before launching GUI: {}", e);
        } else {
            log_and_print!("Bot stopped before launching GUI");
        }
        
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
        if let Err(e) = stop_bot() {
            log_and_print!("ERROR: Failed to stop bot during restart: {}", e);
        } else {
            // Wait a moment for the process to fully stop
            std::thread::sleep(Duration::from_millis(1000));
            // Start the bot again
            if let Err(e) = start_bot() {
                log_and_print!("ERROR: Failed to start bot during restart: {}", e);
            } else {
                log_and_print!("Bot restarted successfully");
            }
        }
        update_tray_tooltip(app);
        Ok::<(), std::io::Error>(())
    }).unwrap();
    
    app.add_menu_item("Stop Bot", move |app: &mut Application| {
        if let Err(e) = stop_bot() {
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
        
        // Stop the bot before exiting
        if let Err(e) = stop_bot() {
            log_and_print!("ERROR: Failed to stop bot on exit: {}", e);
        }
        
        // Clean up PowerShell processes used for OBS monitoring
        cleanup_powershell_processes();
        
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
