//! Roguelite 局生命周期：Playing → Settled（直接使用默认武器开始）

use bevy::prelude::*;
use bevy_egui::egui;

use game_core::components::*;
use game_core::events::{GameEvent, RunResult};
use game_core::meta::state::MetaState;
use game_core::resources::LocalPlayerEntity;

use crate::egui_setup::EguiColors;
use crate::storage;
use crate::sync::http_sync::{SyncManager, UploadPending};
use crate::sync::LocalPlayerMarker;
use crate::GameState;

use super::{ActiveReplay, TickCounter};

/// 局内运行状态
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default, Resource)]
pub enum RunState {
    #[default]
    Playing,
    Settled,
}

/// 结算数据
#[derive(Resource, Default)]
pub struct SettlementData {
    pub survived_secs: f32,
    pub total_kills: u32,
    pub total_damage: f32,
    pub kills_elite: u32,
    pub kills_boss: u32,
    pub final_level: u32,
    pub max_hp: f32,
    pub max_chunk_difficulty: f32,
}

pub struct RunLifecyclePlugin;

impl Plugin for RunLifecyclePlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<RunState>()
            .init_resource::<SettlementData>()
            .init_resource::<MetaState>()
            .add_systems(
                Update,
                death_detection
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(RunState::Playing)),
            )
            .add_systems(OnEnter(RunState::Settled), prepare_settlement)
            // egui 系统：必须在 EguiPrimaryContextPass 中运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                settlement_ui
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(RunState::Settled)),
            );
    }
}

/// 监听 GameEvent::EntityKilled，检测本地玩家死亡
fn death_detection(
    mut events: MessageReader<GameEvent>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut next_run_state: ResMut<NextState<RunState>>,
) {
    let Some(local_player) = local_player else {
        return;
    };

    for event in events.read() {
        if let GameEvent::EntityKilled { entity, .. } = event {
            if *entity == local_player.entity {
                next_run_state.set(RunState::Settled);
                return;
            }
        }
    }
}

/// 进入 Settled 时收集结算数据，写入存档和录像，触发上传
fn prepare_settlement(
    mut settlement: ResMut<SettlementData>,
    query: Query<(&Level, &Health), With<LocalPlayerMarker>>,
    tick_counter: Res<TickCounter>,
    mut replay: ResMut<ActiveReplay>,
    mut sync_mgr: ResMut<SyncManager>,
    upload_pending: Res<UploadPending>,
) {
    if let Ok((level, health)) = query.single() {
        settlement.final_level = level.value;
        settlement.max_hp = health.max;
    }

    // 存活时间 = tick 数 * TICK_DT
    settlement.survived_secs = tick_counter.0 as f32 * game_core::data::constants::TICK_DT;

    // 构建 RunResult
    let run_result = RunResult {
        survived_secs: settlement.survived_secs,
        total_damage: settlement.total_damage,
        kills_normal: settlement.total_kills,
        kills_elite: settlement.kills_elite,
        kills_boss: settlement.kills_boss,
        max_chunk_difficulty: settlement.max_chunk_difficulty,
        final_level: settlement.final_level,
    };

    // 填充录像结果并写入 IndexedDB
    replay.0.result = Some(run_result.clone());
    let bincode_config = bincode::config::standard();
    if let Ok(replay_bytes) = bincode::encode_to_vec(&replay.0, bincode_config) {
        let timestamp = js_sys::Date::now() as u64;
        let key = format!("{timestamp}");
        storage::write_replay(&key, &replay_bytes);
    }

    // 将 RunResult 写入 pending_uploads（JSON 格式，供 JS 层 fetch 发送）
    {
        let json = format!(
            r#"{{"survived_secs":{},"total_damage":{},"kills_normal":{},"kills_elite":{},"kills_boss":{},"max_chunk_difficulty":{},"final_level":{}}}"#,
            run_result.survived_secs,
            run_result.total_damage,
            run_result.kills_normal,
            run_result.kills_elite,
            run_result.kills_boss,
            run_result.max_chunk_difficulty,
            run_result.final_level,
        );
        let timestamp = js_sys::Date::now() as u64;
        let key = format!("{timestamp}");
        storage::write_pending_upload(&key, json.as_bytes());
    }

    // 6.2.2 结算后立即触发上传
    if !sync_mgr.is_syncing {
        sync_mgr.is_syncing = true;
        crate::sync::http_sync::start_upload_pending(&upload_pending);
    }
}

/// 结算 UI（egui）
fn settlement_ui(
    mut contexts: bevy_egui::EguiContexts,
    settlement: Res<SettlementData>,
    mut next_game_state: ResMut<NextState<GameState>>,
    mut next_run_state: ResMut<NextState<RunState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(8, 13, 23, 242)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.2);

                ui.label(
                    egui::RichText::new(crate::js_bridge::t("ui.lifecycle.run_complete"))
                        .size(36.0)
                        .color(EguiColors::CYAN),
                );
                ui.add_space(24.0);

                let stat = |ui: &mut egui::Ui, label: &str, value: &str| {
                    ui.label(
                        egui::RichText::new(format!("{label}  {value}"))
                            .size(16.0)
                            .color(EguiColors::TEXT_LIGHT),
                    );
                    ui.add_space(4.0);
                };

                stat(
                    ui,
                    &crate::js_bridge::t("ui.lifecycle.survived"),
                    &format!("{:.1}s", settlement.survived_secs),
                );
                stat(
                    ui,
                    &crate::js_bridge::t("ui.lifecycle.level"),
                    &format!("{}", settlement.final_level),
                );
                stat(
                    ui,
                    &crate::js_bridge::t("ui.lifecycle.kills"),
                    &format!("{}", settlement.total_kills),
                );
                stat(
                    ui,
                    &crate::js_bridge::t("ui.lifecycle.damage"),
                    &format!("{:.0}", settlement.total_damage),
                );

                ui.add_space(32.0);

                let btn = egui::Button::new(
                    egui::RichText::new(crate::js_bridge::t("ui.lifecycle.return_menu"))
                        .size(20.0)
                        .color(EguiColors::TEXT_PRIMARY),
                )
                .fill(EguiColors::BTN_FILL)
                .stroke(egui::Stroke::new(1.0, EguiColors::BORDER_CYAN))
                .min_size(egui::vec2(240.0, 50.0));

                if ui.add(btn).clicked() {
                    next_run_state.set(RunState::Playing);
                    next_game_state.set(GameState::Menu);
                }
            });
        });
}
