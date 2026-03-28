//! 水平分组布局组件

use bevy_egui::egui;

/// 水平分组布局，自定义间距
pub fn group(ui: &mut egui::Ui, spacing: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.horizontal(|ui| {
        ui.spacing_mut().item_spacing.x = spacing;
        add_contents(ui);
    });
}
