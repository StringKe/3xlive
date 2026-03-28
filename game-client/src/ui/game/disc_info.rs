//! Disc 武器信息区：图标 + 品质名 + 属性统计

use bevy_egui::egui;
use game_core::components::DiscEquipped;
use game_core::data::disc::DiscQuality;

use super::SlotLoc;
use crate::egui_setup::EguiColors;

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

/// Disc 品质 -> i18n key
fn quality_i18n_key(q: &DiscQuality) -> &'static str {
    match q {
        DiscQuality::Common => "ui.quality.common",
        DiscQuality::Uncommon => "ui.quality.uncommon",
        DiscQuality::Rare => "ui.quality.rare",
        DiscQuality::Epic => "ui.quality.epic",
        DiscQuality::Legendary => "ui.quality.legendary",
    }
}

/// 渲染 Disc 信息区（图标 + 品质名 + 伤害/精准属性）
///
/// 整体作为 EquippedDisc drop target。拖拽 Disc 到此处可装备。
/// 返回 `Some(source_loc)` 表示有物品被拖入。
pub fn disc_info(
    ui: &mut egui::Ui,
    disc: Option<&DiscEquipped>,
    icon_size: f32,
    font_m: f32,
    font_s: f32,
    spacing: f32,
) -> Option<SlotLoc> {
    let mut drop_source: Option<SlotLoc> = None;

    let zone_resp = ui
        .scope(|ui| {
            ui.set_min_width(ui.available_width());
            ui.horizontal(|ui| {
                ui.spacing_mut().item_spacing.x = spacing;
                // Disc 六边形图标
                let (icon_rect, _) =
                    ui.allocate_exact_size(egui::vec2(icon_size, icon_size), egui::Sense::hover());
                let p = ui.painter();
                p.rect_filled(icon_rect, 0.0, EguiColors::BG_INSET);
                p.rect_stroke(
                    icon_rect,
                    0.0,
                    egui::Stroke::new(1.0, EguiColors::CYAN),
                    egui::StrokeKind::Inside,
                );
                p.text(
                    icon_rect.center(),
                    egui::Align2::CENTER_CENTER,
                    "\u{2B21}", // ⬡
                    egui::FontId::proportional(icon_size * 0.6),
                    EguiColors::CYAN,
                );

                // 品质名 + 属性
                if let Some(disc) = disc {
                    ui.vertical(|ui| {
                        ui.spacing_mut().item_spacing.y = spacing * 0.5;
                        ui.label(
                            egui::RichText::new(t(quality_i18n_key(&disc.disc.quality)))
                                .size(font_m)
                                .color(EguiColors::COLOR_XP)
                                .strong(),
                        );
                        ui.horizontal_wrapped(|ui| {
                            ui.spacing_mut().item_spacing.x = spacing;
                            super::stat_label(
                                ui,
                                &t("ui.hud.stat_dmg"),
                                EguiColors::COLOR_ENEMY,
                                &format!("{:.0}", 1.0 / disc.disc.base_fire_rate),
                                font_s,
                                spacing,
                            );
                            super::stat_label(
                                ui,
                                &t("ui.hud.stat_acc"),
                                EguiColors::CYAN,
                                &format!("{:.0}\u{00B0}", disc.disc.base_spread.to_degrees()),
                                font_s,
                                spacing,
                            );
                        });
                    });
                }
            });
        })
        .response;

    // 拖拽高亮
    if zone_resp.dnd_hover_payload::<SlotLoc>().is_some() {
        ui.painter().rect_stroke(
            zone_resp.rect,
            2.0,
            egui::Stroke::new(1.5, EguiColors::CYAN),
            egui::StrokeKind::Inside,
        );
    }

    // Drop 检测
    if let Some(source) = zone_resp.dnd_release_payload::<SlotLoc>() {
        drop_source = Some(*source);
    }

    drop_source
}
