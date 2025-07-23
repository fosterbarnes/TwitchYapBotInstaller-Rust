// IPC server logic for TwitchYapBot
// Handles listening to restart requests from the settings window

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::thread;
use std::io::Read;

pub fn start_ipc_server(ipc_restart_flag: Arc<AtomicBool>) {
    thread::spawn(move || {
        use std::net::TcpListener;
        let listener = TcpListener::bind("127.0.0.1:9876").expect("Failed to bind IPC port");
        for stream in listener.incoming() {
            if let Ok(mut stream) = stream {
                let mut buf = [0u8; 32];
                if let Ok(n) = stream.read(&mut buf) {
                    let msg = String::from_utf8_lossy(&buf[..n]);
                    if msg.trim() == "RESTART_BOT" {
                        ipc_restart_flag.store(true, Ordering::SeqCst);
                    }
                }
            }
        }
    });
} 