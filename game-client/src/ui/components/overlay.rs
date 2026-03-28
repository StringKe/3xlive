//! Overlay 组件：在屏幕固定位置放置前景浮层

use bevy_egui::egui;

/// 浮层锚点位置
#[derive(Clone, Copy, Debug)]
pub enum OverlayAnchor {
    TopLeft,
    TopRight,
    BottomLeft,
}

/// 在屏幕指定锚点位置放置不可交互的前景浮层
///
/// - `content_left` / `content_right`：内容区域左右边界 x 坐标
/// - `screen_h`：屏幕高度
/// - `margin`：距边缘的间距
#[allow(clippy::too_many_arguments)]
pub fn overlay(
    ctx: &egui::Context,
    id: &str,
    anchor: OverlayAnchor,
    content_left: f32,
    content_right: f32,
    screen_h: f32,
    margin: f32,
    add_contents: impl FnOnce(&mut egui::Ui),
) {
    let area = egui::Area::new(egui::Id::new(id)).order(egui::Order::Foreground);

    match anchor {
        OverlayAnchor::TopLeft => {
            area.fixed_pos(egui::pos2(content_left + margin, margin))
                .interactable(false)
                .show(ctx, |ui| add_contents(ui));
        }
        OverlayAnchor::TopRight => {
            area.anchor(
                egui::Align2::RIGHT_TOP,
                [
                    -(margin + (ctx.content_rect().width() - content_right)),
                    margin,
                ],
            )
            .interactable(false)
            .show(ctx, |ui| add_contents(ui));
        }
        OverlayAnchor::BottomLeft => {
            area.fixed_pos(egui::pos2(content_left + margin, screen_h - margin - 100.0))
                .interactable(false)
                .show(ctx, |ui| add_contents(ui));
        }
    };
}
