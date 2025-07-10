//! GUI rendering components for the YapBot Installer
//! 
//! This module contains all the UI rendering methods and components.

use eframe::egui;
use crate::{
    config::APP_VERSION,
    data_structures::YapBotInstaller,
    edit_settings_py,
    migrate_db_files_with_callback_and_channel,
    data_structures::copy_embedded_twitch_markovchain_to,
};

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Embed the TwitchYapBot.exe runner
const TWITCH_YAP_BOT_EXE: &[u8] = include_bytes!("../resources/runner/TwitchYapBot.exe");

impl YapBotInstaller {
    /// Render the application header
    pub fn render_header(&self, ui: &mut egui::Ui) {
        ui.add_space(3.0);
        let title = format!("Yap Bot Installer v{}", APP_VERSION);
        ui.label(egui::RichText::new(title)
            .font(egui::FontId::new(17.0, egui::FontFamily::Name("consolas_titles".into())))
            .color(egui::Color32::from_rgb(189, 147, 249)));
        ui.add_space(4.0);
    }

    /// Render the main content area
    pub fn render_main_content(&self, ui: &mut egui::Ui) {
        ui.horizontal(|ui| {
            ui.add_space(20.0);
            ui.vertical(|ui| {
                ui.label(egui::RichText::new("Welcome to the Yap Bot Installer!").size(13.0));
                ui.add_space(7.0);
                ui.label(egui::RichText::new("This tool will help you install and configure Yap Bot for Twitch.").size(13.0));
            });
        });
    }

    /// Render Python installation status
    pub fn render_python_status(&mut self, ui: &mut egui::Ui) {
        ui.add_space(10.0);
        
        // Step 1: Python Installation
        let open1 = if !self.dependencies_installed {
            Some(true)
        } else if self.step1_just_changed {
            self.step1_just_changed = false;
            Some(self.step1_open)
        } else {
            None
        };
        let response1 = egui::CollapsingHeader::new(
            egui::RichText::new("Step 1: Python Installation")
                .font(egui::FontId::new(16.0, egui::FontFamily::Name("consolas_titles".into())))
                .color(egui::Color32::from_rgb(189, 147, 249))
        )
            .open(open1)
            .show(ui, |ui| {
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    ui.add_space(20.0);
                    ui.vertical(|ui| {
                        if self.installing_python {
                            ui.horizontal(|ui| {
                                // Draw spinner with smoother animation
                                let time = ui.ctx().input(|i| i.time) as f32;
                                let rotation_speed = 4.0; // Increased speed for smoother animation
                                let angle = (time * rotation_speed) % (2.0 * std::f32::consts::PI);
                                let center = ui.cursor().min + egui::vec2(8.0, 8.0);
                                let radius = 6.0;
                                let painter = ui.painter();
                                let start_angle = angle;
                                let end_angle = angle + std::f32::consts::PI * 1.5;
                                let segments = 20; // More segments for smoother appearance
                                let angle_step = (end_angle - start_angle) / segments as f32;
                                for i in 0..segments {
                                    let angle1 = start_angle + i as f32 * angle_step;
                                    let angle2 = start_angle + (i + 1) as f32 * angle_step;
                                    let p1 = center + egui::vec2(radius * angle1.cos(), radius * angle1.sin());
                                    let p2 = center + egui::vec2(radius * angle2.cos(), radius * angle2.sin());
                                    painter.line_segment([p1, p2], egui::Stroke::new(2.0, egui::Color32::from_rgb(189, 147, 249)));
                                }
                                ui.add_space(20.0);
                                ui.label(egui::RichText::new("Installing Python... (Check your taskbar for a UAC prompt. This may take a minute or two.)").size(13.0));
                            });
                        } else if self.is_python_installed() {
                            ui.horizontal(|ui| {
                                ui.label(egui::RichText::new("✅").size(13.0).color(egui::Color32::from_rgb(80, 250, 123)));
                                ui.label(egui::RichText::new(format!(
                                    "Python is installed: {}",
                                    self.get_python_version().unwrap_or(&"Unknown version".to_string())
                                )).size(13.0));
                            });
                            // Dependencies installation section
                            ui.add_space(3.0);
                            if self.installing_dependencies {
                                ui.horizontal(|ui| {
                                    // Draw spinner
                                    let time = ui.ctx().input(|i| i.time) as f32;
                                    let rotation_speed = 4.0;
                                    let angle = (time * rotation_speed) % (2.0 * std::f32::consts::PI);
                                    let center = ui.cursor().min + egui::vec2(8.0, 8.0);
                                    let radius = 6.0;
                                    let painter = ui.painter();
                                    let start_angle = angle;
                                    let end_angle = angle + std::f32::consts::PI * 1.5;
                                    let segments = 20;
                                    let angle_step = (end_angle - start_angle) / segments as f32;
                                    for i in 0..segments {
                                        let angle1 = start_angle + i as f32 * angle_step;
                                        let angle2 = start_angle + (i + 1) as f32 * angle_step;
                                        let p1 = center + egui::vec2(radius * angle1.cos(), radius * angle1.sin());
                                        let p2 = center + egui::vec2(radius * angle2.cos(), radius * angle2.sin());
                                        painter.line_segment([p1, p2], egui::Stroke::new(2.0, egui::Color32::from_rgb(189, 147, 249)));
                                    }
                                    ui.add_space(20.0);
                                    ui.label(egui::RichText::new("Installing Yap Bot Dependencies").size(13.0));
                                });
                            } else {
                                // Check if dependencies installation completed successfully
                                if self.dependencies_installed {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("✅").size(13.0).color(egui::Color32::from_rgb(80, 250, 123)));
                                        ui.label(egui::RichText::new("Yap Bot Dependencies are installed.").size(13.0));
                                        ui.add_space(15.0);
                                    });
                                } else {
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("📦").size(13.0).color(egui::Color32::from_rgb(139, 233, 253)));
                                        ui.label(egui::RichText::new("Installing Yap Bot Dependencies").size(13.0));
                                    });
                                }
                            }
                        } else {
                            ui.label(egui::RichText::new("❌ Python not found. Yap Bot depends on Python to work properly. Please click the install button below:").size(13.0));
                            ui.add_space(5.0);
                            #[cfg(windows)]
                            if ui.button("Install Python").clicked() {
                                self.start_python_install();
                            }
                            #[cfg(not(windows))]
                            {
                                ui.label(egui::RichText::new("Please install Python 3 and python3-pip using your package manager, then restart the installer.").size(13.0));
                            }
                        }
                    });
                });
            });
        self.step1_open = open1.unwrap_or(self.step1_open);
        if self.dependencies_installed && !self.step4_skipped_to_from_settings {
            if let Some(settings) = &self.loaded_settings {
                if settings.is_complete() {
                    self.step2_visible = true; // ensure Step 2 is shown
                    self.step2_open = false;   // but collapsed
                    self.step3_visible = false;
                    self.step4_visible = true;
                    self.step1_open = false;
                    self.step3_open = false;
                    self.step4_open = true;
                    self.step4_just_changed = true; // force open
                    self.step4_skipped_to_from_settings = true;
                    self.show_paste_token_btn = true; // always show paste button
                    // Restore DB prompt answer state
                    if let Some(ans) = settings.step4_db_prompt_answered_yes {
                        self.step4_db_prompt_answered = true;
                        self.step4_db_prompt_answered_yes = Some(ans);
                    }
                }
            }
        }
        if response1.body_returned.is_some() {
            ui.add_space(12.0);
        }
        // Step 2: Bot Account Login
        if self.dependencies_installed && self.step2_visible {
            let token_valid = self.bot_oauth_token.as_ref().map_or(false, |t| t.strip_prefix("oauth:").map_or(false, |s| s.len() == 30));
            let open2 = if !token_valid {
                Some(true)
            } else if self.step2_just_changed {
                self.step2_just_changed = false;
                Some(self.step2_open)
            } else {
                None
            };
            let response2 = egui::CollapsingHeader::new(
                egui::RichText::new("Step 2: Bot Account Login")
                    .font(egui::FontId::new(16.0, egui::FontFamily::Name("consolas_titles".into())))
                    .color(egui::Color32::from_rgb(189, 147, 249))
            )
                .open(open2)
                .show(ui, |ui| {
                    ui.add_space(5.0);
                    ui.horizontal(|ui| {
                        ui.add_space(30.0);
                        ui.vertical(|ui| {
                            ui.label(egui::RichText::new("We need to allow Yap Bot to use your twitch bot account: ").size(13.0));
                            ui.add_space(3.0);
                            ui.label(egui::RichText::new(" - In your default web browser, log out of your main twitch account (e.g. FosterBarnes)").size(13.0));
                            ui.add_space(3.0);
                            ui.label(egui::RichText::new(" - Log into your bot account (e.g. FosterB0t)").size(13.0));
                            ui.add_space(3.0);
                            ui.label(egui::RichText::new(" - If you don't have a bot account, create an alternate twitch account. Using your main account will not work correctly.").size(13.0));
                            ui.add_space(10.0);
                            ui.horizontal(|ui| {
                                let token_btn = ui.add_sized([ui.available_width().min(180.0), 28.0], egui::Button::new("Generate Bot Chat Token"));
                                let btn_width = token_btn.rect.width();
                                if token_btn.clicked() {
                                    if let Err(e) = self.open_token_generator() {
                                        ui.label(egui::RichText::new(format!("Failed to open browser: {}", e)).size(13.0).color(egui::Color32::from_rgb(255, 85, 85)));
                                    }
                                    self.show_paste_token_btn = true;
                                }
                                ui.label(egui::RichText::new("( Select \"Bot Chat Token\" when asked. Copy the \"Access Token\" once logged in )").size(13.0));
                                self.last_token_btn_width = btn_width;
                            });
                            if self.show_paste_token_btn {
                                ui.add_space(8.0);
                                let width = self.last_token_btn_width.max(1.0); // fallback to 1.0 if not set
                                ui.horizontal(|ui| {
                                    let paste_btn = egui::Button::new("Paste Access Token");
                                    let response = ui.add_sized([width, 28.0], paste_btn);
                                    if response.clicked() {
                                        if let Ok(token) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                                            let formatted = if token.starts_with("oauth:") {
                                                token.clone()
                                            } else {
                                                format!("oauth:{}", token)
                                            };
                                            // Check if the token (without prefix) is exactly 30 chars
                                            let token_part = formatted.strip_prefix("oauth:").unwrap_or(&formatted);
                                            if token_part.len() == 30 {
                                                self.bot_oauth_token = Some(formatted.clone());
                                                println!("PASTED OAUTH TOKEN: {}", formatted);
                                                self.token_just_pasted = true;
                                                return;
                                            } else {
                                                self.bot_oauth_token = Some(formatted.clone());
                                                self.step3_visible = false;
                                            }
                                        }
                                    }
                                    // Always show the oauth token if present
                                    if let Some(token) = &self.bot_oauth_token {
                                        let token_part = token.strip_prefix("oauth:").unwrap_or(token);
                                        if token_part.len() == 30 {
                                            ui.label(egui::RichText::new(format!("( {} )", token)).size(13.0));
                                        } else {
                                            ui.label(egui::RichText::new("( Invalid Access Token )").size(13.0).color(egui::Color32::from_rgb(255, 85, 85)));
                                        }
                                    }
                                });
                            }
                        });
                        ui.add_space(20.0);
                    });
                });
            self.step2_open = open2.unwrap_or(self.step2_open);
            // After Step 2 header, if token_just_pasted, show Step 3 and collapse Step 2
            if self.token_just_pasted {
                self.show_step3();
                self.token_just_pasted = false;
            }
            if response2.body_returned.is_some() {
                ui.add_space(12.0);
            }
        }
        // Step 3: YapBot Configuration
        let open3 = if self.step3_just_changed {
            self.step3_just_changed = false;
            Some(self.step3_open)
        } else {
            None
        };
        let response3 = egui::CollapsingHeader::new(
            egui::RichText::new("Step 3: Yap Bot Configuration")
                .font(egui::FontId::new(16.0, egui::FontFamily::Name("consolas_titles".into())))
                .color(egui::Color32::from_rgb(189, 147, 249))
        )
            .open(open3)
            .show(ui, |ui| {
                ui.add_space(5.0);
                // Step 3 instructional text
                ui.label(egui::RichText::new("Now we need to configure Yap Bot. Please enter all required info and settings").size(13.0));
                ui.add_space(10.0);
                let field_width = ui.available_width() - 32.0;
                let consolas_font = egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into()));
                // Copy previous values to locals to avoid borrow checker issues
                let prev_main = self.prev_main_channel_name.clone();
                let prev_bot = self.prev_bot_channel_name.clone();
                // Main Channel Name
                ui.label(egui::RichText::new("Main Channel Name (The account you stream on):").size(13.0));
                ui.add_space(3.0);
                ui.add_sized([
                    field_width.max(100.0), 0.0
                ], egui::TextEdit::singleline(&mut self.main_channel_name)
                    .font(consolas_font.clone())
                    .min_size(egui::vec2(field_width.max(100.0), 0.0))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                ui.add_space(10.0);
                // Bot Channel Name
                ui.label(egui::RichText::new("Bot Channel Name (The account for your bot. if you do not have a bot account, enter your main channel name):").size(13.0));
                ui.add_space(5.0);
                ui.add_sized([
                    field_width.max(100.0), 0.0
                ], egui::TextEdit::singleline(&mut self.bot_channel_name)
                    .font(consolas_font.clone())
                    .min_size(egui::vec2(field_width.max(100.0), 0.0))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                // Always update denied users based on current and previous values
                self.update_denied_users_on_blur(&prev_main, &prev_bot);
                self.prev_main_channel_name = self.main_channel_name.clone();
                self.prev_bot_channel_name = self.bot_channel_name.clone();
                // Now render Denied Users textbox with the latest value
                ui.label(egui::RichText::new("Denied Users (Comma-separated. These user's chat messages will not be read. Add any additional chat bots to this list):").size(13.0));
                ui.add_space(5.0);
                ui.add_sized([
                    field_width.max(100.0), 0.0
                ], egui::TextEdit::singleline(&mut self.denied_users)
                    .font(consolas_font.clone())
                    .min_size(egui::vec2(field_width.max(100.0), 0.0))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                ui.add_space(5.0);
                // Cooldown
                ui.label(egui::RichText::new("Cooldown (Seconds):").size(13.0));
                ui.add_space(5.0);
                ui.add_sized([
                    field_width.max(100.0), 0.0
                ], egui::TextEdit::singleline(&mut self.cooldown)
                    .font(consolas_font.clone())
                    .min_size(egui::vec2(field_width.max(100.0), 0.0))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                ui.add_space(5.0);
                // Generate Command
                ui.label(egui::RichText::new("Generate Command (Comma-separated. This is the command that will have Yap Bot write a message):").size(13.0));
                ui.add_space(5.0);
                ui.add_sized([
                    field_width.max(100.0), 0.0
                ], egui::TextEdit::singleline(&mut self.generate_command)
                    .font(consolas_font)
                    .min_size(egui::vec2(field_width.max(100.0), 0.0))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                // Save & Continue button
                ui.add_space(12.0);
                let save_btn_width = self.last_token_btn_width.max(180.0); // fallback if not set
                if ui.add_sized([save_btn_width, 28.0], egui::Button::new("Save & Continue")).clicked() {
                    println!("[YapBotInstaller] Step 3 values:");
                    println!("  Main Channel Name: {}", self.main_channel_name);
                    println!("  Bot Channel Name: {}", self.bot_channel_name);
                    println!("  Denied Users: {}", self.denied_users);
                    println!("  Cooldown: {}", self.cooldown);
                    println!("  Generate Command: {}", self.generate_command);
                    self.save_settings_to_file();
                    self.show_step4();
                    self.step3_open = false;
                    self.step3_just_changed = true;
                }
            });
        self.step3_open = open3.unwrap_or(self.step3_open);
        if response3.body_returned.is_some() {
            ui.add_space(12.0);
        }
        // Step 4: YapBot Automatic Setup
        if self.step4_visible {
            // Step 4 automation logic
            if self.step4_action_index == 0 && !self.step4_action_running {
                self.step4_action_running = true;
                let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                let dest = std::path::PathBuf::from(format!("{}/YapBot/TwitchMarkovChain", appdata));
                std::thread::spawn({
                    let tx = self.step4_action_tx.clone();
                    move || {
                        let _ = std::fs::create_dir_all(dest.parent().unwrap());
                        let _ = copy_embedded_twitch_markovchain_to(&dest);
                        tx.send(1).ok();
                    }
                });
            } else if self.step4_action_index == 1 && !self.step4_action_running {
                self.step4_action_running = true;
                let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                let settings_path = std::path::PathBuf::from(format!("{}/YapBot/TwitchMarkovChain/Settings.py", appdata));
                let main_channel = self.main_channel_name.clone();
                let bot_channel = self.bot_channel_name.clone();
                let denied_users = self.denied_users.clone();
                let cooldown = self.cooldown.clone();
                let generate_command = self.generate_command.clone();
                let oauth = self.bot_oauth_token.clone().unwrap_or_default();
                std::thread::spawn({
                    let tx = self.step4_action_tx.clone();
                    move || {
                        let _ = edit_settings_py(&settings_path, &main_channel, &bot_channel, &denied_users, &cooldown, &generate_command, &oauth);
                        tx.send(2).ok();
                    }
                });
            } else if self.step4_action_index == 2 && !self.step4_action_running {
                self.step4_action_running = true;
                let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                let settings_json_path = std::path::PathBuf::from(format!("{}/YapBot/TwitchMarkovChain/settings.json", appdata));
                let channel = format!("#{}", self.main_channel_name.trim());
                let nickname = self.bot_channel_name.trim().to_string();
                let authentication = self.bot_oauth_token.clone().unwrap_or_default();
                let denied_users: Vec<String> = self.denied_users.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                let generate_commands: Vec<String> = self.generate_command.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                let cooldown: i64 = self.cooldown.trim().parse().unwrap_or(0);
                let json = serde_json::json!({
                    "Host": "irc.chat.twitch.tv",
                    "Port": 6667,
                    "Channel": channel,
                    "Nickname": nickname,
                    "Authentication": authentication,
                    "DeniedUsers": denied_users,
                    "AllowedUsers": [],
                    "Cooldown": cooldown,
                    "KeyLength": 2,
                    "MaxSentenceWordAmount": 40,
                    "MinSentenceWordAmount": -1,
                    "HelpMessageTimer": 216000,
                    "AutomaticGenerationTimer": -1,
                    "WhisperCooldown": false,
                    "EnableGenerateCommand": true,
                    "SentenceSeparator": " - ",
                    "AllowGenerateParams": true,
                    "GenerateCommands": generate_commands,
                });
                std::thread::spawn({
                    let tx = self.step4_action_tx.clone();
                    move || {
                        let _ = std::fs::write(&settings_json_path, serde_json::to_string_pretty(&json).unwrap());
                        tx.send(3).ok();
                    }
                });
            } else if self.step4_action_index == 3 && !self.step4_action_running {
                self.step4_action_running = true;
                let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                let exe_dest = std::path::PathBuf::from(format!("{}/YapBot/TwitchYapBot.exe", appdata));
                let exe_bytes = TWITCH_YAP_BOT_EXE;
                std::thread::spawn({
                    let tx = self.step4_action_tx.clone();
                    move || {
                        let _ = std::fs::create_dir_all(exe_dest.parent().unwrap());
                        let _ = std::fs::write(&exe_dest, exe_bytes);
                        // Create Desktop and Start Menu shortcuts using PowerShell
                        #[cfg(windows)]
                        {
                            use std::process::Command;
                            if let Some(desktop) = dirs::desktop_dir() {
                                let shortcut_path = desktop.join("Twitch Yap Bot.lnk");
                                let target_path = exe_dest.to_string_lossy();
                                let ps_cmd = format!(
                                    "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut(\"{}\"); $Shortcut.TargetPath = \"{}\"; $Shortcut.Save();",
                                    shortcut_path.display(), target_path
                                );
                                let _ = Command::new("powershell")
                                    .args(["-NoProfile", "-Command", &ps_cmd])
                                    .creation_flags(0x08000000) // CREATE_NO_WINDOW
                                    .output();
                            }
                            // Start Menu shortcut
                            let start_menu = std::path::Path::new(&appdata)
                                .join("Microsoft/Windows/Start Menu/Programs");
                            let shortcut_path = start_menu.join("Twitch Yap Bot.lnk");
                            let target_path = exe_dest.to_string_lossy();
                            let ps_cmd = format!(
                                "$WshShell = New-Object -ComObject WScript.Shell; $Shortcut = $WshShell.CreateShortcut(\"{}\"); $Shortcut.TargetPath = \"{}\"; $Shortcut.Save();",
                                shortcut_path.display(), target_path
                            );
                            let _ = Command::new("powershell")
                                .args(["-NoProfile", "-Command", &ps_cmd])
                                .creation_flags(0x08000000) // CREATE_NO_WINDOW
                                .output();
                        }
                        tx.send(4).ok();
                    }
                });
            }
            // Handle action completion
            if let Ok(idx) = self.step4_action_rx.try_recv() {
                self.step4_action_index = idx;
                self.step4_action_running = false;
                // If we just finished the last action (idx == 4), move to Step 5 only if:
                // - settings were recovered on start (step4_skipped_to_from_settings), OR
                // - the DB prompt has been answered (step4_db_prompt_answered)
                if idx == 4 {
                    if self.step4_skipped_to_from_settings || self.step4_db_prompt_answered {
                        self.step4_open = false;
                        self.step4_just_changed = true;
                        self.step5_visible = true;
                        self.step5_open = true;
                        self.step5_just_changed = true;
                    }
                }
            }
            let open4 = if self.step4_just_changed {
                self.step4_just_changed = false;
                Some(self.step4_open)
            } else {
                None
            };
            let response4 = egui::CollapsingHeader::new(
                egui::RichText::new("Step 4: Yap Bot Automatic Setup")
                    .font(egui::FontId::new(16.0, egui::FontFamily::Name("consolas_titles".into())))
                    .color(egui::Color32::from_rgb(189, 147, 249))
            )
                .open(open4)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            ui.add_space(5.0);
                            let actions = [
                                "Copying \"YapBot\\TwitchMarkovChain\" to \"AppData\\Roaming\"",
                                "Applying configuration to \"AppData\\Roaming\\YapBot\\TwitchMarkovChain\\Settings.py \"",
                                "Writing settings.json to \"AppData\\Roaming\\YapBot\\TwitchMarkovChain\"",
                                "Copying TwitchYapBot.exe and creating Desktop and Start Menu shortcuts",
                            ];
                            for (i, action) in actions.iter().enumerate() {
                                if self.step4_action_index == i && self.step4_action_running {
                                    // Show spinner while operation is in progress
                                    ui.horizontal(|ui| {
                                        let time = ui.ctx().input(|i| i.time) as f32;
                                        let rotation_speed = 4.0;
                                        let angle = (time * rotation_speed) % (2.0 * std::f32::consts::PI);
                                        let center = ui.cursor().min + egui::vec2(8.0, 8.0);
                                        let radius = 6.0;
                                        let painter = ui.painter();
                                        let start_angle = angle;
                                        let end_angle = angle + std::f32::consts::PI * 1.5;
                                        let segments = 20;
                                        let angle_step = (end_angle - start_angle) / segments as f32;
                                        for i in 0..segments {
                                            let angle1 = start_angle + i as f32 * angle_step;
                                            let angle2 = start_angle + (i + 1) as f32 * angle_step;
                                            let p1 = center + egui::vec2(radius * angle1.cos(), radius * angle1.sin());
                                            let p2 = center + egui::vec2(radius * angle2.cos(), radius * angle2.sin());
                                            painter.line_segment([p1, p2], egui::Stroke::new(2.0, egui::Color32::from_rgb(189, 147, 249)));
                                        }
                                        ui.add_space(20.0);
                                        // Do not show the message until operation is done
                                    });
                                } else if self.step4_action_index > i {
                                    // Only show checkmark and message after operation is validated complete
                                    ui.horizontal(|ui| {
                                        ui.label(egui::RichText::new("✅").size(13.0).color(egui::Color32::from_rgb(80, 250, 123)));
                                        ui.label(egui::RichText::new(*action).size(13.0));
                                    });
                                }
                                ui.add_space(8.0);
                            }
                            // DB migration prompt as part of Step 4
                            // Only show DB prompt after all actions (including shortcut creation) are complete
                            if self.step4_action_index >= 2 && !self.step4_db_prompt_answered && self.step4_action_index == 4 {
                                self.step4_db_prompt_visible = true;
                            } else {
                                self.step4_db_prompt_visible = false;
                            }
                            if self.step4_db_prompt_visible {
                                ui.add_space(16.0);
                                ui.label(egui::RichText::new("Have you ever used YapBot with the previous installer?").size(13.0));
                                ui.add_space(8.0);
                                let already_answered = self.step4_db_prompt_answered_yes.is_some();
                                ui.horizontal(|ui| {
                                    let btn_size = egui::vec2(80.0, 28.0);
                                    // Buttons are disabled if answered in this session OR if loaded from settings
                                    let disable_buttons = already_answered || self.step4_db_prompt_answered;
                                    let yes_clicked = ui.add_enabled(!disable_buttons, egui::Button::new("Yes").min_size(btn_size)).clicked();
                                    let no_clicked = ui.add_enabled(!disable_buttons, egui::Button::new("No").min_size(btn_size)).clicked();
                                    if yes_clicked {
                                        self.step4_db_prompt_answered = true;
                                        self.step4_db_prompt_answered_yes = Some(true);
                                        self.step4_db_prompt_running = true;
                                        let (tx, rx) = std::sync::mpsc::channel();
                                        self.step4_db_file_tx = Some(tx.clone());
                                        self.step4_db_file_rx = Some(rx);
                                        let tx2 = Some(tx);
                                        std::thread::spawn(move || {
                                            let _ = migrate_db_files_with_callback_and_channel(|src, dest| {
                                                let _ = tx2.as_ref().unwrap().send((src.to_string(), dest.to_string()));
                                            }, tx2.as_ref());
                                        });
                                        self.save_settings_to_file();
                                        // If all actions are already complete, move to Step 5 now
                                        if self.step4_action_index == 4 {
                                            self.step4_open = false;
                                            self.step4_just_changed = true;
                                            self.step5_visible = true;
                                            self.step5_open = true;
                                            self.step5_just_changed = true;
                                        }
                                    }
                                    if no_clicked {
                                        self.step4_db_prompt_answered = true;
                                        self.step4_db_prompt_answered_yes = Some(false);
                                        self.save_settings_to_file();
                                        // If all actions are already complete, move to Step 5 now
                                        if self.step4_action_index == 4 {
                                            self.step4_open = false;
                                            self.step4_just_changed = true;
                                            self.step5_visible = true;
                                            self.step5_open = true;
                                            self.step5_just_changed = true;
                                        }
                                    }
                                });
                                if (self.step4_db_prompt_answered && self.step4_db_prompt_answered_yes == Some(true)) || self.step4_db_prompt_answered_yes == Some(true) {
                                    ui.add_space(8.0);
                                    ui.label(egui::RichText::new("Existing database file copied to \"AppData\\Roaming\\YapBot\\TwitchMarkovChain\"").size(13.0));
                                }
                            }
                        });
                    });
                });
            self.step4_open = open4.unwrap_or(self.step4_open);
            if response4.body_returned.is_some() {
                ui.add_space(12.0);
            }
        }
        // Step 5: Install Complete
        let open5 = if self.step5_just_changed {
            self.step5_just_changed = false;
            Some(self.step5_open)
        } else {
            None
        };
        if self.step5_visible {
            let response5 = egui::CollapsingHeader::new(
                egui::RichText::new("Step 5: Install Complete")
                    .font(egui::FontId::new(16.0, egui::FontFamily::Name("consolas_titles".into())))
                    .color(egui::Color32::from_rgb(189, 147, 249))
            )
                .open(open5)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.add_space(20.0);
                        ui.vertical(|ui| {
                            ui.add_space(10.0);
                            ui.label(egui::RichText::new("WOOOOOOOOOO THAT'S WHAT I'M TALKIN ABOUT BAYBEEEEE!!!!").size(13.0));
                            ui.add_space(5.0);
                            ui.label(egui::RichText::new("Yap Bot is now installed! You can launch it from the Desktop or Start Menu shortcut. Once it's running, type '!yap' in chat to activate it. If you selected 'Yes' during the previous step, your database has been restored from your existing install.").size(13.0));
                            ui.add_space(20.0);
                            if ui.add_sized([280.0, 36.0], egui::Button::new("Start Twitch Yap Bot & Exit Installer")).clicked() {
                                // Launch the copied TwitchYapBot.exe and exit
                                #[cfg(windows)]
                                {
                                    use std::process::Command;
                                    if let Ok(appdata) = std::env::var("APPDATA") {
                                        let exe_path = std::path::Path::new(&appdata).join("YapBot").join("TwitchYapBot.exe");
                                        let _ = Command::new(exe_path).spawn();
                                    }
                                }
                                std::process::exit(0);
                            }
                        });
                        ui.add_space(20.0);
                    });
                });
            self.step5_open = open5.unwrap_or(self.step5_open);
            if response5.body_returned.is_some() {
                ui.add_space(12.0);
            }
        }
    }
}

impl eframe::App for YapBotInstaller {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle installation states (like rustitles does)
        self.handle_installation_states();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            // Render header
            self.render_header(ui);
            
            // Render main content
            self.render_main_content(ui);
            
            // Render Python installation status
            self.render_python_status(ui);
        });
        
        // If installing Python or dependencies, repaint at 60 FPS for smooth spinner
        if self.installing_python || self.installing_dependencies {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Cleanup when the application exits
    }
} 