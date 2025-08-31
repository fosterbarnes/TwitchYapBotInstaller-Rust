use std::process::{Command, Stdio};
use std::sync::{Arc, Mutex};
use std::thread::{sleep, spawn};
use std::time::Duration;
use once_cell::sync::Lazy;

use yap_bot_installer::log_and_print;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

const CREATE_NO_WINDOW: u32 = 0x08000000;
const OBS_PROCESSES: [&str; 2] = ["obs64.exe", "Streamlabs OBS.exe"];
const FALLBACK_CHECK_INTERVAL_SECONDS: u64 = 5;

// Global state to track PowerShell process IDs for cleanup
static POWERSHELL_PIDS: Lazy<Arc<Mutex<Vec<u32>>>> = 
    Lazy::new(|| Arc::new(Mutex::new(Vec::new())));

/// Clean up all tracked PowerShell processes
pub fn cleanup_powershell_processes() {
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
pub fn is_obs_running() -> bool {
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
pub fn start_obs_monitoring_with_direct_exit<F>(stop_bot_callback: F) 
where 
    F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + 'static
{
    spawn(move || {
        log_and_print!("[OBS_MONITOR] Starting OBS/Streamlabs monitoring with direct exit");
        start_powershell_monitoring_direct_exit(stop_bot_callback);
    });
}

/// Monitoring with direct exit using PowerShell WaitForProgram and Wait-Process
fn start_powershell_monitoring_direct_exit<F>(stop_bot_callback: F) 
where 
    F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + 'static
{
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
            start_polling_monitoring_direct_exit(stop_bot_callback);
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
                    if let Err(e) = stop_bot_callback() {
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
fn start_polling_monitoring_direct_exit<F>(stop_bot_callback: F) 
where 
    F: Fn() -> Result<(), Box<dyn std::error::Error>> + Send + 'static
{
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
            if let Err(e) = stop_bot_callback() {
                log_and_print!("[OBS_MONITOR] Failed to stop bot: {}", e);
            }
            
            // Exit directly
            std::process::exit(0);
        }
    }
}
