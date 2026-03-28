//! 本地模拟插件：驱动 game-core 的游戏逻辑（替代 WebSocket 实时同步）

pub mod run_lifecycle;

use bevy::prelude::*;

use bevy::window::PrimaryWindow;

use game_core::components::*;
use game_core::data::constants::PLAYER_SPEED;
use game_core::data::disc::{make_starter_disc_with_quality, Disc};
use game_core::data::program::{Program, ProgramType};
use game_core::events::{GameEvent, GameEventQueue};
use game_core::meta::state::MetaState;
use game_core::replay::{InputFrame, ReplayData};
use game_core::resources::*;
use game_core::systems::environment::DataStormState;
use game_core::systems::high_risk_event::HighRiskEventManager;
use game_core::systems::DeltaTime;
use game_core::tick::run_game_tick;

use crate::sync::LocalPlayerMarker;
use crate::ui::{ClientDisc, ClientInventory, ClientItem, ClientProgram};
use crate::GameState;

use run_lifecycle::RunState;

/// 游戏模式
#[derive(Resource, Default, Debug, Clone, Copy, PartialEq, Eq)]
pub enum GameMode {
    #[default]
    Normal,
    Training,
}

/// 当前录像数据 Resource
#[derive(Resource)]
pub struct ActiveReplay(pub ReplayData);

/// Tick 计数器
#[derive(Resource, Default)]
pub struct TickCounter(pub u32);

/// 丢弃物品请求事件
/// 用于将物品丢弃到地面（背包槽位或 Disc 槽位）
#[derive(Message)]
pub struct DropItemRequest {
    /// 背包槽位索引（从背包丢弃时使用）
    pub bag_index: Option<usize>,
    /// Disc 槽位索引（从 Disc 槽位丢弃时使用）
    pub disc_slot_index: Option<usize>,
    /// 要丢弃的物品（用于生成到地面）
    pub item: Option<Program>,
    /// 丢弃位置（玩家位置）
    pub pos: Vec2,
}

/// 玩家丢弃物品的 despawn 时间（秒）
const PLAYER_DROP_DESPAWN_TIME: f32 = 180.0;

pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        // 使用 Bevy 默认 64Hz（2 的幂，f32 无损精度）
        // 无需手动设置，Bevy 0.15 默认即 64Hz

        app.add_message::<GameEvent>()
            .add_message::<DropItemRequest>()
            .init_resource::<GameMode>()
            .add_systems(OnExit(GameState::Playing), cleanup_game_world)
            .add_systems(OnEnter(GameState::Playing), init_game_world)
            .add_systems(
                OnEnter(RunState::Playing),
                apply_training_mode.run_if(resource_equals(GameMode::Training)),
            )
            .add_systems(
                FixedUpdate,
                (
                    save_previous_positions,
                    apply_local_input,
                    run_game_tick,
                    replay_recorder,
                )
                    .chain()
                    .run_if(in_state(GameState::Playing))
                    .run_if(in_state(RunState::Playing)),
            )
            .add_systems(
                Update,
                (
                    game_event_bridge,
                    update_viewport_size,
                    handle_loot_pickup,
                    handle_level_up,
                    handle_drop_item,
                    sync_disc_to_ecs,
                )
                    .run_if(in_state(GameState::Playing)),
            )
            .add_plugins(run_lifecycle::RunLifecyclePlugin);
    }
}

/// 初始化游戏世界：插入所有 Resource，spawn 本地玩家
fn init_game_world(mut commands: Commands) {
    // 插入游戏引擎所需 Resource
    commands.insert_resource(GameEventQueue::default());
    commands.insert_resource(ChunkRegistry::default());
    commands.insert_resource(ChunkSpawner::default());
    commands.insert_resource(WaveManager::default());
    commands.insert_resource(DataStormState::default());
    commands.insert_resource(HighRiskEventManager::default());
    commands.insert_resource(DeltaTime(game_core::data::constants::TICK_DT));
    commands.init_resource::<game_core::resources::ViewportSize>();

    // 本地模拟模式：解锁全部 Program 类型
    let mut meta = MetaState::default();
    meta.unlocked_program_types
        .insert(ProgramType::TrajectoryMod);
    meta.unlocked_program_types.insert(ProgramType::QuantityMod);
    meta.unlocked_program_types.insert(ProgramType::Trigger);
    meta.unlocked_program_types
        .insert(ProgramType::StatusEffect);
    meta.unlocked_program_types
        .insert(ProgramType::FormationMod);
    meta.unlocked_program_types.insert(ProgramType::TimingMod);
    meta.unlocked_program_types.insert(ProgramType::DamageMod);
    meta.unlocked_program_types
        .insert(ProgramType::ProjectileMod);
    meta.unlocked_program_types.insert(ProgramType::FieldMod);
    commands.insert_resource(meta);

    // 初始化录像和 tick 计数器
    commands.insert_resource(ActiveReplay(ReplayData::new(0, 0, (0, 0))));
    commands.insert_resource(TickCounter(0));

    // 直接使用 Common 品质默认武器（4 slot）
    let disc = make_starter_disc_with_quality(game_core::data::disc::DiscQuality::Common);
    let inv_slots = ClientInventory::INITIAL_SLOTS;

    // 初始化 ClientInventory 装备 Disc
    commands.insert_resource(ClientInventory {
        slots: vec![None; inv_slots],
        max_slots: inv_slots,
        equipped_disc: Some(disc_to_client(&disc)),
    });

    // spawn 本地玩家
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
                value: 1,
                xp: 0,
                xp_to_next: 100,
            },
            AimDirection { x: 1.0, y: 0.0 },
            DiscEquipped {
                disc,
                cooldown_timer: 0.0,
                active_timing: None,
                timing_state: TimingState::default(),
            },
            Inventory::new(inv_slots),
            ActiveBuffs::default(),
            ContactIframes::default(),
            LocalPlayerMarker,
        ))
        .id();

    commands.insert_resource(LocalPlayerEntity {
        entity: player_entity,
    });

    commands.insert_resource(RunState::Playing);
}

/// 清理游戏世界：despawn 所有带 Position 的游戏实体，移除游戏 Resource
fn cleanup_game_world(
    mut commands: Commands,
    entities: Query<Entity, With<Position>>,
    mut game_mode: ResMut<GameMode>,
) {
    for entity in &entities {
        if let Ok(mut cmd) = commands.get_entity(entity) {
            cmd.despawn();
        }
    }
    commands.remove_resource::<GameEventQueue>();
    commands.remove_resource::<ChunkRegistry>();
    commands.remove_resource::<ChunkSpawner>();
    commands.remove_resource::<WaveManager>();
    commands.remove_resource::<DataStormState>();
    commands.remove_resource::<HighRiskEventManager>();
    commands.remove_resource::<LocalPlayerEntity>();
    commands.remove_resource::<ActiveReplay>();
    commands.remove_resource::<TickCounter>();
    commands.remove_resource::<game_core::resources::ViewportSize>();

    // 重置游戏模式
    *game_mode = GameMode::Normal;
}

/// 从 LocalInput 读取输入，写入本地玩家的 Velocity
fn apply_local_input(
    input: Res<crate::input::LocalInput>,
    local_player: Res<LocalPlayerEntity>,
    mut query: Query<&mut Velocity>,
) {
    if let Ok(mut vel) = query.get_mut(local_player.entity) {
        vel.x = input.move_x * PLAYER_SPEED;
        vel.y = input.move_y * PLAYER_SPEED;
    }
}

/// 将 GameEventQueue 中的事件转发为 Bevy Event
pub fn game_event_bridge(
    mut event_queue: ResMut<GameEventQueue>,
    mut writer: MessageWriter<GameEvent>,
) {
    for event in event_queue.drain() {
        writer.write(event);
    }
}

/// 每 tick 开始前保存当前位置，用于渲染插值
fn save_previous_positions(
    mut query: Query<(
        &Position,
        &mut crate::render::entity_shapes::PreviousPosition,
    )>,
) {
    for (pos, mut prev) in &mut query {
        prev.x = pos.x;
        prev.y = pos.y;
    }
}

/// 每 tick 录制当前输入快照到 ReplayData
fn replay_recorder(
    input: Res<crate::input::LocalInput>,
    local_player: Res<LocalPlayerEntity>,
    aim_query: Query<&AimDirection>,
    mut replay: ResMut<ActiveReplay>,
    mut tick_counter: ResMut<TickCounter>,
) {
    let (aim_x, aim_y) = aim_query
        .get(local_player.entity)
        .map(|aim| (aim.x, aim.y))
        .unwrap_or((1.0, 0.0));
    let frame = InputFrame {
        tick_number: tick_counter.0,
        move_x: input.move_x,
        move_y: input.move_y,
        aim_x,
        aim_y,
    };
    replay.0.push_frame(frame);
    tick_counter.0 += 1;
}

/// 从浏览器窗口同步视口尺寸到 ViewportSize Resource
fn update_viewport_size(
    window_q: Query<&bevy::window::Window, With<PrimaryWindow>>,
    viewport: Option<ResMut<ViewportSize>>,
) {
    let Some(mut vp) = viewport else { return };
    let Ok(window) = window_q.single() else {
        return;
    };
    vp.width = window.width();
    vp.height = window.height();
}

/// 拾取事件 → 添加到 ClientInventory
fn handle_loot_pickup(
    mut events: MessageReader<GameEvent>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut client_inv: ResMut<ClientInventory>,
) {
    let Some(local_player) = local_player else {
        return;
    };
    for event in events.read() {
        if let GameEvent::LootPickedUp { picker, item, .. } = event {
            if *picker != local_player.entity {
                continue;
            }
            let client_item = match item {
                LootItem::DiscDrop(d) => Some(ClientItem::Disc(disc_to_client(d))),
                LootItem::ProgramDrop(p) => Some(ClientItem::Program(program_to_client(p))),
                _ => None, // XP/Buff 不进背包
            };
            if let Some(ci) = client_item {
                // 找第一个空位
                if let Some(slot) = client_inv.slots.iter_mut().find(|s| s.is_none()) {
                    *slot = Some(ci);
                }
            }
        }
    }
}

/// 升级事件 → 扩展 ClientInventory
fn handle_level_up(
    mut events: MessageReader<GameEvent>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut client_inv: ResMut<ClientInventory>,
) {
    let Some(local_player) = local_player else {
        return;
    };
    for event in events.read() {
        if let GameEvent::LevelUp { entity, new_level } = event {
            if *entity != local_player.entity {
                continue;
            }
            let new_max = ClientInventory::slots_for_level(*new_level);
            client_inv.resize(new_max);
        }
    }
}

/// 处理丢弃物品请求：从背包槽位或 Disc 槽位丢弃 + 生成掉落物
fn handle_drop_item(
    mut events: MessageReader<DropItemRequest>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut client_inv: ResMut<ClientInventory>,
    mut inv_q: Query<(&mut Inventory, &Position), With<LocalPlayerMarker>>,
    mut event_queue: Option<ResMut<GameEventQueue>>,
) {
    let Some(local_player) = local_player else {
        return;
    };
    for req in events.read() {
        // 从 ClientInventory 移除（Bag 或 DiscSlot）
        let client_item = if let Some(idx) = req.bag_index {
            // 从背包移除
            client_inv.slots.get_mut(idx).and_then(|s| s.take())
        } else if let Some(idx) = req.disc_slot_index {
            // 从 Disc 槽位移除
            if let Some(disc) = &mut client_inv.equipped_disc {
                disc.slots
                    .get_mut(idx)
                    .and_then(|s| s.take().map(ClientItem::Program))
            } else {
                None
            }
        } else {
            None
        };

        let Some(client_item) = client_item else {
            continue;
        };

        // 从 game-core Inventory 移除 + 生成掉落物
        let Ok((mut core_inv, player_pos)) = inv_q.get_mut(local_player.entity) else {
            // 放回 ClientInventory（仅背包场景）
            if let Some(idx) = req.bag_index {
                if let Some(slot) = client_inv.slots.get_mut(idx) {
                    *slot = Some(client_item);
                }
            } else if let Some(idx) = req.disc_slot_index {
                // 放回 Disc 槽位
                if let Some(disc) = &mut client_inv.equipped_disc {
                    if idx < disc.slots.len() {
                        disc.slots[idx] = if let ClientItem::Program(p) = client_item {
                            Some(p)
                        } else {
                            None
                        };
                    }
                }
            }
            continue;
        };

        // 移除 game-core 对应槽位（仅背包场景）
        if let Some(idx) = req.bag_index {
            if idx < core_inv.slots.len() {
                core_inv.slots[idx] = None;
            }
        }

        // 构建掉落物
        let loot_item = match &client_item {
            ClientItem::Disc(d) => LootItem::DiscDrop(Disc {
                quality: d.quality,
                slots: d
                    .slots
                    .iter()
                    .map(|s| {
                        s.as_ref().map(|p| Program {
                            program_id: p.program_id,
                            program_type: p.program_type,
                            params: p.params.clone(),
                        })
                    })
                    .collect(),
                base_fire_rate: d.base_fire_rate,
                base_spread: d.base_spread,
            }),
            ClientItem::Program(p) => LootItem::ProgramDrop(Program {
                program_id: p.program_id,
                program_type: p.program_type,
                params: p.params.clone(),
            }),
        };

        // 生成掉落物实体
        let pos = (player_pos.x, player_pos.y);
        let item_type = match &loot_item {
            LootItem::DiscDrop(_) => 1u8,
            LootItem::ProgramDrop(_) => 2,
            _ => 0,
        };

        if let Some(eq) = &mut event_queue {
            eq.push(GameEvent::LootSpawned {
                entity: Entity::PLACEHOLDER,
                item_type,
                x: pos.0,
                y: pos.1,
            });
        }
    }
}

// ─── 类型转换辅助函数 ───

fn disc_to_client(d: &Disc) -> ClientDisc {
    ClientDisc {
        quality: d.quality,
        slots: d
            .slots
            .iter()
            .map(|s| s.as_ref().map(program_to_client))
            .collect(),
        base_fire_rate: d.base_fire_rate,
        base_spread: d.base_spread,
    }
}

fn program_to_client(p: &Program) -> ClientProgram {
    ClientProgram {
        program_id: p.program_id,
        program_type: p.program_type,
        params: p.params.clone(),
    }
}

/// 将 ClientInventory 的 Disc 配置同步回 DiscEquipped ECS 组件
fn sync_disc_to_ecs(
    inventory: Res<ClientInventory>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut disc_q: Query<&mut DiscEquipped, With<LocalPlayerMarker>>,
) {
    if !inventory.is_changed() {
        return;
    }
    let Some(local_player) = local_player else {
        return;
    };
    let Ok(mut disc_eq) = disc_q.get_mut(local_player.entity) else {
        return;
    };
    let Some(client_disc) = &inventory.equipped_disc else {
        return;
    };

    // 同步 Disc 属性
    disc_eq.disc.quality = client_disc.quality;
    disc_eq.disc.base_fire_rate = client_disc.base_fire_rate;
    disc_eq.disc.base_spread = client_disc.base_spread;

    // 同步 Program 槽位
    disc_eq.disc.slots = client_disc
        .slots
        .iter()
        .map(|s| {
            s.as_ref().map(|p| Program {
                program_id: p.program_id,
                program_type: p.program_type,
                params: p.params.clone(),
            })
        })
        .collect();
}

// ─── 演练场（Training Ground）───

use game_core::data::program::{
    DamageModType, FieldType, FormationType, ProgramParams, ProjectileModType, StatusEffectType,
    TimingType, TrajectoryType, TriggerType,
};

/// 演练场模式：禁用波次 + 12 格背包 + spawn 靶标和掉落物
fn apply_training_mode(
    mut commands: Commands,
    mut wave_mgr: ResMut<WaveManager>,
    mut client_inv: ResMut<ClientInventory>,
    local_player: Option<Res<LocalPlayerEntity>>,
    mut inv_q: Query<&mut Inventory>,
) {
    // 禁用波次和 Boss
    wave_mgr.wave_timer = f32::MAX;
    wave_mgr.boss_timer = f32::MAX;

    // Arena 模式背包扩展到最大
    client_inv.resize(ClientInventory::MAX_SLOTS);
    if let Some(lp) = local_player {
        if let Ok(mut inv) = inv_q.get_mut(lp.entity) {
            inv.slots.resize_with(ClientInventory::MAX_SLOTS, || None);
        }
    }

    // spawn 靶标和掉落物
    spawn_training_entities(&mut commands);
}

/// spawn 靶标怪物 + 地面 Program 掉落物 + Disc 掉落物
fn spawn_training_entities(commands: &mut Commands) {
    // 靶标怪物：高 HP、大碰撞体、无 MonsterAI（不追击、不移动）
    commands.spawn((
        MonsterComp {
            monster_type: 0,
            damage: 0.0,
        },
        Position { x: 200.0, y: 0.0 },
        Velocity { x: 0.0, y: 0.0 },
        Health {
            current: 99999.0,
            max: 99999.0,
        },
        CollisionRadius(48.0),
        ContactAttackTimer { remaining: 0.0 },
    ));

    // 每种 ProgramType 生成 6 个地面掉落物
    let program_types = [
        ProgramType::Projectile,
        ProgramType::TrajectoryMod,
        ProgramType::QuantityMod,
        ProgramType::Trigger,
        ProgramType::StatusEffect,
        ProgramType::FormationMod,
        ProgramType::TimingMod,
        ProgramType::DamageMod,
        ProgramType::ProjectileMod,
        ProgramType::FieldMod,
    ];

    let mut next_id: u32 = 1000;

    for (type_idx, &ptype) in program_types.iter().enumerate() {
        for copy in 0..6u32 {
            let x = -300.0 + (type_idx as f32) * 150.0;
            let y = -200.0 + (copy as f32) * 70.0;
            let program = make_training_program(ptype, next_id, copy);
            next_id += 1;
            commands.spawn((
                Position { x, y },
                Velocity { x: 0.0, y: 0.0 },
                LootDrop {
                    item: LootItem::ProgramDrop(program),
                    despawn_timer: f32::MAX,
                },
            ));
        }
    }

    // 2 个 Disc 掉落（Uncommon、Rare）
    for (i, quality) in [
        game_core::data::disc::DiscQuality::Uncommon,
        game_core::data::disc::DiscQuality::Rare,
    ]
    .iter()
    .enumerate()
    {
        let disc = make_starter_disc_with_quality(*quality);
        commands.spawn((
            Position {
                x: 350.0 + i as f32 * 100.0,
                y: -100.0,
            },
            Velocity { x: 0.0, y: 0.0 },
            LootDrop {
                item: LootItem::DiscDrop(disc),
                despawn_timer: f32::MAX,
            },
        ));
    }
}

/// 为训练模式创建各类型的 Program
fn make_training_program(ptype: ProgramType, id: u32, variant: u32) -> Program {
    match ptype {
        ProgramType::Projectile => {
            // 6 个变体：不同 damage/speed 组合
            let (damage, speed, size) = match variant % 6 {
                0 => (10.0, 500.0, 4.0),
                1 => (25.0, 400.0, 6.0),
                2 => (5.0, 800.0, 3.0),
                3 => (40.0, 300.0, 8.0),
                4 => (15.0, 600.0, 5.0),
                _ => (20.0, 500.0, 4.0),
            };
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::Projectile {
                    damage,
                    speed,
                    lifetime: 2.0,
                    size,
                    pierce: 0,
                    crit_chance: 0.0,
                    crit_multiplier: 2.0,
                    knockback: 0.0,
                },
            }
        }
        ProgramType::TrajectoryMod => {
            let mod_types = [
                TrajectoryType::Homing,
                TrajectoryType::Orbit,
                TrajectoryType::Spiral,
                TrajectoryType::Wave,
                TrajectoryType::Boomerang,
                TrajectoryType::Homing,
            ];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::TrajectoryMod {
                    mod_type: mod_types[variant as usize % mod_types.len()],
                },
            }
        }
        ProgramType::QuantityMod => {
            let count = match variant % 6 {
                0 | 1 => 2,
                2 | 3 => 3,
                4 => 4,
                _ => 5,
            };
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::QuantityMod { count },
            }
        }
        ProgramType::Trigger => {
            let trigger_types = [
                TriggerType::OnHit,
                TriggerType::OnKill,
                TriggerType::OnExpire,
                TriggerType::OnPierce,
                TriggerType::OnHit,
                TriggerType::OnKill,
            ];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::Trigger {
                    trigger_type: trigger_types[variant as usize % trigger_types.len()],
                    payload_slots: 1,
                },
            }
        }
        ProgramType::StatusEffect => {
            let effects = [
                (StatusEffectType::Poison, 3.0, 5.0),
                (StatusEffectType::Freeze, 2.0, 1.0),
                (StatusEffectType::Burn, 4.0, 8.0),
                (StatusEffectType::Slow, 3.0, 0.5),
                (StatusEffectType::Poison, 5.0, 10.0),
                (StatusEffectType::Burn, 2.0, 15.0),
            ];
            let (effect_type, duration, magnitude) = effects[variant as usize % effects.len()];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::StatusEffect {
                    effect_type,
                    duration,
                    magnitude,
                },
            }
        }
        ProgramType::FormationMod => {
            let formations = [
                FormationType::Fan,
                FormationType::Parallel,
                FormationType::Cross,
                FormationType::Star,
                FormationType::Ring,
                FormationType::Fan,
            ];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::FormationMod {
                    formation: formations[variant as usize % formations.len()],
                },
            }
        }
        ProgramType::TimingMod => {
            let timings = [
                TimingType::Burst,
                TimingType::Salvo,
                TimingType::Delayed,
                TimingType::Alternating,
                TimingType::Charge,
                TimingType::Burst,
            ];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::TimingMod {
                    timing: timings[variant as usize % timings.len()],
                },
            }
        }
        ProgramType::DamageMod => {
            let mods = [
                (DamageModType::Lifesteal, 15.0),
                (DamageModType::ArmorPen, 20.0),
                (DamageModType::BonusVsElite, 25.0),
                (DamageModType::BonusVsBoss, 30.0),
                (DamageModType::Execute, 50.0),
                (DamageModType::Lifesteal, 10.0),
            ];
            let (mod_type, value) = mods[variant as usize % mods.len()];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::DamageMod { mod_type, value },
            }
        }
        ProgramType::ProjectileMod => {
            let mods = [
                (ProjectileModType::Chain, 2.0, 3),
                (ProjectileModType::Split, 3.0, 2),
                (ProjectileModType::Explode, 64.0, 1),
                (ProjectileModType::Return, 1.0, 1),
                (ProjectileModType::GrowSize, 2.0, 1),
                (ProjectileModType::Chain, 3.0, 4),
            ];
            let (mod_type, value, count) = mods[variant as usize % mods.len()];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::ProjectileMod {
                    mod_type,
                    value,
                    count,
                },
            }
        }
        ProgramType::FieldMod => {
            let fields = [
                (FieldType::DamageZone, 96.0, 3.0, 10.0),
                (FieldType::SlowField, 128.0, 4.0, 0.5),
                (FieldType::HealZone, 80.0, 5.0, 8.0),
                (FieldType::GravityPull, 160.0, 3.0, 200.0),
                (FieldType::MineField, 64.0, 10.0, 25.0),
                (FieldType::DamageZone, 128.0, 5.0, 15.0),
            ];
            let (field_type, radius, duration, magnitude) = fields[variant as usize % fields.len()];
            Program {
                program_id: id,
                program_type: ptype,
                params: ProgramParams::FieldMod {
                    field_type,
                    radius,
                    duration,
                    magnitude,
                },
            }
        }
    }
}
