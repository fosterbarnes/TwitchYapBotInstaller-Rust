//! Bot process control and management for TwitchYapBot
//!
//! This module handles starting, stopping, and restarting the MarkovChainBot.py process for the TwitchYapBot executable.
//!
//! Responsibilities:
//! - Launch the Python MarkovChainBot as a subprocess
//! - Capture and relay stdout/stderr output to the GUI
//! - Handle process termination and restart logic
//! - Log all relevant events for debugging and auditing

use std::sync::{Arc, Mutex};
use std::process::{Child, Command, Stdio};
use std::sync::mpsc;
use chrono::Local;
use std::io::{BufRead, BufReader};
use std::thread;
#[cfg(windows)]
use std::os::windows::process::CommandExt;
use std::collections::VecDeque;

use crate::gui::TwitchYapBotApp;
use crate::log_util;
use crate::log_and_print;
use crate::config::TWITCH_MARKOVCHAIN_DIR; // Centralized directory name for MarkovChainBot resources

/// Stop the running MarkovChainBot process, if any.
///
/// On Windows, uses taskkill to ensure the process is terminated.
/// Logs all actions and updates the GUI state.
pub fn stop_bot(app: &mut TwitchYapBotApp) {
    #[cfg(windows)]
    {
        if let Some(pid) = app.child_pid {
            let now = chrono::Local::now();
            let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
            let msg = format!("{} [DEBUG] Ran: taskkill /PID {} /F /T", timestamp, pid);
            log_util::log_message(&msg);
            if cfg!(debug_assertions) {
                println!("{} [DEBUG] Ran: taskkill /PID {} /F /T", timestamp, pid);
            }
            log_and_print!("[DEBUG] Ran: taskkill /PID {} /F /T", pid);
            let tk_result = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F", "/T"]).output();
            if let Ok(ref out) = tk_result {
                let result_str = String::from_utf8_lossy(&out.stdout).trim().to_string();
                let now = chrono::Local::now();
                let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
                for line in result_str.lines() {
                    let msg = format!("{} {}", timestamp, line.trim());
                    log_util::log_message(&msg);
                    if cfg!(debug_assertions) {
                        println!("{} {}", timestamp, line.trim());
                    }
                }
            }
        }
    }
    // Clear process state and log the event
    app.child = None;
    app.child_pid = None;
    let msg = format!("Yap Bot has been destroyed by your own hands...");
    log_and_print!("{}", msg);
    app.output_lines.lock().unwrap().push_back(format!("[{}] {}", chrono::Local::now().format("%m/%d/%Y - %H:%M:%S"), msg));
}

/// Restart the MarkovChainBot process, optionally logging a custom message.
///
/// Stops any existing process, spawns a new one, and updates the GUI state.
pub fn restart_bot(app: &mut TwitchYapBotApp, output_msg: &str) {
    if app.child.is_some() || app.child_pid.is_some() {
        stop_bot(app);
    }
    // Set up new output channels and spawn the bot in a background thread
    let output_lines = app.output_lines.clone();
    let (tx, rx) = mpsc::channel();
    let output_lines_clone = output_lines.clone();
    let (child_sender, child_receiver) = mpsc::channel();
    thread::spawn(move || {
        let (child_arc, pid) = run_markov_chain_bot(tx, output_lines_clone);
        let _ = child_sender.send((child_arc, pid));
    });
    let (child, child_pid) = child_receiver.recv().unwrap_or((None, None));
    app.child = child;
    app.child_pid = child_pid;
    app.rx = Some(rx);
    app.marker_index = None;
    let now = Local::now();
    let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
    app.output_lines.lock().unwrap().push_back(format!("{} {}", timestamp, output_msg));
    let _ = app.settings_dialog.load_settings();
}

/// Launch the MarkovChainBot.py process as a subprocess, capturing its output.
///
/// - Sets up the working directory and environment variables
/// - Spawns the process and captures both stdout and stderr
/// - Forwards output lines to the GUI and logs them
/// - Handles process exit and cleanup in background threads
///
/// Returns: (child process handle, process ID)
pub fn run_markov_chain_bot(
    tx: mpsc::Sender<String>,
    output_lines: Arc<Mutex<VecDeque<String>>>,
) -> (Option<Arc<Mutex<Child>>>, Option<u32>) {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    // Construct the working directory for the MarkovChainBot.py process using the centralized constant
    let workdir = std::path::PathBuf::from(format!("{}\\YapBot\\{}", appdata, TWITCH_MARKOVCHAIN_DIR));
    let python = which::which("python").unwrap_or_else(|_| "python".into());
    let script = "MarkovChainBot.py";
    println!("\n--- Twitch Yap Bot Run ---\n");
    log_util::log_message("--- Twitch Yap Bot Run ---");
    let mut cmd = Command::new(python);
    cmd.arg("-u")
        .arg(script)
        .current_dir(&workdir)
        .env("PYTHONUNBUFFERED", "1")
        .env("PYTHONIOENCODING", "utf-8")
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    #[cfg(windows)]
    {
        cmd.creation_flags(0x08000000); // CREATE_NO_WINDOW
    }
    // Spawn the process and wrap it in Arc<Mutex<Child>> for thread-safe access
    let child = cmd.spawn().expect("Failed to start MarkovChainBot.py");
    let pid = Some(child.id());
    let child_arc = Arc::new(Mutex::new(child));
    // Capture stdout and forward lines to the GUI and log
    let stdout = child_arc.lock().unwrap().stdout.take().unwrap();
    let stderr = child_arc.lock().unwrap().stderr.take().unwrap();
    let tx3 = tx.clone();
    let out_lines = output_lines.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                // Filter out manual trigger lines for clarity
                if !line.contains("Generate command triggered manually") {
                    log_util::log_message(&format!("[MarkovChainBot.py] {}", line));
                    if cfg!(debug_assertions) {
                        println!("[MarkovChainBot.py] {}", line);
                    }
                    let _ = tx3.send(line.clone());
                    out_lines.lock().unwrap().push_back(line);
                }
            }
        }
    });
    // Capture stderr and forward lines to the GUI and log
    let tx2 = tx.clone();
    let out_lines = output_lines.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.contains("Generate command triggered manually") {
                    log_util::log_message(&format!("[MarkovChainBot.py][stderr] {}", line));
                    if cfg!(debug_assertions) {
                        println!("[MarkovChainBot.py][stderr] {}", line);
                    }
                    let _ = tx2.send(line.clone());
                    out_lines.lock().unwrap().push_back(line);
                }
            }
        }
    });
    // Wait for the process to exit and log the exit status
    let child_arc_clone = child_arc.clone();
    thread::spawn(move || {
        let status = child_arc_clone.lock().unwrap().wait().expect("Failed to wait on child");
        log_util::log_message(&format!("[MarkovChainBot.py] exited with status: {}", status));
        if cfg!(debug_assertions) {
            println!("[MarkovChainBot.py] exited with status: {}", status);
        }
    });
    (Some(child_arc), pid)
} 