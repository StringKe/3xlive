//! 文本组件：统一文本渲染，支持多种尺寸和权重

use bevy_egui::egui;

use crate::egui_setup::EguiColors;

/// 文本尺寸
#[derive(Clone, Copy, Debug, Default)]
pub enum TextSize {
    /// 7.0px
    Xs,
    /// 9.0px
    Sm,
    /// 11.0px（默认）
    #[default]
    Md,
    /// 14.0px
    Lg,
    /// 18.0px
    Xl,
}

impl TextSize {
    /// 返回对应的像素值
    pub fn px(self) -> f32 {
        match self {
            Self::Xs => 7.0,
            Self::Sm => 9.0,
            Self::Md => 11.0,
            Self::Lg => 14.0,
            Self::Xl => 18.0,
        }
    }
}

/// 文本权重
#[derive(Clone, Copy, Debug, Default)]
pub enum TextWeight {
    #[default]
    Normal,
    Bold,
}

/// 文本属性
pub struct TextProps {
    pub text: String,
    pub size: TextSize,
    pub color: egui::Color32,
    pub weight: TextWeight,
    pub font: egui::FontFamily,
}

impl Default for TextProps {
    fn default() -> Self {
        Self {
            text: String::new(),
            size: TextSize::Md,
            color: EguiColors::TEXT_PRIMARY,
            weight: TextWeight::Normal,
            font: egui::FontFamily::Proportional,
        }
    }
}

/// 渲染文本
pub fn text(ui: &mut egui::Ui, props: &TextProps) {
    let font_id = egui::FontId::new(props.size.px(), props.font.clone());
    let job = egui::text::LayoutJob::simple_singleline(props.text.clone(), font_id, props.color);
    let galley = ui.fonts_mut(|f| f.layout_job(job));

    // 根据 weight 决定是否加粗（用描边模拟）
    let (rect, _response) = ui.allocate_exact_size(galley.size(), egui::Sense::hover());
    let pos = rect.min;
    let painter = ui.painter();

    match props.weight {
        TextWeight::Normal => {
            painter.galley(pos, galley, props.color);
        }
        TextWeight::Bold => {
            // 绘制两次：水平偏移 0.5px 模拟粗体
            painter.galley(pos, galley.clone(), props.color);
            painter.galley(pos + egui::vec2(0.5, 0.0), galley, props.color);
        }
    }
}
