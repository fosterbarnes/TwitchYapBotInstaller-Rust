// Settings dialog logic split from main.rs
use std::path::PathBuf;
use std::io::Read;
use std::io::Write;
use serde::{Deserialize, Serialize};
use yap_bot_installer::bubbles::bubble_list_ui;
use yap_bot_installer::data_structures::edit_settings_py;
use eframe::egui;
use std::fs;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(rename_all = "PascalCase")]
pub struct BotSettings {
    pub channel: String,
    pub nickname: String,
    pub authentication: String,
    pub denied_users: Vec<String>,
    pub allowed_users: Vec<String>,
    pub cooldown: i32,
    pub key_length: i32,
    pub max_sentence_word_amount: i32,
    pub min_sentence_word_amount: i32,
    pub automatic_generation_timer: i32,
    pub generate_commands: Vec<String>,
    pub sound_enabled: bool,
}

impl Default for BotSettings {
    fn default() -> Self {
        Self {
            channel: "#<channel>".to_string(),
            nickname: "<name>".to_string(),
            authentication: "oauth:<auth>".to_string(),
            denied_users: vec!["StreamElements".to_string(), "Nightbot".to_string(), "Moobot".to_string(), "Marbiebot".to_string()],
            allowed_users: vec![],
            cooldown: 20,
            key_length: 2,
            max_sentence_word_amount: 40,
            min_sentence_word_amount: -1,
            automatic_generation_timer: -1,
            generate_commands: vec!["!generate".to_string(), "!g".to_string()],
            sound_enabled: true,
        }
    }
}

pub struct SettingsDialog {
    pub is_open: bool,
    pub settings: BotSettings,
    pub temp_settings: BotSettings,
    pub needs_restart: bool,
    pub denied_input: String,
    pub allowed_input: String,
    pub generate_command_input: String,
    // Add tweakable left spacing for each field
    pub denied_left_spacing: f32,
    pub allowed_left_spacing: f32,
    pub commands_left_spacing: f32,
}

impl SettingsDialog {
    pub fn new() -> Self {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        let appdata_settings_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\YapBotInstallerSettings.json", appdata));
        if !appdata_settings_path.exists() {
            let local_settings = PathBuf::from("YapBotInstallerSettings.json");
            if local_settings.exists() {
                if let Ok(mut src) = std::fs::File::open(&local_settings) {
                    let mut contents = String::new();
                    let _ = src.read_to_string(&mut contents);
                    let _ = std::fs::create_dir_all(appdata_settings_path.parent().unwrap());
                    let _ = std::fs::File::create(&appdata_settings_path).and_then(|mut f| f.write_all(contents.as_bytes()));
                    println!("[DEBUG] Copied YapBotInstallerSettings.json to AppData");
                }
            } else {
                println!("[DEBUG] No local YapBotInstallerSettings.json found to copy");
            }
        }
        Self {
            is_open: true,
            settings: BotSettings::default(),
            temp_settings: BotSettings::default(),
            needs_restart: false,
            denied_input: String::new(),
            allowed_input: String::new(),
            generate_command_input: String::new(),
            denied_left_spacing: 0.0,
            allowed_left_spacing: -5.0,
            commands_left_spacing: 11.0,
        }
    }

    pub fn load_settings(&mut self) -> Result<(), Box<dyn std::error::Error>> {
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        let appdata_settings_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\YapBotInstallerSettings.json", appdata));
        println!("[DEBUG] Loading GUI settings from: {}", appdata_settings_path.display());
        println!("[DEBUG] File exists: {}", appdata_settings_path.exists());
        if appdata_settings_path.exists() {
            match fs::read_to_string(&appdata_settings_path) {
                Ok(content) => {
                    match serde_json::from_str::<BotSettings>(&content) {
                        Ok(settings) => {
                            self.settings = settings;
                            self.temp_settings = self.settings.clone();
                            println!("[DEBUG] Successfully loaded GUI settings from file.");
                        }
                        Err(e) => {
                            println!("[DEBUG] Failed to parse YapBotInstallerSettings.json as BotSettings: {}", e);
                            #[derive(serde::Deserialize)]
                            struct OldInstallerSettings {
                                oauth: Option<String>,
                                main_channel_name: Option<String>,
                                bot_channel_name: Option<String>,
                                denied_users: Option<String>,
                                cooldown: Option<String>,
                                generate_command: Option<String>,
                            }
                            match serde_json::from_str::<OldInstallerSettings>(&content) {
                                Ok(old) => {
                                    println!("[DEBUG] Migrating old installer settings to BotSettings format");
                                    let denied_users = old.denied_users
                                        .as_deref()
                                        .unwrap_or("")
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect::<Vec<_>>();
                                    let generate_commands = old.generate_command
                                        .as_deref()
                                        .unwrap_or("")
                                        .split(',')
                                        .map(|s| s.trim().to_string())
                                        .filter(|s| !s.is_empty())
                                        .collect::<Vec<_>>();
                                    let settings = BotSettings {
                                        channel: format!("#{}", old.main_channel_name.as_deref().unwrap_or("<channel>").trim()),
                                        nickname: old.bot_channel_name.unwrap_or_else(|| "<name>".to_string()),
                                        authentication: old.oauth.unwrap_or_else(|| "oauth:<auth>".to_string()),
                                        denied_users,
                                        allowed_users: vec![],
                                        cooldown: old.cooldown.and_then(|c| c.parse().ok()).unwrap_or(20),
                                        key_length: 2,
                                        max_sentence_word_amount: 40,
                                        min_sentence_word_amount: -1,
                                        automatic_generation_timer: -1,
                                        generate_commands,
                                        sound_enabled: true,
                                    };
                                    self.settings = settings.clone();
                                    self.temp_settings = settings.clone();
                                    if let Ok(json) = serde_json::to_string_pretty(&settings) {
                                        let _ = std::fs::write(&appdata_settings_path, json);
                                        println!("[DEBUG] Migrated and saved settings in new format.");
                                    }
                                }
                                Err(e2) => {
                                    println!("[DEBUG] Failed to parse as old installer settings: {}", e2);
                                    self.settings = BotSettings::default();
                                    self.temp_settings = self.settings.clone();
                                }
                            }
                        }
                    }
                }
                Err(e) => {
                    println!("[DEBUG] Failed to read YapBotInstallerSettings.json: {}", e);
                    self.settings = BotSettings::default();
                    self.temp_settings = self.settings.clone();
                }
            }
        } else {
            println!("[DEBUG] YapBotInstallerSettings.json does not exist, using defaults.");
            self.settings = BotSettings::default();
            self.temp_settings = self.settings.clone();
        }
        Ok(())
    }

    pub fn save_settings(&mut self) {
        self.settings = self.temp_settings.clone();
        let appdata = std::env::var("APPDATA").unwrap_or_else(|_| "".to_string());
        let appdata_settings_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\YapBotInstallerSettings.json", appdata));
        if let Ok(json) = serde_json::to_string_pretty(&self.settings) {
            let _ = std::fs::create_dir_all(appdata_settings_path.parent().unwrap());
            let _ = std::fs::write(&appdata_settings_path, json);
            println!("[DEBUG] Saved GUI settings to YapBotInstallerSettings.json");
        }
        let settings_py_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\Settings.py", appdata));
        let settings_json_path = PathBuf::from(format!("{}\\YapBot\\TwitchMarkovChain\\settings.json", appdata));
        let channel_with_hash = if self.settings.channel.starts_with('#') {
            self.settings.channel.clone()
        } else {
            format!("#{}", self.settings.channel)
        };
        let python_bot_settings = serde_json::json!({
            "Host": "irc.chat.twitch.tv",
            "Port": 6667,
            "Channel": channel_with_hash,
            "Nickname": self.settings.nickname,
            "Authentication": self.settings.authentication,
            "DeniedUsers": self.settings.denied_users,
            "AllowedUsers": self.settings.allowed_users,
            "Cooldown": self.settings.cooldown,
            "KeyLength": self.settings.key_length,
            "MaxSentenceWordAmount": self.settings.max_sentence_word_amount,
            "MinSentenceWordAmount": self.settings.min_sentence_word_amount,
            "HelpMessageTimer": 216000,
            "AutomaticGenerationTimer": self.settings.automatic_generation_timer,
            "WhisperCooldown": true,
            "EnableGenerateCommand": true,
            "SentenceSeparator": " - ",
            "AllowGenerateParams": true,
            "GenerateCommands": self.settings.generate_commands
        });
        if let Ok(json) = serde_json::to_string_pretty(&python_bot_settings) {
            let _ = std::fs::write(&settings_json_path, json);
            println!("[DEBUG] Saved settings.json for Python bot");
        }
        let _ = edit_settings_py(
            &settings_py_path,
            "irc.chat.twitch.tv",
            6667,
            &channel_with_hash,
            &self.settings.nickname,
            &self.settings.authentication,
            &self.settings.denied_users,
            &self.settings.allowed_users,
            self.settings.cooldown,
            self.settings.key_length,
            self.settings.max_sentence_word_amount,
            self.settings.min_sentence_word_amount,
            216000,
            self.settings.automatic_generation_timer,
            true,
            true,
            " - ",
            true,
            &self.settings.generate_commands,
        );
        println!("[DEBUG] Saved Settings.py for Python bot");
        self.needs_restart = true;
        // Send RESTART_BOT message to main GUI via TCP
        if let Ok(mut stream) = std::net::TcpStream::connect("127.0.0.1:9876") {
            let _ = stream.write_all(b"RESTART_BOT");
        }
    }
}

// Helper to render a labeled input row with a plus button and associated bubble list for Vec<String>.
fn input_with_bubbles(
    ui: &mut egui::Ui,
    label: &str,
    input: &mut String,
    items: &mut Vec<String>,
    font_id: egui::FontId,
    bubble_height: f32,
    id_prefix: &str,
    left_spacing: f32, // new param
) {
    egui::Grid::new(format!("{}_grid", id_prefix)).num_columns(2).spacing([30.0, 8.0]).show(ui, |ui| {
        ui.label(label);
        ui.horizontal(|ui| {
            ui.add_space(left_spacing); // independently tweakable
            let input_id = ui.make_persistent_id(format!("{}_input", id_prefix));
            let input_widget = ui.add(egui::TextEdit::singleline(input).desired_width(250.0).id(input_id));
            let plus_btn = ui.add(egui::Button::new("+").min_size([20.0, 20.0].into()));
            let mut should_refocus = false;
            if (plus_btn.clicked() || (input_widget.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)))) && !input.trim().is_empty() {
                let new_item = input.trim().to_string();
                if !items.iter().any(|u| u.eq_ignore_ascii_case(&new_item)) {
                    items.push(new_item);
                }
                input.clear();
                should_refocus = true;
            }
            if should_refocus {
                ui.memory_mut(|mem| mem.request_focus(input_id));
            }
        });
        ui.end_row();
    });
    ui.add_space(2.0);
    egui::ScrollArea::vertical()
        .id_source(format!("{}_scroll", id_prefix))
        .max_height(80.0)
        .show(ui, |ui| {
            if let Some(idx) = bubble_list_ui(
                ui,
                items,
                None,
                bubble_height,
                font_id,
                egui::Color32::from_rgb(224, 224, 224),
                egui::Color32::BLACK,
            ) {
                items.remove(idx);
            }
        });
    ui.add_space(5.0);
}

impl SettingsDialog {
    pub fn show(&mut self, ctx: &egui::Context) {
        if !self.is_open {
            return;
        }
        let mut save_clicked = false;
        let mut cancel_clicked = false;
        egui::CentralPanel::default().show(ctx, |ui| {
            egui::ScrollArea::vertical().show(ui, |ui| {
                // App Settings
                ui.add_space(2.0);
                ui.label(egui::RichText::new("App Settings").size(18.0).strong());
                ui.add_space(2.0);
                ui.separator();
                ui.horizontal(|ui| {
                    ui.label("Sound:");
                    ui.checkbox(&mut self.temp_settings.sound_enabled, "Enable sound");
                });
                // Twitch Account Info
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Twitch Account Info").size(18.0).strong());
                ui.add_space(2.0);
                ui.separator();
                egui::Grid::new("twitch_account_info_grid")
                    .num_columns(2)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Channel:");
                        ui.add(egui::TextEdit::singleline(&mut self.temp_settings.channel).desired_width(250.0));
                        ui.end_row();
                        ui.label("Bot Account:");
                        ui.add(egui::TextEdit::singleline(&mut self.temp_settings.nickname).desired_width(250.0));
                        ui.end_row();
                        ui.label("Authentication:");
                        ui.add(egui::TextEdit::singleline(&mut self.temp_settings.authentication).desired_width(250.0));
                        ui.end_row();
                        // Removed Sound option from here
                    });
                ui.add_space(16.0);
                ui.label(egui::RichText::new("Bot Settings").size(18.0).strong());
                ui.add_space(2.0);
                ui.separator();
                egui::Grid::new("bot_settings_grid")
                    .num_columns(4)
                    .spacing([20.0, 8.0])
                    .show(ui, |ui| {
                        ui.label("Cooldown (seconds):");
                        ui.add(egui::DragValue::new(&mut self.temp_settings.cooldown).speed(1));
                        ui.label("Max Sentence Word Amount:");
                        ui.add(egui::DragValue::new(&mut self.temp_settings.max_sentence_word_amount).speed(1));
                        ui.end_row();
                        ui.label("Key Length:");
                        ui.add(egui::DragValue::new(&mut self.temp_settings.key_length).speed(1));
                        ui.label("Min Sentence Word Amount:");
                        ui.add(egui::DragValue::new(&mut self.temp_settings.min_sentence_word_amount).speed(1));
                        ui.end_row();
                        ui.label("Automatic Generation Timer (seconds):");
                        ui.add(egui::DragValue::new(&mut self.temp_settings.automatic_generation_timer).speed(1));
                        ui.end_row();
                    });
                ui.add_space(5.0);

                // User Lists and commands
                let bubble_height = 18.0;
                let font_id = egui::FontId::new(13.0, egui::FontFamily::Proportional);
                let font_id2 = font_id.clone();
                let font_id3 = font_id.clone();
                input_with_bubbles(ui, "Denied Users:", &mut self.denied_input, &mut self.temp_settings.denied_users, font_id.clone(), bubble_height, "denied_users", self.denied_left_spacing);
                input_with_bubbles(ui, "Allowed Users:", &mut self.allowed_input, &mut self.temp_settings.allowed_users, font_id2.clone(), bubble_height, "allowed_users", self.allowed_left_spacing);
                input_with_bubbles(ui, "Commands:", &mut self.generate_command_input, &mut self.temp_settings.generate_commands, font_id3.clone(), bubble_height, "commands", self.commands_left_spacing);
                ui.separator();
                ui.horizontal(|ui| {
                    if ui.button("Save").clicked() {
                        save_clicked = true;
                    }
                    if ui.button("Cancel").clicked() {
                        self.temp_settings = self.settings.clone();
                        cancel_clicked = true;
                    }
                    if ui.button("Reset to Defaults").clicked() {
                        self.temp_settings = BotSettings::default();
                    }
                });
            });
        });
        if cancel_clicked {
            self.is_open = false;
        }
        if save_clicked {
            self.save_settings();
        }
    }
} 