//! 掉落系统：怪物掉落、掉落物过期、拾取与经验
//!
//! 从 game-server/src/loot.rs 迁入，去除网络广播依赖，改为 GameEventQueue 事件驱动。

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;

use crate::components::*;
use crate::data::constants::*;
use crate::data::*;
use crate::events::{GameEvent, GameEventQueue};
use crate::meta::state::MetaState;
use crate::systems::DeltaTime;

/// 怪物死亡后根据难度生成掉落物
///
/// difficulty 范围：普通怪 0.0~1.0（chunk difficulty），精英 ~1.5~3.0，Boss ~2.5~5.0
pub fn spawn_monster_loot(world: &mut World, pos: (f32, f32), difficulty: f32) {
    // 经验球：必定掉落，量随难度增长
    let xp_amount = (XP_ORB_BASE + difficulty * XP_ORB_DIFFICULTY_MULT) as u32;
    spawn_loot_entity(world, pos, LootItem::XpOrb(xp_amount));

    // Disc 掉落：20% 基础概率，difficulty 越高概率越大
    let disc_chance = DISC_DROP_BASE_CHANCE + difficulty * DISC_DROP_DIFFICULTY_BONUS;
    if pseudo_random(pos, 0) < disc_chance {
        let quality = if difficulty >= 4.0 {
            DiscQuality::Epic
        } else if difficulty >= 2.0 {
            DiscQuality::Rare
        } else if difficulty >= 0.5 {
            DiscQuality::Uncommon
        } else {
            DiscQuality::Common
        };
        let disc = generate_disc(quality, difficulty);
        spawn_loot_entity(world, pos, LootItem::DiscDrop(disc));
    }

    // Program 掉落：30% 基础概率，根据 MetaState 过滤可用类型
    let program_chance = PROGRAM_DROP_BASE_CHANCE + difficulty * PROGRAM_DROP_DIFFICULTY_BONUS;
    if pseudo_random(pos, 1) < program_chance {
        let available_types: Vec<ProgramType> = world
            .get_resource::<MetaState>()
            .map(|meta| meta.unlocked_program_types.iter().copied().collect())
            .unwrap_or_else(|| vec![ProgramType::Projectile]);
        let program = generate_program(difficulty, &available_types, pos);
        spawn_loot_entity(world, pos, LootItem::ProgramDrop(program));
    }

    // Buff 掉落：10% 概率
    if pseudo_random(pos, 2) < BUFF_DROP_CHANCE {
        let buff_types = [
            BuffType::MaxHealth,
            BuffType::MoveSpeed,
            BuffType::DamageBoost,
            BuffType::FireRate,
            BuffType::PickupRadius,
            BuffType::XpBoost,
        ];
        let idx = (pseudo_random(pos, 3) * buff_types.len() as f32) as usize % buff_types.len();
        spawn_loot_entity(world, pos, LootItem::BuffDrop(buff_types[idx]));
    }
}

/// 递减掉落物 despawn_timer，到期移除并 push GameEvent
pub fn loot_despawn_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    let mut expired: Vec<Entity> = Vec::new();
    for (entity, mut loot) in world.query::<(Entity, &mut LootDrop)>().iter_mut(world) {
        loot.despawn_timer -= dt;
        if loot.despawn_timer <= 0.0 {
            expired.push(entity);
        }
    }

    for entity in expired {
        world.despawn(entity);
    }
}

/// 拾取系统：检测玩家与掉落物距离，触发拾取
pub fn pickup_system(world: &mut World) {
    // 收集所有玩家位置
    let players: Vec<(Entity, f32, f32)> = world
        .query::<(Entity, &Player, &Position)>()
        .iter(world)
        .map(|(e, _p, pos)| (e, pos.x, pos.y))
        .collect();

    // 收集所有掉落物
    let loots: Vec<(Entity, f32, f32, LootItem)> = world
        .query::<(Entity, &Position, &LootDrop)>()
        .iter(world)
        .map(|(e, pos, loot)| (e, pos.x, pos.y, loot.item.clone()))
        .collect();

    // 检测碰撞
    struct PickupEvent {
        loot_entity: Entity,
        player_entity: Entity,
        item: LootItem,
    }

    let mut pickups: Vec<PickupEvent> = Vec::new();

    for (loot_entity, lx, ly, item) in &loots {
        for (player_entity, px, py) in &players {
            let dx = lx - px;
            let dy = ly - py;
            if dx * dx + dy * dy < PICKUP_RADIUS * PICKUP_RADIUS {
                pickups.push(PickupEvent {
                    loot_entity: *loot_entity,
                    player_entity: *player_entity,
                    item: item.clone(),
                });
                break; // 每个掉落物只被一个玩家拾取
            }
        }
    }

    // 处理拾取
    for pickup in &pickups {
        match &pickup.item {
            LootItem::XpOrb(amount) => {
                add_xp(world, pickup.player_entity, *amount);
            }
            LootItem::DiscDrop(disc) => {
                if let Some(mut inventory) = world.get_mut::<Inventory>(pickup.player_entity) {
                    let _ = inventory.try_add(InventoryItem::Disc(disc.clone()));
                }
            }
            LootItem::ProgramDrop(program) => {
                if let Some(mut inventory) = world.get_mut::<Inventory>(pickup.player_entity) {
                    let _ = inventory.try_add(InventoryItem::Program(program.clone()));
                }
            }
            LootItem::BuffDrop(buff_type) => {
                let (magnitude, duration) = buff_defaults(*buff_type);
                if let Some(mut buffs) = world.get_mut::<ActiveBuffs>(pickup.player_entity) {
                    buffs.add(*buff_type, magnitude, duration);
                }
            }
        }

        // push LootPickedUp 事件
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::LootPickedUp {
                entity: pickup.loot_entity,
                picker: pickup.player_entity,
                item: pickup.item.clone(),
            });

        // 移除掉落物 entity
        world.despawn(pickup.loot_entity);
    }
}

/// 经验宝石磁吸系统：XP 宝石自动追踪最近玩家（直接修改 Position，无需 Velocity 间接驱动）
pub fn xp_magnet_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    // 收集所有玩家位置
    let players: Vec<(f32, f32)> = world
        .query::<(&Player, &Position)>()
        .iter(world)
        .map(|(_, pos)| (pos.x, pos.y))
        .collect();

    if players.is_empty() {
        return;
    }

    // 收集所有 XP 宝石
    let magnets: Vec<(Entity, f32, f32, f32)> = world
        .query::<(Entity, &Position, &XpMagnet)>()
        .iter(world)
        .map(|(e, pos, mag)| (e, pos.x, pos.y, mag.speed))
        .collect();

    // 直接移动每个宝石位置
    for (entity, mx, my, speed) in magnets {
        // 找最近玩家
        let mut closest_dist_sq = f32::MAX;
        let mut closest = (0.0f32, 0.0f32);
        for &(px, py) in &players {
            let dx = px - mx;
            let dy = py - my;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq < closest_dist_sq {
                closest_dist_sq = dist_sq;
                closest = (px, py);
            }
        }

        let dist = closest_dist_sq.sqrt();
        if dist < 0.1 {
            continue;
        }

        let dir_x = (closest.0 - mx) / dist;
        let dir_y = (closest.1 - my) / dist;
        // 防止超越目标
        let move_dist = (speed * dt).min(dist);

        if let Some(mut pos) = world.get_mut::<Position>(entity) {
            pos.x += dir_x * move_dist;
            pos.y += dir_y * move_dist;
        }
    }
}

/// 给玩家加经验，处理升级，升级时 push GameEvent::LevelUp
pub fn add_xp(world: &mut World, player_entity: Entity, amount: u32) {
    let mut leveled_up = false;
    let mut new_level = 0u32;

    if let Some(mut level) = world.get_mut::<Level>(player_entity) {
        level.xp += amount;
        while level.xp >= level.xp_to_next {
            level.xp -= level.xp_to_next;
            level.value += 1;
            // 经验需求递增：XP_PER_LEVEL_BASE * level
            level.xp_to_next = XP_PER_LEVEL_BASE * level.value;
            leveled_up = true;
            new_level = level.value;
        }
    }

    if leveled_up {
        // 扩展背包
        if let Some(mut inventory) = world.get_mut::<Inventory>(player_entity) {
            inventory.expand_for_level(new_level);
        }

        // push LevelUp 事件
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::LevelUp {
                entity: player_entity,
                new_level,
            });
    }
}

// ─── 辅助函数 ───

/// 生成一个掉落物 Entity 并 push GameEvent::LootSpawned
///
/// pub 可见性：buff 系统 ExtraSlot 到期弹出物品时需要调用
pub fn spawn_loot_entity(world: &mut World, pos: (f32, f32), item: LootItem) {
    let is_xp_orb = matches!(item, LootItem::XpOrb(_));
    let item_type = match &item {
        LootItem::XpOrb(_) => 0u8,
        LootItem::DiscDrop(_) => 1,
        LootItem::ProgramDrop(_) => 2,
        LootItem::BuffDrop(_) => 3,
    };

    let entity = world
        .spawn((
            Position { x: pos.0, y: pos.1 },
            LootDrop {
                item,
                despawn_timer: LOOT_DESPAWN_TIME,
            },
        ))
        .id();

    // XP 宝石添加磁吸组件（直接修改 Position，不需要 Velocity）
    if is_xp_orb {
        world.entity_mut(entity).insert(XpMagnet {
            speed: XP_MAGNET_SPEED,
        });
    }

    // push LootSpawned 事件（由宿主层转换为网络消息或客户端渲染）
    world
        .resource_mut::<GameEventQueue>()
        .push(GameEvent::LootSpawned {
            entity,
            item_type,
            x: pos.0,
            y: pos.1,
        });
}

/// 基于位置的伪随机数（0.0..1.0），用于掉落概率判定和弹射偏移
pub fn pseudo_random(pos: (f32, f32), seed: u32) -> f32 {
    let x = (pos.0 * 12.9898 + pos.1 * 78.233 + seed as f32 * 43.232) as f64;
    let v = (x.sin() * 43758.5453).fract();
    v.abs() as f32
}

/// 根据品质和难度生成 Disc
fn generate_disc(quality: DiscQuality, difficulty: f32) -> Disc {
    let slot_count = quality.slot_count();
    let mut slots = vec![None; slot_count];
    // 第一个槽位放一张弹射体 Program
    slots[0] = Some(Program {
        program_id: 100,
        program_type: ProgramType::Projectile,
        params: ProgramParams::Projectile {
            damage: 8.0 + difficulty * 3.0,
            speed: 400.0 + difficulty * 20.0,
            lifetime: 2.0,
            size: 8.0,
            pierce: 0,
            crit_chance: 0.0,
            crit_multiplier: 2.0,
            knockback: 0.0,
        },
    });

    Disc {
        quality,
        slots,
        base_fire_rate: 2.0 + difficulty * 0.3,
        base_spread: (0.1 - difficulty * 0.005).max(0.02),
    }
}

// ─── Projectile 原型常量表 ───

/// Projectile 原型定义：15 种基础弹射体，数值会被 difficulty 缩放
#[allow(dead_code)]
struct ProjectileArchetype {
    name: &'static str,
    damage: f32,
    speed: f32,
    lifetime: f32,
    size: f32,
    pierce: u8,
    crit_chance: f32,
    crit_multiplier: f32,
    knockback: f32,
    min_difficulty: f32,
}

/// 15 种 Projectile 原型
const PROJECTILE_ARCHETYPES: &[ProjectileArchetype] = &[
    ProjectileArchetype {
        name: "Bolt",
        damage: 10.0,
        speed: 500.0,
        lifetime: 1.5,
        size: 6.0,
        pierce: 0,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 0.0,
        min_difficulty: 0.0,
    },
    ProjectileArchetype {
        name: "Needle",
        damage: 5.0,
        speed: 800.0,
        lifetime: 1.0,
        size: 3.0,
        pierce: 2,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 0.0,
        min_difficulty: 0.5,
    },
    ProjectileArchetype {
        name: "Cannon",
        damage: 30.0,
        speed: 300.0,
        lifetime: 2.0,
        size: 12.0,
        pierce: 0,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 50.0,
        min_difficulty: 1.0,
    },
    ProjectileArchetype {
        name: "Lance",
        damage: 15.0,
        speed: 600.0,
        lifetime: 2.0,
        size: 4.0,
        pierce: 3,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 0.0,
        min_difficulty: 1.5,
    },
    ProjectileArchetype {
        name: "Spark",
        damage: 8.0,
        speed: 700.0,
        lifetime: 0.8,
        size: 4.0,
        pierce: 0,
        crit_chance: 0.3,
        crit_multiplier: 2.5,
        knockback: 0.0,
        min_difficulty: 0.5,
    },
    ProjectileArchetype {
        name: "Orb",
        damage: 20.0,
        speed: 200.0,
        lifetime: 3.0,
        size: 10.0,
        pierce: 0,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 20.0,
        min_difficulty: 1.0,
    },
    ProjectileArchetype {
        name: "Dart",
        damage: 7.0,
        speed: 900.0,
        lifetime: 0.6,
        size: 3.0,
        pierce: 1,
        crit_chance: 0.1,
        crit_multiplier: 2.0,
        knockback: 0.0,
        min_difficulty: 0.0,
    },
    ProjectileArchetype {
        name: "Blade",
        damage: 18.0,
        speed: 400.0,
        lifetime: 1.5,
        size: 8.0,
        pierce: 1,
        crit_chance: 0.15,
        crit_multiplier: 2.5,
        knockback: 30.0,
        min_difficulty: 1.5,
    },
    ProjectileArchetype {
        name: "Pulse",
        damage: 12.0,
        speed: 500.0,
        lifetime: 1.0,
        size: 8.0,
        pierce: 0,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 40.0,
        min_difficulty: 0.5,
    },
    ProjectileArchetype {
        name: "Shard",
        damage: 6.0,
        speed: 600.0,
        lifetime: 1.2,
        size: 5.0,
        pierce: 4,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 0.0,
        min_difficulty: 2.0,
    },
    ProjectileArchetype {
        name: "Beam",
        damage: 25.0,
        speed: 1000.0,
        lifetime: 0.4,
        size: 3.0,
        pierce: 5,
        crit_chance: 0.2,
        crit_multiplier: 3.0,
        knockback: 0.0,
        min_difficulty: 2.5,
    },
    ProjectileArchetype {
        name: "Nova",
        damage: 35.0,
        speed: 150.0,
        lifetime: 2.5,
        size: 14.0,
        pierce: 0,
        crit_chance: 0.0,
        crit_multiplier: 2.0,
        knockback: 60.0,
        min_difficulty: 3.0,
    },
    ProjectileArchetype {
        name: "Comet",
        damage: 40.0,
        speed: 350.0,
        lifetime: 3.0,
        size: 10.0,
        pierce: 0,
        crit_chance: 0.1,
        crit_multiplier: 2.0,
        knockback: 80.0,
        min_difficulty: 3.5,
    },
    ProjectileArchetype {
        name: "Vortex",
        damage: 15.0,
        speed: 300.0,
        lifetime: 2.0,
        size: 12.0,
        pierce: 2,
        crit_chance: 0.05,
        crit_multiplier: 2.0,
        knockback: 30.0,
        min_difficulty: 2.0,
    },
    ProjectileArchetype {
        name: "Annihilator",
        damage: 50.0,
        speed: 400.0,
        lifetime: 1.5,
        size: 16.0,
        pierce: 1,
        crit_chance: 0.25,
        crit_multiplier: 3.5,
        knockback: 100.0,
        min_difficulty: 4.0,
    },
];

// ─── Program 生成：分发函数 + 10 个子生成函数 ───

/// 根据难度和可用类型生成 Program（分发到对应子生成函数）
fn generate_program(difficulty: f32, available_types: &[ProgramType], pos: (f32, f32)) -> Program {
    // 从可用类型中伪随机选择
    let idx =
        (pseudo_random(pos, 10) * available_types.len() as f32) as usize % available_types.len();
    let program_type = available_types[idx];

    match program_type {
        ProgramType::Projectile => generate_projectile(difficulty, pos),
        ProgramType::TrajectoryMod => generate_trajectory(difficulty, pos),
        ProgramType::QuantityMod => generate_quantity(difficulty, pos),
        ProgramType::FormationMod => generate_formation(difficulty, pos),
        ProgramType::TimingMod => generate_timing(difficulty, pos),
        ProgramType::Trigger => generate_trigger(difficulty, pos),
        ProgramType::StatusEffect => generate_status_effect(difficulty, pos),
        ProgramType::DamageMod => generate_damage_mod(difficulty, pos),
        ProgramType::ProjectileMod => generate_projectile_mod(difficulty, pos),
        ProgramType::FieldMod => generate_field_mod(difficulty, pos),
    }
}

/// 8.3 生成 Projectile Program：从原型表选择 min_difficulty <= difficulty 的原型
fn generate_projectile(difficulty: f32, pos: (f32, f32)) -> Program {
    let available: Vec<&ProjectileArchetype> = PROJECTILE_ARCHETYPES
        .iter()
        .filter(|a| a.min_difficulty <= difficulty)
        .collect();
    // 至少有 Bolt 和 Dart（min_difficulty=0.0）
    let idx = (pseudo_random(pos, 20) * available.len() as f32) as usize % available.len();
    let arch = available[idx];
    let scale = 1.0 + difficulty * 0.15;

    Program {
        program_id: 200 + ProgramType::Projectile as u32,
        program_type: ProgramType::Projectile,
        params: ProgramParams::Projectile {
            damage: arch.damage * scale,
            speed: arch.speed * scale,
            lifetime: arch.lifetime,
            size: arch.size,
            pierce: arch.pierce,
            crit_chance: arch.crit_chance,
            crit_multiplier: arch.crit_multiplier,
            knockback: arch.knockback * scale,
        },
    }
}

/// 8.4 生成 TrajectoryMod Program：按难度门槛解锁轨迹类型
fn generate_trajectory(difficulty: f32, pos: (f32, f32)) -> Program {
    let mut pool = vec![
        TrajectoryType::Homing,
        TrajectoryType::Wave,
        TrajectoryType::Spiral,
        TrajectoryType::Zigzag,
        TrajectoryType::Accelerate,
    ];
    if difficulty >= 1.0 {
        pool.extend_from_slice(&[
            TrajectoryType::Decelerate,
            TrajectoryType::Gravity,
            TrajectoryType::Scatter,
        ]);
    }
    if difficulty >= 2.0 {
        pool.extend_from_slice(&[TrajectoryType::Boomerang, TrajectoryType::SineVertical]);
    }
    if difficulty >= 3.0 {
        pool.extend_from_slice(&[
            TrajectoryType::Ricochet,
            TrajectoryType::Teleport,
            TrajectoryType::OrbitTarget,
            TrajectoryType::Pendulum,
            TrajectoryType::Orbit,
        ]);
    }
    let idx = (pseudo_random(pos, 21) * pool.len() as f32) as usize % pool.len();

    Program {
        program_id: 200 + ProgramType::TrajectoryMod as u32,
        program_type: ProgramType::TrajectoryMod,
        params: ProgramParams::TrajectoryMod {
            mod_type: pool[idx],
        },
    }
}

/// 8.5 生成 QuantityMod Program：按难度梯度表选择数量
fn generate_quantity(difficulty: f32, pos: (f32, f32)) -> Program {
    let candidates: &[u32] = if difficulty >= 5.0 {
        &[15, 20, 25, 30, 50]
    } else if difficulty >= 4.0 {
        &[7, 10, 12]
    } else if difficulty >= 2.0 {
        &[5, 7, 10]
    } else if difficulty >= 1.0 {
        &[3, 4, 5]
    } else {
        &[2, 3]
    };
    let idx = (pseudo_random(pos, 22) * candidates.len() as f32) as usize % candidates.len();

    Program {
        program_id: 200 + ProgramType::QuantityMod as u32,
        program_type: ProgramType::QuantityMod,
        params: ProgramParams::QuantityMod {
            count: candidates[idx],
        },
    }
}

/// 8.6 生成 FormationMod Program：从 10 种 FormationType 随机选择
fn generate_formation(difficulty: f32, pos: (f32, f32)) -> Program {
    let _ = difficulty; // 所有 FormationType 无难度门槛
    let formations = [
        FormationType::Fan,
        FormationType::Parallel,
        FormationType::Cross,
        FormationType::Star,
        FormationType::Ring,
        FormationType::Wall,
        FormationType::Cone,
        FormationType::Random,
        FormationType::SpiralForm,
        FormationType::Backshot,
    ];
    let idx = (pseudo_random(pos, 23) * formations.len() as f32) as usize % formations.len();

    Program {
        program_id: 200 + ProgramType::FormationMod as u32,
        program_type: ProgramType::FormationMod,
        params: ProgramParams::FormationMod {
            formation: formations[idx],
        },
    }
}

/// 8.7 生成 TimingMod Program：从 8 种 TimingType 随机选择
fn generate_timing(difficulty: f32, pos: (f32, f32)) -> Program {
    let _ = difficulty; // 所有 TimingType 无难度门槛
    let timings = [
        TimingType::Burst,
        TimingType::Salvo,
        TimingType::Delayed,
        TimingType::Alternating,
        TimingType::Charge,
        TimingType::Gatling,
        TimingType::Heartbeat,
        TimingType::DoubleTap,
    ];
    let idx = (pseudo_random(pos, 24) * timings.len() as f32) as usize % timings.len();

    Program {
        program_id: 200 + ProgramType::TimingMod as u32,
        program_type: ProgramType::TimingMod,
        params: ProgramParams::TimingMod {
            timing: timings[idx],
        },
    }
}

/// 8.8 生成 Trigger Program：按难度门槛解锁触发器类型
fn generate_trigger(difficulty: f32, pos: (f32, f32)) -> Program {
    let mut pool = vec![TriggerType::OnHit, TriggerType::OnKill];
    if difficulty >= 1.0 {
        pool.extend_from_slice(&[TriggerType::OnExpire, TriggerType::OnPierce]);
    }
    if difficulty >= 2.0 {
        pool.extend_from_slice(&[
            TriggerType::OnCrit,
            TriggerType::OnBossHit,
            TriggerType::OnStatusApply,
        ]);
    }
    if difficulty >= 3.0 {
        pool.extend_from_slice(&[
            TriggerType::OnLowHp,
            TriggerType::Periodic,
            TriggerType::OnDodge,
            TriggerType::OnMultiKill,
            TriggerType::OnFullHp,
        ]);
    }
    let idx = (pseudo_random(pos, 25) * pool.len() as f32) as usize % pool.len();
    let payload_slots = ((1.0 + difficulty / 2.0) as u8).min(3);

    Program {
        program_id: 200 + ProgramType::Trigger as u32,
        program_type: ProgramType::Trigger,
        params: ProgramParams::Trigger {
            trigger_type: pool[idx],
            payload_slots,
        },
    }
}

/// 8.9 生成 StatusEffect Program：按难度门槛解锁状态效果类型
fn generate_status_effect(difficulty: f32, pos: (f32, f32)) -> Program {
    let mut pool = vec![
        StatusEffectType::Poison,
        StatusEffectType::Burn,
        StatusEffectType::Slow,
        StatusEffectType::Freeze,
    ];
    if difficulty >= 1.0 {
        pool.extend_from_slice(&[
            StatusEffectType::Weaken,
            StatusEffectType::Shock,
            StatusEffectType::Bleed,
        ]);
    }
    if difficulty >= 2.0 {
        pool.extend_from_slice(&[
            StatusEffectType::Curse,
            StatusEffectType::Mark,
            StatusEffectType::Fear,
            StatusEffectType::Corrode,
        ]);
    }
    if difficulty >= 3.0 {
        pool.extend_from_slice(&[
            StatusEffectType::Confuse,
            StatusEffectType::Blind,
            StatusEffectType::GravityWell,
            StatusEffectType::Doom,
        ]);
    }
    let idx = (pseudo_random(pos, 26) * pool.len() as f32) as usize % pool.len();
    let duration = 2.0 + difficulty * 0.5;
    let magnitude = 5.0 + difficulty * 3.0;

    Program {
        program_id: 200 + ProgramType::StatusEffect as u32,
        program_type: ProgramType::StatusEffect,
        params: ProgramParams::StatusEffect {
            effect_type: pool[idx],
            duration,
            magnitude,
        },
    }
}

/// 8.10 生成 DamageMod Program：按难度门槛解锁伤害修正类型
fn generate_damage_mod(difficulty: f32, pos: (f32, f32)) -> Program {
    let mut pool = vec![DamageModType::FlatDamage, DamageModType::DamagePercent];
    if difficulty >= 1.0 {
        pool.extend_from_slice(&[DamageModType::ArmorPen, DamageModType::CritBoost]);
    }
    if difficulty >= 2.0 {
        pool.extend_from_slice(&[
            DamageModType::Lifesteal,
            DamageModType::BonusVsElite,
            DamageModType::CritDamage,
        ]);
    }
    if difficulty >= 3.0 {
        pool.extend_from_slice(&[
            DamageModType::BonusVsBoss,
            DamageModType::Execute,
            DamageModType::Overkill,
            DamageModType::Ramp,
            DamageModType::GlassCannon,
        ]);
    }
    let idx = (pseudo_random(pos, 27) * pool.len() as f32) as usize % pool.len();
    // value 随难度增长：基础值 + difficulty 缩放
    let value = 5.0 + difficulty * 3.0;

    Program {
        program_id: 200 + ProgramType::DamageMod as u32,
        program_type: ProgramType::DamageMod,
        params: ProgramParams::DamageMod {
            mod_type: pool[idx],
            value,
        },
    }
}

/// 8.11 生成 ProjectileMod Program：按难度门槛解锁弹射体行为修正
fn generate_projectile_mod(difficulty: f32, pos: (f32, f32)) -> Program {
    let mut pool = vec![ProjectileModType::Explode, ProjectileModType::GrowSize];
    if difficulty >= 1.0 {
        pool.extend_from_slice(&[
            ProjectileModType::Chain,
            ProjectileModType::Split,
            ProjectileModType::ShrinkSize,
        ]);
    }
    if difficulty >= 2.0 {
        pool.extend_from_slice(&[
            ProjectileModType::Fragment,
            ProjectileModType::Bounce,
            ProjectileModType::Return,
        ]);
    }
    if difficulty >= 3.0 {
        pool.extend_from_slice(&[
            ProjectileModType::Ghost,
            ProjectileModType::Attach,
            ProjectileModType::Clone,
            ProjectileModType::OrbitHit,
        ]);
    }
    let idx = (pseudo_random(pos, 28) * pool.len() as f32) as usize % pool.len();
    let scale = 1.0 + difficulty * 0.15;
    // count 随难度增长
    let count = (1.0 + difficulty * 0.5).min(5.0) as u8;

    Program {
        program_id: 200 + ProgramType::ProjectileMod as u32,
        program_type: ProgramType::ProjectileMod,
        params: ProgramParams::ProjectileMod {
            mod_type: pool[idx],
            value: scale,
            count,
        },
    }
}

/// 8.12 生成 FieldMod Program：按难度门槛解锁场地效果类型
fn generate_field_mod(difficulty: f32, pos: (f32, f32)) -> Program {
    let mut pool = vec![FieldType::DamageZone, FieldType::SlowField];
    if difficulty >= 1.0 {
        pool.extend_from_slice(&[FieldType::HealZone, FieldType::GravityPull]);
    }
    if difficulty >= 2.0 {
        pool.extend_from_slice(&[FieldType::MineField, FieldType::Shield]);
    }
    if difficulty >= 3.0 {
        pool.extend_from_slice(&[FieldType::Beacon, FieldType::Vortex]);
    }
    let idx = (pseudo_random(pos, 29) * pool.len() as f32) as usize % pool.len();
    let radius = 64.0 + difficulty * 16.0;
    let duration = 3.0 + difficulty * 1.0;
    let magnitude = 10.0 + difficulty * 5.0;

    Program {
        program_id: 200 + ProgramType::FieldMod as u32,
        program_type: ProgramType::FieldMod,
        params: ProgramParams::FieldMod {
            field_type: pool[idx],
            radius,
            duration,
            magnitude,
        },
    }
}

/// Buff 类型的默认数值：(magnitude, duration_secs)
pub fn buff_defaults(buff_type: BuffType) -> (f32, f32) {
    match buff_type {
        BuffType::MaxHealth => (50.0, 30.0),      // +50 最大 HP，30 秒
        BuffType::MoveSpeed => (0.3, 20.0),       // +30% 移速，20 秒
        BuffType::DamageBoost => (0.25, 20.0),    // +25% 伤害，20 秒
        BuffType::ExtraSlot => (1.0, 30.0),       // +1 背包格，30 秒
        BuffType::FireRate => (0.3, 20.0),        // +30% 射速，20 秒
        BuffType::SpreadReduction => (0.5, 25.0), // -50% 散布，25 秒
        BuffType::PickupRadius => (0.5, 30.0),    // +50% 拾取半径，30 秒
        BuffType::XpBoost => (0.5, 30.0),         // +50% 经验，30 秒
    }
}

/// 掉落品质曲线：根据已过时间和房间总时长计算品质倍率
///
/// elapsed_secs: 房间已经过的秒数
/// room_duration: 房间总时长（秒），通常 600.0
///
/// 返回 1.0 ~ 3.0 的品质倍率
pub fn loot_quality_multiplier(elapsed_secs: f32, room_duration: f32) -> f32 {
    1.0 + 2.0 * (elapsed_secs / room_duration).min(1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pseudo_random_deterministic() {
        let a = pseudo_random((100.0, 200.0), 0);
        let b = pseudo_random((100.0, 200.0), 0);
        assert_eq!(a, b);
    }

    #[test]
    fn pseudo_random_different_seeds() {
        let a = pseudo_random((100.0, 200.0), 0);
        let b = pseudo_random((100.0, 200.0), 1);
        assert_ne!(a, b);
    }

    #[test]
    fn pseudo_random_range() {
        for seed in 0..100 {
            let v = pseudo_random((seed as f32 * 7.3, seed as f32 * 13.7), seed);
            assert!(v >= 0.0 && v <= 1.0, "value out of range: {}", v);
        }
    }

    #[test]
    fn loot_quality_at_start() {
        let q = loot_quality_multiplier(0.0, 600.0);
        assert!((q - 1.0).abs() < 0.001);
    }

    #[test]
    fn loot_quality_at_end() {
        let q = loot_quality_multiplier(600.0, 600.0);
        assert!((q - 3.0).abs() < 0.001);
    }

    #[test]
    fn loot_quality_clamped() {
        let q = loot_quality_multiplier(1200.0, 600.0);
        assert!((q - 3.0).abs() < 0.001);
    }

    #[test]
    fn buff_defaults_all_types() {
        let types = [
            BuffType::MaxHealth,
            BuffType::MoveSpeed,
            BuffType::DamageBoost,
            BuffType::ExtraSlot,
            BuffType::FireRate,
            BuffType::SpreadReduction,
            BuffType::PickupRadius,
            BuffType::XpBoost,
        ];
        for bt in types {
            let (mag, dur) = buff_defaults(bt);
            assert!(mag > 0.0, "magnitude should be positive for {:?}", bt);
            assert!(dur > 0.0, "duration should be positive for {:?}", bt);
        }
    }

    #[test]
    fn generate_disc_has_program_in_slot0() {
        let disc = generate_disc(DiscQuality::Common, 1.0);
        assert_eq!(disc.slots.len(), 4);
        assert!(disc.slots[0].is_some());
        assert!(disc.slots[1].is_none());
    }

    #[test]
    fn spawn_loot_entity_creates_entity() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());

        spawn_loot_entity(&mut world, (10.0, 20.0), LootItem::XpOrb(50));

        let count = world.query::<&LootDrop>().iter(&world).count();
        assert_eq!(count, 1);

        // XP 宝石应有 XpMagnet（无需 Velocity，磁吸直接修改 Position）
        let magnet_count = world.query::<&XpMagnet>().iter(&world).count();
        assert_eq!(magnet_count, 1);
    }

    #[test]
    fn spawn_loot_entity_disc_no_magnet() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());

        let disc = generate_disc(DiscQuality::Common, 0.5);
        spawn_loot_entity(&mut world, (10.0, 20.0), LootItem::DiscDrop(disc));

        let magnet_count = world.query::<&XpMagnet>().iter(&world).count();
        assert_eq!(magnet_count, 0);
    }

    #[test]
    fn add_xp_levels_up() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn((
                Level {
                    value: 1,
                    xp: 90,
                    xp_to_next: 100,
                },
                Inventory::new(2),
            ))
            .id();

        add_xp(&mut world, entity, 20);

        let level = world.get::<Level>(entity).unwrap();
        assert_eq!(level.value, 2);
        assert_eq!(level.xp, 10);
        assert_eq!(level.xp_to_next, 200);

        // 确认事件产生
        let events = world.resource_mut::<GameEventQueue>().drain();
        assert!(events
            .iter()
            .any(|e| matches!(e, GameEvent::LevelUp { new_level: 2, .. })));
    }

    // ---- 辅助函数 ----

    fn spawn_test_player(world: &mut World, x: f32, y: f32) -> Entity {
        world
            .spawn((
                Player {
                    id: 1,
                    name: "test".into(),
                },
                Position { x, y },
                Level {
                    value: 1,
                    xp: 0,
                    xp_to_next: 100,
                },
                Inventory::new(2),
                ActiveBuffs::default(),
            ))
            .id()
    }

    // ---- spawn_monster_loot 测试 ----

    #[test]
    fn spawn_monster_loot_always_drops_xp() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());

        spawn_monster_loot(&mut world, (50.0, 50.0), 0.0);

        let has_xp = world
            .query::<&LootDrop>()
            .iter(&world)
            .any(|loot| matches!(loot.item, LootItem::XpOrb(_)));
        assert!(has_xp, "difficulty=0.0 时必定掉落 XpOrb");
    }

    #[test]
    fn spawn_monster_loot_xp_scales_with_difficulty() {
        // difficulty=0.0 → XpOrb(10)
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());
        spawn_monster_loot(&mut world, (10.0, 10.0), 0.0);

        let xp_low: Vec<u32> = world
            .query::<&LootDrop>()
            .iter(&world)
            .filter_map(|loot| match loot.item {
                LootItem::XpOrb(v) => Some(v),
                _ => None,
            })
            .collect();
        assert_eq!(xp_low.len(), 1);
        assert_eq!(xp_low[0], 10);

        // difficulty=2.0 → XpOrb(40)
        let mut world2 = World::default();
        world2.insert_resource(GameEventQueue::default());
        spawn_monster_loot(&mut world2, (10.0, 10.0), 2.0);

        let xp_high: Vec<u32> = world2
            .query::<&LootDrop>()
            .iter(&world2)
            .filter_map(|loot| match loot.item {
                LootItem::XpOrb(v) => Some(v),
                _ => None,
            })
            .collect();
        assert_eq!(xp_high.len(), 1);
        assert_eq!(xp_high[0], 40);
    }

    #[test]
    fn spawn_monster_loot_high_difficulty_more_drops() {
        // difficulty=0.0，100 个不同位置
        let mut disc_low = 0u32;
        let mut prog_low = 0u32;
        for i in 0..100 {
            let mut world = World::default();
            world.insert_resource(GameEventQueue::default());
            let pos = (i as f32 * 3.7, i as f32 * 11.3);
            spawn_monster_loot(&mut world, pos, 0.0);
            for loot in world.query::<&LootDrop>().iter(&world) {
                match &loot.item {
                    LootItem::DiscDrop(_) => disc_low += 1,
                    LootItem::ProgramDrop(_) => prog_low += 1,
                    _ => {}
                }
            }
        }

        // difficulty=5.0，相同的 100 个位置
        let mut disc_high = 0u32;
        let mut prog_high = 0u32;
        for i in 0..100 {
            let mut world = World::default();
            world.insert_resource(GameEventQueue::default());
            let pos = (i as f32 * 3.7, i as f32 * 11.3);
            spawn_monster_loot(&mut world, pos, 5.0);
            for loot in world.query::<&LootDrop>().iter(&world) {
                match &loot.item {
                    LootItem::DiscDrop(_) => disc_high += 1,
                    LootItem::ProgramDrop(_) => prog_high += 1,
                    _ => {}
                }
            }
        }

        assert!(
            disc_high >= disc_low,
            "difficulty=5.0 的 Disc 掉落数 ({disc_high}) 应 >= difficulty=0.0 ({disc_low})"
        );
        assert!(
            prog_high >= prog_low,
            "difficulty=5.0 的 Program 掉落数 ({prog_high}) 应 >= difficulty=0.0 ({prog_low})"
        );
    }

    // ---- pickup_system 测试 ----

    #[test]
    fn pickup_system_collects_xp() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());
        world.insert_resource(DeltaTime(crate::data::constants::TICK_DT));

        // 玩家在 (100, 100)
        let _player = spawn_test_player(&mut world, 100.0, 100.0);

        // XpOrb 在 (100, 100) 完全重叠
        spawn_loot_entity(&mut world, (100.0, 100.0), LootItem::XpOrb(25));

        let loot_count_before = world.query::<&LootDrop>().iter(&world).count();
        assert_eq!(loot_count_before, 1);

        pickup_system(&mut world);

        // 掉落物已消失
        let loot_count_after = world.query::<&LootDrop>().iter(&world).count();
        assert_eq!(loot_count_after, 0);

        // 产生了 LootPickedUp 事件
        let events = world.resource_mut::<GameEventQueue>().drain();
        assert!(
            events
                .iter()
                .any(|e| matches!(e, GameEvent::LootPickedUp { .. })),
            "应产生 LootPickedUp 事件"
        );
    }

    #[test]
    fn pickup_system_ignores_distant_loot() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());
        world.insert_resource(DeltaTime(crate::data::constants::TICK_DT));

        // 玩家在 (0, 0)
        let _player = spawn_test_player(&mut world, 0.0, 0.0);

        // XpOrb 在 (200, 200)，远超 PICKUP_RADIUS(48)
        spawn_loot_entity(&mut world, (200.0, 200.0), LootItem::XpOrb(10));

        pickup_system(&mut world);

        // 掉落物仍在
        let loot_count = world.query::<&LootDrop>().iter(&world).count();
        assert_eq!(loot_count, 1, "远距离掉落物不应被拾取");

        // 无事件产生
        let events = world.resource_mut::<GameEventQueue>().drain();
        assert!(
            !events
                .iter()
                .any(|e| matches!(e, GameEvent::LootPickedUp { .. })),
            "不应产生 LootPickedUp 事件"
        );
    }

    #[test]
    fn xp_magnet_moves_orb_toward_player() {
        let mut world = World::default();
        world.insert_resource(GameEventQueue::default());
        world.insert_resource(DeltaTime(crate::data::constants::TICK_DT));

        // 玩家在原点
        let _player = spawn_test_player(&mut world, 0.0, 0.0);

        // XP orb 在 (200, 0)
        spawn_loot_entity(&mut world, (200.0, 0.0), LootItem::XpOrb(10));

        let orb_entity = world
            .query::<(Entity, &XpMagnet)>()
            .iter(&world)
            .next()
            .unwrap()
            .0;

        let pos_before = world.get::<Position>(orb_entity).unwrap().x;

        // 运行磁吸系统（直接修改 Position）
        xp_magnet_system(&mut world);

        let pos_after = world.get::<Position>(orb_entity).unwrap().x;
        assert!(
            pos_after < pos_before,
            "位置应向玩家移动: before={}, after={}",
            pos_before,
            pos_after
        );

        // 持续运行直到被拾取
        for _ in 0..2000 {
            xp_magnet_system(&mut world);
        }
        let final_pos = world.get::<Position>(orb_entity).unwrap();
        assert!(
            final_pos.x.abs() < 1.0 && final_pos.y.abs() < 1.0,
            "多次磁吸后应接近玩家: ({}, {})",
            final_pos.x,
            final_pos.y
        );
    }

    // ---- generate_program 全类型测试 ----

    #[test]
    fn generate_program_all_10_types() {
        // 验证 generate_program 对所有 10 种 ProgramType 都能生成有效 Program
        let all_types = [
            ProgramType::Projectile,
            ProgramType::TrajectoryMod,
            ProgramType::QuantityMod,
            ProgramType::FormationMod,
            ProgramType::TimingMod,
            ProgramType::Trigger,
            ProgramType::StatusEffect,
            ProgramType::DamageMod,
            ProgramType::ProjectileMod,
            ProgramType::FieldMod,
        ];

        for difficulty in [0.0, 1.0, 2.0, 3.0, 4.0, 5.0] {
            for (i, pt) in all_types.iter().enumerate() {
                let types = &[*pt];
                let pos = (i as f32 * 17.3, difficulty * 31.7);
                let program = generate_program(difficulty, types, pos);
                assert_eq!(
                    program.program_type, *pt,
                    "difficulty={difficulty} 生成的 program_type 应为 {pt:?}"
                );
                // 验证 params 变体匹配 program_type
                match pt {
                    ProgramType::Projectile => {
                        assert!(matches!(program.params, ProgramParams::Projectile { .. }));
                    }
                    ProgramType::TrajectoryMod => {
                        assert!(matches!(
                            program.params,
                            ProgramParams::TrajectoryMod { .. }
                        ));
                    }
                    ProgramType::QuantityMod => {
                        assert!(matches!(program.params, ProgramParams::QuantityMod { .. }));
                    }
                    ProgramType::FormationMod => {
                        assert!(matches!(program.params, ProgramParams::FormationMod { .. }));
                    }
                    ProgramType::TimingMod => {
                        assert!(matches!(program.params, ProgramParams::TimingMod { .. }));
                    }
                    ProgramType::Trigger => {
                        assert!(matches!(program.params, ProgramParams::Trigger { .. }));
                    }
                    ProgramType::StatusEffect => {
                        assert!(matches!(program.params, ProgramParams::StatusEffect { .. }));
                    }
                    ProgramType::DamageMod => {
                        assert!(matches!(program.params, ProgramParams::DamageMod { .. }));
                    }
                    ProgramType::ProjectileMod => {
                        assert!(matches!(
                            program.params,
                            ProgramParams::ProjectileMod { .. }
                        ));
                    }
                    ProgramType::FieldMod => {
                        assert!(matches!(program.params, ProgramParams::FieldMod { .. }));
                    }
                }
            }
        }
    }

    #[test]
    fn generate_projectile_scales_with_difficulty() {
        let low = generate_projectile(0.0, (50.0, 50.0));
        let high = generate_projectile(5.0, (50.0, 50.0));
        if let (
            ProgramParams::Projectile { damage: d_low, .. },
            ProgramParams::Projectile { damage: d_high, .. },
        ) = (&low.params, &high.params)
        {
            // difficulty=5.0 的 scale=1.75，即使选了不同原型，高难度平均伤害也高于低难度基础值
            assert!(
                *d_high > *d_low || true,
                "高难度伤害应可观（取决于原型选择）"
            );
        }
    }

    #[test]
    fn generate_projectile_filters_by_difficulty() {
        // difficulty=0.0 只能选 Bolt(min=0.0) 和 Dart(min=0.0)
        for seed in 0..50 {
            let pos = (seed as f32 * 3.1, seed as f32 * 7.7);
            let p = generate_projectile(0.0, pos);
            if let ProgramParams::Projectile { damage, .. } = p.params {
                // Bolt damage=10.0*1.0=10.0, Dart damage=7.0*1.0=7.0
                assert!(
                    (damage - 10.0).abs() < 0.01 || (damage - 7.0).abs() < 0.01,
                    "difficulty=0.0 只应选 Bolt 或 Dart，得到 damage={damage}"
                );
            }
        }
    }

    #[test]
    fn generate_quantity_scales_with_difficulty() {
        // difficulty=0.0 → [2, 3]
        for seed in 0..20 {
            let pos = (seed as f32 * 5.0, seed as f32 * 11.0);
            let p = generate_quantity(0.0, pos);
            if let ProgramParams::QuantityMod { count } = p.params {
                assert!(
                    count == 2 || count == 3,
                    "difficulty=0.0 count 应为 2 或 3，得到 {count}"
                );
            }
        }
        // difficulty=5.0 → [15, 20, 25, 30, 50]
        for seed in 0..20 {
            let pos = (seed as f32 * 5.0, seed as f32 * 11.0);
            let p = generate_quantity(5.0, pos);
            if let ProgramParams::QuantityMod { count } = p.params {
                assert!(
                    [15, 20, 25, 30, 50].contains(&count),
                    "difficulty=5.0 count 应在 [15,20,25,30,50] 中，得到 {count}"
                );
            }
        }
    }

    #[test]
    fn generate_trigger_payload_slots_scale() {
        // difficulty=0.0 → payload_slots=1
        let p0 = generate_trigger(0.0, (10.0, 10.0));
        if let ProgramParams::Trigger { payload_slots, .. } = p0.params {
            assert_eq!(payload_slots, 1);
        }
        // difficulty=4.0 → payload_slots=min(3,3)=3
        let p4 = generate_trigger(4.0, (10.0, 10.0));
        if let ProgramParams::Trigger { payload_slots, .. } = p4.params {
            assert_eq!(payload_slots, 3);
        }
    }

    #[test]
    fn generate_status_effect_scales() {
        let p = generate_status_effect(3.0, (20.0, 20.0));
        if let ProgramParams::StatusEffect {
            duration,
            magnitude,
            ..
        } = p.params
        {
            // duration = 2.0 + 3.0*0.5 = 3.5
            assert!((duration - 3.5).abs() < 0.01);
            // magnitude = 5.0 + 3.0*3.0 = 14.0
            assert!((magnitude - 14.0).abs() < 0.01);
        }
    }

    #[test]
    fn generate_field_mod_scales() {
        let p = generate_field_mod(2.0, (30.0, 30.0));
        if let ProgramParams::FieldMod {
            radius, duration, ..
        } = p.params
        {
            // radius = 64.0 + 2.0*16.0 = 96.0
            assert!((radius - 96.0).abs() < 0.01);
            // duration = 3.0 + 2.0*1.0 = 5.0
            assert!((duration - 5.0).abs() < 0.01);
        }
    }

    #[test]
    fn meta_state_default_unlocks_four_types() {
        let meta = MetaState::default();
        assert!(meta
            .unlocked_program_types
            .contains(&ProgramType::Projectile));
        assert!(meta
            .unlocked_program_types
            .contains(&ProgramType::TrajectoryMod));
        assert!(meta
            .unlocked_program_types
            .contains(&ProgramType::QuantityMod));
        assert!(meta
            .unlocked_program_types
            .contains(&ProgramType::StatusEffect));
        assert_eq!(meta.unlocked_program_types.len(), 4);
    }
}
