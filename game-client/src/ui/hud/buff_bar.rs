//! Buff 颜色工具（buff 显示已移入 bottom_panel）

use bevy_egui::egui;
use game_core::components::BuffType;

use crate::egui_setup::EguiColors;

/// Buff 类型对应的 egui 颜色
pub(crate) fn buff_color(bt: BuffType) -> egui::Color32 {
    match bt {
        BuffType::MaxHealth => EguiColors::BUFF_MAX_HEALTH,
        BuffType::MoveSpeed => EguiColors::BUFF_MOVE_SPEED,
        BuffType::DamageBoost => EguiColors::BUFF_DAMAGE_BOOST,
        BuffType::ExtraSlot => EguiColors::BUFF_EXTRA_SLOT,
        BuffType::FireRate => EguiColors::BUFF_FIRE_RATE,
        BuffType::SpreadReduction => EguiColors::BUFF_SPREAD_REDUCTION,
        BuffType::PickupRadius => EguiColors::BUFF_PICKUP_RADIUS,
        BuffType::XpBoost => EguiColors::BUFF_XP_BOOST,
    }
}

/// Buff 类型 → i18n key
pub(crate) fn buff_name_key(bt: &BuffType) -> &'static str {
    match bt {
        BuffType::MaxHealth => "ui.buff.max_health",
        BuffType::MoveSpeed => "ui.buff.move_speed",
        BuffType::DamageBoost => "ui.buff.damage_boost",
        BuffType::ExtraSlot => "ui.buff.extra_slot",
        BuffType::FireRate => "ui.buff.fire_rate",
        BuffType::SpreadReduction => "ui.buff.spread_reduction",
        BuffType::PickupRadius => "ui.buff.pickup_radius",
        BuffType::XpBoost => "ui.buff.xp_boost",
    }
}
