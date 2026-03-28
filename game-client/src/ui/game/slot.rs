//! 通用槽位组件：支持拖拽、tooltip、多种内容类型

use bevy_egui::egui;

use crate::egui_setup::EguiColors;
use crate::ui::inventory::{
    format_item_tooltip, format_program_tooltip, program_type_color, program_type_icon,
    quality_color,
};
use crate::ui::{ClientItem, ClientProgram};

#[cfg(target_arch = "wasm32")]
use crate::js_bridge;

/// i18n helper
fn t(key: &str) -> String {
    #[cfg(target_arch = "wasm32")]
    {
        js_bridge::t(key)
    }
    #[cfg(not(target_arch = "wasm32"))]
    {
        key.to_string()
    }
}

/// Program 类型 -> 短名 i18n key
fn program_type_short_key(pt: game_core::data::program::ProgramType) -> &'static str {
    match pt {
        game_core::data::program::ProgramType::Projectile => "ui.program_type.short.projectile",
        game_core::data::program::ProgramType::TrajectoryMod => {
            "ui.program_type.short.trajectory_mod"
        }
        game_core::data::program::ProgramType::QuantityMod => "ui.program_type.short.quantity_mod",
        game_core::data::program::ProgramType::Trigger => "ui.program_type.short.trigger",
        game_core::data::program::ProgramType::StatusEffect => {
            "ui.program_type.short.status_effect"
        }
        game_core::data::program::ProgramType::FormationMod => {
            "ui.program_type.short.formation_mod"
        }
        game_core::data::program::ProgramType::TimingMod => "ui.program_type.short.timing_mod",
        game_core::data::program::ProgramType::DamageMod => "ui.program_type.short.damage_mod",
        game_core::data::program::ProgramType::ProjectileMod => {
            "ui.program_type.short.projectile_mod"
        }
        game_core::data::program::ProgramType::FieldMod => "ui.program_type.short.field_mod",
    }
}

// ─── SlotLoc ───

/// 槽位拖拽标识
/// EquippedDisc：只能作为 drop target（不能作为 source 拖出）
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum SlotLoc {
    /// 背包格子
    Bag(usize),
    /// Disc 上的 Program 槽位
    DiscSlot(usize),
    /// 武器装配区，只能作为 drop target
    EquippedDisc,
    /// 垃圾桶
    Trash,
}

// ─── SlotContent ───

/// 槽位内容描述
pub enum SlotContent<'a> {
    /// 空槽位
    Empty,
    /// 锁定槽位（未解锁）
    Locked,
    /// 背包物品（Disc 或 Program）
    BagItem(&'a ClientItem),
    /// Disc 上的 Program 槽位（None 表示空槽位）
    DiscProgram(Option<&'a ClientProgram>),
    /// Buff 槽位
    Buff { color: egui::Color32, name: String },
}

// ─── SlotResponse ───

/// 槽位交互结果
pub struct SlotResponse {
    /// 是否被双击
    pub double_clicked: bool,
    /// 如果有物品被拖入，返回来源 SlotLoc
    pub drop_source: Option<SlotLoc>,
}

// ─── 主函数 ───

/// 通用槽位渲染
///
/// 支持拖拽（dnd_enabled=true 时作为 drag source + drop target）、
/// hover tooltip、双击快捷操作。
///
/// `font_l` / `font_s` 由 slot size 推导。
pub fn slot(
    ui: &mut egui::Ui,
    id: egui::Id,
    loc: SlotLoc,
    content: &SlotContent<'_>,
    size: f32,
    dnd_enabled: bool,
) -> SlotResponse {
    let slot_vec = egui::vec2(size, size);
    let font_l = (size * 0.4).clamp(11.0, 18.0);
    let font_s = (size * 0.25).clamp(7.0, 13.0);

    let mut dbl_clicked = false;
    let mut drop_source: Option<SlotLoc> = None;

    // 渲染闭包
    let paint = |ui: &mut egui::Ui| -> bool {
        let (rect, resp) = ui.allocate_exact_size(slot_vec, egui::Sense::click());
        paint_slot(ui.painter(), rect, content, font_l, font_s);
        resp.double_clicked()
    };

    // 拖拽 / 非拖拽渲染
    let response = if dnd_enabled && !matches!(content, SlotContent::Locked) {
        let resp = ui.dnd_drag_source(id, loc, |ui| {
            dbl_clicked = paint(ui);
        });
        resp.response
    } else {
        // 无拖拽的槽位也需要占位
        let (rect, resp) = ui.allocate_exact_size(slot_vec, egui::Sense::click());
        paint_slot(ui.painter(), rect, content, font_l, font_s);
        dbl_clicked = resp.double_clicked();
        resp
    };

    // Drop 检测
    if dnd_enabled {
        if let Some(source) = response.dnd_release_payload::<SlotLoc>() {
            drop_source = Some(*source);
        }

        // Hover 高亮
        if response.dnd_hover_payload::<SlotLoc>().is_some() {
            ui.painter().rect_stroke(
                response.rect,
                4.0,
                egui::Stroke::new(2.0, EguiColors::CYAN),
                egui::StrokeKind::Inside,
            );
        }
    }

    // Tooltip（非拖拽中 + 非 Locked/Empty/Buff 时）
    let dragging = ui.ctx().dragged_id().is_some();
    if !dragging {
        if let Some(pos) = ui.ctx().pointer_hover_pos() {
            if response.rect.contains(pos) {
                let tooltip_text = match content {
                    SlotContent::BagItem(item) => Some(format_item_tooltip(item)),
                    SlotContent::DiscProgram(Some(prog)) => Some(format_program_tooltip(prog)),
                    _ => None,
                };
                if let Some(text) = tooltip_text {
                    let tip_id = id.with("tip");
                    egui::Tooltip::always_open(
                        ui.ctx().clone(),
                        egui::LayerId::new(egui::Order::Tooltip, tip_id),
                        tip_id,
                        egui::PopupAnchor::Pointer,
                    )
                    .show(|ui| {
                        ui.set_min_width(200.0);
                        ui.label(text);
                    });
                }
            }
        }
    }

    SlotResponse {
        double_clicked: dbl_clicked,
        drop_source,
    }
}

// ─── 绘制实现 ───

/// 根据 SlotContent variant 绘制槽位
fn paint_slot(
    p: &egui::Painter,
    rect: egui::Rect,
    content: &SlotContent<'_>,
    font_l: f32,
    font_s: f32,
) {
    match content {
        SlotContent::Empty => {
            p.rect_filled(rect, 4.0, EguiColors::BG_INSET);
            p.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, EguiColors::SLOT_BORDER_DIM),
                egui::StrokeKind::Inside,
            );
            p.text(
                rect.center(),
                egui::Align2::CENTER_CENTER,
                "+",
                egui::FontId::proportional(font_l),
                EguiColors::SLOT_BORDER_DIM,
            );
        }
        SlotContent::Locked => {
            p.rect_filled(rect, 4.0, EguiColors::SLOT_LOCKED_BG);
            p.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, EguiColors::SLOT_BORDER_DIM),
                egui::StrokeKind::Inside,
            );
            // X 叉号
            let cx = rect.center();
            let d = rect.width() * 0.15;
            let stroke = egui::Stroke::new(1.5, EguiColors::TEXT_MUTED);
            p.line_segment([cx - egui::vec2(d, d), cx + egui::vec2(d, d)], stroke);
            p.line_segment([cx - egui::vec2(-d, d), cx + egui::vec2(-d, d)], stroke);
        }
        SlotContent::BagItem(item) => {
            let (icon, icon_color, border_color) = match item {
                ClientItem::Disc(disc) => (
                    "\u{25C6}", // ◆
                    quality_color(disc.quality),
                    quality_color(disc.quality),
                ),
                ClientItem::Program(prog) => (
                    program_type_icon(prog.program_type),
                    program_type_color(prog.program_type),
                    EguiColors::PROGRAM_SLOT_ACTIVE,
                ),
            };
            p.rect_filled(rect, 4.0, EguiColors::BG_INSET);
            p.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, border_color),
                egui::StrokeKind::Inside,
            );
            if !icon.is_empty() {
                p.text(
                    rect.center(),
                    egui::Align2::CENTER_CENTER,
                    icon,
                    egui::FontId::proportional(font_l),
                    icon_color,
                );
            }
        }
        SlotContent::DiscProgram(opt_prog) => {
            let (icon, icon_color, border_color) = match opt_prog {
                Some(prog) => (
                    program_type_icon(prog.program_type),
                    program_type_color(prog.program_type),
                    EguiColors::PROGRAM_SLOT_ACTIVE,
                ),
                None => (
                    "+",
                    EguiColors::SLOT_BORDER_DIM,
                    EguiColors::SLOT_BORDER_DIM,
                ),
            };

            p.rect_filled(rect, 4.0, EguiColors::BG_INSET);
            p.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, border_color),
                egui::StrokeKind::Inside,
            );

            // 图标位置：有短名时上移，无短名时居中
            let icon_y_offset = if opt_prog.is_some() {
                font_s * 0.3
            } else {
                0.0
            };
            p.text(
                rect.center() - egui::vec2(0.0, icon_y_offset),
                egui::Align2::CENTER_CENTER,
                icon,
                egui::FontId::proportional(font_l),
                icon_color,
            );

            // 短名
            if let Some(prog) = opt_prog {
                let label = t(program_type_short_key(prog.program_type));
                p.text(
                    egui::pos2(rect.center().x, rect.bottom() - font_s * 0.5),
                    egui::Align2::CENTER_CENTER,
                    label,
                    egui::FontId::proportional(font_s * 0.7),
                    EguiColors::TEXT_SECONDARY,
                );
            }
        }
        SlotContent::Buff { color, name: _ } => {
            p.rect_filled(rect, 4.0, *color);
            p.rect_stroke(
                rect,
                4.0,
                egui::Stroke::new(1.0, EguiColors::CYAN),
                egui::StrokeKind::Inside,
            );
        }
    }
}
