use eframe::egui::{self, Color32, FontId};

/// Render a list of bubbles with removable 'x' buttons, supporting multi-line wrapping.
/// Optionally, a special (non-removable) bubble can be rendered at the start of the first line.
pub fn bubble_list_ui(
    ui: &mut egui::Ui,
    items: &mut Vec<String>,
    special: Option<&str>, // e.g., bot channel name
    bubble_height: f32,
    font_id: FontId,
    bubble_color: Color32,
    text_color: Color32,
) -> Option<usize> {
    let max_width = ui.available_width();
    let bubble_padding = 12.0 * 2.0 + 22.0 + 4.0; // left+right padding + X btn + gap
    let mut lines: Vec<Vec<usize>> = vec![vec![]];
    let mut to_remove = None;
    let ctx = ui.ctx();
    // Measure special bubble width (if present)
    let mut first_line_width = 0.0;
    if let Some(special_text) = special {
        if !special_text.is_empty() {
            let galley = ctx.fonts(|f| f.layout_no_wrap(special_text.to_string(), font_id.clone(), Color32::BLACK));
            let bubble_width = galley.size().x + bubble_padding;
            first_line_width = bubble_width + 4.0;
        }
    }
    let mut current_width = first_line_width;
    for (i, item) in items.iter().enumerate() {
        let galley = ctx.fonts(|f| f.layout_no_wrap(item.to_string(), font_id.clone(), Color32::BLACK));
        let bubble_width = galley.size().x + bubble_padding;
        if current_width + bubble_width > max_width && !lines.last().unwrap().is_empty() {
            lines.push(vec![]);
            current_width = 0.0;
        }
        lines.last_mut().unwrap().push(i);
        current_width += bubble_width + 4.0;
    }
    let line_count = lines.len();
    for (line_idx, line) in lines.into_iter().enumerate() {
        ui.horizontal_top(|ui| {
            // Render special bubble as the first bubble if not empty
            if let Some(special_text) = special {
                if !special_text.is_empty() && line_idx == 0 {
                    let bubble = egui::Frame::none()
                        .fill(Color32::from_rgb(224, 224, 224))
                        .rounding(egui::Rounding::same(16.0))
                        .inner_margin(egui::Margin::symmetric(12.0, 4.0));
                    bubble.show(ui, |ui| {
                        ui.horizontal(|ui| {
                            let label = egui::Label::new(
                                egui::RichText::new(special_text)
                                    .strong()
                                    .color(Color32::BLACK)
                                    .font(font_id.clone())
                            );
                            let _ = ui.add_sized([ui.spacing().interact_size.x, bubble_height], label);
                        });
                    });
                    ui.add_space(4.0);
                }
            }
            for &i in &line {
                // Don't render the special bubble as a normal bubble
                if let Some(special_text) = special {
                    if !special_text.is_empty() && items[i].eq_ignore_ascii_case(special_text) {
                        continue;
                    }
                }
                let bubble = egui::Frame::none()
                    .fill(bubble_color)
                    .rounding(egui::Rounding::same(16.0))
                    .inner_margin(egui::Margin::symmetric(12.0, 4.0));
                bubble.show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let label = egui::Label::new(
                            egui::RichText::new(&items[i])
                                .strong()
                                .color(text_color)
                                .font(font_id.clone())
                        );
                        let _ = ui.add_sized([ui.spacing().interact_size.x, bubble_height], label);
                        let x_btn_size = bubble_height;
                        let x_btn_response = ui.add_sized([
                            x_btn_size, x_btn_size
                        ], egui::Button::new("").frame(false));
                        let x_btn_rect = x_btn_response.rect;
                        let painter = ui.painter();
                        let is_hovered = x_btn_response.hovered();
                        let circle_color = if is_hovered {
                            Color32::from_rgb(100, 100, 100)
                        } else {
                            Color32::from_rgb(41, 41, 41)
                        };
                        painter.circle_filled(x_btn_rect.center(), x_btn_size / 2.0, circle_color);
                        if is_hovered {
                            painter.circle_stroke(x_btn_rect.center(), x_btn_size / 2.0, egui::Stroke::new(1.5, Color32::WHITE));
                        }
                        let x_len = 4.0;
                        let x_color = Color32::WHITE;
                        let center = x_btn_rect.center();
                        painter.line_segment([
                            center + egui::vec2(-x_len, -x_len),
                            center + egui::vec2(x_len, x_len)
                        ], egui::Stroke::new(2.0, x_color));
                        painter.line_segment([
                            center + egui::vec2(-x_len, x_len),
                            center + egui::vec2(x_len, -x_len)
                        ], egui::Stroke::new(2.0, x_color));
                        if x_btn_response.clicked() {
                            to_remove = Some(i);
                        }
                    });
                });
                ui.add_space(4.0);
            }
        });
        if line_idx + 1 < line_count {
            ui.add_space(6.0);
        }
    }
    to_remove
} 