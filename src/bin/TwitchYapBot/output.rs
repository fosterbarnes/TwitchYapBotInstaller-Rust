// Output log display for TwitchYapBot
// Handles rendering, scrolling, formatting, and filtering of logs

use eframe::egui;
use crate::gui::TwitchYapBotApp;
use std::collections::VecDeque;

pub fn render_output_log(app: &mut TwitchYapBotApp, ctx: &egui::Context) {
    egui::CentralPanel::default().show(ctx, |ui| {

        ui.label(
            egui::RichText::new("Yap Bot Output:")
                .size(15.0)
                .font(egui::FontId::new(15.0, egui::FontFamily::Name("consolas".into())))
        );
        ui.separator();
        let scroll_id = ui.make_persistent_id("output_scroll_area");
        let mut num_displayed = 0;
        let mut end_rect = None;
        egui::ScrollArea::vertical().id_source(scroll_id).auto_shrink([false; 2]).show(ui, |ui| {
            let lines: VecDeque<String> = {
                let guard = app.output_lines.lock().unwrap();
                guard.clone()
            };
            let websocket_marker = "[TwitchWebsocket.TwitchWebsocket] [INFO    ] - Attempting to initialize websocket connection.";
            if app.marker_index.is_none() {
                app.marker_index = lines.iter().position(|line| line.contains(websocket_marker));
            }
            let start = app.marker_index.unwrap_or(usize::MAX);
            let mut seen = std::collections::HashSet::new();
            let display_lines: Vec<String> = if start < lines.len() {
                let after_marker: Vec<&String> = lines.iter().skip(start).collect();
                if after_marker.len() > 200 {
                    after_marker[after_marker.len() - 200..].iter().cloned().cloned().collect()
                } else {
                    after_marker.iter().cloned().cloned().collect()
                }
            } else {
                vec![]
            };
            let log_re = regex::Regex::new(r"^\[(\d{2})/(\d{2})/(\d{4}) - (\d{2}):(\d{2}):(\d{2})\]: (.+)$").unwrap();
            for line in display_lines.iter() {
                if !seen.insert(line) {
                    continue; // skip all duplicates
                }
                if line.contains("Fetching mod list...") ||
                   line.contains("Unrecognized command: /mods") ||
                   line.contains("Unrecognized command: /w") {
                    continue;
                }
                if line.contains("SyntaxWarning: invalid escape sequence '\\w'") && line.contains("MarkovChainBot.py") {
                    continue;
                }
                if line.contains("self.link_regex = re.compile(\"\\w+\\.[a-z]{2,}\")") {
                    continue;
                }
                if line.contains("[TwitchWebsocket.TwitchWebsocket] [INFO    ] - Attempting to initialize websocket connection.") {
                    let timecode_re = regex::Regex::new(r"(\[\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}\]:)").unwrap();
                    if let Some(cap) = timecode_re.captures(line) {
                        let formatted = format!("{} Yap Bot is initializing websocket connection...", &cap[1]);
                        let rich = egui::RichText::new(formatted)
                            .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())));
                        ui.label(rich);
                    } else {
                        let rich = egui::RichText::new("[??/??/???? - ??:??:??]: Yap Bot is initializing websocket connection...")
                            .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())));
                        ui.label(rich);
                    }
                    num_displayed += 1;
                    continue;
                }
                let rich = if let Some(caps) = log_re.captures(line) {
                    let formatted = format!(
                        "[{}/{}/{} - {}:{}:{}]: {}",
                        &caps[1], &caps[2], &caps[3], &caps[4], &caps[5], &caps[6], &caps[7]
                    );
                    egui::RichText::new(formatted)
                        .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                } else {
                    egui::RichText::new(line)
                        .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                };
                if let Some(caps) = log_re.captures(line) {
                    let msg = &caps[7];
                    let is_status = msg.contains("Yap Bot is initializing websocket connection...")
                        || msg.contains("TCP server for manual triggers listening")
                        || msg.contains("Successfully joined channel:")
                        || msg.contains("Yap Bot has been destroyed by your own hands...")
                        || msg.contains("Reviving Yap Bot from the depths of hell...");
                    if !msg.trim_start().starts_with("(manual trigger)") && !is_status {
                        use egui::text::{LayoutJob, TextFormat};
                        let mut job = LayoutJob::default();
                        let timestamp = format!(
                            "[{}/{}/{} - {}:{}:{}]:",
                            &caps[1], &caps[2], &caps[3], &caps[4], &caps[5], &caps[6]
                        );
                        let font_id = egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into()));
                        job.append(
                            &timestamp,
                            0.0,
                            TextFormat {
                                font_id: font_id.clone(),
                                color: egui::Color32::WHITE, // #f8f8f2
                                ..Default::default()
                            },
                        );
                        let msg = if msg.starts_with(" ") { msg } else { &format!(" {}", msg) };
                        job.append(
                            msg,
                            0.0,
                            TextFormat {
                                font_id,
                                color: egui::Color32::from_rgb(189, 147, 249), // #9591f9
                                ..Default::default()
                            },
                        );
                        ui.label(job);
                    } else if msg.contains("Yap Bot has been destroyed by your own hands...") {
                        use egui::text::{LayoutJob, TextFormat};
                        let mut job = LayoutJob::default();
                        let timestamp = format!(
                            "[{}/{}/{} - {}:{}:{}]:",
                            &caps[1], &caps[2], &caps[3], &caps[4], &caps[5], &caps[6]
                        );
                        let font_id = egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into()));
                        job.append(
                            &timestamp,
                            0.0,
                            TextFormat {
                                font_id: font_id.clone(),
                                color: egui::Color32::WHITE, // #f8f8f2
                                ..Default::default()
                            },
                        );
                        let msg = if msg.starts_with(" ") { msg } else { &format!(" {}", msg) };
                        job.append(
                            msg,
                            0.0,
                            TextFormat {
                                font_id,
                                color: egui::Color32::from_rgb(255, 85, 85), // #ff5555
                                ..Default::default()
                            },
                        );
                        ui.label(job);
                    } else if msg.contains("Reviving Yap Bot from the depths of hell...") {
                        use egui::text::{LayoutJob, TextFormat};
                        let mut job = LayoutJob::default();
                        let timestamp = format!(
                            "[{}/{}/{} - {}:{}:{}]:",
                            &caps[1], &caps[2], &caps[3], &caps[4], &caps[5], &caps[6]
                        );
                        let font_id = egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into()));
                        job.append(
                            &timestamp,
                            0.0,
                            TextFormat {
                                font_id: font_id.clone(),
                                color: egui::Color32::WHITE, // #f8f8f2
                                ..Default::default()
                            },
                        );
                        let msg = if msg.starts_with(" ") { msg } else { &format!(" {}", msg) };
                        job.append(
                            msg,
                            0.0,
                            TextFormat {
                                font_id,
                                color: egui::Color32::from_rgb(238, 234, 113), // #eeea71
                                ..Default::default()
                            },
                        );
                        ui.label(job);
                    } else if line.contains("error") || line.contains("Error") {
                        ui.colored_label(egui::Color32::from_rgb(255, 85, 85), rich); // #ff5555
                    } else {
                        ui.label(rich);
                    }
                } else if line.contains("error") || line.contains("Error") {
                    ui.colored_label(egui::Color32::from_rgb(255, 85, 85), rich); // #ff5555
                } else {
                    ui.label(rich);
                }
                num_displayed += 1;
            }
            let end_resp = ui.label(egui::RichText::new("").font(egui::FontId::new(1.0, egui::FontFamily::Monospace)));
            if app.auto_scroll && num_displayed > app.last_num_displayed {
                end_resp.scroll_to_me(Some(egui::Align::BOTTOM));
            }
            end_rect = Some(end_resp.rect);
        });
        let at_bottom = if let Some(rect) = end_rect {
            let clip_bottom = ctx.input(|i| i.screen_rect().bottom());
            (rect.bottom() - clip_bottom).abs() < 5.0 || rect.bottom() < clip_bottom
        } else {
            true
        };
        if at_bottom {
            app.auto_scroll = true;
        } else {
            app.auto_scroll = false;
        }
        app.last_num_displayed = num_displayed;
    });
} 