//! HUD 通知系统（升级、击杀等）

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use game_core::events::GameEvent;

use crate::egui_setup::EguiColors;
use crate::js_bridge;
use crate::ui::components::rem;

// ─── 组件 ───

/// HUD 通知条目
#[derive(Component)]
pub(crate) struct HudNotification {
    pub timer: Timer,
}

/// 通知文本数据（纯数据，不含 Bevy UI 组件）
#[derive(Component)]
pub(crate) struct NotificationData {
    pub message: String,
}

/// 收到 LevelUp 事件时 spawn 通知实体
pub(crate) fn notification_handler(mut commands: Commands, mut reader: MessageReader<GameEvent>) {
    for ev in reader.read() {
        if let GameEvent::LevelUp { new_level, .. } = ev {
            let message =
                js_bridge::t("ui.hud.player_level_up").replace("{0}", &new_level.to_string());
            commands.spawn((
                HudNotification {
                    timer: Timer::from_seconds(3.0, TimerMode::Once),
                },
                NotificationData { message },
            ));
        }
    }
}

/// egui 渲染通知文字（顶部居中，后半段淡出）
pub(crate) fn notification_render_system(
    mut contexts: EguiContexts,
    query: Query<(&HudNotification, &NotificationData)>,
    hud: Res<super::HudLayout>,
) {
    let notifications: Vec<_> = query.iter().collect();
    if notifications.is_empty() {
        return;
    }

    let sw = hud.screen_w;
    let font_size = rem(sw, 1.1).max(10.0);

    egui::Area::new(egui::Id::new("notifications"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, rem(sw, 2.8)])
        .interactable(false)
        .show(contexts.ctx_mut().expect("egui context"), |ui| {
            for (notif, data) in &notifications {
                let t = notif.timer.fraction();
                let alpha = if t > 0.5 {
                    ((1.0 - (t - 0.5) * 2.0).max(0.0) * 255.0) as u8
                } else {
                    255
                };
                let [r, g, b, _] = EguiColors::NOTIFICATION_GOLD.to_array();
                let color = egui::Color32::from_rgba_unmultiplied(r, g, b, alpha);
                ui.label(
                    egui::RichText::new(&data.message)
                        .size(font_size)
                        .color(color),
                );
            }
        });
}

/// 通知计时器更新 + 超时 despawn
pub(crate) fn notification_fade(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(Entity, &mut HudNotification)>,
) {
    for (entity, mut notif) in &mut query {
        notif.timer.tick(time.delta());
        if notif.timer.is_finished() {
            if let Ok(mut cmd) = commands.get_entity(entity) {
                cmd.despawn();
            }
        }
    }
}
