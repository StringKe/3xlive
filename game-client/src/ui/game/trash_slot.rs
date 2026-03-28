//! 垃圾桶槽位：拖入物品丢弃

use bevy_egui::egui;

use super::SlotLoc;
use crate::egui_setup::EguiColors;

/// 垃圾桶 drop target
///
/// 渲染一个带删除图标的槽位。拖拽物品到此处触发丢弃。
/// 返回 `Some(source)` 表示有物品被拖入（来源 SlotLoc）。
pub fn trash_slot(ui: &mut egui::Ui, size: f32, font_size: f32) -> Option<SlotLoc> {
    let slot_vec = egui::vec2(size, size);
    let (rect, resp) = ui.allocate_exact_size(slot_vec, egui::Sense::hover());

    let is_hover = resp.dnd_hover_payload::<SlotLoc>().is_some();
    let border_color = if is_hover {
        EguiColors::COLOR_ENEMY
    } else {
        EguiColors::BORDER_SUBTLE
    };
    let icon_color = if is_hover {
        EguiColors::COLOR_ENEMY
    } else {
        EguiColors::TEXT_TERTIARY
    };

    let p = ui.painter();
    p.rect_filled(rect, 4.0, EguiColors::BG_INSET);
    p.rect_stroke(
        rect,
        4.0,
        egui::Stroke::new(1.0, border_color),
        egui::StrokeKind::Inside,
    );
    // 删除符号 ␡
    p.text(
        rect.center(),
        egui::Align2::CENTER_CENTER,
        "\u{2421}",
        egui::FontId::proportional(font_size * 0.7),
        icon_color,
    );

    resp.dnd_release_payload::<SlotLoc>().map(|src| *src)
}
