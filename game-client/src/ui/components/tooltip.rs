//! Tooltip 组件：hover 时在指针位置显示浮层

use bevy_egui::egui;

/// 在指定矩形区域 hover 时显示 tooltip
///
/// 拖拽进行中时自动跳过，避免干扰拖拽操作
pub fn tooltip_on_hover(
    ctx: &egui::Context,
    id: egui::Id,
    rect: egui::Rect,
    min_width: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    // 拖拽中不显示 tooltip
    if ctx.dragged_id().is_some() {
        return;
    }
    if let Some(pos) = ctx.pointer_hover_pos() {
        if rect.contains(pos) {
            let tip_id = id.with("tip");
            egui::Tooltip::always_open(
                ctx.clone(),
                egui::LayerId::new(egui::Order::Tooltip, tip_id),
                tip_id,
                egui::PopupAnchor::Pointer,
            )
            .show(|ui| {
                ui.set_min_width(min_width);
                add_contents(ui);
            });
        }
    }
}
