//! 按钮组件：Primary / Secondary / Ghost 三种变体

use bevy_egui::egui;

use crate::egui_setup::EguiColors;

/// 按钮变体
#[derive(Clone, Copy, Debug, Default)]
pub enum ButtonVariant {
    #[default]
    Primary,
    Secondary,
    Ghost,
}

/// 按钮属性
pub struct ButtonProps {
    pub label: String,
    pub variant: ButtonVariant,
    pub font_size: f32,
    pub enabled: bool,
    pub min_width: Option<f32>,
    /// 最小高度，None 时由 egui 自动计算
    pub min_height: Option<f32>,
    /// 自定义文字颜色，None 时由变体决定
    pub color: Option<egui::Color32>,
}

impl Default for ButtonProps {
    fn default() -> Self {
        Self {
            label: String::new(),
            variant: ButtonVariant::Primary,
            font_size: 11.0,
            enabled: true,
            min_width: None,
            min_height: None,
            color: None,
        }
    }
}

/// 渲染按钮，返回是否被点击
pub fn button(ui: &mut egui::Ui, props: &ButtonProps) -> bool {
    let text = egui::RichText::new(&props.label).size(props.font_size);

    let default_color = match props.variant {
        ButtonVariant::Primary => EguiColors::TEXT_PRIMARY,
        ButtonVariant::Secondary => EguiColors::TEXT_SECONDARY,
        ButtonVariant::Ghost => EguiColors::CYAN,
    };
    let text = text.color(props.color.unwrap_or(default_color));

    let mut btn = egui::Button::new(text);

    match props.variant {
        ButtonVariant::Primary => {
            btn = btn
                .fill(EguiColors::BTN_FILL)
                .stroke(egui::Stroke::new(1.0, EguiColors::BORDER_CYAN));
        }
        ButtonVariant::Secondary => {
            btn = btn
                .fill(EguiColors::BTN_FILL)
                .stroke(egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE));
        }
        ButtonVariant::Ghost => {
            btn = btn
                .fill(egui::Color32::TRANSPARENT)
                .stroke(egui::Stroke::NONE);
        }
    }

    let min_w = props.min_width.unwrap_or(0.0);
    let min_h = props.min_height.unwrap_or(0.0);
    if min_w > 0.0 || min_h > 0.0 {
        btn = btn.min_size(egui::vec2(min_w, min_h));
    }

    let response = ui.add_enabled(props.enabled, btn);
    response.clicked()
}
