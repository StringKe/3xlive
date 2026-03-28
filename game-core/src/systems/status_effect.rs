use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::StatusEffectType;
use crate::events::{GameEvent, GameEventQueue};

use super::collision::apply_damage;

use crate::data::constants::{DOT_MIN_DAMAGE, DOT_TICK_INTERVAL, TICK_DT};

/// 状态效果 tick：DoT 扣血、减速递减、引力拉扯、末日爆发、到期移除
/// DoT 使用 dot_timer 累积间隔，每 DOT_TICK_INTERVAL 触发一次伤害，不再每帧触发
pub fn status_effect_tick(world: &mut World) {
    let mut dot_damages: Vec<(Entity, f32)> = Vec::new();
    let mut gravity_pulls: Vec<(Entity, f32, f32)> = Vec::new();
    let mut doom_damages: Vec<(Entity, f32)> = Vec::new();

    for (entity, mut effects, hp) in world
        .query::<(Entity, &mut StatusEffects, Option<&Health>)>()
        .iter_mut(world)
    {
        let mut i = 0;
        while i < effects.effects.len() {
            let eff = &mut effects.effects[i];
            eff.remaining -= TICK_DT;

            match eff.effect_type {
                StatusEffectType::Poison | StatusEffectType::Burn => {
                    // DoT：累积到 DOT_TICK_INTERVAL 触发一次
                    eff.dot_timer += TICK_DT;
                    if eff.dot_timer >= DOT_TICK_INTERVAL {
                        let dmg = (eff.magnitude * DOT_TICK_INTERVAL).max(DOT_MIN_DAMAGE);
                        dot_damages.push((entity, dmg));
                        eff.dot_timer -= DOT_TICK_INTERVAL;
                    }
                }
                StatusEffectType::Shock => {
                    // 感电 DoT + 移动速度归零（在移动系统中读取）
                    eff.dot_timer += TICK_DT;
                    if eff.dot_timer >= DOT_TICK_INTERVAL {
                        let dmg = (eff.magnitude * DOT_TICK_INTERVAL).max(DOT_MIN_DAMAGE);
                        dot_damages.push((entity, dmg));
                        eff.dot_timer -= DOT_TICK_INTERVAL;
                    }
                }
                StatusEffectType::Bleed => {
                    // 流血 DoT：血量越低伤害越高
                    eff.dot_timer += TICK_DT;
                    if eff.dot_timer >= DOT_TICK_INTERVAL {
                        let hp_ratio = hp
                            .map(|h| if h.max > 0.0 { h.current / h.max } else { 1.0 })
                            .unwrap_or(1.0);
                        let dmg = (eff.magnitude * (1.0 - hp_ratio) * DOT_TICK_INTERVAL)
                            .max(DOT_MIN_DAMAGE);
                        dot_damages.push((entity, dmg));
                        eff.dot_timer -= DOT_TICK_INTERVAL;
                    }
                }
                StatusEffectType::Corrode => {
                    // 腐蚀 DoT
                    eff.dot_timer += TICK_DT;
                    if eff.dot_timer >= DOT_TICK_INTERVAL {
                        let dmg = (eff.magnitude * DOT_TICK_INTERVAL).max(DOT_MIN_DAMAGE);
                        dot_damages.push((entity, dmg));
                        eff.dot_timer -= DOT_TICK_INTERVAL;
                    }
                }
                StatusEffectType::GravityWell => {
                    // 引力井：每 tick 将目标拉向 anchor 点
                    if let Some((ax, ay)) = eff.anchor {
                        gravity_pulls.push((entity, ax, ay));
                        // 拉力存储在 magnitude 中，实际位移在下方计算
                        // 这里暂存 anchor 坐标，magnitude 在后续读取
                    }
                }
                StatusEffectType::Doom => {
                    // 末日：到期时爆发伤害 = magnitude
                    if eff.remaining <= 0.0 {
                        doom_damages.push((entity, eff.magnitude));
                    }
                    // duration 递减已在上方完成
                }
                StatusEffectType::Freeze
                | StatusEffectType::Slow
                | StatusEffectType::Weaken
                | StatusEffectType::Curse
                | StatusEffectType::Mark
                | StatusEffectType::Fear
                | StatusEffectType::Confuse
                | StatusEffectType::Blind => {
                    // 这些效果只递减 duration，由其他系统读取 magnitude
                }
            }

            if eff.remaining <= 0.0 {
                effects.effects.swap_remove(i);
            } else {
                i += 1;
            }
        }
    }

    // 应用 GravityWell 位移
    for (entity, anchor_x, anchor_y) in gravity_pulls {
        // 读取目标 Position 和效果 magnitude
        let pull_info = {
            let pos = world.get::<Position>(entity);
            let effects = world.get::<StatusEffects>(entity);
            if let (Some(pos), Some(effects)) = (pos, effects) {
                // 找到该实体的 GravityWell 效果 magnitude
                let magnitude = effects
                    .effects
                    .iter()
                    .find(|e| {
                        e.effect_type == StatusEffectType::GravityWell
                            && e.anchor == Some((anchor_x, anchor_y))
                    })
                    .map(|e| e.magnitude)
                    .unwrap_or(0.0);
                Some((pos.x, pos.y, magnitude))
            } else {
                None
            }
        };

        if let Some((px, py, magnitude)) = pull_info {
            let dx = anchor_x - px;
            let dy = anchor_y - py;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 1.0 {
                let pull_dist = magnitude * TICK_DT;
                // 不超过到 anchor 的距离
                let actual_pull = pull_dist.min(dist);
                let move_x = dx / dist * actual_pull;
                let move_y = dy / dist * actual_pull;
                if let Some(mut pos) = world.get_mut::<Position>(entity) {
                    pos.x += move_x;
                    pos.y += move_y;
                }
            }
        }
    }

    // 应用 DoT 伤害
    for (entity, dmg) in dot_damages {
        let remaining_hp = apply_damage(world, entity, dmg);
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::EntityDamaged {
                entity,
                damage: dmg,
                remaining_hp,
            });
        if remaining_hp <= 0.0 {
            world
                .resource_mut::<GameEventQueue>()
                .push(GameEvent::EntityKilled {
                    entity,
                    killer: None,
                });
        }
    }

    // 应用 Doom 爆发伤害
    for (entity, dmg) in doom_damages {
        let remaining_hp = apply_damage(world, entity, dmg);
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::EntityDamaged {
                entity,
                damage: dmg,
                remaining_hp,
            });
        if remaining_hp <= 0.0 {
            world
                .resource_mut::<GameEventQueue>()
                .push(GameEvent::EntityKilled {
                    entity,
                    killer: None,
                });
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;

    /// 6.4.1: Bleed DoT 测试：HP 50% 时验证间隔 tick 后 Bleed 伤害
    #[test]
    fn bleed_dot_scales_with_missing_hp() {
        use crate::data::constants::DOT_TICK_INTERVAL;

        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Health {
                    current: 50.0,
                    max: 100.0,
                },
                StatusEffects {
                    effects: vec![ActiveEffect {
                        effect_type: StatusEffectType::Bleed,
                        remaining: 5.0,
                        magnitude: 20.0,
                        anchor: None,
                        dot_timer: 0.0,
                    }],
                },
            ))
            .id();

        // 连续 tick 直到 dot_timer 累积超过 DOT_TICK_INTERVAL
        let ticks_needed = (DOT_TICK_INTERVAL / TICK_DT).ceil() as usize;
        for _ in 0..ticks_needed {
            status_effect_tick(&mut world);
        }

        // HP 50% → missing ratio = 0.5 → damage = max(20.0 * 0.5 * DOT_TICK_INTERVAL, 1.0)
        let raw_dmg = 20.0 * 0.5 * DOT_TICK_INTERVAL;
        let expected_dmg = raw_dmg.max(1.0);
        let hp = world.get::<Health>(entity).unwrap();
        let expected_hp = 50.0 - expected_dmg;
        assert!(
            (hp.current - expected_hp).abs() < 0.1,
            "期望 HP={:.4}，实际 HP={:.4}",
            expected_hp,
            hp.current
        );
    }

    /// 6.4.2: Doom 爆发测试：duration 设为单 tick 内到期，验证施加 magnitude 点伤害
    #[test]
    fn doom_explodes_on_expiry() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                StatusEffects {
                    effects: vec![ActiveEffect {
                        effect_type: StatusEffectType::Doom,
                        // 设为恰好一个 tick 后到期
                        remaining: TICK_DT,
                        magnitude: 50.0,
                        anchor: None,
                        dot_timer: 0.0,
                    }],
                },
            ))
            .id();

        status_effect_tick(&mut world);

        // Doom 到期，施加 50.0 伤害
        let hp = world.get::<Health>(entity).unwrap();
        assert_eq!(hp.current, 50.0);

        // Doom 效果已被移除
        let effects = world.get::<StatusEffects>(entity).unwrap();
        assert!(effects.effects.is_empty(), "Doom 效果应在到期后移除");
    }

    /// 6.4.3: GravityWell 拉扯测试：anchor=(0,0)，目标 Position=(100,0)，验证拉向 anchor
    #[test]
    fn gravity_well_pulls_toward_anchor() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let entity = world
            .spawn((
                Position { x: 100.0, y: 0.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                StatusEffects {
                    effects: vec![ActiveEffect {
                        effect_type: StatusEffectType::GravityWell,
                        remaining: 5.0,
                        magnitude: 200.0, // 200 像素/秒的拉力
                        anchor: Some((0.0, 0.0)),
                        dot_timer: 0.0,
                    }],
                },
            ))
            .id();

        status_effect_tick(&mut world);

        let pos = world.get::<Position>(entity).unwrap();
        // 目标在 (100, 0)，anchor 在 (0, 0)，方向 = (-1, 0)
        // 拉力距离 = 200.0 * TICK_DT
        let expected_pull = 200.0 * TICK_DT;
        let expected_x = 100.0 - expected_pull;
        assert!(
            (pos.x - expected_x).abs() < 0.001,
            "期望 x={:.4}，实际 x={:.4}",
            expected_x,
            pos.x
        );
        assert!(pos.y.abs() < 0.001, "y 坐标不应变化，实际 y={:.4}", pos.y);
    }
}
