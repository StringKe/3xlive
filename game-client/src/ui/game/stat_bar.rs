//! 带标签的进度条（HP/XP/能量等）

use bevy_egui::egui;

use crate::egui_setup::EguiColors;

/// 带标签的进度条
///
/// 布局：`[label_w] [spacing] [bar] [spacing] [value_w]`
/// 总宽度由 `total_width` 控制，bar 自适应填充剩余空间。
#[allow(clippy::too_many_arguments)]
pub fn stat_bar(
    ui: &mut egui::Ui,
    label: &str,
    color: egui::Color32,
    ratio: f32,
    value_text: &str,
    total_width: f32,
    label_w: f32,
    bar_h: f32,
    font_s: f32,
    spacing: f32,
) {
    let value_w = font_s * 5.0;
    let bar_width = (total_width - label_w - value_w - spacing * 2.0).max(40.0);

    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;

        // 标签
        let (label_rect, _) =
            ui.allocate_exact_size(egui::vec2(label_w, bar_h), egui::Sense::hover());
        ui.painter().text(
            label_rect.left_center(),
            egui::Align2::LEFT_CENTER,
            label,
            egui::FontId::proportional(font_s),
            color,
        );

        // 进度条
        let (bar_rect, _) =
            ui.allocate_exact_size(egui::vec2(bar_width, bar_h), egui::Sense::hover());
        let p = ui.painter();
        p.rect_filled(bar_rect, 0.0, EguiColors::BG_INSET);
        p.rect_stroke(
            bar_rect,
            0.0,
            egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE),
            egui::StrokeKind::Inside,
        );
        if ratio > 0.0 {
            let fill =
                egui::Rect::from_min_size(bar_rect.min, egui::vec2(bar_width * ratio, bar_h));
            p.rect_filled(fill, 0.0, color);
        }

        // 数值文本
        let (val_rect, _) =
            ui.allocate_exact_size(egui::vec2(value_w, bar_h), egui::Sense::hover());
        ui.painter().text(
            val_rect.right_center(),
            egui::Align2::RIGHT_CENTER,
            value_text,
            egui::FontId::proportional(font_s),
            EguiColors::TEXT_SECONDARY,
        );
    });
}
