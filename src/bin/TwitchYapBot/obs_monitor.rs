//! OBS/Streamlabs monitoring for TwitchYapBot
//!
//! This module handles monitoring OBS and Streamlabs processes using PowerShell's
//! Wait-Process for efficient, real-time process lifecycle monitoring.

use std::sync::mpsc;
use std::process::Command;
use std::time::Duration;
use std::thread;
use std::sync::{Arc, Mutex};
use crate::log_and_print;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const OBS_PROCESSES: [&str; 2] = ["obs64.exe", "Streamlabs OBS.exe"];
const FALLBACK_CHECK_INTERVAL_SECONDS: u64 = 5;

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







/// Check if any OBS process is currently running (optimized fallback method)
pub fn is_obs_running() -> bool {
    // Use single tasklist call for both processes - more efficient
    if let Ok(output) = Command::new("tasklist")
        .args(["/FI", &format!("IMAGENAME eq {}", OBS_PROCESSES[0]), "/FI", &format!("IMAGENAME eq {}", OBS_PROCESSES[1])])
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .output() {
        
        let output_str = String::from_utf8_lossy(&output.stdout);
        return OBS_PROCESSES.iter().any(|process| output_str.contains(process));
    }
    false
}

/// Start OBS monitoring using Wait-Process
/// Returns a receiver that will receive a message when OBS closes
pub fn start_obs_monitoring() -> mpsc::Receiver<()> {
    let (tx, rx) = mpsc::channel();
    
    thread::spawn(move || {
        log_and_print!("[OBS_MONITOR] Starting OBS/Streamlabs monitoring");
        
        // Start monitoring using Wait-Process
        log_and_print!("[OBS_MONITOR] Starting monitoring");
        start_polling_monitoring(tx);
    });
    
    rx
}

/// Start OBS monitoring that directly exits the process when OBS closes
/// This is used when the app is minimized to tray to ensure immediate shutdown
pub fn start_obs_monitoring_with_direct_exit() {
    thread::spawn(move || {
        log_and_print!("[OBS_MONITOR] Starting OBS/Streamlabs monitoring with direct exit");
        
        // Start monitoring using Wait-Process
        log_and_print!("[OBS_MONITOR] Starting monitoring with direct exit");
        start_polling_monitoring_direct_exit();
    });
}





/// Monitoring with direct exit using Wait-Process
fn start_polling_monitoring_direct_exit() {
    log_and_print!("[OBS_MONITOR] Starting monitoring with direct exit using Wait-Process");
    
    // Phase 1: Wait for OBS to start using WaitForProgram function
    log_and_print!("[OBS_MONITOR] Waiting for OBS/Streamlabs to start...");
    
    // Phase 2: Use Wait-Process to monitor for OBS closing
    let wait_script = r#"
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
    
    if let Ok(mut child) = Command::new("powershell")
        .args(["-Command", wait_script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn() {
        
        // Track the PowerShell process for cleanup
        {
            let mut pids = POWERSHELL_PIDS.lock().unwrap();
            pids.push(child.id());
        }
        
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                log_and_print!("[OBS_MONITOR] Failed to capture stdout");
                return;
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                log_and_print!("[OBS_MONITOR] Failed to capture stderr");
                return;
            }
        };
        
        // Handle stdout in a separate thread
        thread::spawn(move || {
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
                        crate::bot_manager::stop_bot_direct();
                        
                        // Shutdown logger
                        crate::log_util::shutdown_logger();
                        
                        // Exit directly
                        std::process::exit(0);
                    } else if line.starts_with("OBS_PROCESS_NOT_FOUND") {
                        log_and_print!("[OBS_MONITOR] No OBS processes found, falling back to polling");
                        break;
                    } else if line.starts_with("WAIT_PROCESS_ERROR:") {
                        let error = line.trim_start_matches("WAIT_PROCESS_ERROR:");
                        log_and_print!("[OBS_MONITOR] Wait-Process error: {}", error);
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
                log_and_print!("[OBS_MONITOR] PowerShell Wait-Process exited with status: {}", status);
            }
        });
    } else {
        log_and_print!("[OBS_MONITOR] Failed to start Wait-Process monitoring, falling back to polling");
        // Fallback to the old polling method if Wait-Process fails
        loop {
            thread::sleep(Duration::from_secs(FALLBACK_CHECK_INTERVAL_SECONDS));
            
            if !is_obs_running() {
                log_and_print!("[OBS_MONITOR] OBS/Streamlabs process closed, triggering direct exit");
                
                // Clean up PowerShell processes used for OBS monitoring
                log_and_print!("[OBS_MONITOR] Cleaning up PowerShell processes due to OBS shutdown");
                cleanup_powershell_processes();
                

                
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
}

/// Monitoring using Wait-Process
fn start_polling_monitoring(tx: mpsc::Sender<()>) {
    log_and_print!("[OBS_MONITOR] Starting monitoring with Wait-Process");
    
    // Phase 1: Wait for OBS to start using WaitForProgram function
    log_and_print!("[OBS_MONITOR] Waiting for OBS/Streamlabs to start...");
    
    // Phase 2: Use Wait-Process to monitor for OBS closing
    let wait_script = r#"
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
    
    if let Ok(mut child) = Command::new("powershell")
        .args(["-Command", wait_script])
        .stdout(std::process::Stdio::piped())
        .stderr(std::process::Stdio::piped())
        .creation_flags(0x08000000) // CREATE_NO_WINDOW
        .spawn() {
        
        // Track the PowerShell process for cleanup
        {
            let mut pids = POWERSHELL_PIDS.lock().unwrap();
            pids.push(child.id());
        }
        
        let stdout = match child.stdout.take() {
            Some(stdout) => stdout,
            None => {
                log_and_print!("[OBS_MONITOR] Failed to capture stdout");
                return;
            }
        };
        let stderr = match child.stderr.take() {
            Some(stderr) => stderr,
            None => {
                log_and_print!("[OBS_MONITOR] Failed to capture stderr");
                return;
            }
        };
        
        // Handle stdout in a separate thread
        thread::spawn(move || {
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
                        let _ = tx.send(());
                        break;
                    } else if line.starts_with("OBS_PROCESS_NOT_FOUND") {
                        log_and_print!("[OBS_MONITOR] No OBS processes found, falling back to polling");
                        break;
                    } else if line.starts_with("WAIT_PROCESS_ERROR:") {
                        let error = line.trim_start_matches("WAIT_PROCESS_ERROR:");
                        log_and_print!("[OBS_MONITOR] Wait-Process error: {}", error);
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
                log_and_print!("[OBS_MONITOR] PowerShell Wait-Process exited with status: {}", status);
            }
        });
    } else {
        log_and_print!("[OBS_MONITOR] Failed to start Wait-Process monitoring, falling back to polling");
        // Fallback to the old polling method if Wait-Process fails
        loop {
            thread::sleep(Duration::from_secs(FALLBACK_CHECK_INTERVAL_SECONDS));
            
            if !is_obs_running() {
                log_and_print!("[OBS_MONITOR] OBS/Streamlabs process closed, triggering shutdown");
                let _ = tx.send(());
                break;
            }
        }
    }
}
