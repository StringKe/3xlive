//! Tick 编排：run_game_tick 作为 exclusive system 注册到 Bevy FixedUpdate

use bevy_ecs::prelude::*;

use crate::chunk::ChunkCoord;
use crate::components::*;
use crate::data::StatusEffectType;
use crate::resources::ChunkRegistry;
use crate::systems::DeltaTime;

/// 主游戏 tick，作为 exclusive system 注册到 FixedUpdate
/// 签名 fn(world: &mut World) 可直接用于 app.add_systems(FixedUpdate, run_game_tick)
pub fn run_game_tick(world: &mut World) {
    // 1. Buff 效果应用
    crate::systems::buff::buff_apply_system(world);

    // 2. Movement
    inline_movement_system(world);

    // 3. 自动瞄准
    crate::systems::auto_aim::auto_aim_system(world);

    // 4. Disc 开火
    crate::systems::projectile::disc_fire_system(world);

    // 4.5. 弹射体轨迹修正（开火后、碰撞前）
    crate::systems::trajectory::trajectory_tick_system(world);

    // 5. 碰撞检测
    crate::systems::collision::projectile_monster_collision(world);
    crate::systems::collision::monster_player_collision(world);
    crate::systems::collision::monster_monster_collision(world);

    // 6. 状态效果 + Buff tick
    crate::systems::status_effect::status_effect_tick(world);
    crate::systems::buff::buff_tick_system(world);

    // 6.5. 元素反应（状态效果 tick 之后）
    crate::systems::elemental_reaction::elemental_reaction_system(world);

    // 6.6. 场地效果 tick（元素反应之后，可处理新生成的场地效果）
    crate::systems::field_effect::field_effect_tick_system(world);

    // 7. 弹射体生命周期
    crate::systems::projectile::projectile_despawn_system(world);

    // 8. 波次 + Boss
    crate::ai::wave::wave_system(world);
    crate::ai::wave::boss_spawn_system(world);
    crate::ai::wave::boss_attack_system(world);

    // 9. 掉落物
    crate::loot::loot_despawn_system(world);
    crate::loot::xp_magnet_system(world);
    crate::loot::pickup_system(world);

    // 10. 怪物死亡
    monster_death_system(world);

    // 11. 环境系统
    crate::systems::environment::data_storm_system(world);
    crate::systems::high_risk_event::high_risk_event_system(world);

    // 12. 怪物 AI（VS-Style：始终追击最近玩家）
    crate::ai::monster_ai::monster_chase_system(world);

    // 13. Chunk 管理（仅更新 chunk 追踪，刷怪统一由 wave_system 负责）
    crate::chunk::update_chunks(world);

    // 14. 等级上限
    level_cap_system(world);
}

/// 全局递增计数器，用于 Confuse 方向偏移的伪随机种子
static mut TICK_COUNTER: u32 = 0;

/// 内联移动系统（避免 Bevy system 参数约束）
/// 读取状态效果：Freeze/Shock→速度归零，Slow→速度降低，Fear→方向反转，Confuse→方向随机偏移
fn inline_movement_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    // WASM 单线程安全
    let tick_count = unsafe {
        TICK_COUNTER = TICK_COUNTER.wrapping_add(1);
        TICK_COUNTER
    };

    let updates: Vec<(Entity, f32, f32)> = world
        .query::<(Entity, &Velocity, Option<&StatusEffects>)>()
        .iter(world)
        .map(|(e, vel, status)| {
            let mut dx = vel.x * dt;
            let mut dy = vel.y * dt;

            if let Some(effects) = status {
                let mut frozen = false;
                let mut slow_factor = 1.0f32;
                let mut fear = false;
                let mut confuse = false;

                for eff in &effects.effects {
                    match eff.effect_type {
                        StatusEffectType::Freeze | StatusEffectType::Shock => {
                            // 冻结/感电：速度归零
                            frozen = true;
                        }
                        StatusEffectType::Slow => {
                            // 减速：乘以 (1.0 - magnitude) 系数，取最强减速
                            slow_factor = slow_factor.min(1.0 - eff.magnitude);
                        }
                        StatusEffectType::Fear => {
                            // 恐惧：方向反转
                            fear = true;
                        }
                        StatusEffectType::Confuse => {
                            // 混乱：方向随机偏移
                            confuse = true;
                        }
                        _ => {}
                    }
                }

                if frozen {
                    dx = 0.0;
                    dy = 0.0;
                } else {
                    // 应用减速
                    let factor = slow_factor.max(0.0);
                    dx *= factor;
                    dy *= factor;

                    // 恐惧：方向反转
                    if fear {
                        dx = -dx;
                        dy = -dy;
                    }

                    // 混乱：基于 tick 计数旋转方向
                    if confuse {
                        // 使用 entity index + tick_count 生成伪随机角度偏移
                        let seed = e
                            .index_u32()
                            .wrapping_mul(2654435761)
                            .wrapping_add(tick_count);
                        // 映射到 [-PI, PI] 的角度偏移
                        let angle = (seed as f32 / u32::MAX as f32) * std::f32::consts::TAU
                            - std::f32::consts::PI;
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();
                        let new_dx = dx * cos_a - dy * sin_a;
                        let new_dy = dx * sin_a + dy * cos_a;
                        dx = new_dx;
                        dy = new_dy;
                    }
                }
            }

            (e, dx, dy)
        })
        .collect();

    for (entity, dx, dy) in updates {
        if let Some(mut pos) = world.get_mut::<Position>(entity) {
            pos.x += dx;
            pos.y += dy;
        }
    }
}

/// 怪物死亡检测：HP <= 0 的怪物 despawn + 掉落 + 计分
fn monster_death_system(world: &mut World) {
    // 阶段 1：收集死亡怪物基础数据（entity, x, y, is_elite, is_boss）
    // 不在 query 迭代中调用 world.get，先收集 entity list 再逐个检查 marker
    let dead_entities: Vec<(Entity, f32, f32)> = world
        .query::<(Entity, &Health, &Position, &MonsterComp)>()
        .iter(world)
        .filter(|(_, hp, _, _)| hp.current <= 0.0)
        .map(|(entity, _, pos, _)| (entity, pos.x, pos.y))
        .collect();

    let dead_monsters: Vec<(Entity, f32, f32, bool, bool)> = dead_entities
        .into_iter()
        .map(|(entity, x, y)| {
            let is_elite = world.get::<EliteMarker>(entity).is_some();
            let is_boss = world.get::<BossMarker>(entity).is_some();
            (entity, x, y, is_elite, is_boss)
        })
        .collect();

    // 阶段 2：查 chunk difficulty 计算掉落参数
    let dead_with_diff: Vec<(Entity, f32, f32, f32, bool, bool)> = dead_monsters
        .into_iter()
        .map(|(entity, x, y, is_elite, is_boss)| {
            let chunk_coord = ChunkCoord::from_world_pos(x, y);
            let base_diff = world
                .resource::<ChunkRegistry>()
                .loaded_chunks
                .get(&chunk_coord)
                .map(|d| d.difficulty)
                .unwrap_or(0.5);
            let difficulty = if is_boss {
                base_diff * 5.0 + 2.0
            } else if is_elite {
                base_diff * 3.0 + 1.0
            } else {
                base_diff + 0.5
            };
            (entity, x, y, difficulty, is_elite, is_boss)
        })
        .collect();

    // 找最近玩家位置（用于掉落物弹射方向）
    let player_positions: Vec<(f32, f32)> = world
        .query::<(&Player, &Position)>()
        .iter(world)
        .map(|(_, pos)| (pos.x, pos.y))
        .collect();

    // 阶段 3：处理每个死亡怪物
    for (entity, x, y, difficulty, _is_elite, _is_boss) in &dead_with_diff {
        // 掉落物弹射：沿玩家反方向偏移 40-80px，让掉落物落在怪物群外围
        let loot_pos = if let Some(&(px, py)) = player_positions.first() {
            let dx = *x - px;
            let dy = *y - py;
            let dist = (dx * dx + dy * dy).sqrt();
            if dist > 1.0 {
                // 用怪物位置做伪随机，偏移 40-80px
                let offset = 40.0 + crate::loot::pseudo_random((*x, *y), 99) * 40.0;
                (*x + dx / dist * offset, *y + dy / dist * offset)
            } else {
                (*x, *y)
            }
        } else {
            (*x, *y)
        };
        crate::loot::spawn_monster_loot(world, loot_pos, *difficulty);

        // Chunk spawner 计数递减
        crate::chunk::on_monster_death(world, (*x, *y));

        // 销毁实体
        world.despawn(*entity);
    }
}

/// 等级上限系统
fn level_cap_system(world: &mut World) {
    let cap = crate::data::constants::DEFAULT_LEVEL_CAP;
    for mut level in world.query::<&mut Level>().iter_mut(world) {
        if level.value > cap {
            level.value = cap;
        }
    }
}
