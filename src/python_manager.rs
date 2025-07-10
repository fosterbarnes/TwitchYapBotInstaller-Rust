//! Python installation and management utilities for YapBot Installer
//! 
//! This module handles Python installation and environment configuration.

use std::env;
use std::io;
use std::io::Write;
use std::path::PathBuf;
use std::process::{Command, Stdio};

// Windows-specific imports
#[cfg(windows)]
use std::fs::File;
#[cfg(windows)]
use std::ptr::null_mut;
#[cfg(windows)]
use winreg::enums::*;
#[cfg(windows)]
use winreg::RegKey;
#[cfg(windows)]
use windows::Win32::Foundation::{WPARAM, LPARAM};
#[cfg(windows)]
use windows::Win32::UI::WindowsAndMessaging::{SendMessageTimeoutW, HWND_BROADCAST, WM_SETTINGCHANGE, SMTO_ABORTIFHUNG};

// Linux-specific imports
#[cfg(not(windows))]
use dirs;

/// Python installation and management utilities
pub struct PythonManager;

impl PythonManager {
    /// Check if Python is installed and return its version
    pub fn get_version() -> Option<String> {
        // On Linux, check python3 first, then python, then py
        for cmd in &["python3", "python", "py"] {
            if let Ok(output) = Self::run_command_hidden(cmd, &["--version"], &std::collections::HashMap::new()) {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    let stderr = String::from_utf8_lossy(&output.stderr).trim().to_string();
                    let version = if !stdout.is_empty() { stdout } else { stderr };
                    // Only accept Python 3.x.y
                    if version.starts_with("Python 3.") {
                        return Some(version);
                    }
                }
            }
        }
        None
    }

    /// Run a command with hidden output
    pub fn run_command_hidden(cmd: &str, args: &[&str], env_vars: &std::collections::HashMap<String, String>) -> io::Result<std::process::Output> {
        let mut command = Command::new(cmd);
        command.args(args);
        command.envs(env_vars);
        command.stdout(Stdio::piped());
        command.stderr(Stdio::piped());
        #[cfg(windows)]
        {
            use windows::Win32::System::Threading::CREATE_NO_WINDOW;
            use std::os::windows::process::CommandExt;
            command.creation_flags(CREATE_NO_WINDOW.0);
        }
        command.output()
    }

    /// Download Python installer (Windows only)
    #[cfg(windows)]
    pub fn download_installer() -> io::Result<PathBuf> {
        let url = "https://www.python.org/ftp/python/3.13.5/python-3.13.5-amd64.exe";
        let response = reqwest::blocking::get(url).map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;

        let temp_dir = env::temp_dir();
        let installer_path = temp_dir.join("python-installer.exe");
        let mut file = File::create(&installer_path)?;
        let bytes = response.bytes().map_err(|e| io::Error::new(io::ErrorKind::Other, e))?;
        file.write_all(&bytes)?;
        Ok(installer_path)
    }

    /// Install Python silently (Windows only)
    #[cfg(windows)]
    pub fn install_silent(installer_path: &PathBuf) -> io::Result<bool> {
        let mut command = Command::new(installer_path);
        command.args(&[
            "/quiet",
            "InstallAllUsers=1",
            "PrependPath=1",
            "Include_pip=1",
        ]);
        
        // On Windows, try to hide the console window
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000); // CREATE_NO_WINDOW
        
        let status = command.status()?;
        Ok(status.success())
    }

    /// Refresh environment variables (Windows only)
    #[cfg(windows)]
    pub fn refresh_environment() -> Result<(), String> {
        // Get the updated PATH from registry
        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu.open_subkey_with_flags("Environment", KEY_READ)
            .map_err(|e| format!("Failed to open registry: {}", e))?;

        let user_path: String = env.get_value("Path").unwrap_or_else(|_| "".into());
        
        // Get system PATH
        let hklm = RegKey::predef(HKEY_LOCAL_MACHINE);
        let sys_env = hklm.open_subkey_with_flags("SYSTEM\\CurrentControlSet\\Control\\Session Manager\\Environment", KEY_READ)
            .map_err(|e| format!("Failed to open system registry: {}", e))?;
        
        let system_path: String = sys_env.get_value("Path").unwrap_or_else(|_| "".into());
        
        // Combine system and user paths
        let combined_path = if system_path.trim().is_empty() {
            user_path
        } else if user_path.trim().is_empty() {
            system_path
        } else {
            format!("{system_path};{user_path}")
        };
        
        // Update current process environment
        std::env::set_var("PATH", combined_path);
        
        Ok(())
    }

    /// Add Python Scripts directory to PATH (Windows only)
    #[cfg(windows)]
    pub fn add_scripts_to_path() -> Result<(), String> {
        let mut base_path = None;

        for cmd in &["python", "py"] {
            let output = Self::run_command_hidden(cmd, &["-m", "site", "--user-base"], &std::collections::HashMap::new());

            match output {
                Ok(output) if output.status.success() => {
                    let path = String::from_utf8_lossy(&output.stdout).trim().to_string();
                    if !path.is_empty() {
                        base_path = Some(path);
                        break;
                    }
                }
                Ok(output) => {
                    let err = String::from_utf8_lossy(&output.stderr);
                    eprintln!("Failed to get user base with {}: {}", cmd, err);
                }
                Err(e) => {
                    eprintln!("Failed to execute {}: {}", cmd, e);
                }
            }
        }

        let base_path = base_path.ok_or_else(|| "Failed to get user base path from python/py".to_string())?;
        let scripts_path = format!("{}\\Scripts", base_path);

        let hkcu = RegKey::predef(HKEY_CURRENT_USER);
        let env = hkcu.open_subkey_with_flags("Environment", KEY_READ | KEY_WRITE)
            .map_err(|e| format!("Failed to open registry: {}", e))?;

        let current_path: String = env.get_value("Path").unwrap_or_else(|_| "".into());

        if !current_path.to_lowercase().contains(&scripts_path.to_lowercase()) {
            let new_path = if current_path.trim().is_empty() {
                scripts_path.clone()
            } else {
                format!("{current_path};{scripts_path}")
            };

            env.set_value("Path", &new_path)
                .map_err(|e| format!("Failed to set PATH: {}", e))?;

            unsafe {
                let param = "Environment\0"
                    .encode_utf16()
                    .collect::<Vec<u16>>();

                SendMessageTimeoutW(
                    HWND_BROADCAST,
                    WM_SETTINGCHANGE,
                    WPARAM(0),
                    LPARAM(param.as_ptr() as isize),
                    SMTO_ABORTIFHUNG,
                    5000,
                    Some(null_mut()),
                );
            }
        }

        Ok(())
    }

    /// Install YapBot dependencies
    pub fn install_dependencies() -> Result<(), String> {
        // Write embedded requirements.txt to a temp file
        let temp_dir = std::env::temp_dir();
        let req_path = temp_dir.join("yapbot_requirements.txt");
        if let Err(e) = std::fs::write(&req_path, include_str!("../TwitchMarkovChain/requirements.txt")) {
            return Err(format!("Failed to write requirements.txt to temp: {}", e));
        }
        let requirements_url = req_path.to_str().unwrap();
        let pip_commands = ["pip", "pip3", "python", "py"];
        let mut last_error = String::new();
        for pip_cmd in &pip_commands {
            let mut command = if *pip_cmd == "python" || *pip_cmd == "py" {
                let mut cmd = std::process::Command::new(pip_cmd);
                cmd.args(["-m", "pip", "install", "-r", requirements_url]);
                cmd
            } else {
                let mut cmd = std::process::Command::new(pip_cmd);
                cmd.args(["install", "-r", requirements_url]);
                cmd
            };
            command.stdout(std::process::Stdio::piped());
            command.stderr(std::process::Stdio::piped());
            #[cfg(windows)]
            {
                use windows::Win32::System::Threading::CREATE_NO_WINDOW;
                use std::os::windows::process::CommandExt;
                command.creation_flags(CREATE_NO_WINDOW.0);
            }
            match command.output() {
                Ok(output) => {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    println!("PIP OUTPUT for '{}':", pip_cmd);
                    println!("STDOUT: {}", stdout);
                    println!("STDERR: {}", stderr);
                    println!("STATUS: {}", output.status);
                    if output.status.success() {
                        println!("PIP SUCCESS: Dependencies installed successfully!");
                        let _ = std::fs::remove_file(&req_path); // Clean up temp file
                        // After requirements install, run nltk downloads
                        let python_cmds = [
                            ["-c", "import nltk; nltk.download('punkt_tab'); nltk.download('punkt')"],
                        ];
                        let mut nltk_success = false;
                        for py in ["python", "py", "python3"] {
                            for args in &python_cmds {
                                let mut cmd = std::process::Command::new(py);
                                cmd.args(args);
                                cmd.stdout(std::process::Stdio::piped());
                                cmd.stderr(std::process::Stdio::piped());
                                #[cfg(windows)]
                                {
                                    use windows::Win32::System::Threading::CREATE_NO_WINDOW;
                                    use std::os::windows::process::CommandExt;
                                    cmd.creation_flags(CREATE_NO_WINDOW.0);
                                }
                                match cmd.output() {
                                    Ok(nltk_output) if nltk_output.status.success() => {
                                        println!("NLTK download succeeded with {}", py);
                                        nltk_success = true;
                                        break;
                                    }
                                    Ok(nltk_output) => {
                                        println!("NLTK download failed with {}: {}", py, String::from_utf8_lossy(&nltk_output.stderr));
                                    }
                                    Err(e) => {
                                        println!("Failed to run python for NLTK download: {}", e);
                                    }
                                }
                            }
                            if nltk_success { break; }
                        }
                        if !nltk_success {
                            println!("WARNING: NLTK punkt_tab/punkt download failed!");
                        }
                        return Ok(());
                    } else {
                        last_error = format!("pip install failed with status {}: {}", output.status, stderr);
                        println!("PIP FAILED: {}", last_error);
                    }
                }
                Err(e) => {
                    last_error = format!("Failed to run pip command '{}': {}", pip_cmd, e);
                }
            }
        }
        let _ = std::fs::remove_file(&req_path); // Clean up temp file on failure too
        Err(format!("All pip commands failed. Last error: {}", last_error))
    }
} 