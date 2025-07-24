//! Asynchronous logging utility for TwitchYapBot
//!
//! This module provides a background-thread logger, log file rotation, and macro-based logging for the TwitchYapBot executable.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;
use chrono::Local;
use once_cell::sync::Lazy;
use crate::config::{get_log_dir, MAX_LOG_FILES};
use std::sync::mpsc::{self, Sender};
use std::thread;

static LOGGER: Lazy<Logger> = Lazy::new(|| Logger::init());

enum LogMsg {
    Line(String),
    Shutdown,
}

struct Logger {
    sender: Sender<LogMsg>,
}

fn rotate_logs(log_dir: &PathBuf) {
    if let Ok(entries) = fs::read_dir(&log_dir) {
        let mut log_files: Vec<_> = entries
            .filter_map(|e| e.ok())
            .filter(|e| e.path().extension().map(|ext| ext == "log").unwrap_or(false))
            .collect();
        log_files.sort_by_key(|e| e.metadata().and_then(|m| m.created()).ok());
        while log_files.len() > MAX_LOG_FILES {
            if let Some(oldest) = log_files.first() {
                let _ = fs::remove_file(oldest.path());
                log_files.remove(0);
            } else {
                break;
            }
        }
    }
}

impl Logger {
    fn init() -> Self {
        let (tx, rx) = mpsc::channel::<LogMsg>();
        // Determine log file path (same logic as before)
        let log_path = if let Ok(env_path) = std::env::var("YAPBOT_LOG_PATH") {
            PathBuf::from(env_path)
        } else {
            let log_dir = get_log_dir();
            if !log_dir.exists() {
                let _ = fs::create_dir_all(&log_dir);
            }
            rotate_logs(&log_dir);
            let now = Local::now();
            let filename = now.format("%m-%d-%y_%H-%M-%S.log").to_string();
            log_dir.join(filename)
        };
        // Spawn background thread
        thread::spawn(move || {
            let log_dir = log_path.parent().unwrap().to_path_buf();
            let mut file = OpenOptions::new().create(true).append(true).open(&log_path).expect("Failed to open log file");
            rotate_logs(&log_dir);
            while let Ok(msg) = rx.recv() {
                match msg {
                    LogMsg::Line(line) => {
                        let _ = writeln!(file, "{}", line);
                        rotate_logs(&log_dir);
                    }
                    LogMsg::Shutdown => {
                        let _ = file.flush();
                        break;
                    }
                }
            }
        });
        Logger { sender: tx }
    }
    fn log(&self, line: String) {
        let _ = self.sender.send(LogMsg::Line(line));
    }
    fn shutdown(&self) {
        let _ = self.sender.send(LogMsg::Shutdown);
    }
}

pub fn log_message(msg: &str) {
    LOGGER.log(msg.to_string());
}

#[macro_export]
macro_rules! log_and_print {
    ($($arg:tt)*) => {{
        let msg = format!($($arg)*);
        $crate::log_util::log_and_print(&msg);
    }};
}

pub fn log_and_print(msg: &str) {
    let now = chrono::Local::now();
    let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
    let full_msg = format!("[TwitchYapBot.exe] {} {}", timestamp, msg);
    log_message(&full_msg);
    if cfg!(debug_assertions) {
        println!("{}", full_msg);
    }
}

/// Call this on shutdown to flush the log
pub fn shutdown_logger() {
    LOGGER.shutdown();
} 