// Bot process control for TwitchYapBot
// Handles starting, stopping, and restarting the Python bot process

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

pub fn stop_bot(app: &mut TwitchYapBotApp) {
    #[cfg(windows)]
    {
        if let Some(pid) = app.child_pid {
            let tk_result = Command::new("taskkill").args(["/PID", &pid.to_string(), "/F", "/T"]).output();
            println!("[GUI] Ran: taskkill /PID {} /F /T", pid);
            if let Ok(ref out) = tk_result {
                println!("[GUI] taskkill result: {}", String::from_utf8_lossy(&out.stdout).trim());
            }
        }
    }
    #[cfg(unix)]
    {
        if let Some(child_arc) = &app.child {
            let mut child = child_arc.lock().unwrap();
            let _ = child.kill();
        }
    }
    app.child = None;
    app.child_pid = None;
    let now = Local::now();
    let timestamp = now.format("[%m/%d/%Y - %H:%M:%S]:");
    app.output_lines.lock().unwrap().push_back(format!("{} Yap Bot has been destroyed by your own hands...", timestamp));
}

pub fn restart_bot(app: &mut TwitchYapBotApp, output_msg: &str) {
    if app.child.is_some() || app.child_pid.is_some() {
        stop_bot(app);
    }
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

pub fn run_markov_chain_bot(
    tx: mpsc::Sender<String>,
    output_lines: Arc<Mutex<VecDeque<String>>>,
) -> (Option<Arc<Mutex<Child>>>, Option<u32>) {
    let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
    let workdir = std::path::PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain", appdata));
    let python = which::which("python").unwrap_or_else(|_| "python".into());
    let script = "MarkovChainBot.py";
    println!("\n--- Twitch Yap Bot Run ---\n");
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
    let child = cmd.spawn().expect("Failed to start MarkovChainBot.py");
    let pid = Some(child.id());
    let child_arc = Arc::new(Mutex::new(child));
    let stdout = child_arc.lock().unwrap().stdout.take().unwrap();
    let stderr = child_arc.lock().unwrap().stderr.take().unwrap();
    let tx3 = tx.clone();
    let out_lines = output_lines.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stdout);
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.contains("Generate command triggered manually") {
                    println!("[MarkovChainBot.py] {}", line);
                    let _ = tx3.send(line.clone());
                    out_lines.lock().unwrap().push_back(line);
                }
            }
        }
    });
    let tx2 = tx.clone();
    let out_lines = output_lines.clone();
    thread::spawn(move || {
        let reader = BufReader::new(stderr);
        for line in reader.lines() {
            if let Ok(line) = line {
                if !line.contains("Generate command triggered manually") {
                    println!("[MarkovChainBot.py][stderr] {}", line);
                    let _ = tx2.send(line.clone());
                    out_lines.lock().unwrap().push_back(line);
                }
            }
        }
    });
    let child_arc_clone = child_arc.clone();
    thread::spawn(move || {
        let status = child_arc_clone.lock().unwrap().wait().expect("Failed to wait on child");
        println!("[MarkovChainBot.py] exited with status: {}", status);
    });
    (Some(child_arc), pid)
} 