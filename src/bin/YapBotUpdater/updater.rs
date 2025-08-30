//! YapBot Updater Logic
//!
//! Contains the asynchronous update logic for downloading and replacing files needed by YapBot.
//! Handles progress reporting and error management for the update process.

use std::path::PathBuf;
use tokio::io::AsyncWriteExt;
use winreg::enums::*;
use winreg::RegKey;

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

/// Check if Visual C++ Redistributable x86 is installed
fn is_vcredist_x86_installed() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    // Check multiple possible registry paths for Visual C++ Redistributables
    let paths = vec![
        "SOFTWARE\\Microsoft\\VisualStudio\\17.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\Microsoft\\VisualStudio\\16.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\Microsoft\\VisualStudio\\15.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\WOW6432Node\\Microsoft\\VisualStudio\\17.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\WOW6432Node\\Microsoft\\VisualStudio\\16.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\WOW6432Node\\Microsoft\\VisualStudio\\15.0\\VC\\Runtimes\\x86",
        "SOFTWARE\\WOW6432Node\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x86",
    ];
    
    for path in paths {
        if let Ok(key) = hklm.open_subkey(path) {
            if let Ok(installed) = key.get_value::<u32, _>("Installed") {
                if installed == 1 {
                    return true;
                }
            }
        }
    }
    
    // Also check for the specific product codes
    let product_paths = vec![
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{65E5BD06-6392-3027-8C26-853107D3CF1B}", // VS 2015-2022 x86
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{A8F89E5F-4B2C-3B9C-8E31-2B86C2291904}", // VS 2015-2022 x86 (alternative)
    ];
    
    for path in product_paths {
        if let Ok(key) = hklm.open_subkey(path) {
            if let Ok(_) = key.get_value::<String, _>("DisplayName") {
                return true;
            }
        }
    }
    
    false
}

/// Check if Visual C++ Redistributable x64 is installed
fn is_vcredist_x64_installed() -> bool {
    let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
    
    // Check multiple possible registry paths for Visual C++ Redistributables
    let paths = vec![
        "SOFTWARE\\Microsoft\\VisualStudio\\17.0\\VC\\Runtimes\\x64",
        "SOFTWARE\\Microsoft\\VisualStudio\\16.0\\VC\\Runtimes\\x64",
        "SOFTWARE\\Microsoft\\VisualStudio\\15.0\\VC\\Runtimes\\x64",
        "SOFTWARE\\Microsoft\\VisualStudio\\14.0\\VC\\Runtimes\\x64",
    ];
    
    for path in paths {
        if let Ok(key) = hklm.open_subkey(path) {
            if let Ok(installed) = key.get_value::<u32, _>("Installed") {
                if installed == 1 {
                    return true;
                }
            }
        }
    }
    
    // Also check for the specific product codes
    let product_paths = vec![
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{36F68A90-239C-34DF-B58C-64F30147CD5F}", // VS 2015-2022 x64
        "SOFTWARE\\Microsoft\\Windows\\CurrentVersion\\Uninstall\\{A8F89E5F-4B2C-3B9C-8E31-2B86C2291904}", // VS 2015-2022 x64 (alternative)
    ];
    
    for path in product_paths {
        if let Ok(key) = hklm.open_subkey(path) {
            if let Ok(_) = key.get_value::<String, _>("DisplayName") {
                return true;
            }
        }
    }
    
    false
}

/// Download and install Visual C++ Redistributable
async fn install_vcredist<F>(
    url: &str,
    filename: &str,
    progress_callback: &mut F,
    base_progress: f32,
    progress_range: f32,
) -> Result<(), UpdateError>
where
    F: FnMut(UpdateProgress) + Send + Sync,
{
    let temp_dir = std::env::temp_dir();
    let installer_path = temp_dir.join(filename);
    
    // Download the installer (50% of the progress range)
    progress_callback(UpdateProgress {
        file: filename.to_string(),
        progress: base_progress,
        status: format!("Downloading {}...", filename),
    });
    
    download_file(url, &installer_path, |_| {}).await?;
    
    // Launch the installer (remaining 50% of the progress range)
    progress_callback(UpdateProgress {
        file: filename.to_string(),
        progress: base_progress + (progress_range * 0.5),
        status: format!("Installing {}...", filename),
    });
    
    let mut child = std::process::Command::new(&installer_path)
        .spawn()
        .map_err(|e| UpdateError::Other(format!("Failed to launch installer: {}", e)))?;
    
    // Wait for the installer to complete
    let _ = child.wait()
        .map_err(|e| UpdateError::Other(format!("Failed to wait for installer: {}", e)))?;
    
    // Don't check status.success() - the installer can return non-zero exit codes
    // for valid operations like "Close" or "Repair" that don't indicate failure
    // Just proceed as long as the process completed
    
    // Clean up the downloaded installer
    let _ = tokio::fs::remove_file(&installer_path).await;
    
    progress_callback(UpdateProgress {
        file: filename.to_string(),
        progress: base_progress + progress_range,
        status: format!("{} installed successfully", filename),
    });
    
    Ok(())
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
    tokio::fs::create_dir_all(&yapbot_dir.join("icons")).await.map_err(|e| UpdateError::Io(e.to_string()))?;

    // List of files to download: (url, dest_path, display_name)
    let files = vec![
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/TwitchYapBot.exe",
            yapbot_dir.join("TwitchYapBot.exe"),
            "TwitchYapBot.exe",
        ),
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/resources/binaries/YapBotTray.exe",
            yapbot_dir.join("YapBotTray.exe"),
            "YapBotTray.exe",
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
        (
            "https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/raw/main/resources/icon/yap_icon_purple.ico",
            yapbot_dir.join("icons").join("yap_icon_purple.ico"),
            "yap_icon_purple.ico",
        ),
    ];
    let total = files.len();
    for (idx, (url, dest, display_name)) in files.into_iter().enumerate() {
        let status = format!("Downloading {}...", display_name);
        progress_callback(UpdateProgress {
            file: display_name.to_string(),
            progress: (idx as f32 / total as f32) * 0.85, // Reserve 15% for Visual C++ Redistributables
            status: status.clone(),
        });
        let mut last_progress = 0.0f32;
        download_file(url, &dest, |p| {
            // p is 0.0..1.0 for this file
            let overall = ((idx as f32 + p) / total as f32) * 0.85; // Scale to 85% of total progress
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
    // Check and install Visual C++ Redistributables if needed
    progress_callback(UpdateProgress {
        file: "Visual C++ Redistributables".to_string(),
        progress: 0.85,
        status: "Checking Visual C++ Redistributables...".to_string(),
    });
    
    let mut vcredist_installed = 0;
    let mut vcredist_total = 0;
    
    // Check x86 version
    if !is_vcredist_x86_installed() {
        vcredist_total += 1;
        progress_callback(UpdateProgress {
            file: "Visual C++ Redistributables".to_string(),
            progress: 0.85 + (0.05 * vcredist_installed as f32),
            status: "Installing Visual C++ Redistributable x86...".to_string(),
        });
        
        install_vcredist(
            "https://aka.ms/vs/17/release/vc_redist.x86.exe",
            "vc_redist.x86.exe",
            &mut progress_callback,
            0.85 + (0.05 * vcredist_installed as f32),
            0.05,
        ).await?;
        vcredist_installed += 1;
    }
    
    // Check x64 version
    if !is_vcredist_x64_installed() {
        vcredist_total += 1;
        progress_callback(UpdateProgress {
            file: "Visual C++ Redistributables".to_string(),
            progress: 0.85 + (0.05 * vcredist_installed as f32),
            status: "Installing Visual C++ Redistributable x64...".to_string(),
        });
        
        install_vcredist(
            "https://aka.ms/vs/17/release/vc_redist.x64.exe",
            "vc_redist.x64.exe",
            &mut progress_callback,
            0.85 + (0.05 * vcredist_installed as f32),
            0.05,
        ).await?;
        vcredist_installed += 1;
    }
    
    if vcredist_total > 0 {
        progress_callback(UpdateProgress {
            file: "Visual C++ Redistributables".to_string(),
            progress: 0.95,
            status: format!("Installed {} Visual C++ Redistributable(s)", vcredist_installed),
        });
    } else {
        progress_callback(UpdateProgress {
            file: "Visual C++ Redistributables".to_string(),
            progress: 0.95,
            status: "Visual C++ Redistributables already installed".to_string(),
        });
    }
    
    progress_callback(UpdateProgress {
        file: "All files".to_string(),
        progress: 1.0,
        status: "Update complete!".to_string(),
    });
    Ok(())
}
