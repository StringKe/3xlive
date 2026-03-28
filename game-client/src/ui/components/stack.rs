//! 垂直堆叠布局组件

use bevy_egui::egui;

/// 垂直堆叠布局，自定义间距
pub fn stack(ui: &mut egui::Ui, spacing: f32, add_contents: impl FnOnce(&mut egui::Ui)) {
    ui.vertical(|ui| {
        ui.spacing_mut().item_spacing.y = spacing;
        add_contents(ui);
    });
}
