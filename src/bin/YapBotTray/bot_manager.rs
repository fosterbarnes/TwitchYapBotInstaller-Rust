use std::process::{Command, Stdio};
use std::path::PathBuf;
use std::env;
use std::time::Duration;

use yap_bot_installer::log_and_print;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;

/// Get the path to the MarkovChainBot directory
pub fn get_markovchain_path() -> PathBuf {
    let appdata = env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain", appdata))
}

/// Check if the MarkovChainBot Python process is running
pub fn is_bot_running() -> bool {
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
pub fn start_bot() -> Result<(), Box<dyn std::error::Error>> {
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
pub fn stop_bot() -> Result<(), Box<dyn std::error::Error>> {
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
