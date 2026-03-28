//! 场地效果 tick 系统
//!
//! 遍历所有 FieldEffect 实体，按类型对范围内目标施加效果。

use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::constants::{MAX_FIELD_EFFECTS, TICK_DT};
use crate::data::{FieldType, StatusEffectType};
use crate::events::{GameEvent, GameEventQueue};
use crate::systems::collision::apply_damage;

/// 场地效果 tick 系统
pub fn field_effect_tick_system(world: &mut World) {
    // 收集所有场地效果数据
    struct FieldData {
        entity: Entity,
        field_type: FieldType,
        x: f32,
        y: f32,
        radius: f32,
        magnitude: f32,
        remaining: f32,
        owner_id: u32,
    }

    let fields: Vec<FieldData> = world
        .query::<(Entity, &FieldEffect, &Position)>()
        .iter(world)
        .map(|(entity, field, pos)| FieldData {
            entity,
            field_type: field.field_type,
            x: pos.x,
            y: pos.y,
            radius: field.radius,
            magnitude: field.magnitude,
            remaining: field.remaining_duration,
            owner_id: field.owner_id,
        })
        .collect();

    // 递减持续时间
    for fd in &fields {
        if let Some(mut field) = world.get_mut::<FieldEffect>(fd.entity) {
            field.remaining_duration -= TICK_DT;
        }
    }

    // 按类型施加效果
    for fd in &fields {
        if fd.remaining <= 0.0 {
            continue;
        }

        let radius_sq = fd.radius * fd.radius;

        match fd.field_type {
            FieldType::DamageZone => {
                // 范围内所有非 owner 的实体 HP -= magnitude * TICK_DT
                let targets: Vec<Entity> = world
                    .query::<(Entity, &Position, &Health)>()
                    .iter(world)
                    .filter(|(_, pos, _)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, _, _)| e)
                    .collect();

                let damage = fd.magnitude * TICK_DT;
                for target in targets {
                    // 跳过 owner 的实体（通过 Player 组件检查）
                    if let Some(player) = world.get::<Player>(target) {
                        if player.id == fd.owner_id {
                            continue;
                        }
                    }
                    let remaining_hp = apply_damage(world, target, damage);
                    world
                        .resource_mut::<GameEventQueue>()
                        .push(GameEvent::EntityDamaged {
                            entity: target,
                            damage,
                            remaining_hp,
                        });
                }
            }
            FieldType::SlowField => {
                // 范围内所有怪物添加 Slow 效果
                let monsters: Vec<Entity> = world
                    .query_filtered::<(Entity, &Position), With<MonsterComp>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, _)| e)
                    .collect();

                for monster in monsters {
                    if let Some(mut effects) = world.get_mut::<StatusEffects>(monster) {
                        // 刷新或添加 Slow 效果（短 duration，每 tick 刷新）
                        let has_slow = effects
                            .effects
                            .iter_mut()
                            .find(|e| e.effect_type == StatusEffectType::Slow);
                        if let Some(slow) = has_slow {
                            slow.remaining = 0.1;
                            slow.magnitude = slow.magnitude.max(fd.magnitude);
                        } else {
                            effects.effects.push(ActiveEffect {
                                effect_type: StatusEffectType::Slow,
                                remaining: 0.1,
                                magnitude: fd.magnitude,
                                anchor: None,
                                dot_timer: 0.0,
                            });
                        }
                    }
                }
            }
            FieldType::HealZone => {
                // 范围内所有玩家 HP += magnitude * TICK_DT
                let players: Vec<Entity> = world
                    .query_filtered::<(Entity, &Position), With<Player>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, _)| e)
                    .collect();

                let heal = fd.magnitude * TICK_DT;
                for player in players {
                    if let Some(mut hp) = world.get_mut::<Health>(player) {
                        hp.current = (hp.current + heal).min(hp.max);
                    }
                }
            }
            FieldType::GravityPull => {
                // 范围内所有怪物的 Position 向中心移动
                let monsters: Vec<(Entity, f32, f32)> = world
                    .query_filtered::<(Entity, &Position), With<MonsterComp>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, pos)| (e, pos.x, pos.y))
                    .collect();

                for (monster, mx, my) in monsters {
                    let dx = fd.x - mx;
                    let dy = fd.y - my;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 1.0 {
                        let pull = (fd.magnitude * TICK_DT).min(dist);
                        let move_x = dx / dist * pull;
                        let move_y = dy / dist * pull;
                        if let Some(mut pos) = world.get_mut::<Position>(monster) {
                            pos.x += move_x;
                            pos.y += move_y;
                        }
                    }
                }
            }
            FieldType::MineField => {
                // 怪物进入范围时爆炸（一次性大伤害，然后销毁场地效果）
                let monsters_in_range: Vec<Entity> = world
                    .query_filtered::<(Entity, &Position), With<MonsterComp>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, _)| e)
                    .collect();

                if !monsters_in_range.is_empty() {
                    // 对范围内所有怪物造成伤害
                    for monster in &monsters_in_range {
                        let remaining_hp = apply_damage(world, *monster, fd.magnitude);
                        world
                            .resource_mut::<GameEventQueue>()
                            .push(GameEvent::EntityDamaged {
                                entity: *monster,
                                damage: fd.magnitude,
                                remaining_hp,
                            });
                    }
                    // 标记为到期（下方统一 despawn）
                    if let Some(mut field) = world.get_mut::<FieldEffect>(fd.entity) {
                        field.remaining_duration = 0.0;
                    }
                }
            }
            FieldType::Vortex => {
                // GravityPull + DamageZone 组合效果
                let monsters: Vec<(Entity, f32, f32)> = world
                    .query_filtered::<(Entity, &Position), With<MonsterComp>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, pos)| (e, pos.x, pos.y))
                    .collect();

                let damage = fd.magnitude * 0.5 * TICK_DT; // Vortex 伤害为 magnitude 的一半
                for (monster, mx, my) in &monsters {
                    // 引力牵引
                    let dx = fd.x - mx;
                    let dy = fd.y - my;
                    let dist = (dx * dx + dy * dy).sqrt();
                    if dist > 1.0 {
                        let pull = (fd.magnitude * TICK_DT).min(dist);
                        let move_x = dx / dist * pull;
                        let move_y = dy / dist * pull;
                        if let Some(mut pos) = world.get_mut::<Position>(*monster) {
                            pos.x += move_x;
                            pos.y += move_y;
                        }
                    }
                    // 伤害
                    let remaining_hp = apply_damage(world, *monster, damage);
                    world
                        .resource_mut::<GameEventQueue>()
                        .push(GameEvent::EntityDamaged {
                            entity: *monster,
                            damage,
                            remaining_hp,
                        });
                }
            }
            FieldType::Shield => {
                // 护盾区域：范围内所有玩家每 tick 获得微量 HP 回复
                let players: Vec<Entity> = world
                    .query_filtered::<(Entity, &Position), With<Player>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, _)| e)
                    .collect();

                let heal = fd.magnitude * 0.1 * TICK_DT;
                for player in players {
                    if let Some(mut hp) = world.get_mut::<Health>(player) {
                        hp.current = (hp.current + heal).min(hp.max);
                    }
                }
            }
            FieldType::Beacon => {
                // 信标区域：对范围内所有怪物施加短暂 Mark 效果（受到的伤害增加）
                let monsters: Vec<Entity> = world
                    .query_filtered::<(Entity, &Position), With<MonsterComp>>()
                    .iter(world)
                    .filter(|(_, pos)| {
                        let dx = pos.x - fd.x;
                        let dy = pos.y - fd.y;
                        dx * dx + dy * dy <= radius_sq
                    })
                    .map(|(e, _)| e)
                    .collect();

                for monster in monsters {
                    if let Some(mut effects) = world.get_mut::<StatusEffects>(monster) {
                        // 刷新或添加 Mark 效果（短 duration，每 tick 持续刷新）
                        let has_mark = effects
                            .effects
                            .iter_mut()
                            .find(|e| e.effect_type == StatusEffectType::Mark);
                        if let Some(mark) = has_mark {
                            mark.remaining = 0.1;
                            mark.magnitude = mark.magnitude.max(fd.magnitude * 0.5);
                        } else {
                            effects.effects.push(ActiveEffect {
                                effect_type: StatusEffectType::Mark,
                                remaining: 0.1,
                                magnitude: fd.magnitude * 0.5,
                                anchor: None,
                                dot_timer: 0.0,
                            });
                        }
                    }
                }
            }
        }
    }

    // 收集到期的场地效果并 despawn
    let to_despawn: Vec<Entity> = world
        .query::<(Entity, &FieldEffect)>()
        .iter(world)
        .filter(|(_, field)| field.remaining_duration <= 0.0)
        .map(|(e, _)| e)
        .collect();

    for entity in to_despawn {
        world.despawn(entity);
    }

    // 场地效果数量上限检查：超过上限时 despawn remaining_duration 最小的
    let mut field_entities: Vec<(Entity, f32)> = world
        .query::<(Entity, &FieldEffect)>()
        .iter(world)
        .map(|(e, f)| (e, f.remaining_duration))
        .collect();

    if field_entities.len() as u32 > MAX_FIELD_EFFECTS {
        // 按 remaining_duration 升序排序
        field_entities.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
        let excess = field_entities.len() as u32 - MAX_FIELD_EFFECTS;
        for item in field_entities.iter().take(excess as usize) {
            world.despawn(item.0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;

    /// DamageZone 测试：范围内实体受到持续伤害
    #[test]
    fn damage_zone_hurts_entities_in_range() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        // 创建 DamageZone 场地效果
        world.spawn((
            Position { x: 0.0, y: 0.0 },
            FieldEffect {
                field_type: FieldType::DamageZone,
                radius: 100.0,
                remaining_duration: 5.0,
                magnitude: 20.0, // 20 DPS
                owner_id: u32::MAX,
            },
        ));

        // 范围内的怪物（距离 50 < 100）
        let monster_in = world
            .spawn((
                Position { x: 50.0, y: 0.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MonsterComp {
                    monster_type: 1,
                    damage: 5.0,
                },
            ))
            .id();

        // 范围外的怪物（距离 150 > 100）
        let monster_out = world
            .spawn((
                Position { x: 150.0, y: 0.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MonsterComp {
                    monster_type: 1,
                    damage: 5.0,
                },
            ))
            .id();

        field_effect_tick_system(&mut world);

        // 范围内怪物应受到 20.0 * TICK_DT 伤害
        let expected_damage = 20.0 * TICK_DT;
        let hp_in = world.get::<Health>(monster_in).unwrap();
        assert!(
            (hp_in.current - (100.0 - expected_damage)).abs() < 0.001,
            "范围内怪物应受伤，期望 HP={:.4}，实际 HP={:.4}",
            100.0 - expected_damage,
            hp_in.current
        );

        // 范围外怪物不受伤
        let hp_out = world.get::<Health>(monster_out).unwrap();
        assert_eq!(hp_out.current, 100.0, "范围外怪物不应受伤");
    }

    /// 到期 despawn 测试
    #[test]
    fn expired_field_effect_despawns() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let field_entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                FieldEffect {
                    field_type: FieldType::DamageZone,
                    radius: 50.0,
                    remaining_duration: TICK_DT, // 恰好一个 tick 后到期
                    magnitude: 10.0,
                    owner_id: u32::MAX,
                },
            ))
            .id();

        field_effect_tick_system(&mut world);

        // 场地效果应被 despawn
        assert!(
            world.get_entity(field_entity).is_err(),
            "到期的场地效果应被 despawn"
        );
    }
}
