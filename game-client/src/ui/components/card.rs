//! Card 组件：带背景色和内边距的矩形容器

use bevy_egui::egui;

/// 在指定矩形区域绘制带背景的卡片，内部内容有 padding
pub fn card(
    ui: &mut egui::Ui,
    rect: egui::Rect,
    bg: egui::Color32,
    rounding: f32,
    padding: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    ui.painter().rect_filled(rect, rounding, bg);
    let inner = rect.shrink(padding);
    ui.scope_builder(egui::UiBuilder::new().max_rect(inner), |ui| {
        add_contents(ui);
    });
}
