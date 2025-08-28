//! Traymond IPC integration for TwitchYapBot
//!
//! This module handles the integration with traymond-tcp.exe to provide
//! system tray functionality for the TwitchYapBot window.

use std::process::{Command, Stdio};
use std::net::TcpStream;
use std::io::Write;
use std::time::Duration;
use std::path::PathBuf;
use crate::log_and_print;
use crate::config::app_version;
use windows::Win32::UI::WindowsAndMessaging::{EnumWindows, GetWindowTextA, GetWindowThreadProcessId, IsWindowVisible, SetForegroundWindow};
use windows::Win32::Foundation::{BOOL, HWND, LPARAM};


/// Path to traymond-tcp.exe in the user's AppData directory
fn get_traymond_path() -> PathBuf {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    PathBuf::from(format!("{}\\YapBot\\traymond-tcp.exe", appdata))
}

/// Check if traymond-tcp is already running by attempting to connect to its TCP server
pub fn is_traymond_running() -> bool {
    match TcpStream::connect("127.0.0.1:8766") {
        Ok(mut stream) => {
            // Send a simple command to test the connection
            if let Ok(_) = stream.write_all(b"SHOW_ALL") {
                log_and_print!("[TRAYMOND] TCP connection test successful - traymond-tcp is running");
                true
            } else {
                log_and_print!("[TRAYMOND] TCP connection test failed - could not send command");
                false
            }
        }
        Err(_) => {
            log_and_print!("[TRAYMOND] TCP connection test failed - traymond-tcp is not running");
            false
        }
    }
}

/// Launch traymond-tcp with the -noTray flag
pub fn launch_traymond() -> Result<std::process::Child, std::io::Error> {
    let traymond_path = get_traymond_path();
    
    if !traymond_path.exists() {
        log_and_print!("[TRAYMOND] ERROR: traymond-tcp.exe not found at: {}", traymond_path.display());
        return Err(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            format!("traymond-tcp.exe not found at {}", traymond_path.display())
        ));
    }
    
    log_and_print!("[TRAYMOND] Launching traymond-tcp.exe with -noTray and -noHotkey flags");
    
    let child = Command::new(&traymond_path)
        .arg("-noTray")
        .arg("-noHotkey")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;
    
    log_and_print!("[TRAYMOND] traymond-tcp.exe launched successfully (PID: {})", child.id());
    Ok(child)
}



/// Non-blocking check if traymond-tcp is ready (returns immediately)
pub fn is_traymond_ready() -> bool {
    is_traymond_running()
}

/// Send a command to traymond-tcp via TCP
pub fn send_traymond_command(command: &str) -> Result<(), std::io::Error> {
    log_and_print!("[TRAYMOND] Sending command: {}", command);
    
    let mut stream = TcpStream::connect("127.0.0.1:8766")?;
    stream.set_write_timeout(Some(Duration::from_secs(5)))?;
    
    stream.write_all(command.as_bytes())?;
    
    // Don't wait for response since traymond-tcp doesn't send responses
    log_and_print!("[TRAYMOND] Command sent successfully");
    
    Ok(())
}

/// Focus the TwitchYapBot window and then minimize it using MINIMIZE_CURRENT
pub fn focus_and_minimize_twitch_yap_bot() -> Result<(), std::io::Error> {
    if let Some(handle) = find_twitch_yap_bot_window() {
        log_and_print!("[TRAYMOND] Focusing TwitchYapBot window with handle: {}", handle);
        
        // Parse the handle and focus the window
        if let Ok(handle_value) = u64::from_str_radix(&handle, 16) {
            unsafe {
                let hwnd = HWND(handle_value as isize);
                SetForegroundWindow(hwnd);
                log_and_print!("[TRAYMOND] SetForegroundWindow called for handle: {}", handle);
            }
            
            // Small delay to ensure window is focused
            std::thread::sleep(Duration::from_millis(50));
            
            // Now use MINIMIZE_CURRENT since the window should be focused
            log_and_print!("[TRAYMOND] Using MINIMIZE_CURRENT on focused window");
            send_traymond_command("MINIMIZE_CURRENT")
        } else {
            log_and_print!("[TRAYMOND] ERROR: Could not parse window handle, falling back to MINIMIZE_CURRENT");
            send_traymond_command("MINIMIZE_CURRENT")
        }
    } else {
        log_and_print!("[TRAYMOND] ERROR: Could not find TwitchYapBot window, falling back to MINIMIZE_CURRENT");
        send_traymond_command("MINIMIZE_CURRENT")
    }
}

/// Minimize the TwitchYapBot window to tray by finding it by title
pub fn minimize_twitch_yap_bot_to_tray() -> Result<(), std::io::Error> {
    // Try the focus and minimize approach first (most efficient)
    log_and_print!("[TRAYMOND] Trying focus and minimize approach");
    match focus_and_minimize_twitch_yap_bot() {
        Ok(_) => {
            log_and_print!("[TRAYMOND] Focus and minimize approach completed");
            Ok(())
        }
        Err(e) => {
            log_and_print!("[TRAYMOND] Focus and minimize failed: {}, falling back to MINIMIZE_CURRENT", e);
            // Simple fallback to MINIMIZE_CURRENT without complex handle validation
            send_traymond_command("MINIMIZE_CURRENT")
        }
    }
}

/// Test if a window handle is valid by trying to get its title
#[allow(dead_code)]
pub fn is_window_handle_valid(handle: &str) -> bool {
    // Try to parse the handle
    if let Ok(handle_value) = u64::from_str_radix(handle, 16) {
        unsafe {
            let hwnd = HWND(handle_value as isize);
            let mut title_buffer = [0u8; 256];
            let title_len = GetWindowTextA(hwnd, &mut title_buffer);
            title_len > 0
        }
    } else {
        false
    }
}

/// Minimize a specific window by handle to tray
#[allow(dead_code)]
pub fn minimize_window_by_handle(handle: &str) -> Result<(), std::io::Error> {
    // Try both hex and decimal formats
    let command_hex = format!("MINIMIZE_BY_HANDLE:HANDLE:{}", handle);
    log_and_print!("[TRAYMOND] Trying hex format: {}", command_hex);
    
    // Also try decimal format
    if let Ok(handle_decimal) = u64::from_str_radix(handle, 16) {
        let command_decimal = format!("MINIMIZE_BY_HANDLE:HANDLE:{}", handle_decimal);
        log_and_print!("[TRAYMOND] Also trying decimal format: {}", command_decimal);
        
        // Try hex first
        match send_traymond_command(&command_hex) {
            Ok(_) => {
                log_and_print!("[TRAYMOND] Hex format command sent successfully");
                return Ok(());
            }
            Err(_) => {
                log_and_print!("[TRAYMOND] Hex format failed, trying decimal format");
                send_traymond_command(&command_decimal)
            }
        }
    } else {
        send_traymond_command(&command_hex)
    }
}

/// Show all hidden windows from tray
#[allow(dead_code)]
pub fn show_all_windows() -> Result<(), std::io::Error> {
    send_traymond_command("SHOW_ALL")
}

/// Exit traymond-tcp
pub fn exit_traymond() -> Result<(), std::io::Error> {
    log_and_print!("[TRAYMOND] Sending EXIT command to traymond-tcp");
    send_traymond_command("EXIT")
}

/// Find the TwitchYapBot window by title and return its handle
pub fn find_twitch_yap_bot_window() -> Option<String> {
    let expected_title = format!("Twitch Yap Bot v{}", app_version());
    log_and_print!("[TRAYMOND] Looking for window with title: {}", expected_title);
    
    let mut found_handle: Option<String> = None;
    
    unsafe {
        let result = EnumWindows(Some(enum_window_callback), LPARAM(&mut found_handle as *mut _ as isize));
        match result {
            Ok(_) => {
                if let Some(handle) = &found_handle {
                    log_and_print!("[TRAYMOND] Found TwitchYapBot window with handle: {}", handle);
                } else {
                    log_and_print!("[TRAYMOND] TwitchYapBot window not found");
                }
            }
            Err(_) => {
                log_and_print!("[TRAYMOND] ERROR: Failed to enumerate windows");
            }
        }
    }
    
    found_handle
}

/// Callback function for EnumWindows to find the TwitchYapBot window
unsafe extern "system" fn enum_window_callback(hwnd: HWND, lparam: LPARAM) -> BOOL {
    let found_handle = &mut *(lparam.0 as *mut Option<String>);
    
    // Get window title
    let mut title_buffer = [0u8; 256];
    let title_len = GetWindowTextA(hwnd, &mut title_buffer);
    
    if title_len > 0 {
        let title = String::from_utf8_lossy(&title_buffer[..title_len as usize]);
        let expected_title = format!("Twitch Yap Bot v{}", app_version());
        
        // Debug: Log all windows with titles that contain "Twitch" or "Yap"
        if title.contains("Twitch") || title.contains("Yap") || title.contains("yap") {
            let mut process_id = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            let current_pid = std::process::id();
            let is_our_process = process_id == current_pid;
            let is_visible = IsWindowVisible(hwnd).as_bool();
            
            log_and_print!("[TRAYMOND] DEBUG: Found window - Title: '{}', Handle: {:X}, Process: {}, Visible: {}, OurProcess: {}", 
                title, hwnd.0, process_id, is_visible, is_our_process);
        }
        
        // Check if this is our window
        if title == expected_title {
            // Verify it belongs to our process (removed visibility check)
            let mut process_id = 0u32;
            GetWindowThreadProcessId(hwnd, Some(&mut process_id));
            
            let current_pid = std::process::id();
            if process_id == current_pid {
                // Convert handle to hex string
                let handle_hex = format!("{:X}", hwnd.0);
                *found_handle = Some(handle_hex.clone());
                log_and_print!("[TRAYMOND] DEBUG: Found matching window! Title: '{}', Handle: {}", title, handle_hex);
                return BOOL::from(false); // Stop enumeration
            }
        }
    }
    
    BOOL::from(true) // Continue enumeration
}
