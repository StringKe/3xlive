//! 波次刷新系统与 Boss 攻击系统（单人模式）

use bevy_ecs::prelude::*;
use bevy_ecs::world::World;

use crate::components::*;
use crate::data::constants::*;
use crate::events::{GameEvent, GameEventQueue};
use crate::resources::{ViewportSize, WaveManager};
use crate::systems::collision::apply_damage;
use crate::systems::DeltaTime;

use super::spawner::spawn_monster;

/// 伪随机（splitmix32，比单轮 xorshift 分布更均匀）
fn pseudo_random(seed: u32) -> f32 {
    let mut z = seed.wrapping_add(0x9E3779B9);
    z = (z ^ (z >> 16)).wrapping_mul(0x45D9F3B);
    z = (z ^ (z >> 16)).wrapping_mul(0x45D9F3B);
    z = z ^ (z >> 16);
    z as f32 / u32::MAX as f32
}

// ─── 波次系统 ───

pub fn wave_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    let (wave_ready, wave_number) = {
        let mut wm = world.resource_mut::<WaveManager>();
        wm.wave_timer -= dt;
        if wm.wave_timer <= 0.0 {
            wm.wave_number += 1;
            let wn = wm.wave_number;
            wm.wave_timer = WAVE_INTERVAL;
            (true, wn)
        } else {
            (false, 0)
        }
    };

    if !wave_ready {
        return;
    }

    // 场上怪物数量上限检查
    let current_monster_count = world.query::<&MonsterComp>().iter(world).count() as u32;
    if current_monster_count >= MONSTER_MAX_COUNT {
        return;
    }

    // 基于视口尺寸动态计算生成距离：屏幕半对角线 + 缓冲
    let vp = world
        .get_resource::<ViewportSize>()
        .cloned()
        .unwrap_or_default();
    let half_diag = vp.half_diagonal();
    let spawn_r_min = half_diag + 100.0;
    let spawn_r_max = spawn_r_min + 400.0;

    // 收集玩家位置和最高等级
    let players: Vec<(f32, f32)> = world
        .query::<(&Player, &Position)>()
        .iter(world)
        .map(|(_, pos)| (pos.x, pos.y))
        .collect();

    let max_level: u32 = world
        .query::<(&Player, &Level)>()
        .iter(world)
        .map(|(_, lv)| lv.value)
        .max()
        .unwrap_or(1);

    let raw_count = (WAVE_BASE_COUNT * (1.0 + wave_number as f32 * WAVE_COUNT_GROWTH)) as u32;
    let count = raw_count.min(MONSTER_MAX_COUNT - current_monster_count);
    let spawn_elite = wave_number % ELITE_SPAWN_WAVE_INTERVAL == 0 && wave_number > 0;

    // 用递增的 spawn_seed 作为种子基础，每次刷新位置都不同
    let spawn_seed = {
        let mut wm = world.resource_mut::<WaveManager>();
        wm.spawn_seed = wm.spawn_seed.wrapping_add(1);
        wm.spawn_seed
    };
    let mut seed_counter: u32 = spawn_seed
        .wrapping_mul(31337)
        .wrapping_add(wave_number.wrapping_mul(7919));

    for &(px, py) in &players {
        seed_counter = seed_counter.wrapping_add(13);
        let mut spawned_positions: Vec<(f32, f32)> = Vec::with_capacity(count as usize);

        for i in 0..count {
            // 每个怪物独立随机角度和半径
            seed_counter = seed_counter.wrapping_add(i.wrapping_mul(97).wrapping_add(31));
            let angle = pseudo_random(seed_counter) * std::f32::consts::TAU;
            seed_counter = seed_counter.wrapping_add(i.wrapping_mul(53).wrapping_add(17));
            // sqrt 矫正：在环形区域内实现面积均匀分布
            let r_min_sq = spawn_r_min * spawn_r_min;
            let r_max_sq = spawn_r_max * spawn_r_max;
            let radius = (r_min_sq + pseudo_random(seed_counter) * (r_max_sq - r_min_sq)).sqrt();

            let mut sx = px + angle.cos() * radius;
            let mut sy = py + angle.sin() * radius;

            // 间距检查：与本批次已生成怪物保持 >= SPAWN_MIN_SPACING，最多重试 3 次
            let spacing_sq = SPAWN_MIN_SPACING * SPAWN_MIN_SPACING;
            for retry in 0..3u32 {
                let too_close = spawned_positions.iter().any(|&(ox, oy)| {
                    let dx = sx - ox;
                    let dy = sy - oy;
                    dx * dx + dy * dy < spacing_sq
                });
                if !too_close {
                    break;
                }
                seed_counter = seed_counter.wrapping_add(retry + 100);
                let offset = if pseudo_random(seed_counter) > 0.5 {
                    30.0
                } else {
                    -30.0
                };
                let new_radius = radius + offset;
                sx = px + angle.cos() * new_radius;
                sy = py + angle.sin() * new_radius;
            }
            spawned_positions.push((sx, sy));

            let is_elite = spawn_elite && i == 0;

            let hp = if is_elite {
                BASE_MONSTER_HP * max_level as f32 * ELITE_HP_MULT
            } else {
                BASE_MONSTER_HP * max_level as f32
            };
            let damage = if is_elite {
                BASE_MONSTER_DAMAGE * ELITE_DAMAGE_MULT
            } else {
                BASE_MONSTER_DAMAGE
            };

            let monster_type: u8 = if is_elite { ELITE_MONSTER_TYPE } else { 1 };

            // 碰撞半径匹配客户端视觉大小，内缩 1%
            let collision_radius = if is_elite {
                // 菱形 18x18 * scale 1.3 → half-side 11.7
                ELITE_COLLISION_RADIUS
            } else if i % 2 == 0 {
                // 三角形 circumradius 14.0
                MONSTER_COLLISION_RADIUS_TRIANGLE
            } else {
                // 方块 22x22 → half-side 11.0
                MONSTER_COLLISION_RADIUS_SQUARE
            };

            let entity = spawn_monster(
                world,
                monster_type,
                sx,
                sy,
                hp,
                damage,
                MONSTER_DETECTION_RADIUS,
                collision_radius,
            );

            if is_elite {
                world.entity_mut(entity).insert(EliteMarker);
            }
        }
    }
}

// ─── Boss 刷新系统 ───

pub fn boss_spawn_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    let should_spawn = {
        let mut wm = world.resource_mut::<WaveManager>();
        wm.boss_timer -= dt;
        if wm.boss_timer <= 0.0 {
            wm.boss_timer = BOSS_SPAWN_INTERVAL;
            true
        } else {
            false
        }
    };

    if !should_spawn {
        return;
    }

    let player_pos: Option<(f32, f32)> = world
        .query::<(&Player, &Position)>()
        .iter(world)
        .map(|(_, pos)| (pos.x, pos.y))
        .next();

    let (px, py) = match player_pos {
        Some(p) => p,
        None => return,
    };

    let max_level: u32 = world
        .query::<(&Player, &Level)>()
        .iter(world)
        .map(|(_, lv)| lv.value)
        .max()
        .unwrap_or(1);

    let boss_hp = BASE_MONSTER_HP * max_level as f32 * BOSS_HP_MULT;
    let boss_damage = BASE_MONSTER_DAMAGE * BOSS_DAMAGE_MULT;

    // Boss 生成在玩家前方 BOSS_SPAWN_OFFSET px
    let sx = px + BOSS_SPAWN_OFFSET;
    let sy = py;

    // 六边形 circumradius 16.0 * scale 3.0 = 48.0，内缩 1%
    let collision_radius = BOSS_COLLISION_RADIUS;

    let entity = spawn_monster(
        world,
        BOSS_MONSTER_TYPE, // Boss 类型
        sx,
        sy,
        boss_hp,
        boss_damage,
        BOSS_DETECTION_RADIUS, // Boss 检测半径
        collision_radius,
    );

    world.entity_mut(entity).insert((
        BossMarker,
        BossAttackPattern {
            patterns: vec![
                BossAttack {
                    pattern_type: BossPattern::AreaAoe,
                    cooldown: BOSS_AOE_COOLDOWN,
                    cooldown_timer: BOSS_AOE_COOLDOWN,
                    damage: BOSS_AOE_DAMAGE,
                    radius: BOSS_AOE_RADIUS,
                },
                BossAttack {
                    pattern_type: BossPattern::Barrage,
                    cooldown: BOSS_BARRAGE_COOLDOWN,
                    cooldown_timer: BOSS_BARRAGE_COOLDOWN,
                    damage: BOSS_BARRAGE_DAMAGE,
                    radius: BOSS_BARRAGE_SPREAD, // 扇形半角(弧度)
                },
                BossAttack {
                    pattern_type: BossPattern::Charge,
                    cooldown: BOSS_CHARGE_COOLDOWN,
                    cooldown_timer: BOSS_CHARGE_COOLDOWN,
                    damage: BOSS_CHARGE_DAMAGE,
                    radius: BOSS_CHARGE_WIDTH, // 冲锋宽度
                },
            ],
        },
    ));
}

// ─── Boss 攻击系统 ───

pub fn boss_attack_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    // 收集 boss 数据
    struct BossData {
        entity: Entity,
        x: f32,
        y: f32,
        patterns: BossAttackPattern,
    }

    let bosses: Vec<BossData> = world
        .query::<(Entity, &Position, &BossAttackPattern, &BossMarker)>()
        .iter(world)
        .map(|(e, pos, pat, _)| BossData {
            entity: e,
            x: pos.x,
            y: pos.y,
            patterns: pat.clone(),
        })
        .collect();

    // 收集存活玩家位置（单人模式无 DeadPlayer 检查）
    let players: Vec<(Entity, f32, f32, u32)> = world
        .query::<(Entity, &Position, &Player)>()
        .iter(world)
        .map(|(e, pos, p)| (e, pos.x, pos.y, p.id))
        .collect();

    if players.is_empty() {
        return;
    }

    for boss in &bosses {
        // 找最近玩家作为目标
        let (_target_e, tx, ty, _target_pid) = players
            .iter()
            .min_by(|a, b| {
                let da = (a.1 - boss.x).powi(2) + (a.2 - boss.y).powi(2);
                let db = (b.1 - boss.x).powi(2) + (b.2 - boss.y).powi(2);
                da.partial_cmp(&db).unwrap_or(std::cmp::Ordering::Equal)
            })
            .copied()
            .unwrap();

        // 递减 cooldown 并执行攻击
        let mut updated_patterns = boss.patterns.clone();
        for attack in &mut updated_patterns.patterns {
            attack.cooldown_timer -= dt;
            if attack.cooldown_timer > 0.0 {
                continue;
            }
            attack.cooldown_timer = attack.cooldown;

            match attack.pattern_type {
                BossPattern::AreaAoe => {
                    let r2 = attack.radius * attack.radius;
                    let mut hit_players: Vec<Entity> = Vec::new();
                    for &(pe, px, py, _pid) in &players {
                        let dx = px - boss.x;
                        let dy = py - boss.y;
                        if dx * dx + dy * dy < r2 {
                            hit_players.push(pe);
                        }
                    }
                    let dmg = attack.damage;
                    for pe in hit_players {
                        let remaining_hp = apply_damage(world, pe, dmg);
                        world
                            .resource_mut::<GameEventQueue>()
                            .push(GameEvent::EntityDamaged {
                                entity: pe,
                                damage: dmg,
                                remaining_hp,
                            });
                        if remaining_hp <= 0.0 {
                            world
                                .resource_mut::<GameEventQueue>()
                                .push(GameEvent::EntityKilled {
                                    entity: pe,
                                    killer: Some(boss.entity),
                                });
                        }
                    }
                }
                BossPattern::Barrage => {
                    // 朝目标发射 5 个弹射体，扇形扩散
                    let dx = tx - boss.x;
                    let dy = ty - boss.y;
                    let base_angle = dy.atan2(dx);
                    let spread = attack.radius; // 半角
                    let count = BOSS_BARRAGE_COUNT as usize;
                    let proj_speed = BOSS_BARRAGE_SPEED;

                    for i in 0..count {
                        let t = if count > 1 {
                            i as f32 / (count - 1) as f32
                        } else {
                            0.5
                        };
                        let angle = base_angle - spread + 2.0 * spread * t;
                        let vx = angle.cos() * proj_speed;
                        let vy = angle.sin() * proj_speed;

                        let proj_entity = world
                            .spawn((
                                Position {
                                    x: boss.x,
                                    y: boss.y,
                                },
                                Velocity { x: vx, y: vy },
                                ProjectileComp {
                                    damage: attack.damage,
                                    lifetime: BOSS_BARRAGE_LIFETIME,
                                    remaining_lifetime: BOSS_BARRAGE_LIFETIME,
                                    size: BOSS_BARRAGE_SIZE,
                                    owner_id: u32::MAX, // Boss 弹射体，非玩家
                                    pierce: 0,
                                    hits_remaining: 1,
                                    crit_chance: 0.0,
                                    crit_multiplier: 1.0,
                                    knockback: 0.0,
                                    damage_mods: Vec::new(),
                                    projectile_mods: Vec::new(),
                                    field_mods: Vec::new(),
                                    applied_effects: Vec::new(),
                                    trajectory_mods: Vec::new(),
                                    hit_entities: Vec::new(),
                                    on_hit: None,
                                    on_kill: None,
                                    on_expire: None,
                                    on_crit: None,
                                    on_boss_hit: None,
                                    on_periodic: None,
                                },
                                CollisionRadius(PROJECTILE_HIT_RADIUS),
                            ))
                            .id();

                        world
                            .resource_mut::<GameEventQueue>()
                            .push(GameEvent::ProjectileSpawned {
                                entity: proj_entity,
                                x: boss.x,
                                y: boss.y,
                                owner_id: u32::MAX,
                            });
                    }
                }
                BossPattern::Charge => {
                    // Boss 向目标冲锋：设置 Velocity 方向
                    let dx = tx - boss.x;
                    let dy = ty - boss.y;
                    let dist = (dx * dx + dy * dy).sqrt().max(1.0);
                    let charge_speed = BOSS_CHARGE_SPEED;
                    if let Some(mut vel) = world.get_mut::<Velocity>(boss.entity) {
                        vel.x = dx / dist * charge_speed;
                        vel.y = dy / dist * charge_speed;
                    }
                }
            }
        }

        // 写回更新后的 pattern（cooldown_timer 变化）
        if let Some(mut pat) = world.get_mut::<BossAttackPattern>(boss.entity) {
            *pat = updated_patterns;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;
    use crate::resources::WaveManager;
    use crate::systems::DeltaTime;

    /// 构建最小可用 World：插入 DeltaTime、WaveManager、GameEventQueue 三个 Resource
    fn setup_world(dt: f32) -> World {
        let mut world = World::new();
        world.insert_resource(DeltaTime(dt));
        world.insert_resource(WaveManager::default());
        world.insert_resource(GameEventQueue::default());
        world
    }

    /// 插入一个带 Player + Position + Level 的实体（wave_system 需要查询玩家位置和等级）
    fn spawn_player(world: &mut World) -> Entity {
        world
            .spawn((
                Player {
                    id: 1,
                    name: "tester".into(),
                },
                Position { x: 0.0, y: 0.0 },
                Level {
                    value: 1,
                    xp: 0,
                    xp_to_next: 100,
                },
            ))
            .id()
    }

    #[test]
    fn pseudo_random_in_range() {
        for seed in 0..100u32 {
            let v = pseudo_random(seed);
            assert!(
                (0.0..=1.0).contains(&v),
                "seed={seed} 产出 {v} 超出 [0,1] 范围"
            );
        }
    }

    #[test]
    fn wave_timer_decrements() {
        let mut world = setup_world(1.0);
        spawn_player(&mut world);

        // 首波 timer=0 会立即触发并重置为 WAVE_INTERVAL，再调一次验证递减
        wave_system(&mut world); // 触发首波，timer 重置为 WAVE_INTERVAL
        wave_system(&mut world); // 递减 1.0

        let wm = world.resource::<WaveManager>();
        let expected = WAVE_INTERVAL - 1.0;
        assert!(
            (wm.wave_timer - expected).abs() < f32::EPSILON,
            "wave_timer 应为 {expected}，实际 {}",
            wm.wave_timer
        );
    }

    #[test]
    fn wave_spawns_monsters_on_timer_expire() {
        let mut world = setup_world(1.0);
        spawn_player(&mut world);

        // 设置 wave_timer 即将到期
        world.resource_mut::<WaveManager>().wave_timer = 0.5;

        wave_system(&mut world);

        let monster_count = world.query::<&MonsterComp>().iter(&world).count();
        assert!(
            monster_count >= 1,
            "wave_timer 到期后应生成怪物，实际生成 {monster_count}"
        );
    }

    #[test]
    fn wave_spawn_count_scales_with_wave_number() {
        let mut world = setup_world(crate::data::constants::TICK_DT);
        spawn_player(&mut world);

        {
            let mut wm = world.resource_mut::<WaveManager>();
            wm.wave_number = 4; // 触发后 wave_number 变成 5
            wm.wave_timer = 0.01; // 立即到期
        }

        wave_system(&mut world);

        // count = (8.0 * (1.0 + 5 * 0.15)) as u32 = (8.0 * 1.75) as u32 = 14
        let monster_count = world.query::<&MonsterComp>().iter(&world).count();
        assert_eq!(
            monster_count, 14,
            "wave_number=5 时应生成 14 个怪物，实际 {monster_count}"
        );
    }

    #[test]
    fn boss_spawns_on_timer_expire() {
        let mut world = setup_world(crate::data::constants::TICK_DT);
        spawn_player(&mut world);

        world.resource_mut::<WaveManager>().boss_timer = 0.01;

        boss_spawn_system(&mut world);

        let boss_count = world.query::<&BossMarker>().iter(&world).count();
        assert_eq!(
            boss_count, 1,
            "boss_timer 到期后应生成 1 个 Boss，实际 {boss_count}"
        );
    }

    #[test]
    fn boss_not_spawned_before_timer() {
        let mut world = setup_world(crate::data::constants::TICK_DT);
        spawn_player(&mut world);

        // boss_timer 默认 300.0，远未到期
        boss_spawn_system(&mut world);

        let boss_count = world.query::<&BossMarker>().iter(&world).count();
        assert_eq!(
            boss_count, 0,
            "boss_timer 未到期时不应生成 Boss，实际 {boss_count}"
        );
    }
}
