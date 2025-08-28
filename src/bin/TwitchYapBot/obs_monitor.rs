//! OBS/Streamlabs monitoring for TwitchYapBot
//!
//! This module handles monitoring OBS and Streamlabs processes using WMI events
//! for efficient, real-time process lifecycle monitoring.

use std::sync::mpsc;
use std::process::Command;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use crate::log_and_print;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const OBS_PROCESSES: [&str; 2] = ["obs64.exe", "Streamlabs OBS.exe"];
const FALLBACK_CHECK_INTERVAL_SECONDS: u64 = 30;

// Global state to track PowerShell process IDs for cleanup
static POWERSHELL_PIDS: once_cell::sync::Lazy<Arc<Mutex<Vec<u32>>>> = 
    once_cell::sync::Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Clean up all tracked PowerShell processes
pub fn cleanup_powershell_processes() {
    let pids = {
        let mut pids = POWERSHELL_PIDS.lock().unwrap();
        pids.drain(..).collect::<Vec<u32>>()
    };
    
    for pid in pids {
        if let Err(e) = Command::new("taskkill")
            .args(["/PID", &pid.to_string(), "/F", "/T"])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output() {
            log_and_print!("[OBS_MONITOR] Failed to kill PowerShell process {}: {}", pid, e);
        } else {
            log_and_print!("[OBS_MONITOR] Successfully killed PowerShell process {}", pid);
        }
    }
}

/// Check if any OBS process is currently running (fallback method)
pub fn is_obs_running() -> bool {
    for process_name in &OBS_PROCESSES {
        if let Ok(output) = Command::new("tasklist")
            .args(["/FI", &format!("IMAGENAME eq {}", process_name)])
            .creation_flags(0x08000000) // CREATE_NO_WINDOW
            .output() {
            
            let output_str = String::from_utf8_lossy(&output.stdout);
            // If the process is found, tasklist will show it in the output
            if output_str.contains(process_name) {
                return true;
            }
        }
    }
    false
}

/// Start OBS monitoring using WMI events
/// Returns a receiver that will receive a message when OBS closes
pub fn start_obs_monitoring() -> mpsc::Receiver<()> {
    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move || {
        log_and_print!("[OBS_MONITOR] Starting WMI-based OBS/Streamlabs monitoring");
        
        // Try WMI monitoring first, fall back to polling if it fails
        if let Err(e) = start_wmi_monitoring(tx.clone()) {
            log_and_print!("[OBS_MONITOR] WMI monitoring failed: {}, falling back to polling", e);
            start_polling_monitoring(tx);
        }
    });
    
    rx
}

/// Start OBS monitoring that directly exits the process when OBS closes
/// This is used when the app is minimized to tray to ensure immediate shutdown
pub fn start_obs_monitoring_with_direct_exit() {
    thread::spawn(move || {
        log_and_print!("[OBS_MONITOR] Starting WMI-based OBS/Streamlabs monitoring with direct exit");
        
        // Try WMI monitoring first, fall back to polling if it fails
        if let Err(e) = start_wmi_monitoring_direct_exit() {
            log_and_print!("[OBS_MONITOR] WMI monitoring failed: {}, falling back to polling", e);
            start_polling_monitoring_direct_exit();
        }
    });
}

/// Start WMI-based monitoring for OBS processes
fn start_wmi_monitoring(tx: mpsc::Sender<()>) -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    // Create WMI query to monitor process creation and deletion
    let wmi_query = r#"
        SELECT * FROM __InstanceDeletionEvent WITHIN 1 
        WHERE TargetInstance ISA 'Win32_Process' 
        AND (TargetInstance.Name = 'obs64.exe' OR TargetInstance.Name = 'Streamlabs OBS.exe')
    "#;
    
    log_and_print!("[OBS_MONITOR] Starting WMI event subscription");
    
    // Use PowerShell to subscribe to WMI events
    let powershell_script = format!(
        r#"
        $query = "{}"
        $watcher = New-Object System.Management.ManagementEventWatcher($query)
        $watcher.Start()
        Write-Host "WMI_WATCHER_READY"
        while ($true) {{
            try {{
                $event = $watcher.WaitForNextEvent()
                $processName = $event.TargetInstance.Name
                Write-Host "OBS_CLOSED:$processName"
                break
            }}
            catch {{
                Write-Host "WMI_ERROR:$($_.Exception.Message)"
                break
            }}
        }}
        $watcher.Stop()
        "#,
        wmi_query.replace("\n", " ").replace("  ", " ")
    );
    
    // Start PowerShell process
    let mut child = Command::new("powershell")
        .args(["-Command", &powershell_script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()?;
    
    // Track the PowerShell process for cleanup
    {
        let mut pids = POWERSHELL_PIDS.lock().unwrap();
        pids.push(child.id());
    }
    
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    
    // Handle stdout in a separate thread
    let tx_clone = tx.clone();
    thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);
        
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("WMI_WATCHER_READY") {
                    log_and_print!("[OBS_MONITOR] WMI watcher ready, monitoring for OBS process closure");
                } else if line.starts_with("OBS_CLOSED:") {
                    let process_name = line.trim_start_matches("OBS_CLOSED:");
                    log_and_print!("[OBS_MONITOR] WMI detected OBS process closed: {}", process_name);
                    let _ = tx_clone.send(());
                    break;
                } else if line.starts_with("WMI_ERROR:") {
                    let error = line.trim_start_matches("WMI_ERROR:");
                    log_and_print!("[OBS_MONITOR] WMI error: {}", error);
                    break;
                }
            }
        }
    });
    
    // Handle stderr in a separate thread
    thread::spawn(move || {
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
    thread::spawn(move || {
        if let Ok(status) = child.wait() {
            log_and_print!("[OBS_MONITOR] PowerShell WMI process exited with status: {}", status);
        }
    });
    
    Ok(())
}

/// Start WMI-based monitoring for OBS processes with direct exit
fn start_wmi_monitoring_direct_exit() -> Result<(), Box<dyn std::error::Error>> {
    use std::process::Command;
    
    // Create WMI query to monitor process creation and deletion
    let wmi_query = r#"
        SELECT * FROM __InstanceDeletionEvent WITHIN 1 
        WHERE TargetInstance ISA 'Win32_Process' 
        AND (TargetInstance.Name = 'obs64.exe' OR TargetInstance.Name = 'Streamlabs OBS.exe')
    "#;
    
    log_and_print!("[OBS_MONITOR] Starting WMI event subscription with direct exit");
    
    // Use PowerShell to subscribe to WMI events
    let powershell_script = format!(
        r#"
        $query = "{}"
        $watcher = New-Object System.Management.ManagementEventWatcher($query)
        $watcher.Start()
        Write-Host "WMI_WATCHER_READY"
        while ($true) {{
            try {{
                $event = $watcher.WaitForNextEvent()
                $processName = $event.TargetInstance.Name
                Write-Host "OBS_CLOSED:$processName"
                break
            }}
            catch {{
                Write-Host "WMI_ERROR:$($_.Exception.Message)"
                break
            }}
        }}
        $watcher.Stop()
        "#,
        wmi_query.replace("\n", " ").replace("  ", " ")
    );
    
    // Start PowerShell process
    let mut child = Command::new("powershell")
        .args(["-Command", &powershell_script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn()?;
    
    // Track the PowerShell process for cleanup
    {
        let mut pids = POWERSHELL_PIDS.lock().unwrap();
        pids.push(child.id());
    }
    
    let stdout = child.stdout.take().ok_or("Failed to capture stdout")?;
    let stderr = child.stderr.take().ok_or("Failed to capture stderr")?;
    
    // Handle stdout in a separate thread
    thread::spawn(move || {
        use std::io::{BufRead, BufReader};
        let reader = BufReader::new(stdout);
        
        for line in reader.lines() {
            if let Ok(line) = line {
                if line.contains("WMI_WATCHER_READY") {
                    log_and_print!("[OBS_MONITOR] WMI watcher ready, monitoring for OBS process closure");
                } else if line.starts_with("OBS_CLOSED:") {
                    let process_name = line.trim_start_matches("OBS_CLOSED:");
                    log_and_print!("[OBS_MONITOR] WMI detected OBS process closed: {}", process_name);
                    log_and_print!("[OBS_MONITOR] Direct exit triggered due to OBS shutdown");
                    
                    // Clean up PowerShell processes used for OBS monitoring
                    log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes due to OBS shutdown");
                    cleanup_powershell_processes();
                    
                    // Clean up traymond before exiting
                    log_and_print!("[TRAYMOND] Closing traymond-tcp due to OBS shutdown");
                    if let Err(e) = crate::traymond::exit_traymond() {
                        log_and_print!("[TRAYMOND] ERROR: Failed to exit traymond-tcp: {}", e);
                    }
                    
                    // Stop the Python bot
                    log_and_print!("[OBS_MONITOR] Stopping Python bot due to OBS shutdown");
                    crate::bot_manager::stop_bot_direct();
                    
                    // Shutdown logger
                    crate::log_util::shutdown_logger();
                    
                    // Exit directly
                    std::process::exit(0);
                } else if line.starts_with("WMI_ERROR:") {
                    let error = line.trim_start_matches("WMI_ERROR:");
                    log_and_print!("[OBS_MONITOR] WMI error: {}", error);
                    break;
                }
            }
        }
    });
    
    // Handle stderr in a separate thread
    thread::spawn(move || {
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
    thread::spawn(move || {
        if let Ok(status) = child.wait() {
            log_and_print!("[OBS_MONITOR] PowerShell WMI process exited with status: {}", status);
        }
    });
    
    Ok(())
}

/// Fallback polling-based monitoring with direct exit
fn start_polling_monitoring_direct_exit() {
    log_and_print!("[OBS_MONITOR] Starting fallback polling-based monitoring with direct exit");
    
    // Phase 1: Wait for OBS to start (check every 5 seconds)
    log_and_print!("[OBS_MONITOR] Waiting for OBS/Streamlabs to start...");
    while !is_obs_running() {
        thread::sleep(Duration::from_secs(5));
    }
    
    log_and_print!("[OBS_MONITOR] OBS/Streamlabs process found, starting close monitoring");
    
    // Phase 2: Monitor for OBS closing (check every 30 seconds)
    loop {
        thread::sleep(Duration::from_secs(FALLBACK_CHECK_INTERVAL_SECONDS));
        
        if !is_obs_running() {
            log_and_print!("[OBS_MONITOR] OBS/Streamlabs process closed, triggering direct exit");
            
            // Clean up PowerShell processes used for OBS monitoring
            log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes due to OBS shutdown");
            cleanup_powershell_processes();
            
            // Clean up traymond before exiting
            log_and_print!("[TRAYMOND] Closing traymond-tcp due to OBS shutdown");
            if let Err(e) = crate::traymond::exit_traymond() {
                log_and_print!("[TRAYMOND] ERROR: Failed to exit traymond-tcp: {}", e);
            }
            
            // Stop the Python bot
            log_and_print!("[OBS_MONITOR] Stopping Python bot due to OBS shutdown");
            crate::bot_manager::stop_bot_direct();
            
            // Shutdown logger
            crate::log_util::shutdown_logger();
            
            // Exit directly
            std::process::exit(0);
        }
    }
}

/// Fallback polling-based monitoring
fn start_polling_monitoring(tx: mpsc::Sender<()>) {
    log_and_print!("[OBS_MONITOR] Starting fallback polling-based monitoring");
    
    // Phase 1: Wait for OBS to start (check every 5 seconds)
    log_and_print!("[OBS_MONITOR] Waiting for OBS/Streamlabs to start...");
    while !is_obs_running() {
        thread::sleep(Duration::from_secs(5));
    }
    
    log_and_print!("[OBS_MONITOR] OBS/Streamlabs process found, starting close monitoring");
    
    // Phase 2: Monitor for OBS closing (check every 30 seconds)
    loop {
        thread::sleep(Duration::from_secs(FALLBACK_CHECK_INTERVAL_SECONDS));
        
        if !is_obs_running() {
            log_and_print!("[OBS_MONITOR] OBS/Streamlabs process closed, triggering shutdown");
            let _ = tx.send(());
            break;
        }
    }
}
