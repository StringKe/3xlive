//! DPS 竞技场模块
//!
//! 60 秒限时局：固定靶标，测试玩家 DPS build 输出。
//! 结算后提交 ReplayData 到服务端验证。

use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};

use game_core::components::*;
use game_core::data::disc::{make_starter_disc_with_quality, DiscQuality};
use game_core::events::{GameEvent, GameEventQueue};
use game_core::meta::state::MetaState;
use game_core::replay::ReplayData;
use game_core::resources::*;
use game_core::systems::environment::DataStormState;
use game_core::systems::high_risk_event::HighRiskEventManager;
use game_core::systems::DeltaTime;
use game_core::tick::run_game_tick;

use crate::egui_setup::EguiColors;
use crate::input::LocalInput;
use crate::simulation::{ActiveReplay, TickCounter};
use crate::sync::http_sync::WasmShared;
use crate::sync::LocalPlayerMarker;
use crate::ui::components::button::{button, ButtonProps, ButtonVariant};
use crate::GameState;

/// 竞技场状态
#[derive(States, Debug, Clone, PartialEq, Eq, Hash, Default)]
pub enum ArenaState {
    #[default]
    Inactive,
    Building,
    Fighting,
    Result,
}

/// 竞技场配置
const ARENA_DURATION_SECS: f32 = 60.0;
const ARENA_TICKS: u32 = (ARENA_DURATION_SECS / game_core::data::constants::TICK_DT) as u32;

/// 竞技场当前 tick
#[derive(Resource, Default)]
struct ArenaTick(u32);

/// 竞技场伤害统计
#[derive(Resource, Default)]
pub struct ArenaDamage(pub f32);

/// 竞技场提交结果
#[derive(Resource, Default)]
struct ArenaSubmitResult(WasmShared<Vec<u8>>);

/// 靶标标记组件
#[derive(Component)]
struct ArenaTarget;

pub struct ArenaPlugin;

impl Plugin for ArenaPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<ArenaState>()
            .init_resource::<ArenaTick>()
            .init_resource::<ArenaDamage>()
            .init_resource::<ArenaSubmitResult>()
            // egui 系统：必须在 EguiPrimaryContextPass 中运行
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                arena_building_ui.run_if(in_state(ArenaState::Building)),
            )
            .add_systems(OnEnter(ArenaState::Fighting), setup_arena_world)
            .add_systems(
                FixedUpdate,
                (
                    arena_input,
                    run_game_tick,
                    arena_tick_counter,
                    arena_damage_tracker,
                )
                    .chain()
                    .run_if(in_state(ArenaState::Fighting)),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                arena_hud.run_if(in_state(ArenaState::Fighting)),
            )
            .add_systems(
                bevy_egui::EguiPrimaryContextPass,
                arena_result_ui.run_if(in_state(ArenaState::Result)),
            );
    }
}

/// Build 自定义界面：选 Disc 品质
fn arena_building_ui(
    mut contexts: EguiContexts,
    meta: Res<MetaState>,
    mut next_arena: ResMut<NextState<ArenaState>>,
    mut next_game: ResMut<NextState<GameState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let mut qualities: Vec<DiscQuality> = meta.unlocked_disc_qualities.iter().copied().collect();
    qualities.sort_by_key(|q| q.slot_count());

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(8, 13, 23, 242)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.2);

                // 标题
                ui.label(
                    egui::RichText::new("DPS ARENA")
                        .size(48.0)
                        .color(EguiColors::CYAN),
                );
                ui.add_space(8.0);
                ui.label(
                    egui::RichText::new("60 second damage test")
                        .size(16.0)
                        .color(EguiColors::TEXT_SECONDARY),
                );
                ui.add_space(24.0);

                ui.label(
                    egui::RichText::new("SELECT DISC QUALITY")
                        .size(12.0)
                        .color(EguiColors::TEXT_TERTIARY),
                );
                ui.add_space(8.0);

                // Disc 品质选择按钮
                for quality in &qualities {
                    let slots = quality.slot_count();
                    if button(
                        ui,
                        &ButtonProps {
                            label: format!("{quality:?}  ({slots} slots)"),
                            variant: ButtonVariant::Primary,
                            font_size: 16.0,
                            min_width: Some(240.0),
                            min_height: Some(40.0),
                            ..Default::default()
                        },
                    ) {
                        next_arena.set(ArenaState::Fighting);
                    }
                    ui.add_space(4.0);
                }

                ui.add_space(16.0);

                // 返回按钮
                if button(
                    ui,
                    &ButtonProps {
                        label: "BACK".to_string(),
                        variant: ButtonVariant::Secondary,
                        font_size: 16.0,
                        min_width: Some(120.0),
                        min_height: Some(36.0),
                        ..Default::default()
                    },
                ) {
                    next_arena.set(ArenaState::Inactive);
                    next_game.set(GameState::Menu);
                }
            });
        });
}

/// 初始化竞技场世界
fn setup_arena_world(mut commands: Commands) {
    // 插入游戏引擎 Resource
    commands.insert_resource(GameEventQueue::default());
    commands.insert_resource(ChunkRegistry::default());
    commands.insert_resource(ChunkSpawner::default());
    commands.insert_resource(WaveManager::default());
    commands.insert_resource(DataStormState::default());
    commands.insert_resource(HighRiskEventManager::default());
    commands.insert_resource(DeltaTime(game_core::data::constants::TICK_DT));
    commands.insert_resource(ActiveReplay(ReplayData::new(0, 0, (0, 0))));
    commands.insert_resource(TickCounter(0));
    commands.insert_resource(ArenaTick(0));
    commands.insert_resource(ArenaDamage(0.0));

    // spawn 玩家
    let player_entity = commands
        .spawn((
            Player {
                id: 1,
                name: "Player".into(),
            },
            Position { x: 0.0, y: 0.0 },
            Velocity { x: 0.0, y: 0.0 },
            Health {
                current: 100.0,
                max: 100.0,
            },
            Level {
                value: 20,
                xp: 0,
                xp_to_next: 99999,
            },
            AimDirection { x: 1.0, y: 0.0 },
            DiscEquipped {
                disc: make_starter_disc_with_quality(DiscQuality::Common),
                cooldown_timer: 0.0,
                active_timing: None,
                timing_state: TimingState::default(),
            },
            Inventory::new(12),
            ActiveBuffs::default(),
            ContactIframes::default(),
            LocalPlayerMarker,
        ))
        .id();

    commands.insert_resource(LocalPlayerEntity {
        entity: player_entity,
    });

    // spawn 靶标（无限 HP）
    commands.spawn((
        Position { x: 200.0, y: 0.0 },
        Health {
            current: f32::MAX,
            max: f32::MAX,
        },
        CollisionRadius(24.0),
        MonsterComp {
            monster_type: 0,
            damage: 0.0,
        },
        game_core::components::ContactAttackTimer { remaining: 0.0 },
        Velocity { x: 0.0, y: 0.0 },
        MonsterAI {
            state: game_core::components::AIState::Wandering,
            target: None,
            wander_target_x: 200.0,
            wander_target_y: 0.0,
            detection_radius: 0.0, // 不追玩家
        },
        ArenaTarget,
    ));
}

/// 竞技场输入：复用 LocalInput
fn arena_input(
    input: Res<LocalInput>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut query: Query<&mut Velocity>,
) {
    let Some(local_player) = local_player else {
        return;
    };
    if let Ok(mut vel) = query.get_mut(local_player.entity) {
        vel.x = input.move_x * 200.0;
        vel.y = input.move_y * 200.0;
    }
}

/// 竞技场 tick 计数 + 时间到自动结束
fn arena_tick_counter(mut tick: ResMut<ArenaTick>, mut next_state: ResMut<NextState<ArenaState>>) {
    tick.0 += 1;
    if tick.0 >= ARENA_TICKS {
        next_state.set(ArenaState::Result);
    }
}

/// 追踪竞技场伤害（从 GameEventQueue 读取 EntityDamaged 事件）
fn arena_damage_tracker(
    mut events: MessageReader<GameEvent>,
    mut damage: ResMut<ArenaDamage>,
    targets: Query<(), With<ArenaTarget>>,
) {
    for event in events.read() {
        if let GameEvent::EntityDamaged {
            entity,
            damage: dmg,
            ..
        } = event
        {
            if targets.contains(*entity) {
                damage.0 += dmg;
            }
        }
    }
}

/// 竞技场 HUD：显示剩余时间和累计伤害
fn arena_hud(mut contexts: EguiContexts, tick: Res<ArenaTick>, damage: Res<ArenaDamage>) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    let remaining =
        (ARENA_DURATION_SECS - tick.0 as f32 * game_core::data::constants::TICK_DT).max(0.0);

    egui::Area::new(egui::Id::new("arena_hud"))
        .anchor(egui::Align2::CENTER_TOP, [0.0, 16.0])
        .interactable(false)
        .order(egui::Order::Foreground)
        .show(ctx, |ui| {
            egui::Frame::new()
                .fill(EguiColors::CARD_BG_ALPHA)
                .stroke(egui::Stroke::new(1.0, EguiColors::BORDER_SUBTLE))
                .inner_margin(egui::Margin::symmetric(16, 8))
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        ui.spacing_mut().item_spacing.x = 32.0;
                        ui.label(
                            egui::RichText::new(format!("{remaining:.1}s"))
                                .size(24.0)
                                .color(EguiColors::CYAN)
                                .strong(),
                        );
                        ui.label(
                            egui::RichText::new(format!("DMG {:.0}", damage.0))
                                .size(24.0)
                                .color(EguiColors::QUALITY_LEGENDARY)
                                .strong(),
                        );
                    });
                });
        });
}

/// 竞技场结算 UI
fn arena_result_ui(
    mut contexts: EguiContexts,
    damage: Res<ArenaDamage>,
    mut next_arena: ResMut<NextState<ArenaState>>,
    mut next_game: ResMut<NextState<GameState>>,
) {
    let Ok(ctx) = contexts.ctx_mut() else {
        return;
    };

    egui::CentralPanel::default()
        .frame(egui::Frame::new().fill(egui::Color32::from_rgba_premultiplied(8, 13, 23, 242)))
        .show(ctx, |ui| {
            ui.vertical_centered(|ui| {
                ui.add_space(ui.available_height() * 0.25);

                ui.label(
                    egui::RichText::new("ARENA COMPLETE")
                        .size(48.0)
                        .color(EguiColors::CYAN),
                );
                ui.add_space(24.0);

                ui.label(
                    egui::RichText::new(format!("TOTAL DAMAGE  {:.0}", damage.0))
                        .size(28.0)
                        .color(EguiColors::QUALITY_LEGENDARY)
                        .strong(),
                );
                ui.add_space(8.0);

                let dps = damage.0 / ARENA_DURATION_SECS;
                ui.label(
                    egui::RichText::new(format!("DPS  {dps:.1}"))
                        .size(20.0)
                        .color(EguiColors::TEXT_LIGHT),
                );

                ui.add_space(32.0);

                // 返回菜单按钮
                if button(
                    ui,
                    &ButtonProps {
                        label: "RETURN TO MENU".to_string(),
                        variant: ButtonVariant::Primary,
                        font_size: 20.0,
                        min_width: Some(240.0),
                        min_height: Some(50.0),
                        ..Default::default()
                    },
                ) {
                    next_arena.set(ArenaState::Inactive);
                    next_game.set(GameState::Menu);
                }
            });
        });
}
