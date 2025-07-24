//! YapBot Updater Logic
//!
//! Contains the asynchronous update logic for downloading and replacing files needed by YapBot.
//! Handles progress reporting and error management for the update process.

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;

pub enum UpdateError {
    Network(String),
    Io(String),
    Other(String),
}

pub struct UpdateProgress {
    pub file: String,
    pub progress: f32,
    pub status: String,
}

/// Download a file from a URL and save it to the given path.
pub async fn download_file(
    url: &str,
    dest: &PathBuf,
    mut progress_callback: impl FnMut(f32) + Send + Sync,
) -> Result<(), UpdateError> {
    let client = reqwest::Client::new();
    let mut response = client.get(url).send().await.map_err(|e| UpdateError::Network(e.to_string()))?;
    let total_size = response.content_length().unwrap_or(0);
    let mut file = tokio::fs::File::create(dest)
        .await
        .map_err(|e| UpdateError::Io(e.to_string()))?;
    let mut downloaded: u64 = 0;
    while let Some(chunk) = response.chunk().await.map_err(|e| UpdateError::Network(e.to_string()))? {
        file.write_all(&chunk)
            .await
            .map_err(|e| UpdateError::Io(e.to_string()))?;
        downloaded += chunk.len() as u64;
        if total_size > 0 {
            progress_callback(downloaded as f32 / total_size as f32);
        }
    }
    Ok(())
}

/// Replace an existing file with a new one (move or copy over).
pub fn replace_file(
    _src: &PathBuf,
    _dest: &PathBuf,
) -> Result<(), UpdateError> {
    // TODO: Implement file replacement logic
    Ok(())
}

/// Main update function: downloads and replaces all required files.
pub async fn perform_update(
    mut progress_callback: impl FnMut(UpdateProgress) + Send + Sync,
) -> Result<(), UpdateError> {
    use std::env;
    // Get AppData path
    let appdata = env::var("APPDATA").map_err(|e| UpdateError::Other(e.to_string()))?;
    let yapbot_dir = PathBuf::from(format!("{}\\YapBot", appdata));
    let markov_dir = yapbot_dir.join("TwitchMarkovChain");
    // Ensure directories exist
    tokio::fs::create_dir_all(&yapbot_dir).await.map_err(|e| UpdateError::Io(e.to_string()))?;
    tokio::fs::create_dir_all(&markov_dir).await.map_err(|e| UpdateError::Io(e.to_string()))?;

    // List of files to download: (url, dest_path, display_name)
    let files = vec![
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchYapBot.exe",
            yapbot_dir.join("TwitchYapBot.exe"),
            "TwitchYapBot.exe",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchMarkovChain/Database.py",
            markov_dir.join("Database.py"),
            "Database.py",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchMarkovChain/Log.py",
            markov_dir.join("Log.py"),
            "Log.py",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchMarkovChain/MarkovChainBot.py",
            markov_dir.join("MarkovChainBot.py"),
            "MarkovChainBot.py",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchMarkovChain/Timer.py",
            markov_dir.join("Timer.py"),
            "Timer.py",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchMarkovChain/Tokenizer.py",
            markov_dir.join("Tokenizer.py"),
            "Tokenizer.py",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchMarkovChain/requirements.txt",
            markov_dir.join("requirements.txt"),
            "requirements.txt",
        ),
    ];
    let total = files.len();
    for (idx, (url, dest, display_name)) in files.into_iter().enumerate() {
        let status = format!("Downloading {}...", display_name);
        progress_callback(UpdateProgress {
            file: display_name.to_string(),
            progress: idx as f32 / total as f32,
            status: status.clone(),
        });
        let mut last_progress = 0.0f32;
        download_file(url, &dest, |p| {
            // p is 0.0..1.0 for this file
            let overall = (idx as f32 + p) / total as f32;
            if (overall - last_progress).abs() > 0.01 {
                progress_callback(UpdateProgress {
                    file: display_name.to_string(),
                    progress: overall,
                    status: status.clone(),
                });
                last_progress = overall;
            }
        })
        .await?;
    }
    progress_callback(UpdateProgress {
        file: "All files".to_string(),
        progress: 1.0,
        status: "Update complete!".to_string(),
    });
    Ok(())
}
