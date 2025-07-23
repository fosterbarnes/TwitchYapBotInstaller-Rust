//! GUI rendering components for the YapBot Installer
//! 
//! This module contains all the UI rendering methods and components.

use eframe::egui;
use crate::{
    data_structures::YapBotInstaller,
    edit_settings_py,
    migrate_db_files_with_callback_and_channel,
    data_structures::copy_embedded_twitch_markovchain_to,
};
use reqwest;
use tokio::runtime::Runtime;
use crate::bubbles::bubble_list_ui;

#[cfg(windows)]
use std::os::windows::process::CommandExt;

// Embed the TwitchYapBot.exe runner
const TWITCH_YAP_BOT_EXE: &[u8] = include_bytes!("../resources/runner/TwitchYapBot.exe");
// Embed the YapBotUpdater.exe
const YAP_BOT_UPDATER_EXE: &[u8] = include_bytes!("../resources/updater/YapBotUpdater.exe");

impl YapBotInstaller {
    /// Draw a spinning progress indicator
    fn draw_spinner(ui: &mut egui::Ui, color: egui::Color32) {
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
            painter.line_segment([p1, p2], egui::Stroke::new(2.0, color));
        }
    }

    /// Get the version from resources/version.txt
    fn get_version() -> &'static str {
        include_str!("version.txt").trim()
    }

    /// Render the application header
    pub fn render_header(&self, ui: &mut egui::Ui) {
        ui.add_space(3.0);
        ui.horizontal(|ui| {
            let version = Self::get_version();
            let title = format!("Yap Bot Installer v{}", version);
            ui.label(egui::RichText::new(title)
                .font(egui::FontId::new(17.0, egui::FontFamily::Name("consolas_titles".into())))
                .color(egui::Color32::from_rgb(189, 147, 249)));
            // Inline version check warning/link
            if self.is_version_checked() {
                if let Some(latest) = self.get_latest_version() {
                    let current = version.trim_start_matches('v');
                    let latest_tag = latest.trim_start_matches('v');
                    if Self::is_outdated(version, latest) {
                        // If current is newer than latest, show special message
                        if Self::is_newer(current, latest_tag) {
                            let exe_url = format!("https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases/tag/{}", latest);
                            let link_text = "(how the fuck did u get an unreleased version?)";
                            let link_rich = egui::RichText::new(link_text)
                                .color(egui::Color32::from_rgb(80, 160, 255))
                                .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas_titles".into())));
                            ui.hyperlink_to(link_rich, exe_url);
                        } else {
                            let exe_url = format!("https://github.com/fosterbarnes/TwitchYapBotInstaller-Rust/releases/tag/{}", latest);
                            let link_text = format!("(Update available: Click here to download {})", latest);
                            let link_rich = egui::RichText::new(link_text)
                                .color(egui::Color32::from_rgb(80, 160, 255))
                                .font(egui::FontId::new(14.0, egui::FontFamily::Name("consolas_titles".into())));
                            ui.hyperlink_to(link_rich, exe_url);
                        }
                    }
                } else if let Some(err) = self.get_version_check_error() {
                    ui.label(egui::RichText::new(format!("(Version check failed: {} )", err)).color(egui::Color32::from_rgb(255, 184, 108)).size(14.0));
                }
            }
        });
        ui.add_space(4.0);
    }

    /// Returns true if current version is newer than latest (semantic version compare, ignoring pre-release)
    pub fn is_newer(current: &str, latest: &str) -> bool {
        use std::cmp::Ordering;
        let parse = |s: &str| {
            s.split(|c| c == '-' || c == '+')
                .next()
                .unwrap_or("")
                .split('.')
                .map(|x| x.parse::<u32>().unwrap_or(0))
                .collect::<Vec<_>>()
        };
        let c = parse(current);
        let l = parse(latest);
        for (a, b) in c.iter().zip(l.iter()) {
            match a.cmp(b) {
                Ordering::Greater => return true,
                Ordering::Less => return false,
                Ordering::Equal => {}
            }
        }
        c.len() > l.len() // e.g. 1.0.1 > 1.0
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
        // Ensure access token and client ID are displayed if loaded from settings
        let mut needs_sync_denied = false;
        let mut needs_sync_generate = false;
        if let Some(settings) = &self.loaded_settings {
            if self.bot_oauth_token.is_none() && !settings.oauth.is_empty() {
                self.bot_oauth_token = Some(settings.oauth.clone());
            }
            if self.twitch_token_client_id.is_none() {
                if let Some(cid) = &settings.twitch_token_client_id {
                    self.twitch_token_client_id = Some(cid.clone());
                }
            }
            // Merge denied users from settings with defaults, deduped
            if !settings.denied_users.is_empty() {
                let mut all = self.denied_users_list.clone();
                for s in settings.denied_users.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                    if !all.iter().any(|u| u.eq_ignore_ascii_case(&s)) {
                        all.push(s);
                    }
                }
                let mut seen = std::collections::HashSet::new();
                all.retain(|u| seen.insert(u.to_lowercase()));
                self.denied_users_list = all;
                needs_sync_denied = true;
            }
            // Merge generate commands from settings with defaults, deduped
            if !settings.generate_command.is_empty() {
                let mut all = self.generate_command_list.clone();
                for s in settings.generate_command.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()) {
                    if !all.iter().any(|c| c.eq_ignore_ascii_case(&s)) {
                        all.push(s);
                    }
                }
                let mut seen = std::collections::HashSet::new();
                all.retain(|c| seen.insert(c.to_lowercase()));
                self.generate_command_list = all;
                needs_sync_generate = true;
            }
        }
        if needs_sync_denied {
            self.sync_denied_users_string();
        }
        if needs_sync_generate {
            self.sync_generate_command_string();
        }
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
                                Self::draw_spinner(ui, egui::Color32::from_rgb(189, 147, 249));
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
                                    Self::draw_spinner(ui, egui::Color32::from_rgb(189, 147, 249));
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
                                                self.twitch_token_client_id = None;
                                                self.twitch_token_username_warning = None;
                                                self.twitch_token_checked_username = None;
                                            } else {
                                                self.bot_oauth_token = Some(formatted.clone());
                                                self.step3_visible = false;
                                            }
                                        }
                                    }
                                    // Show warning if username mismatch
                                    if let Some(warning) = &self.twitch_token_username_warning {
                                        ui.label(egui::RichText::new(warning).color(egui::Color32::from_rgb(255, 85, 85)).size(13.0));
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
                            // If a valid token is present, show the Client ID paste button and field
                            if let Some(token) = &self.bot_oauth_token {
                                let token_part = token.strip_prefix("oauth:").unwrap_or(token);
                                if token_part.len() == 30 {
                                    ui.add_space(8.0);
                                    let client_id_btn_width = self.last_token_btn_width.max(180.0); // fallback to 180.0 if not set
                                    ui.horizontal(|ui| {
                                        let paste_client_id_btn = egui::Button::new("Paste Client ID");
                                        let client_id_response = ui.add_sized([client_id_btn_width, 28.0], paste_client_id_btn);
                                        if client_id_response.clicked() {
                                            if let Ok(client_id) = arboard::Clipboard::new().and_then(|mut c| c.get_text()) {
                                                self.twitch_token_client_id = Some(client_id.trim().to_string());
                                            }
                                        }
                                        // Show the current client ID if present, styled like the access token
                                        if let Some(client_id) = &self.twitch_token_client_id {
                                            ui.label(egui::RichText::new(format!("( {} )", client_id)).size(13.0));
                                        }
                                    });
                                    // If both token and client ID are present, perform the username check
                                    if let (Some(token), Some(client_id)) = (&self.bot_oauth_token, &self.twitch_token_client_id) {
                                        if self.twitch_token_checked_username.is_none() {
                                            let username = Self::check_twitch_token_username(token, client_id);
                                            if let Some(username) = username {
                                                self.twitch_token_checked_username = Some(username.clone());
                                                self.bot_channel_name = username.clone();
                                                self.twitch_token_username_warning = None;
                                                self.token_just_pasted = true;
                                            } else {
                                                self.twitch_token_username_warning = Some("Failed to verify token with Twitch. Please try again.".to_string());
                                                self.bot_oauth_token = None;
                                            }
                                        }
                                    }
                                }
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
                // Remove these unused variables:
                // let prev_main = self.prev_main_channel_name.clone();
                // let prev_bot = self.prev_bot_channel_name.clone();
                // Main Channel Name
                ui.label(egui::RichText::new("Main Channel Name (The account you stream on):").size(13.0));
                ui.add_space(3.0);
                // For main channel name:
                let row_height = 23.0;
                let input_width = (field_width.max(100.0) - 36.0) * 0.25;
                ui.add_sized([
                    input_width, row_height
                ], egui::TextEdit::singleline(&mut self.main_channel_name)
                    .font(consolas_font.clone())
                    .min_size(egui::vec2(input_width, row_height))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                ui.add_space(6.0);
                // After the main channel name field in Step 3 UI:
                if !self.main_channel_name.trim().is_empty() && !self.bot_channel_name.trim().is_empty() && self.main_channel_name.trim().eq_ignore_ascii_case(self.bot_channel_name.trim()) {
                    ui.label(egui::RichText::new("Using your main account for Yap Bot will cause issues").color(egui::Color32::from_rgb(255, 85, 85)).size(13.0));
                    ui.add_space(6.0);
                }
                // Remove the Bot Channel Name label and text field from the UI entirely.
                // Do not display or prompt for the bot channel name at all.
                // Only add the detected bot channel name to the denied users bubble and settings, but do not show it as a separate field.
                // Now render Denied Users textbox with the latest value
                ui.label(egui::RichText::new("Denied Users (These users' chat messages will not be read. Add any additional chat bots to this list):").size(13.0));
                ui.add_space(5.0);
                // Input row: TextEdit + '+' button
                ui.horizontal(|ui| {
                    let row_height = 23.0;
                    let input_width = (field_width.max(100.0) - 36.0) * 0.25;
                    let denied_input_id = ui.make_persistent_id("denied_user_input");
                    let input = ui.add_sized([
                        input_width, row_height
                    ], egui::TextEdit::singleline(&mut self.temp_denied_user_input)
                    .font(consolas_font.clone())
                        .min_size(egui::vec2(input_width, row_height))
                        .margin(egui::Margin::symmetric(8.0, 4.0))
                        .id(denied_input_id));
                    let plus_btn_response = ui.add_sized([
                        row_height, row_height
                    ], egui::Button::new("").frame(true));
                    // Draw a smaller plus sign in the center of the button
                    let plus_rect = plus_btn_response.rect;
                    let painter = ui.painter();
                    let center = plus_rect.center();
                    let plus_len = 4.0;
                    let plus_thickness = 2.5;
                    let plus_color = egui::Color32::WHITE;
                    painter.line_segment([
                        center + egui::vec2(-plus_len, 0.0),
                        center + egui::vec2(plus_len, 0.0)
                    ], egui::Stroke::new(plus_thickness, plus_color));
                    painter.line_segment([
                        center + egui::vec2(0.0, -plus_len),
                        center + egui::vec2(0.0, plus_len)
                    ], egui::Stroke::new(plus_thickness, plus_color));
                    let mut should_refocus_denied = false;
                    if (plus_btn_response.clicked() || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && !self.temp_denied_user_input.trim().is_empty() {
                        let new_user = self.temp_denied_user_input.trim().to_string();
                        if !self.denied_users_list.iter().any(|u| u.eq_ignore_ascii_case(&new_user)) {
                            self.denied_users_list.push(new_user);
                            self.sync_denied_users_string();
                        }
                        self.temp_denied_user_input.clear();
                        should_refocus_denied = true;
                    }
                    if should_refocus_denied {
                        ui.memory_mut(|mem| mem.request_focus(denied_input_id));
                    }
                });
                ui.add_space(8.0);
                // Bubble list of denied users
                let bubble_height = 18.0;
                let font_id = egui::FontId::new(13.0, egui::FontFamily::Proportional);
                let bot_name = self.bot_channel_name.trim();
                if let Some(idx) = bubble_list_ui(
                    ui,
                    &mut self.denied_users_list,
                    if !bot_name.is_empty() { Some(bot_name) } else { None },
                    bubble_height,
                    font_id.clone(),
                    egui::Color32::from_rgb(224, 224, 224),
                    egui::Color32::BLACK,
                ) {
                    self.denied_users_list.remove(idx);
                    self.sync_denied_users_string();
                }
                ui.add_space(5.0);
                // Add bubble logic for generate commands, similar to denied users
                ui.label(egui::RichText::new("Generate Commands (Comma-separated. These are the commands that will have Yap Bot write a message):").size(13.0));
                ui.add_space(5.0);
                ui.horizontal(|ui| {
                    let row_height = 23.0;
                    let input_width = (field_width.max(100.0) - 36.0) * 0.25;
                    let generate_input_id = ui.make_persistent_id("generate_command_input");
                    let input = ui.add_sized([
                        input_width, row_height
                    ], egui::TextEdit::singleline(&mut self.temp_generate_command_input)
                    .font(consolas_font.clone())
                        .min_size(egui::vec2(input_width, row_height))
                        .margin(egui::Margin::symmetric(8.0, 4.0))
                        .id(generate_input_id));
                    let plus_btn_response = ui.add_sized([
                        row_height, row_height
                    ], egui::Button::new("").frame(true));
                    // Draw a smaller plus sign in the center of the button
                    let plus_rect = plus_btn_response.rect;
                    let painter = ui.painter();
                    let center = plus_rect.center();
                    let plus_len = 4.0;
                    let plus_thickness = 2.5;
                    let plus_color = egui::Color32::WHITE;
                    painter.line_segment([
                        center + egui::vec2(-plus_len, 0.0),
                        center + egui::vec2(plus_len, 0.0)
                    ], egui::Stroke::new(plus_thickness, plus_color));
                    painter.line_segment([
                        center + egui::vec2(0.0, -plus_len),
                        center + egui::vec2(0.0, plus_len)
                    ], egui::Stroke::new(plus_thickness, plus_color));
                    let mut should_refocus_generate = false;
                    if (plus_btn_response.clicked() || (input.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && !self.temp_generate_command_input.trim().is_empty() {
                        let mut new_cmd = self.temp_generate_command_input.trim().to_string();
                        if !new_cmd.starts_with('!') {
                            new_cmd = format!("!{}", new_cmd);
                        }
                        if !self.generate_command_list.iter().any(|c| c.eq_ignore_ascii_case(&new_cmd)) {
                            self.generate_command_list.push(new_cmd);
                            self.sync_generate_command_string();
                        }
                        self.temp_generate_command_input.clear();
                        should_refocus_generate = true;
                    }
                    if should_refocus_generate {
                        ui.memory_mut(|mem| mem.request_focus(generate_input_id));
                    }
                });
                ui.add_space(8.0);
                // Bubble list of generate commands
                if let Some(idx) = bubble_list_ui(
                    ui,
                    &mut self.generate_command_list,
                    None,
                    bubble_height,
                    font_id,
                    egui::Color32::from_rgb(224, 224, 224),
                    egui::Color32::BLACK,
                ) {
                    self.generate_command_list.remove(idx);
                    self.sync_generate_command_string();
                }
                ui.add_space(5.0);
                // Cooldown
                ui.label(egui::RichText::new("Cooldown (Seconds):").size(13.0));
                ui.add_space(5.0);
                // For cooldown:
                ui.add_sized([
                    input_width, row_height
                ], egui::TextEdit::singleline(&mut self.cooldown)
                    .font(consolas_font.clone())
                    .min_size(egui::vec2(input_width, row_height))
                    .margin(egui::Margin::symmetric(8.0, 4.0)));
                ui.add_space(10.0);
                // Save & Continue button;
                let save_btn_width = self.last_token_btn_width.max(180.0); // fallback if not set
                if ui.add_sized([save_btn_width, 28.0], egui::Button::new("Save & Continue")).clicked() {
                    println!("[YapBotInstaller] Step 3 values:");
                    println!("  Main Channel Name: {}", self.main_channel_name);
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
                let exe_dest = std::path::PathBuf::from(format!("{}/YapBot/TwitchYapBot.exe", appdata));
                let loaded_settings = self.loaded_settings.clone();
                std::thread::spawn({
                    let tx = self.step4_action_tx.clone();
                    move || {
                        // Always overwrite TwitchYapBot.exe
                        let _ = std::fs::create_dir_all(exe_dest.parent().unwrap());
                        let _ = std::fs::write(&exe_dest, TWITCH_YAP_BOT_EXE);
                        // Always overwrite YapBotUpdater.exe
                        let updater_dest = exe_dest.parent().unwrap().join("YapBotUpdater.exe");
                        let _ = std::fs::write(&updater_dest, YAP_BOT_UPDATER_EXE);
                        // Always copy the updated MarkovChainBot.py with trigger functionality
                        use include_dir::DirEntry;
                        use crate::data_structures::TWITCH_MARKOVCHAIN_DIR;
                        for entry in TWITCH_MARKOVCHAIN_DIR.entries() {
                            if let DirEntry::File(file) = entry {
                                if file.path().file_name().map(|n| n == "MarkovChainBot.py").unwrap_or(false) {
                                    let dest_path = dest.join("MarkovChainBot.py");
                                    let _ = std::fs::write(&dest_path, file.contents());
                                    println!("[YapBotInstaller] Updated MarkovChainBot.py with trigger functionality at {}", dest_path.display());
                                }
                            }
                        }
                        // If MarkovChain folder exists and previous settings are found, only overwrite MarkovChainBot.py
                        if dest.exists() && loaded_settings.is_some() {
                            // MarkovChainBot.py already copied above, so skip full copy
                        } else {
                            // Full copy for fresh installs
                            let _ = copy_embedded_twitch_markovchain_to(&dest);
                        }
                        tx.send(1).ok();
                    }
                });
            } else if self.step4_action_index == 1 && !self.step4_action_running {
                self.step4_action_running = true;
                let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                let settings_path = std::path::PathBuf::from(format!("{}/YapBot/TwitchMarkovChain/Settings.py", appdata));
                let main_channel = self.main_channel_name.clone();
                let bot_channel = self.bot_channel_name.clone();
                let denied_users_list = self.denied_users_list.clone();
                let generate_command_list = self.generate_command_list.clone();
                let authentication = self.bot_oauth_token.clone().unwrap_or_default();
                let cooldown = self.cooldown.clone();
                let tx = self.step4_action_tx.clone();
                std::thread::spawn(move || {
                    let host = "irc.chat.twitch.tv";
                    let port = 6667;
                    let allowed_users: Vec<String> = vec![];
                    let cooldown_val = cooldown.trim().parse().unwrap_or(20);
                    let key_length = 2;
                    let max_sentence_word_amount = 40;
                    let min_sentence_word_amount = -1;
                    let help_message_timer = 216000;
                    let automatic_generation_timer = -1;
                    let whisper_cooldown = false;
                    let enable_generate_command = true;
                    let sentence_separator = " - ";
                    let allow_generate_params = true;
                    let _ = edit_settings_py(
                        &settings_path,
                        host,
                        port,
                        &main_channel,
                        &bot_channel,
                        &authentication,
                        &denied_users_list,
                        &allowed_users,
                        cooldown_val,
                        key_length,
                        max_sentence_word_amount,
                        min_sentence_word_amount,
                        help_message_timer,
                        automatic_generation_timer,
                        whisper_cooldown,
                        enable_generate_command,
                        sentence_separator,
                        allow_generate_params,
                        &generate_command_list
                    );
                    tx.send(2).ok();
                });
            } else if self.step4_action_index == 2 && !self.step4_action_running {
                self.step4_action_running = true;
                let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                let settings_json_path = std::path::PathBuf::from(format!("{}/YapBot/TwitchMarkovChain/settings.json", appdata));
                let channel = format!("#{}", self.main_channel_name.trim());
                let nickname = self.bot_channel_name.trim().to_string();
                let authentication = self.bot_oauth_token.clone().unwrap_or_default();
                // Compose denied users list, always including bot channel name (deduped, case-insensitive)
                let mut denied_users: Vec<String> = self.denied_users
                    .split(',')
                    .map(|s| s.trim().to_string())
                    .filter(|s| !s.is_empty())
                    .collect();
                let bot_name = self.bot_channel_name.trim();
                if !bot_name.is_empty() && !denied_users.iter().any(|u| u.eq_ignore_ascii_case(bot_name)) {
                    denied_users.push(bot_name.to_string());
                }
                // Deduplicate (case-insensitive)
                let mut seen = std::collections::HashSet::new();
                denied_users.retain(|u| seen.insert(u.to_lowercase()));
                let generate_commands: Vec<String> = self.generate_command.split(',').map(|s| s.trim().to_string()).filter(|s| !s.is_empty()).collect();
                let cooldown: i64 = self.cooldown.trim().parse().unwrap_or(0);
                use crate::data_structures::generate_settings_json;
                std::thread::spawn({
                    let tx = self.step4_action_tx.clone();
                    let denied_users = denied_users.clone();
                    let allowed_users: Vec<String> = vec![];
                    let generate_commands = generate_commands.clone();
                    let channel = channel.clone();
                    let nickname = nickname.clone();
                    let authentication = authentication.clone();
                    let cooldown = cooldown as i32;
                    let key_length = 2;
                    let max_sentence_word_amount = 40;
                    let min_sentence_word_amount = -1;
                    let help_message_timer = 216000;
                    let automatic_generation_timer = -1;
                    let whisper_cooldown = false;
                    let enable_generate_command = true;
                    let sentence_separator = " - ";
                    let allow_generate_params = true;
                    let host = "irc.chat.twitch.tv";
                    let port = 6667;
                    move || {
                        let json_str = generate_settings_json(
                            allow_generate_params,
                            &allowed_users,
                            &authentication,
                            automatic_generation_timer,
                            &channel,
                            cooldown,
                            &denied_users,
                            enable_generate_command,
                            &generate_commands,
                            help_message_timer,
                            host,
                            key_length,
                            max_sentence_word_amount,
                            min_sentence_word_amount,
                            &nickname,
                            port,
                            sentence_separator,
                            whisper_cooldown
                        );
                        let _ = std::fs::write(&settings_json_path, json_str);
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
                                "Copying TwitchYapBot.exe. Creating Desktop and Start Menu shortcuts",
                            ];
                            for (i, action) in actions.iter().enumerate() {
                                if self.step4_action_index == i && self.step4_action_running {
                                    // Show spinner while operation is in progress
                                    Self::draw_spinner(ui, egui::Color32::from_rgb(189, 147, 249));
                                    ui.add_space(20.0);
                                    // Do not show the message until operation is done
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
                            ui.label(egui::RichText::new("WOOOOOOOOOO THAT'S WHAT I'M TALKIN ABOUT BAYBEEEEE!!!").size(13.0));
                            ui.add_space(5.0);
                            // Show different message if this was an update (MarkovChain folder exists and previous settings were found)
                            let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
                            let markov_dir = std::path::PathBuf::from(format!("{}/YapBot/TwitchMarkovChain", appdata));
                            let is_update = markov_dir.exists() && self.loaded_settings.is_some();
                            let install_msg = if is_update {
                                "Yap Bot has been updated! You can launch it from the Desktop or Start Menu shortcut. Once it's running, type '!yap' in chat to activate it. If you selected 'Yes' during the previous step, your database has been restored from your existing install."
                            } else {
                                "Yap Bot is now installed! You can launch it from the Desktop or Start Menu shortcut. Once it's running, type '!yap' in chat to activate it. If you selected 'Yes' during the previous step, your database has been restored from your existing install."
                            };
                            ui.label(egui::RichText::new(install_msg).size(13.0));
                            ui.add_space(20.0);
                            ui.horizontal(|ui| {
                                let min_width = 280.0;
                                let button_width = ui.available_width().max(min_width);
                                if ui.add_sized([button_width, 36.0], egui::Button::new("Start Twitch Yap Bot & Exit Installer")).clicked() {
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

    /// Compare two version strings (ignoring 'v' prefix). Returns true if current < latest.
    pub fn is_outdated(current: &str, latest: &str) -> bool {
        // Remove leading 'v' if present
        let current = current.trim_start_matches('v');
        let latest = latest.trim_start_matches('v');
        // If the tags are not equal, show the update link
        current != latest
    }

    /// Check the username associated with the pasted OAuth token using the Twitch API
    fn check_twitch_token_username(token: &str, client_id: &str) -> Option<String> {
        let rt = Runtime::new().ok()?;
        let token = token.to_string();
        let client_id = client_id.to_string();
        let result = rt.block_on(async move {
            let client = reqwest::Client::new();
            let resp = client
                .get("https://api.twitch.tv/helix/users")
                .bearer_auth(token.strip_prefix("oauth:").unwrap_or(&token))
                .header("Client-Id", client_id)
                .send()
                .await
                .ok()?;
            let json: serde_json::Value = resp.json().await.ok()?;
            println!("[Twitch API] Response: {}", json);
            if let Some(arr) = json.get("data").and_then(|d| d.as_array()) {
                if let Some(user) = arr.get(0) {
                    if let Some(login) = user.get("login").and_then(|l| l.as_str()) {
                        return Some(login.to_string());
                    }
                }
                if arr.is_empty() {
                    println!("[Twitch API] Data array is empty. Token may be invalid or Client-ID is incorrect.");
                }
            } else {
                println!("[Twitch API] No 'data' field in response or not an array.");
            }
            None
        });
        result
    }
}

impl eframe::App for YapBotInstaller {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Handle installation states (like rustitles does)
        self.handle_installation_states();
        // Poll for version check result
        self.poll_version_check();
        
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().auto_shrink([false; 2]).show(ui, |ui| {
                // Render header
                self.render_header(ui);
                // Render main content
                self.render_main_content(ui);
                // Render Python installation status
                self.render_python_status(ui);
            });
        });
        
        // If installing Python or dependencies, or step 4 spinner is running, repaint at 60 FPS for smooth spinner
        if self.installing_python || self.installing_dependencies || self.step4_action_running {
            ctx.request_repaint_after(std::time::Duration::from_millis(16));
        }
    }

    fn on_exit(&mut self, _gl: Option<&eframe::glow::Context>) {
        // Cleanup when the application exits
    }
} 