//! 进度条组件：带背景、填充、边框的矩形进度条

use bevy_egui::egui;

use crate::egui_setup::EguiColors;

/// 进度条属性
pub struct ProgressProps {
    pub ratio: f32,
    pub color: egui::Color32,
    pub width: f32,
    pub height: f32,
}

/// 渲染进度条
pub fn progress(ui: &mut egui::Ui, props: &ProgressProps) {
    let (rect, _response) =
        ui.allocate_exact_size(egui::vec2(props.width, props.height), egui::Sense::hover());

    let painter = ui.painter();

    // 底色
    painter.rect_filled(rect, 0.0, EguiColors::BG_INSET);

    // 按 ratio 填充
    let ratio = props.ratio.clamp(0.0, 1.0);
    if ratio > 0.0 {
        let fill_rect =
            egui::Rect::from_min_size(rect.min, egui::vec2(rect.width() * ratio, rect.height()));
        painter.rect_filled(fill_rect, 0.0, props.color);
    }

    // 1px 边框
    painter.rect_stroke(
        rect,
        0.0,
        egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE),
        egui::StrokeKind::Inside,
    );
}
