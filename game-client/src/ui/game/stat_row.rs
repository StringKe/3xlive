//! 统计行组件：标签+值（SCORE/KILLS/属性等）

use bevy_egui::egui;

use crate::egui_setup::EguiColors;

/// 统计行：标签 + 值（标签用 font_s + 指定颜色，值用 font_m + 白色粗体）
pub fn stat_row(
    ui: &mut egui::Ui,
    label: &str,
    value: &str,
    color: egui::Color32,
    font_s: f32,
    font_m: f32,
    spacing: f32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        ui.label(
            egui::RichText::new(label)
                .size(font_s)
                .family(egui::FontFamily::Monospace)
                .color(color),
        );
        ui.label(
            egui::RichText::new(value)
                .size(font_m)
                .family(egui::FontFamily::Monospace)
                .color(EguiColors::TEXT_PRIMARY)
                .strong(),
        );
    });
}

/// 紧凑标签+值（标签灰色，值用指定颜色粗体）
pub fn stat_label(
    ui: &mut egui::Ui,
    label: &str,
    color: egui::Color32,
    value: &str,
    font_s: f32,
    spacing: f32,
) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        ui.label(
            egui::RichText::new(label)
                .size(font_s)
                .color(EguiColors::TEXT_TERTIARY),
        );
        ui.label(
            egui::RichText::new(value)
                .size(font_s)
                .color(color)
                .strong(),
        );
    });
}
