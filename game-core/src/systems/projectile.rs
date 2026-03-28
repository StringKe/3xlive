use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::constants::{MAX_FIELD_EFFECTS, MAX_PROJECTILES};
use crate::data::TimingType;
use crate::disc_engine::DiscExecutor;
use crate::events::{GameEvent, GameEventQueue};
use crate::systems::DeltaTime;

/// Disc 开火系统
pub fn disc_fire_system(world: &mut World) {
    // 4.2 弹射体数量上限检查
    let projectile_count = world.query::<&ProjectileComp>().iter(world).count() as u32;
    if projectile_count >= MAX_PROJECTILES {
        return;
    }

    struct FireData {
        pos: (f32, f32),
        aim: (f32, f32),
        disc_clone: crate::data::Disc,
        owner_id: u32,
        active_timing: Option<TimingType>,
    }

    let mut fire_list: Vec<FireData> = Vec::new();

    let dt = world.resource::<DeltaTime>().0;

    for (mut disc_eq, pos, aim, player, buffs) in world
        .query_filtered::<(
            &mut DiscEquipped,
            &Position,
            &AimDirection,
            &Player,
            Option<&ActiveBuffs>,
        ), Without<AutoAimNoTarget>>()
        .iter_mut(world)
    {
        disc_eq.cooldown_timer -= dt;
        if disc_eq.cooldown_timer > 0.0 {
            continue;
        }

        let fire_rate_bonus = buffs
            .map(|b| b.get_magnitude(BuffType::FireRate))
            .unwrap_or(0.0);
        let effective_fire_rate = disc_eq.disc.base_fire_rate * (1.0 + fire_rate_bonus);
        let base_cooldown = 1.0 / effective_fire_rate;

        // 4.3 TimingMod：根据 active_timing 调整 cooldown
        let timing = disc_eq.active_timing;
        let new_cooldown = match timing {
            Some(TimingType::Burst) => {
                disc_eq.timing_state.burst_count += 1;
                if disc_eq.timing_state.burst_count >= 3 {
                    // 连发 3 次后进入 3x 长冷却
                    disc_eq.timing_state.burst_count = 0;
                    base_cooldown * 3.0
                } else {
                    // 连发间短冷却（基础的 1/5）
                    base_cooldown * 0.2
                }
            }
            Some(TimingType::Heartbeat) => {
                // 固定 0.5s cooldown，忽略 base_fire_rate
                0.5
            }
            Some(TimingType::Gatling) => {
                // 每次开火后 cooldown *= 0.95，最低 0.1 * base_cooldown
                let mult = disc_eq.timing_state.gatling_cooldown_mult;
                let new_mult = (mult * 0.95).max(0.1);
                disc_eq.timing_state.gatling_cooldown_mult = new_mult;
                base_cooldown * new_mult
            }
            // DoubleTap：正常 cooldown，但会执行两次（在 spawn 阶段处理）
            Some(TimingType::DoubleTap) => base_cooldown,
            // Salvo/Delayed/Charge/Alternating：保持默认 cooldown
            _ => base_cooldown,
        };
        disc_eq.cooldown_timer = new_cooldown;

        fire_list.push(FireData {
            pos: (pos.x, pos.y),
            aim: (aim.x, aim.y),
            disc_clone: disc_eq.disc.clone(),
            owner_id: player.id,
            active_timing: timing,
        });
    }

    for data in &fire_list {
        let mut executor = DiscExecutor::new(&data.disc_clone);
        let mut projectiles = executor.execute(data.pos, data.aim);

        // 4.3 DoubleTap：再执行一次
        if data.active_timing == Some(TimingType::DoubleTap) {
            let mut executor2 = DiscExecutor::new(&data.disc_clone);
            projectiles.extend(executor2.execute(data.pos, data.aim));
        }

        for sp in &projectiles {
            let entity = world
                .spawn((
                    Position {
                        x: sp.pos.0,
                        y: sp.pos.1,
                    },
                    Velocity {
                        x: sp.vel.0,
                        y: sp.vel.1,
                    },
                    ProjectileComp {
                        damage: sp.damage,
                        lifetime: sp.lifetime,
                        remaining_lifetime: sp.lifetime,
                        size: sp.size,
                        owner_id: data.owner_id,
                        pierce: sp.pierce,
                        hits_remaining: sp.pierce + 1,
                        crit_chance: sp.crit_chance,
                        crit_multiplier: sp.crit_multiplier,
                        knockback: sp.knockback,
                        damage_mods: sp.damage_mods.clone(),
                        projectile_mods: sp.projectile_mods.clone(),
                        field_mods: sp.field_mods.clone(),
                        applied_effects: sp
                            .applied_effects
                            .iter()
                            .map(|e| (e.effect_type, e.duration, e.magnitude))
                            .collect(),
                        trajectory_mods: sp.trajectory_mods.clone(),
                        hit_entities: Vec::new(),
                        on_hit: sp.on_hit.as_ref().map(|p| p.programs.clone()),
                        on_kill: sp.on_kill.as_ref().map(|p| p.programs.clone()),
                        on_expire: sp.on_expire.as_ref().map(|p| p.programs.clone()),
                        on_crit: sp.on_crit.clone(),
                        on_boss_hit: sp.on_boss_hit.clone(),
                        on_periodic: sp.on_periodic.clone(),
                    },
                    CollisionRadius(crate::data::constants::PROJECTILE_HIT_RADIUS),
                ))
                .id();

            world
                .resource_mut::<GameEventQueue>()
                .push(GameEvent::ProjectileSpawned {
                    entity,
                    x: sp.pos.0,
                    y: sp.pos.1,
                    owner_id: data.owner_id,
                });
        }
    }
}

/// 弹射体生命周期递减 + 过期销毁
pub fn projectile_despawn_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    // 收集待销毁弹射体及其 field_mods 数据
    struct DespawnData {
        entity: Entity,
        pos: Position,
        field_mods: Vec<crate::disc_engine::FieldModInstance>,
        owner_id: u32,
    }

    let mut to_despawn: Vec<DespawnData> = Vec::new();
    for (entity, mut proj, pos) in world
        .query::<(Entity, &mut ProjectileComp, &Position)>()
        .iter_mut(world)
    {
        proj.remaining_lifetime -= dt;
        if proj.remaining_lifetime <= 0.0 {
            to_despawn.push(DespawnData {
                entity,
                pos: *pos,
                field_mods: proj.field_mods.clone(),
                owner_id: proj.owner_id,
            });
        }
    }

    // 4.4 统计当前场地效果数量，用于上限检查
    let current_field_count = world.query::<&FieldEffect>().iter(world).count() as u32;
    let mut spawned_fields = 0u32;

    for dd in &to_despawn {
        // 4.4 弹射体 despawn 时根据 field_mods 生成场地效果
        if !dd.field_mods.is_empty() {
            for fm in &dd.field_mods {
                if current_field_count + spawned_fields >= MAX_FIELD_EFFECTS {
                    break;
                }
                world.spawn((
                    Position {
                        x: dd.pos.x,
                        y: dd.pos.y,
                    },
                    FieldEffect {
                        field_type: fm.field_type,
                        radius: fm.radius,
                        remaining_duration: fm.duration,
                        magnitude: fm.magnitude,
                        owner_id: dd.owner_id,
                    },
                ));
                spawned_fields += 1;
            }
        }

        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::ProjectileDespawned { entity: dd.entity });
        world.despawn(dd.entity);
    }
}
