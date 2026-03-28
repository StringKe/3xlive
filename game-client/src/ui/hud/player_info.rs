//! 玩家信息浮层：左上角显示玩家名、等级、HP/XP 条、KILLS

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use game_core::components::{Health, Level};

use crate::egui_setup::EguiColors;
use crate::sync::LocalPlayerMarker;
use crate::ui::components::{overlay, rem, t, OverlayAnchor};
use crate::ui::game::{stat_bar, stat_row};
use crate::ui::{LocalPlayerInfo, PlayerStats};

use super::HudLayout;

/// 玩家信息浮层：名字、等级、HP/XP 条、KILLS
pub(crate) fn player_info_overlay_system(
    mut contexts: EguiContexts,
    hud: Res<HudLayout>,
    hp_q: Query<&Health, With<LocalPlayerMarker>>,
    level_q: Query<&Level, With<LocalPlayerMarker>>,
    player_info: Res<LocalPlayerInfo>,
    stats: Res<PlayerStats>,
) {
    let sw = hud.screen_w;
    let sb = rem(sw, 0.5);
    let font_s = rem(sw, 0.65);
    let font_m = rem(sw, 0.8);
    let font_l = (sb * 1.4).clamp(11.0, 18.0);
    let bar_h = (sb * 2.0).clamp(10.0, 22.0);

    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    overlay(
        ctx,
        "hud_player_info",
        OverlayAnchor::TopLeft,
        hud.content_left(),
        hud.content_right(),
        hud.screen_h,
        8.0,
        |ui| {
            egui::Frame::new()
                .fill(EguiColors::PANEL_BG)
                .stroke(egui::Stroke::new(1.0, EguiColors::CYAN_FAINT))
                .inner_margin(egui::Margin::symmetric(10, 8))
                .show(ui, |ui| {
                    ui.spacing_mut().item_spacing.y = sb;
                    ui.set_min_width(180.0);

                    // 玩家名称
                    ui.label(
                        egui::RichText::new(&player_info.display_name)
                            .size(font_l)
                            .color(EguiColors::CYAN)
                            .strong(),
                    );

                    // 等级
                    if let Ok(level) = level_q.single() {
                        ui.label(
                            egui::RichText::new(
                                t("ui.hud.level").replace("{0}", &level.value.to_string()),
                            )
                            .size(font_m)
                            .color(EguiColors::TEXT_SECONDARY),
                        );
                    }

                    // HP 条
                    if let Ok(hp) = hp_q.single() {
                        let ratio = if hp.max > 0.0 {
                            (hp.current / hp.max).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let bar_avail = 200.0_f32;
                        let label_w = font_s * 3.0;
                        stat_bar(
                            ui,
                            &t("ui.hud.hp"),
                            EguiColors::COLOR_HP,
                            ratio,
                            &format!("{:.0}/{:.0}", hp.current, hp.max),
                            bar_avail,
                            label_w,
                            bar_h,
                            font_s,
                            sb,
                        );
                    }

                    // XP 条
                    if let Ok(level) = level_q.single() {
                        let ratio = if level.xp_to_next > 0 {
                            (level.xp as f32 / level.xp_to_next as f32).clamp(0.0, 1.0)
                        } else {
                            0.0
                        };
                        let val_text = if level.xp_to_next >= 1000 {
                            format!("{}/{}K", level.xp, level.xp_to_next / 1000)
                        } else {
                            format!("{}/{}", level.xp, level.xp_to_next)
                        };
                        let bar_avail = 200.0_f32;
                        let label_w = font_s * 3.0;
                        stat_bar(
                            ui,
                            &t("ui.hud.xp"),
                            EguiColors::COLOR_XP,
                            ratio,
                            &val_text,
                            bar_avail,
                            label_w,
                            bar_h,
                            font_s,
                            sb,
                        );
                    }

                    // KILLS
                    stat_row(
                        ui,
                        "\u{25C6} KILLS",
                        &stats.kills.to_string(),
                        EguiColors::COLOR_ENEMY,
                        font_s,
                        font_m,
                        sb,
                    );
                });
        },
    );
}
