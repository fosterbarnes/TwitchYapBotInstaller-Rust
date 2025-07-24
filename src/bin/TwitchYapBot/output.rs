//! Output log display for TwitchYapBot
//!
//! This module handles rendering, scrolling, formatting, and filtering of logs for the TwitchYapBot GUI.

use eframe::egui;
use crate::gui::TwitchYapBotApp;
use std::collections::VecDeque;
use crate::log_and_print;

pub fn render_output_log(app: &mut TwitchYapBotApp, ctx: &egui::Context, _frame: &mut eframe::Frame) {
    egui::CentralPanel::default().show(ctx, |ui| {
        // Custom header row: arrow + label, both clickable, flush left
        let arrow_size = 18.0;
        let header_font = egui::FontId::new(15.0, egui::FontFamily::Name("consolas".into()));
        let label_text = "Yap Bot Output:";
        let label_galley = ui.fonts(|f| f.layout_no_wrap(label_text.to_owned(), header_font.clone(), egui::Color32::WHITE));
        let label_size = label_galley.size();
        let total_size = egui::vec2(arrow_size + 6.0 + label_size.x, arrow_size.max(label_size.y));
        let (rect, response) = ui.allocate_exact_size(total_size, egui::Sense::click());
        // Animate arrow rotation
        let anim_speed = 10.0; // higher = faster (units: 1.0 per second)
        let dt = ui.input(|i| i.stable_dt).min(0.05);
        if app.output_log_arrow_animating {
            let target = if app.output_log_arrow_target { 1.0 } else { 0.0 };
            let diff = target - app.output_log_arrow_anim;
            let step = anim_speed * dt;
            if diff.abs() <= step {
                app.output_log_arrow_anim = target;
                app.output_log_arrow_animating = false;
            } else {
                app.output_log_arrow_anim += step * diff.signum();
                ui.ctx().request_repaint();
            }
        }
        // Interpolated angle: 0 = right, 90deg = down
        let angle = app.output_log_arrow_anim * std::f32::consts::FRAC_PI_2;
        let arrow_rect = egui::Rect::from_min_size(rect.min, egui::vec2(arrow_size, arrow_size));
        let mut arrow_pos = arrow_rect.center();
        arrow_pos.y -= 3.0; // Nudge arrow up for better alignment
        // Base triangle (right arrow, centered at origin)
        let h = arrow_size * 0.5;
        let w = arrow_size * 0.5;
        let base_tri = [
            egui::pos2(-w * 0.3, -h * 0.7),
            egui::pos2(-w * 0.3, h * 0.7),
            egui::pos2(w * 0.6, 0.0),
        ];
        // Rotate and translate triangle
        let rot = |p: egui::Pos2| {
            let (sin, cos) = angle.sin_cos();
            egui::pos2(
                cos * p.x - sin * p.y + arrow_pos.x,
                sin * p.x + cos * p.y + arrow_pos.y,
            )
        };
        let tri: [egui::Pos2; 3] = [rot(base_tri[0]), rot(base_tri[1]), rot(base_tri[2])];
        let arrow_color = if response.hovered() {
            egui::Color32::WHITE
        } else {
            egui::Color32::from_rgb(184, 142, 244) // #B88EF4
        };
        ui.painter().add(egui::Shape::convex_polygon(tri.to_vec(), arrow_color, egui::Stroke::NONE));
        // Draw the label immediately to the right of the arrow
        let label_pos = egui::pos2(arrow_rect.right() + 6.0, rect.center().y - label_size.y / 2.0);
        ui.painter().galley(label_pos, label_galley, egui::Color32::WHITE);
        if response.clicked() {
            app.show_output_log = !app.show_output_log;
            app.output_log_arrow_target = app.show_output_log;
            app.output_log_arrow_animating = true;
            app.output_log_fade_target = app.show_output_log;
            app.output_log_fade_animating = true;
            if !app.show_output_log {
                // Collapsing: store current height (no window resize possible in eframe 0.28.1)
                let screen_rect = ctx.input(|i| i.screen_rect);
                let current_size = if screen_rect.is_positive() {
                    [screen_rect.width(), screen_rect.height()]
                } else {
                    [800.0, 517.0]
                };
                app.previous_window_height = Some(current_size[1]);
                app.is_window_minimized = true;
                log_and_print!("[GUI] Output log collapsed (show_output_log: false)");
            } else {
                // Expanding: restore previous height if available
                if app.is_window_minimized {
                    let screen_rect = ctx.input(|i| i.screen_rect);
                    let current_size = if screen_rect.is_positive() {
                        [screen_rect.width(), screen_rect.height()]
                    } else {
                        [800.0, 517.0]
                    };
                    let _restore_height = app.previous_window_height.unwrap_or(current_size[1].max(crate::config::MIN_WINDOW_SIZE[1]));
                    app.is_window_minimized = false;
                }
                log_and_print!("[GUI] Output log un-collapsed (show_output_log: true)");
            }
        }
        // Animate output log fade
        let fade_speed = 3.0; // higher = faster (units: 1.0 per second)
        let dt = ui.input(|i| i.stable_dt).min(0.05);
        if app.output_log_fade_animating {
            let target = if app.output_log_fade_target { 1.0 } else { 0.0 };
            let diff = target - app.output_log_fade_anim;
            let step = fade_speed * dt;
            if diff.abs() <= step {
                app.output_log_fade_anim = target;
                app.output_log_fade_animating = false;
            } else {
                app.output_log_fade_anim += step * diff.signum();
                ui.ctx().request_repaint();
            }
        }
        // Draw a full-width separator line below the header
        let sep_y = rect.bottom() + 2.0;
        let panel_rect = ui.max_rect();
        ui.painter().hline(
            panel_rect.left()..=panel_rect.right(),
            sep_y,
            egui::Stroke::new(1.0, egui::Color32::DARK_GRAY),
        );
        ui.add_space(sep_y - ui.cursor().top() + 8.0); // move cursor below separator
        // Output log
        if app.show_output_log || app.output_log_fade_animating || app.output_log_fade_anim > 0.0 {
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
                let n = display_lines.len().max(1) as f32;
                for (i, line) in display_lines.iter().enumerate() {
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
                    // Calculate opacity for this line
                    let frac = i as f32 / (n - 1.0).max(1.0);
                    let fade = if app.output_log_fade_anim >= 1.0 {
                        1.0
                    } else if app.output_log_fade_anim <= 0.0 {
                        0.0
                    } else {
                        (app.output_log_fade_anim - frac).clamp(0.0, 1.0)
                    };
                    let opacity = (fade * 255.0).round() as u8;
                    if line.contains("[TwitchWebsocket.TwitchWebsocket] [INFO    ] - Attempting to initialize websocket connection.") {
                        let timecode_re = regex::Regex::new(r"(\[\d{2}/\d{2}/\d{4} - \d{2}:\d{2}:\d{2}\]:)").unwrap();
                        if let Some(cap) = timecode_re.captures(line) {
                            let formatted = format!("{} Yap Bot is initializing websocket connection...", &cap[1]);
                            let rich = egui::RichText::new(formatted)
                                .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                                .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity));
                            ui.label(rich);
                        } else {
                            let rich = egui::RichText::new("[??/??/???? - ??:??:??]: Yap Bot is initializing websocket connection...")
                                .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                                .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity));
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
                            .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity))
                    } else {
                        egui::RichText::new(line)
                            .font(egui::FontId::new(13.0, egui::FontFamily::Name("consolas".into())))
                            .color(egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity))
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
                                    color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity),
                                    ..Default::default()
                                },
                            );
                            let msg = if msg.starts_with(" ") { msg } else { &format!(" {}", msg) };
                            job.append(
                                msg,
                                0.0,
                                TextFormat {
                                    font_id,
                                    color: egui::Color32::from_rgba_unmultiplied(189, 147, 249, opacity), // #9591f9
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
                                    color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity),
                                    ..Default::default()
                                },
                            );
                            let msg = if msg.starts_with(" ") { msg } else { &format!(" {}", msg) };
                            job.append(
                                msg,
                                0.0,
                                TextFormat {
                                    font_id,
                                    color: egui::Color32::from_rgba_unmultiplied(255, 85, 85, opacity), // #ff5555
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
                                    color: egui::Color32::from_rgba_unmultiplied(255, 255, 255, opacity),
                                    ..Default::default()
                                },
                            );
                            let msg = if msg.starts_with(" ") { msg } else { &format!(" {}", msg) };
                            job.append(
                                msg,
                                0.0,
                                TextFormat {
                                    font_id,
                                    color: egui::Color32::from_rgba_unmultiplied(238, 234, 113, opacity), // #eeea71
                                    ..Default::default()
                                },
                            );
                            ui.label(job);
                        } else if line.contains("error") || line.contains("Error") {
                            ui.colored_label(egui::Color32::from_rgba_unmultiplied(255, 85, 85, opacity), rich); // #ff5555
                        } else {
                            ui.label(rich);
                        }
                    } else if line.contains("error") || line.contains("Error") {
                        ui.colored_label(egui::Color32::from_rgba_unmultiplied(255, 85, 85, opacity), rich); // #ff5555
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
        }
    });
} 