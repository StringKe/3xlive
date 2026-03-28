//! 居中容器组件：限制最大宽度并水平居中

use bevy_egui::egui;

/// 居中容器，限制最大宽度
pub fn container(ui: &mut egui::Ui, max_width: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    let avail_w = ui.available_width();
    let content_w = avail_w.min(max_width);
    let margin_left = (avail_w - content_w) / 2.0;
    let origin = ui.cursor().min + egui::vec2(margin_left, 0.0);
    let rect = egui::Rect::from_min_size(origin, egui::vec2(content_w, ui.available_height()));
    ui.scope_builder(egui::UiBuilder::new().max_rect(rect), |ui| {
        add_contents(ui);
    });
}
