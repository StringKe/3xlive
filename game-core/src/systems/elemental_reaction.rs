//! 元素反应系统
//!
//! 检测同一实体上同时存在的两种状态效果组合，触发对应的元素反应。

use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::constants::MAX_FIELD_EFFECTS;
use crate::data::{FieldType, ReactionType, StatusEffectType};
use crate::events::{GameEvent, GameEventQueue};
use crate::systems::collision::apply_damage;

/// 反应定义表：(效果A, 效果B, 反应类型)
/// 按优先级排列：Shatter > Melt > Toxic > Sepsis > Shred > BlackHole > Panic
const REACTIONS: &[(StatusEffectType, StatusEffectType, ReactionType)] = &[
    (
        StatusEffectType::Freeze,
        StatusEffectType::Shock,
        ReactionType::Shatter,
    ),
    (
        StatusEffectType::Burn,
        StatusEffectType::Freeze,
        ReactionType::Melt,
    ),
    (
        StatusEffectType::Poison,
        StatusEffectType::Burn,
        ReactionType::Toxic,
    ),
    (
        StatusEffectType::Bleed,
        StatusEffectType::Poison,
        ReactionType::Sepsis,
    ),
    (
        StatusEffectType::Curse,
        StatusEffectType::Weaken,
        ReactionType::Shred,
    ),
    (
        StatusEffectType::Slow,
        StatusEffectType::GravityWell,
        ReactionType::BlackHole,
    ),
    (
        StatusEffectType::Confuse,
        StatusEffectType::Fear,
        ReactionType::Panic,
    ),
];

/// 元素反应 tick 系统
pub fn elemental_reaction_system(world: &mut World) {
    // 收集所有带状态效果的实体及其效果列表
    let entities_with_effects: Vec<(Entity, Vec<ActiveEffect>)> = world
        .query::<(Entity, &StatusEffects)>()
        .iter(world)
        .filter(|(_, effects)| effects.effects.len() >= 2)
        .map(|(entity, effects)| (entity, effects.effects.clone()))
        .collect();

    // 收集需要执行的反应
    struct ReactionData {
        entity: Entity,
        reaction: ReactionType,
        type_a: StatusEffectType,
        type_b: StatusEffectType,
        mag_a: f32,
        mag_b: f32,
        pos: Option<(f32, f32)>,
    }

    let mut reactions: Vec<ReactionData> = Vec::new();

    for (entity, effects) in &entities_with_effects {
        // 每个实体每 tick 最多触发一个反应（最高优先级）
        for (type_a, type_b, reaction) in REACTIONS {
            let found_a = effects.iter().find(|e| e.effect_type == *type_a);
            let found_b = effects.iter().find(|e| e.effect_type == *type_b);

            if let (Some(eff_a), Some(eff_b)) = (found_a, found_b) {
                let pos = world.get::<Position>(*entity).map(|p| (p.x, p.y));
                reactions.push(ReactionData {
                    entity: *entity,
                    reaction: *reaction,
                    type_a: *type_a,
                    type_b: *type_b,
                    mag_a: eff_a.magnitude,
                    mag_b: eff_b.magnitude,
                    pos,
                });
                break; // 最高优先级已触发，跳过后续反应
            }
        }
    }

    // 执行反应
    for rd in &reactions {
        // 消耗两个状态效果
        if let Some(mut effects) = world.get_mut::<StatusEffects>(rd.entity) {
            remove_effect_by_type(&mut effects.effects, rd.type_a);
            remove_effect_by_type(&mut effects.effects, rd.type_b);
        }

        // 推送反应事件
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::ElementalReaction {
                entity: rd.entity,
                reaction: rd.reaction,
                consumed: (rd.type_a, rd.type_b),
            });

        // 施加反应效果
        match rd.reaction {
            ReactionType::Shatter => {
                // 对目标施加 150 点伤害
                let remaining_hp = apply_damage(world, rd.entity, 150.0);
                world
                    .resource_mut::<GameEventQueue>()
                    .push(GameEvent::EntityDamaged {
                        entity: rd.entity,
                        damage: 150.0,
                        remaining_hp,
                    });
            }
            ReactionType::Melt => {
                // 伤害 = max(freeze_mag, burn_mag) * 3.0
                let damage = rd.mag_a.max(rd.mag_b) * 3.0;
                let remaining_hp = apply_damage(world, rd.entity, damage);
                world
                    .resource_mut::<GameEventQueue>()
                    .push(GameEvent::EntityDamaged {
                        entity: rd.entity,
                        damage,
                        remaining_hp,
                    });
            }
            ReactionType::Toxic => {
                // 在目标位置生成 DamageZone 场地效果
                if let Some((x, y)) = rd.pos {
                    let field_count = world.query::<&FieldEffect>().iter(world).count() as u32;
                    if field_count < MAX_FIELD_EFFECTS {
                        let field_entity = world
                            .spawn((
                                Position { x, y },
                                FieldEffect {
                                    field_type: FieldType::DamageZone,
                                    radius: 80.0,
                                    remaining_duration: 5.0,
                                    magnitude: rd.mag_a.max(rd.mag_b),
                                    owner_id: u32::MAX, // 反应生成，无特定 owner
                                },
                            ))
                            .id();

                        world
                            .resource_mut::<GameEventQueue>()
                            .push(GameEvent::FieldSpawned {
                                entity: field_entity,
                                field_type: FieldType::DamageZone,
                                x,
                                y,
                                radius: 80.0,
                                duration: 5.0,
                            });
                    }
                }
            }
            ReactionType::Sepsis => {
                // 施加强 Poison（duration * 2, magnitude * 2）
                if let Some(mut effects) = world.get_mut::<StatusEffects>(rd.entity) {
                    effects.effects.push(ActiveEffect {
                        effect_type: StatusEffectType::Poison,
                        remaining: 10.0, // 基础 5s * 2
                        magnitude: rd.mag_a.max(rd.mag_b) * 2.0,
                        anchor: None,
                        dot_timer: 0.0,
                    });
                }
            }
            ReactionType::Shred => {
                // 施加强 Weaken（magnitude = 1.0, duration = 5.0）
                if let Some(mut effects) = world.get_mut::<StatusEffects>(rd.entity) {
                    effects.effects.push(ActiveEffect {
                        effect_type: StatusEffectType::Weaken,
                        remaining: 5.0,
                        magnitude: 1.0,
                        anchor: None,
                        dot_timer: 0.0,
                    });
                }
            }
            ReactionType::BlackHole => {
                // 生成 GravityPull + DamageZone 组合场地效果（Vortex）
                if let Some((x, y)) = rd.pos {
                    let field_count = world.query::<&FieldEffect>().iter(world).count() as u32;
                    if field_count < MAX_FIELD_EFFECTS {
                        let field_entity = world
                            .spawn((
                                Position { x, y },
                                FieldEffect {
                                    field_type: FieldType::Vortex,
                                    radius: 120.0,
                                    remaining_duration: 4.0,
                                    magnitude: rd.mag_a.max(rd.mag_b) * 2.0,
                                    owner_id: u32::MAX,
                                },
                            ))
                            .id();

                        world
                            .resource_mut::<GameEventQueue>()
                            .push(GameEvent::FieldSpawned {
                                entity: field_entity,
                                field_type: FieldType::Vortex,
                                x,
                                y,
                                radius: 120.0,
                                duration: 4.0,
                            });
                    }
                }
            }
            ReactionType::Panic => {
                // 对目标周围 200px 内所有怪物施加 Fear
                if let Some((x, y)) = rd.pos {
                    let panic_radius = 200.0;
                    let nearby_monsters: Vec<Entity> = world
                        .query_filtered::<(Entity, &Position), With<MonsterComp>>()
                        .iter(world)
                        .filter(|(_, pos)| {
                            let dx = pos.x - x;
                            let dy = pos.y - y;
                            dx * dx + dy * dy <= panic_radius * panic_radius
                        })
                        .map(|(e, _)| e)
                        .collect();

                    for monster in nearby_monsters {
                        if let Some(mut effects) = world.get_mut::<StatusEffects>(monster) {
                            effects.effects.push(ActiveEffect {
                                effect_type: StatusEffectType::Fear,
                                remaining: 3.0,
                                magnitude: 1.0,
                                anchor: None,
                                dot_timer: 0.0,
                            });
                        }
                    }
                }
            }
        }
    }
}

/// 从效果列表中移除第一个匹配类型的效果
fn remove_effect_by_type(effects: &mut Vec<ActiveEffect>, effect_type: StatusEffectType) {
    if let Some(idx) = effects.iter().position(|e| e.effect_type == effect_type) {
        effects.swap_remove(idx);
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;

    /// Shatter 反应触发测试：同时有 Freeze + Shock → 消耗两者 + 施加 150 伤害
    #[test]
    fn shatter_reaction_triggers() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Health {
                    current: 200.0,
                    max: 200.0,
                },
                StatusEffects {
                    effects: vec![
                        ActiveEffect {
                            effect_type: StatusEffectType::Freeze,
                            remaining: 3.0,
                            magnitude: 10.0,
                            anchor: None,
                            dot_timer: 0.0,
                        },
                        ActiveEffect {
                            effect_type: StatusEffectType::Shock,
                            remaining: 3.0,
                            magnitude: 20.0,
                            anchor: None,
                            dot_timer: 0.0,
                        },
                    ],
                },
            ))
            .id();

        elemental_reaction_system(&mut world);

        // HP 应减少 150
        let hp = world.get::<Health>(entity).unwrap();
        assert_eq!(hp.current, 50.0, "Shatter 应造成 150 点伤害");

        // 两个状态效果都应被消耗
        let effects = world.get::<StatusEffects>(entity).unwrap();
        assert!(
            effects.effects.is_empty(),
            "Freeze 和 Shock 应被消耗，剩余效果数={}",
            effects.effects.len()
        );

        // 检查事件队列
        let events = world.resource_mut::<GameEventQueue>().drain();
        let has_reaction = events.iter().any(|e| {
            matches!(
                e,
                GameEvent::ElementalReaction {
                    reaction: ReactionType::Shatter,
                    ..
                }
            )
        });
        assert!(has_reaction, "应触发 Shatter 反应事件");
    }

    /// 优先级测试：同时有 Freeze + Shock + Burn → 只触发 Shatter（最高优先级）
    #[test]
    fn highest_priority_reaction_wins() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Health {
                    current: 200.0,
                    max: 200.0,
                },
                StatusEffects {
                    effects: vec![
                        ActiveEffect {
                            effect_type: StatusEffectType::Freeze,
                            remaining: 3.0,
                            magnitude: 10.0,
                            anchor: None,
                            dot_timer: 0.0,
                        },
                        ActiveEffect {
                            effect_type: StatusEffectType::Shock,
                            remaining: 3.0,
                            magnitude: 20.0,
                            anchor: None,
                            dot_timer: 0.0,
                        },
                        ActiveEffect {
                            effect_type: StatusEffectType::Burn,
                            remaining: 3.0,
                            magnitude: 15.0,
                            anchor: None,
                            dot_timer: 0.0,
                        },
                    ],
                },
            ))
            .id();

        elemental_reaction_system(&mut world);

        // Shatter 触发，Freeze + Shock 消耗，Burn 保留
        let effects = world.get::<StatusEffects>(entity).unwrap();
        assert_eq!(effects.effects.len(), 1, "应只剩 Burn 效果");
        assert_eq!(
            effects.effects[0].effect_type,
            StatusEffectType::Burn,
            "剩余效果应为 Burn"
        );
    }
}
