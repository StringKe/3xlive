//! 分割线组件：水平/垂直 1px 线条

use bevy_egui::egui;

use crate::egui_setup::EguiColors;

/// 分割线方向
#[derive(Clone, Copy, Debug)]
pub enum DividerDirection {
    Horizontal,
    Vertical,
}

/// 绘制分割线
pub fn divider(painter: &egui::Painter, pos: egui::Pos2, length: f32, direction: DividerDirection) {
    let rect = match direction {
        DividerDirection::Horizontal => egui::Rect::from_min_size(pos, egui::vec2(length, 1.0)),
        DividerDirection::Vertical => egui::Rect::from_min_size(pos, egui::vec2(1.0, length)),
    };
    painter.rect_filled(rect, 0.0, EguiColors::BORDER_SUBTLE);
}
