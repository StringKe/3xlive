use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::constants::{
    MONSTER_ATTACK_COOLDOWN, PLAYER_CONTACT_IFRAMES, PROJECTILE_HIT_RADIUS, TICK_DT,
};
use crate::data::{DamageModType, ProjectileModType, StatusEffectType};
use crate::disc_engine::{DamageModInstance, ProjectileModInstance};
use crate::events::{GameEvent, GameEventQueue};

/// 简单伪随机数生成器（基于位混合 hash），WASM 兼容
/// 输入一个 seed，返回 [0.0, 1.0) 的伪随机浮点数
fn pseudo_random(seed: u64) -> f32 {
    // splitmix64 风格的位混合
    let mut z = seed.wrapping_add(0x9e3779b97f4a7c15);
    z = (z ^ (z >> 30)).wrapping_mul(0xbf58476d1ce4e5b9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94d049bb133111eb);
    z = z ^ (z >> 31);
    // 取高 23 位映射到 [0.0, 1.0)
    (z >> 41) as f32 / (1u64 << 23) as f32
}

/// 全局递增 seed，用于暴击等随机判定
static mut RANDOM_COUNTER: u64 = 0;

fn next_random() -> f32 {
    // WASM 单线程环境，安全使用 static mut
    unsafe {
        RANDOM_COUNTER = RANDOM_COUNTER.wrapping_add(1);
        pseudo_random(RANDOM_COUNTER)
    }
}

/// 设置随机种子（测试用）
#[cfg(test)]
pub fn set_random_seed(seed: u64) {
    unsafe {
        RANDOM_COUNTER = seed;
    }
}

/// 对目标实体施加伤害，返回剩余 HP
pub fn apply_damage(world: &mut World, target: Entity, damage: f32) -> f32 {
    if let Some(mut hp) = world.get_mut::<Health>(target) {
        hp.current = (hp.current - damage).max(0.0);
        hp.current
    } else {
        0.0
    }
}

/// 碰撞检测阶段收集的命中信息
struct HitInfo {
    proj_entity: Entity,
    mon_entity: Entity,
    owner_id: u32,
    /// 原始伤害（来自 ProjectileComp.damage）
    base_damage: f32,
    /// 弹射体位置
    proj_x: f32,
    proj_y: f32,
    /// 怪物位置
    mon_x: f32,
    mon_y: f32,
    /// 暴击率
    crit_chance: f32,
    /// 暴击倍率
    crit_multiplier: f32,
    /// 击退力度
    knockback: f32,
    /// 伤害修饰列表
    damage_mods: Vec<DamageModInstance>,
    /// 弹射体修饰列表
    projectile_mods: Vec<ProjectileModInstance>,
    /// 命中时施加的状态效果
    applied_effects: Vec<(StatusEffectType, f32, f32)>,
    /// 已命中次数（用于 Ramp 计算：pierce+1 - hits_remaining，在命中前的值）
    hit_count: u8,
    /// 目标是否为 Elite
    is_elite: bool,
    /// 目标是否为 Boss
    is_boss: bool,
}

/// 待 spawn 的子弹信息（Chain/Split/OrbitHit 产生）
struct PendingProjectile {
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    damage: f32,
    lifetime: f32,
    size: f32,
    owner_id: u32,
    applied_effects: Vec<(StatusEffectType, f32, f32)>,
    damage_mods: Vec<DamageModInstance>,
    /// 继承的已命中实体列表（Chain 需要排除已命中目标）
    hit_entities: Vec<Entity>,
}

/// 弹射体 vs 怪物碰撞
pub fn projectile_monster_collision(world: &mut World) {
    // ─── 阶段一：收集数据 ───

    // 收集弹射体完整数据
    struct ProjData {
        entity: Entity,
        x: f32,
        y: f32,
        radius: f32,
        damage: f32,
        owner_id: u32,
        hits_remaining: u8,
        pierce: u8,
        crit_chance: f32,
        crit_multiplier: f32,
        knockback: f32,
        hit_entities: Vec<Entity>,
        damage_mods: Vec<DamageModInstance>,
        projectile_mods: Vec<ProjectileModInstance>,
        applied_effects: Vec<(StatusEffectType, f32, f32)>,
    }

    let projs: Vec<ProjData> = world
        .query::<(Entity, &Position, &ProjectileComp, Option<&CollisionRadius>)>()
        .iter(world)
        .filter(|(_, _, proj, _)| proj.remaining_lifetime > 0.0)
        .map(|(e, pos, proj, cr)| ProjData {
            entity: e,
            x: pos.x,
            y: pos.y,
            radius: cr.map_or(12.0, |r| r.0),
            damage: proj.damage,
            owner_id: proj.owner_id,
            hits_remaining: proj.hits_remaining,
            pierce: proj.pierce,
            crit_chance: proj.crit_chance,
            crit_multiplier: proj.crit_multiplier,
            knockback: proj.knockback,
            hit_entities: proj.hit_entities.clone(),
            damage_mods: proj.damage_mods.clone(),
            projectile_mods: proj.projectile_mods.clone(),
            applied_effects: proj.applied_effects.clone(),
        })
        .collect();

    struct MonData {
        entity: Entity,
        x: f32,
        y: f32,
        radius: f32,
        is_elite: bool,
        is_boss: bool,
    }

    let monsters: Vec<MonData> = world
        .query::<(
            Entity,
            &Position,
            &MonsterComp,
            Option<&CollisionRadius>,
            Option<&EliteMarker>,
            Option<&BossMarker>,
        )>()
        .iter(world)
        .map(|(e, pos, _, cr, elite, boss)| MonData {
            entity: e,
            x: pos.x,
            y: pos.y,
            radius: cr.map_or(24.0, |r| r.0),
            is_elite: elite.is_some(),
            is_boss: boss.is_some(),
        })
        .collect();

    // ─── 阶段二：碰撞检测 + pierce 逻辑 ───

    let mut hits: Vec<HitInfo> = Vec::new();
    // 记录每个弹射体在此 tick 中还剩多少次命中机会
    // key = proj entity index in projs, value = remaining hits
    let mut proj_hits_remaining: Vec<u8> = projs.iter().map(|p| p.hits_remaining).collect();
    // 记录此 tick 中每个弹射体新命中的实体
    let mut proj_new_hits: Vec<Vec<Entity>> = projs.iter().map(|_| Vec::new()).collect();

    for (pi, proj) in projs.iter().enumerate() {
        if proj_hits_remaining[pi] == 0 {
            continue;
        }
        for mon in &monsters {
            // 5.1 跳过已命中过的怪物（pierce 已命中列表）
            if proj.hit_entities.contains(&mon.entity) {
                continue;
            }
            // 跳过此 tick 中已命中的（同一 tick 多次命中不同怪物）
            if proj_new_hits[pi].contains(&mon.entity) {
                continue;
            }

            let dx = proj.x - mon.x;
            let dy = proj.y - mon.y;
            let combined = proj.radius + mon.radius;
            if dx * dx + dy * dy < combined * combined {
                // 计算已命中次数（用于 Ramp）
                let total_hits = (proj.pierce + 1) - proj_hits_remaining[pi];

                hits.push(HitInfo {
                    proj_entity: proj.entity,
                    mon_entity: mon.entity,
                    owner_id: proj.owner_id,
                    base_damage: proj.damage,
                    proj_x: proj.x,
                    proj_y: proj.y,
                    mon_x: mon.x,
                    mon_y: mon.y,
                    crit_chance: proj.crit_chance,
                    crit_multiplier: proj.crit_multiplier,
                    knockback: proj.knockback,
                    damage_mods: proj.damage_mods.clone(),
                    projectile_mods: proj.projectile_mods.clone(),
                    applied_effects: proj.applied_effects.clone(),
                    hit_count: total_hits,
                    is_elite: mon.is_elite,
                    is_boss: mon.is_boss,
                });

                proj_new_hits[pi].push(mon.entity);
                proj_hits_remaining[pi] -= 1;

                if proj_hits_remaining[pi] == 0 {
                    break; // 命中次数用完，停止检测此弹射体
                }
            }
        }
    }

    // 更新弹射体的 hits_remaining 和 hit_entities，以及是否销毁
    for (pi, proj) in projs.iter().enumerate() {
        if proj_new_hits[pi].is_empty() {
            continue;
        }
        if let Some(mut proj_comp) = world.get_mut::<ProjectileComp>(proj.entity) {
            for &hit_e in &proj_new_hits[pi] {
                proj_comp.hit_entities.push(hit_e);
            }
            proj_comp.hits_remaining = proj_hits_remaining[pi];
            // 5.1 如果 hits_remaining == 0，标记销毁
            if proj_hits_remaining[pi] == 0 {
                proj_comp.remaining_lifetime = 0.0;
            }
        }
    }

    // ─── 阶段三：伤害计算 + 效果应用 ───

    let mut pending_spawns: Vec<PendingProjectile> = Vec::new();

    for hit in &hits {
        let mut damage = hit.base_damage;
        let mut crit_chance = hit.crit_chance;
        let mut crit_multiplier = hit.crit_multiplier;
        let mut lifesteal_ratio = 0.0_f32;
        let mut execute_threshold = 0.0_f32;
        let mut glass_cannon_value = 0.0_f32;

        // ─── 5.4 DamageMod 应用（分两阶段）───

        // 第一阶段：先应用 CritBoost/CritDamage（影响暴击判定参数）
        for dm in &hit.damage_mods {
            match dm.mod_type {
                DamageModType::CritBoost => {
                    crit_chance += dm.value;
                }
                DamageModType::CritDamage => {
                    crit_multiplier += dm.value;
                }
                _ => {}
            }
        }

        // ─── 5.2 暴击判定 ───
        let is_crit = next_random() < crit_chance;
        if is_crit {
            damage *= crit_multiplier;
        }

        // 第二阶段：应用其他 DamageMod
        for dm in &hit.damage_mods {
            match dm.mod_type {
                DamageModType::FlatDamage => {
                    damage += dm.value;
                }
                DamageModType::DamagePercent => {
                    damage *= 1.0 + dm.value;
                }
                DamageModType::ArmorPen => {
                    // 暂时标记，后续使用
                }
                DamageModType::Lifesteal => {
                    lifesteal_ratio += dm.value;
                }
                DamageModType::Execute => {
                    execute_threshold = dm.value;
                }
                DamageModType::BonusVsElite => {
                    if hit.is_elite {
                        damage *= 1.0 + dm.value;
                    }
                }
                DamageModType::BonusVsBoss => {
                    if hit.is_boss {
                        damage *= 1.0 + dm.value;
                    }
                }
                DamageModType::Overkill => {
                    // 暂存溢出伤害，后续扩展
                }
                DamageModType::Ramp => {
                    // 根据已命中次数递增伤害
                    damage *= 1.0 + hit.hit_count as f32 * dm.value;
                }
                DamageModType::GlassCannon => {
                    damage *= 1.0 + dm.value;
                    glass_cannon_value += dm.value;
                }
                // CritBoost/CritDamage 已在第一阶段处理
                DamageModType::CritBoost | DamageModType::CritDamage => {}
            }
        }

        // ─── 5.7 Weaken/Mark 增伤 ───
        if let Some(effects) = world.get::<StatusEffects>(hit.mon_entity) {
            for eff in &effects.effects {
                match eff.effect_type {
                    StatusEffectType::Weaken => {
                        damage *= 1.0 + eff.magnitude;
                    }
                    StatusEffectType::Mark => {
                        damage *= 1.0 + eff.magnitude;
                    }
                    _ => {}
                }
            }
        }

        // ─── 5.4 Execute 斩杀判定 ───
        if execute_threshold > 0.0 {
            if let Some(hp) = world.get::<Health>(hit.mon_entity) {
                let hp_percent = hp.current / hp.max;
                if hp_percent < execute_threshold {
                    // 直接击杀：设置伤害为当前 HP
                    damage = hp.current;
                }
            }
        }

        // ─── 施加伤害 ───
        let remaining_hp = apply_damage(world, hit.mon_entity, damage);

        // 推送事件
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::EntityDamaged {
                entity: hit.mon_entity,
                damage,
                remaining_hp,
            });
        if remaining_hp <= 0.0 {
            world
                .resource_mut::<GameEventQueue>()
                .push(GameEvent::EntityKilled {
                    entity: hit.mon_entity,
                    killer: Some(hit.proj_entity),
                });
        }

        // ─── 5.4 Lifesteal 回血 ───
        if lifesteal_ratio > 0.0 {
            let heal_amount = damage * lifesteal_ratio;
            // 查找 owner 玩家实体
            for (_, player, mut hp) in world
                .query::<(Entity, &Player, &mut Health)>()
                .iter_mut(world)
            {
                if player.id == hit.owner_id {
                    hp.current = (hp.current + heal_amount).min(hp.max);
                    break;
                }
            }
        }

        // ─── 5.4 GlassCannon 自伤 ───
        if glass_cannon_value > 0.0 {
            for (_, player, mut hp) in world
                .query::<(Entity, &Player, &mut Health)>()
                .iter_mut(world)
            {
                if player.id == hit.owner_id {
                    let self_damage = hp.max * glass_cannon_value;
                    hp.current = (hp.current - self_damage).max(1.0); // 不会自杀，最低 1 HP
                    break;
                }
            }
        }

        // ─── 5.3 击退 ───
        if hit.knockback > 0.0 {
            // 计算击退方向：弹射体到怪物的方向向量
            let kb_dx = hit.mon_x - hit.proj_x;
            let kb_dy = hit.mon_y - hit.proj_y;
            let kb_dist = (kb_dx * kb_dx + kb_dy * kb_dy).sqrt();
            if kb_dist > 0.001 {
                let nx = kb_dx / kb_dist;
                let ny = kb_dy / kb_dist;
                if let Some(mut pos) = world.get_mut::<Position>(hit.mon_entity) {
                    pos.x += nx * hit.knockback;
                    pos.y += ny * hit.knockback;
                }
            }
        }

        // ─── 5.6 StatusEffect 施加 ───
        if !hit.applied_effects.is_empty() {
            if let Some(mut effects) = world.get_mut::<StatusEffects>(hit.mon_entity) {
                for &(effect_type, duration, magnitude) in &hit.applied_effects {
                    // 刷新同类型效果的持续时间，或添加新效果
                    if let Some(existing) = effects
                        .effects
                        .iter_mut()
                        .find(|e| e.effect_type == effect_type)
                    {
                        existing.remaining = existing.remaining.max(duration);
                        if magnitude > existing.magnitude {
                            existing.magnitude = magnitude;
                        }
                    } else {
                        effects.effects.push(ActiveEffect {
                            effect_type,
                            remaining: duration,
                            magnitude,
                            anchor: None,
                            dot_timer: 0.0,
                        });
                    }
                }
            }
        }

        // ─── 仇恨系统：被攻击后立刻追击攻击者 ───
        if let Some(mut ai) = world.get_mut::<MonsterAI>(hit.mon_entity) {
            ai.state = AIState::Chasing;
            ai.target = Some(hit.owner_id);
        }

        // ─── 5.5 ProjectileMod 命中时处理 ───
        for pm in &hit.projectile_mods {
            match pm.mod_type {
                ProjectileModType::Explode => {
                    // AoE 伤害：对 radius 范围内所有怪物造成 value * damage 伤害
                    let aoe_radius = pm.value;
                    let aoe_damage = damage * pm.value;
                    let cx = hit.mon_x;
                    let cy = hit.mon_y;

                    // 收集范围内的怪物
                    let aoe_targets: Vec<Entity> = world
                        .query::<(Entity, &Position, &MonsterComp)>()
                        .iter(world)
                        .filter(|(e, pos, _)| {
                            if *e == hit.mon_entity {
                                return false; // 已经受过直接命中伤害
                            }
                            let dx = pos.x - cx;
                            let dy = pos.y - cy;
                            dx * dx + dy * dy < aoe_radius * aoe_radius
                        })
                        .map(|(e, _, _)| e)
                        .collect();

                    for target in aoe_targets {
                        let rem = apply_damage(world, target, aoe_damage);
                        world
                            .resource_mut::<GameEventQueue>()
                            .push(GameEvent::EntityDamaged {
                                entity: target,
                                damage: aoe_damage,
                                remaining_hp: rem,
                            });
                        if rem <= 0.0 {
                            world
                                .resource_mut::<GameEventQueue>()
                                .push(GameEvent::EntityKilled {
                                    entity: target,
                                    killer: Some(hit.proj_entity),
                                });
                        }
                    }
                }
                ProjectileModType::Chain => {
                    // 跳跃子弹：找最近的未命中目标
                    let mut all_hit: Vec<Entity> = Vec::new();
                    if let Some(proj_comp) = world.get::<ProjectileComp>(hit.proj_entity) {
                        all_hit = proj_comp.hit_entities.clone();
                    }
                    all_hit.push(hit.mon_entity);

                    let mut best: Option<(Entity, f32, f32, f32)> = None;
                    for (e, pos, _) in world
                        .query::<(Entity, &Position, &MonsterComp)>()
                        .iter(world)
                    {
                        if all_hit.contains(&e) {
                            continue;
                        }
                        let dx = pos.x - hit.mon_x;
                        let dy = pos.y - hit.mon_y;
                        let dist_sq = dx * dx + dy * dy;
                        if best.is_none() || dist_sq < best.unwrap().3 {
                            best = Some((e, pos.x, pos.y, dist_sq));
                        }
                    }

                    if let Some((_, tx, ty, _)) = best {
                        let dx = tx - hit.mon_x;
                        let dy = ty - hit.mon_y;
                        let dist = (dx * dx + dy * dy).sqrt();
                        let speed = 500.0; // 链式子弹速度
                        let (vx, vy) = if dist > 0.001 {
                            (dx / dist * speed, dy / dist * speed)
                        } else {
                            (speed, 0.0)
                        };

                        pending_spawns.push(PendingProjectile {
                            x: hit.mon_x,
                            y: hit.mon_y,
                            vx,
                            vy,
                            damage: damage * 0.5,
                            lifetime: 2.0,
                            size: 4.0,
                            owner_id: hit.owner_id,
                            applied_effects: hit.applied_effects.clone(),
                            damage_mods: hit.damage_mods.clone(),
                            hit_entities: all_hit,
                        });
                    }
                }
                ProjectileModType::Split => {
                    // 分裂子弹：count 个方向
                    let count = pm.count.max(2);
                    let speed = 400.0;
                    let angle_step = std::f32::consts::TAU / count as f32;
                    for i in 0..count {
                        let angle = angle_step * i as f32;
                        let vx = angle.cos() * speed;
                        let vy = angle.sin() * speed;
                        pending_spawns.push(PendingProjectile {
                            x: hit.mon_x,
                            y: hit.mon_y,
                            vx,
                            vy,
                            damage: damage * 0.5,
                            lifetime: 1.5,
                            size: 3.0,
                            owner_id: hit.owner_id,
                            applied_effects: hit.applied_effects.clone(),
                            damage_mods: hit.damage_mods.clone(),
                            hit_entities: vec![hit.mon_entity],
                        });
                    }
                }
                ProjectileModType::Attach => {
                    // 在目标身上施加 DoT（等同于 Poison 效果）
                    let dot_duration = pm.value;
                    let dot_magnitude = damage * 0.2;
                    if let Some(mut effects) = world.get_mut::<StatusEffects>(hit.mon_entity) {
                        if let Some(existing) = effects
                            .effects
                            .iter_mut()
                            .find(|e| e.effect_type == StatusEffectType::Poison)
                        {
                            existing.remaining = existing.remaining.max(dot_duration);
                            if dot_magnitude > existing.magnitude {
                                existing.magnitude = dot_magnitude;
                            }
                        } else {
                            effects.effects.push(ActiveEffect {
                                effect_type: StatusEffectType::Poison,
                                remaining: dot_duration,
                                magnitude: dot_magnitude,
                                anchor: None,
                                dot_timer: 0.0,
                            });
                        }
                    }
                }
                ProjectileModType::OrbitHit => {
                    // 环绕目标的子弹
                    let speed = 300.0;
                    // 初始方向：垂直于命中方向
                    let dx = hit.mon_x - hit.proj_x;
                    let dy = hit.mon_y - hit.proj_y;
                    let dist = (dx * dx + dy * dy).sqrt();
                    let (vx, vy) = if dist > 0.001 {
                        (-dy / dist * speed, dx / dist * speed)
                    } else {
                        (speed, 0.0)
                    };

                    pending_spawns.push(PendingProjectile {
                        x: hit.mon_x,
                        y: hit.mon_y,
                        vx,
                        vy,
                        damage: damage * 0.3,
                        lifetime: 3.0,
                        size: 3.0,
                        owner_id: hit.owner_id,
                        applied_effects: hit.applied_effects.clone(),
                        damage_mods: Vec::new(),
                        hit_entities: Vec::new(),
                    });
                }
                // 其余 ProjectileModType 在此 Phase 不处理
                ProjectileModType::Return
                | ProjectileModType::GrowSize
                | ProjectileModType::ShrinkSize
                | ProjectileModType::Fragment
                | ProjectileModType::Bounce
                | ProjectileModType::Ghost
                | ProjectileModType::Clone => {}
            }
        }
    }

    // ─── 阶段四：统一 spawn 子弹 ───
    for ps in pending_spawns {
        world.spawn((
            Position { x: ps.x, y: ps.y },
            Velocity { x: ps.vx, y: ps.vy },
            ProjectileComp {
                damage: ps.damage,
                lifetime: ps.lifetime,
                remaining_lifetime: ps.lifetime,
                size: ps.size,
                owner_id: ps.owner_id,
                pierce: 0,
                hits_remaining: 1,
                crit_chance: 0.0,
                crit_multiplier: 1.0,
                knockback: 0.0,
                damage_mods: ps.damage_mods,
                projectile_mods: Vec::new(), // 子弹不继承 ProjectileMod
                field_mods: Vec::new(),
                applied_effects: ps.applied_effects,
                trajectory_mods: Vec::new(),
                hit_entities: ps.hit_entities,
                on_hit: None,
                on_kill: None,
                on_expire: None,
                on_crit: None,
                on_boss_hit: None,
                on_periodic: None,
            },
            CollisionRadius(PROJECTILE_HIT_RADIUS),
        ));
    }
}

/// 怪物 vs 玩家接触伤害（单人模式，无 DeadPlayer 检查）
/// 怪物有攻击冷却 + 玩家受击无敌帧双重限制
pub fn monster_player_collision(world: &mut World) {
    // 先 tick 所有怪物的攻击冷却计时器
    let mut cooldown_query = world.query::<&mut ContactAttackTimer>();
    for mut timer in cooldown_query.iter_mut(world) {
        timer.remaining = (timer.remaining - TICK_DT).max(0.0);
    }

    // tick 玩家无敌帧计时器
    let mut iframe_query = world.query::<&mut ContactIframes>();
    for mut iframes in iframe_query.iter_mut(world) {
        iframes.remaining = (iframes.remaining - TICK_DT).max(0.0);
    }

    let monsters: Vec<(Entity, f32, f32, f32, f32)> = world
        .query::<(Entity, &Position, &MonsterComp, Option<&CollisionRadius>)>()
        .iter(world)
        .map(|(e, pos, mon, cr)| (e, pos.x, pos.y, mon.damage, cr.map_or(24.0, |r| r.0)))
        .collect();

    let players: Vec<(Entity, f32, f32, f32, bool)> = world
        .query::<(
            Entity,
            &Position,
            &Player,
            Option<&CollisionRadius>,
            Option<&ContactIframes>,
        )>()
        .iter(world)
        .map(|(e, pos, _, cr, iframes)| {
            let in_iframes = iframes.is_some_and(|i| i.remaining > 0.0);
            (e, pos.x, pos.y, cr.map_or(16.0, |r| r.0), in_iframes)
        })
        .collect();

    let mut hits: Vec<(Entity, Entity, f32)> = Vec::new();

    for &(player_e, px, py, player_r, in_iframes) in &players {
        // 玩家在无敌帧内，跳过所有接触伤害
        if in_iframes {
            continue;
        }
        for &(mon_e, mx, my, mdmg, mon_r) in &monsters {
            let dx = px - mx;
            let dy = py - my;
            let combined = mon_r + player_r;
            if dx * dx + dy * dy < combined * combined {
                // 检查怪物攻击冷却
                if let Some(timer) = world.get::<ContactAttackTimer>(mon_e) {
                    if timer.remaining > 0.0 {
                        continue;
                    }
                }
                hits.push((player_e, mon_e, mdmg));
                break; // 每 tick 每个玩家只受一次接触伤害
            }
        }
    }

    for (player_e, mon_e, mdmg) in hits {
        let remaining_hp = apply_damage(world, player_e, mdmg);
        world
            .resource_mut::<GameEventQueue>()
            .push(GameEvent::EntityDamaged {
                entity: player_e,
                damage: mdmg,
                remaining_hp,
            });
        if remaining_hp <= 0.0 {
            world
                .resource_mut::<GameEventQueue>()
                .push(GameEvent::EntityKilled {
                    entity: player_e,
                    killer: None,
                });
        }
        // 重置怪物攻击冷却
        if let Some(mut timer) = world.get_mut::<ContactAttackTimer>(mon_e) {
            timer.remaining = MONSTER_ATTACK_COOLDOWN;
        }
        // 激活玩家无敌帧
        if let Some(mut iframes) = world.get_mut::<ContactIframes>(player_e) {
            iframes.remaining = PLAYER_CONTACT_IFRAMES;
        }
    }
}

/// 怪物互碰分离（纯物理，无事件）
pub fn monster_monster_collision(world: &mut World) {
    let monsters: Vec<(Entity, f32, f32, f32)> = world
        .query::<(Entity, &Position, &MonsterComp, Option<&CollisionRadius>)>()
        .iter(world)
        .map(|(e, pos, _, cr)| (e, pos.x, pos.y, cr.map_or(24.0, |r| r.0)))
        .collect();

    let mut displacements: Vec<(Entity, f32, f32)> = Vec::new();

    for (i, &(e1, x1, y1, r1)) in monsters.iter().enumerate() {
        for &(e2, x2, y2, r2) in &monsters[i + 1..] {
            let dx = x1 - x2;
            let dy = y1 - y2;
            let dist_sq = dx * dx + dy * dy;
            let combined = r1 + r2;

            if dist_sq < combined * combined && dist_sq > 0.001 {
                let dist = dist_sq.sqrt();
                let overlap = combined - dist;
                let nx = dx / dist;
                let ny = dy / dist;
                let half = overlap * 0.5;

                displacements.push((e1, nx * half, ny * half));
                displacements.push((e2, -nx * half, -ny * half));
            }
        }
    }

    for (entity, dx, dy) in displacements {
        if let Some(mut pos) = world.get_mut::<Position>(entity) {
            pos.x += dx;
            pos.y += dy;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;

    /// 创建默认 ProjectileComp 的辅助函数
    fn default_projectile(damage: f32, owner_id: u32) -> ProjectileComp {
        ProjectileComp {
            damage,
            lifetime: 2.0,
            remaining_lifetime: 2.0,
            size: 4.0,
            owner_id,
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
        }
    }

    /// 创建默认怪物组件的辅助函数
    fn spawn_monster(world: &mut World, x: f32, y: f32, hp: f32) -> Entity {
        world
            .spawn((
                Position { x, y },
                Health {
                    current: hp,
                    max: hp,
                },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
                MonsterAI {
                    state: AIState::Wandering,
                    target: None,
                    wander_target_x: 0.0,
                    wander_target_y: 0.0,
                    detection_radius: 200.0,
                },
                StatusEffects::default(),
            ))
            .id()
    }

    /// 50HP 实体受 10 伤害后剩余 40HP
    #[test]
    fn apply_damage_reduces_health() {
        let mut world = World::new();
        let target = world
            .spawn(Health {
                current: 50.0,
                max: 50.0,
            })
            .id();

        let remaining = apply_damage(&mut world, target, 10.0);
        assert_eq!(remaining, 40.0);
        assert_eq!(world.get::<Health>(target).unwrap().current, 40.0);
    }

    /// 5HP 实体受 100 伤害后 clamp 到 0
    #[test]
    fn apply_damage_clamps_to_zero() {
        let mut world = World::new();
        let target = world
            .spawn(Health {
                current: 5.0,
                max: 100.0,
            })
            .id();

        let remaining = apply_damage(&mut world, target, 100.0);
        assert_eq!(remaining, 0.0);
        assert_eq!(world.get::<Health>(target).unwrap().current, 0.0);
    }

    /// 没有 Health 组件的实体返回 0.0
    #[test]
    fn apply_damage_missing_health() {
        let mut world = World::new();
        let target = world.spawn(Position { x: 0.0, y: 0.0 }).id();

        let remaining = apply_damage(&mut world, target, 10.0);
        assert_eq!(remaining, 0.0);
    }

    /// 弹射体命中同位置怪物：remaining_lifetime 归零、怪物 HP 减少、产生 EntityDamaged 事件
    #[test]
    fn projectile_hits_monster() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let proj = world
            .spawn((Position { x: 100.0, y: 100.0 }, default_projectile(25.0, 1)))
            .id();

        let monster = world
            .spawn((
                Position { x: 100.0, y: 100.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
                MonsterAI {
                    state: AIState::Wandering,
                    target: None,
                    wander_target_x: 0.0,
                    wander_target_y: 0.0,
                    detection_radius: 200.0,
                },
            ))
            .id();

        projectile_monster_collision(&mut world);

        // 弹射体 remaining_lifetime 归零
        let proj_comp = world.get::<ProjectileComp>(proj).unwrap();
        assert_eq!(proj_comp.remaining_lifetime, 0.0);

        // 怪物 HP 减少 25
        let hp = world.get::<Health>(monster).unwrap();
        assert_eq!(hp.current, 75.0);

        // 怪物 AI 切换为追击状态
        let ai = world.get::<MonsterAI>(monster).unwrap();
        assert_eq!(ai.state, AIState::Chasing);
        assert_eq!(ai.target, Some(1));

        // 产生 EntityDamaged 事件
        let events = world.resource_mut::<GameEventQueue>().drain();
        assert!(!events.is_empty());
        match &events[0] {
            GameEvent::EntityDamaged {
                entity,
                damage,
                remaining_hp,
            } => {
                assert_eq!(*entity, monster);
                assert_eq!(*damage, 25.0);
                assert_eq!(*remaining_hp, 75.0);
            }
            other => panic!("期望 EntityDamaged，实际 {:?}", other),
        }
    }

    /// 弹射体与怪物距离远时不碰撞
    #[test]
    fn projectile_misses_when_far() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        world.spawn((Position { x: 0.0, y: 0.0 }, default_projectile(10.0, 1)));

        world.spawn((
            Position {
                x: 1000.0,
                y: 1000.0,
            },
            Health {
                current: 100.0,
                max: 100.0,
            },
            MonsterComp {
                monster_type: 0,
                damage: 5.0,
            },
        ));

        projectile_monster_collision(&mut world);

        let events = world.resource_mut::<GameEventQueue>().drain();
        assert!(events.is_empty());
    }

    /// 怪物与玩家重叠时，首次接触造成完整伤害，冷却期内不再伤害
    #[test]
    fn monster_damages_player_on_contact() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        world.spawn((
            Position { x: 50.0, y: 50.0 },
            MonsterComp {
                monster_type: 0,
                damage: 20.0,
            },
            ContactAttackTimer { remaining: 0.0 },
        ));

        let player = world
            .spawn((
                Position { x: 50.0, y: 50.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                Player {
                    id: 1,
                    name: "test".to_string(),
                },
            ))
            .id();

        // 首次接触：造成完整 20.0 伤害
        monster_player_collision(&mut world);
        let hp = world.get::<Health>(player).unwrap();
        assert_eq!(hp.current, 80.0, "首次接触应造成完整 20.0 伤害");

        // 冷却期内再次碰撞：不造成伤害
        monster_player_collision(&mut world);
        let hp = world.get::<Health>(player).unwrap();
        assert_eq!(hp.current, 80.0, "冷却期内不应造成伤害");
    }

    /// 两个重叠怪物分离后距离增大
    #[test]
    fn monster_monster_separation() {
        let mut world = World::new();

        let m1 = world
            .spawn((
                Position { x: 100.0, y: 100.0 },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
            ))
            .id();

        let m2 = world
            .spawn((
                Position { x: 105.0, y: 100.0 },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
            ))
            .id();

        let dist_before = {
            let p1 = world.get::<Position>(m1).unwrap();
            let p2 = world.get::<Position>(m2).unwrap();
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            (dx * dx + dy * dy).sqrt()
        };

        monster_monster_collision(&mut world);

        let dist_after = {
            let p1 = world.get::<Position>(m1).unwrap();
            let p2 = world.get::<Position>(m2).unwrap();
            let dx = p1.x - p2.x;
            let dy = p1.y - p2.y;
            (dx * dx + dy * dy).sqrt()
        };

        assert!(
            dist_after > dist_before,
            "分离后距离 ({dist_after}) 应大于分离前 ({dist_before})"
        );
    }

    // ─── 5.8 新增碰撞测试 ───

    /// 5.8.1 pierce 穿透测试：弹射体 pierce=1，验证可以命中 2 个目标后销毁
    #[test]
    fn collision_pierce_hits_two_targets() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        // 创建 pierce=1 的弹射体（hits_remaining = 2）
        let mut proj_comp = default_projectile(20.0, 1);
        proj_comp.pierce = 1;
        proj_comp.hits_remaining = 2;

        let proj = world
            .spawn((Position { x: 100.0, y: 100.0 }, proj_comp))
            .id();

        // 两个怪物在弹射体碰撞范围内
        let m1 = spawn_monster(&mut world, 100.0, 100.0, 100.0);
        let m2 = spawn_monster(&mut world, 110.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 两个怪物都应受到伤害
        let hp1 = world.get::<Health>(m1).unwrap().current;
        let hp2 = world.get::<Health>(m2).unwrap().current;
        assert_eq!(hp1, 80.0, "第一个怪物应受到 20 伤害");
        assert_eq!(hp2, 80.0, "第二个怪物应受到 20 伤害");

        // 弹射体 remaining_lifetime 应归零（2 次命中用完）
        let proj_comp = world.get::<ProjectileComp>(proj).unwrap();
        assert_eq!(proj_comp.remaining_lifetime, 0.0, "弹射体应被销毁");
        assert_eq!(proj_comp.hits_remaining, 0);
        assert_eq!(proj_comp.hit_entities.len(), 2);
    }

    /// pierce=1 弹射体命中第一个目标后继续飞行，hit_entities 正确记录
    #[test]
    fn collision_pierce_continues_after_first_hit() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(15.0, 1);
        proj_comp.pierce = 1;
        proj_comp.hits_remaining = 2;

        let proj = world
            .spawn((Position { x: 100.0, y: 100.0 }, proj_comp))
            .id();

        // 只放一个怪物
        let m1 = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 只命中一个，弹射体应继续飞行
        let proj_comp = world.get::<ProjectileComp>(proj).unwrap();
        assert!(
            proj_comp.remaining_lifetime > 0.0,
            "还有剩余命中次数，弹射体应继续飞行"
        );
        assert_eq!(proj_comp.hits_remaining, 1);
        assert!(proj_comp.hit_entities.contains(&m1));

        let hp = world.get::<Health>(m1).unwrap().current;
        assert_eq!(hp, 85.0);
    }

    /// 5.8.2 crit 暴击测试：crit_chance=1.0（必暴击），验证 damage *= crit_multiplier
    #[test]
    fn collision_crit_multiplies_damage() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.crit_chance = 1.0; // 必暴击
        proj_comp.crit_multiplier = 2.5;

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 10.0 * 2.5 = 25.0 暴击伤害
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 75.0, "暴击应造成 10 * 2.5 = 25 伤害");
    }

    /// crit_chance=0.0 不暴击
    #[test]
    fn collision_no_crit_when_zero_chance() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.crit_chance = 0.0;
        proj_comp.crit_multiplier = 3.0;

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 无暴击，伤害应为 10.0
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 90.0, "crit_chance=0 不应暴击");
    }

    /// 5.8.3 knockback 击退测试：knockback=100.0，验证怪物 Position 发生偏移
    #[test]
    fn collision_knockback_displaces_monster() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.knockback = 100.0;

        // 弹射体在 (90, 100)，怪物在 (100, 100)，方向为正 x
        world.spawn((Position { x: 90.0, y: 100.0 }, proj_comp));

        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        let pos = world.get::<Position>(monster).unwrap();
        // 击退方向 = 怪物 - 弹射体 = (10, 0)，归一化后 = (1, 0)
        // 新位置 = (100 + 100, 100) = (200, 100)
        assert!(
            (pos.x - 200.0).abs() < 1.0,
            "怪物 x 应被击退到约 200.0，实际 {}",
            pos.x
        );
        assert!(
            (pos.y - 100.0).abs() < 1.0,
            "怪物 y 应保持约 100.0，实际 {}",
            pos.y
        );
    }

    /// 5.8.4 Chain 跳跃测试：命中一个怪物后 Chain 到附近另一个怪物
    #[test]
    fn collision_chain_spawns_new_projectile() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(30.0, 1);
        proj_comp.projectile_mods.push(ProjectileModInstance {
            mod_type: ProjectileModType::Chain,
            value: 1.0,
            count: 1,
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        // 被命中的怪物
        spawn_monster(&mut world, 100.0, 100.0, 100.0);
        // Chain 跳跃目标（距离较近）
        spawn_monster(&mut world, 200.0, 100.0, 100.0);

        // 碰撞前：只有 1 个弹射体
        let proj_count_before = world.query::<&ProjectileComp>().iter(&world).count();
        assert_eq!(proj_count_before, 1);

        projectile_monster_collision(&mut world);

        // 碰撞后：原弹射体 + Chain 子弹 = 2 个弹射体
        let proj_count_after = world.query::<&ProjectileComp>().iter(&world).count();
        assert_eq!(proj_count_after, 2, "Chain 应 spawn 一个新弹射体");

        // Chain 子弹的伤害应为原伤害的 50%
        let chain_proj: Vec<f32> = world
            .query::<&ProjectileComp>()
            .iter(&world)
            .filter(|p| p.damage < 30.0) // 排除原弹射体
            .map(|p| p.damage)
            .collect();
        assert_eq!(chain_proj.len(), 1);
        assert_eq!(chain_proj[0], 15.0, "Chain 子弹伤害应为原伤害 50%");
    }

    /// Split 分裂测试
    #[test]
    fn collision_split_spawns_multiple() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(20.0, 1);
        proj_comp.projectile_mods.push(ProjectileModInstance {
            mod_type: ProjectileModType::Split,
            value: 1.0,
            count: 3,
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));
        spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 原弹射体 + 3 个分裂子弹 = 4
        let proj_count = world.query::<&ProjectileComp>().iter(&world).count();
        assert_eq!(proj_count, 4, "Split count=3 应 spawn 3 个子弹");
    }

    /// Explode AoE 伤害测试
    #[test]
    fn collision_explode_aoe_damage() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(20.0, 1);
        proj_comp.projectile_mods.push(ProjectileModInstance {
            mod_type: ProjectileModType::Explode,
            value: 100.0, // radius=100, aoe_damage = 20 * 100 = 2000（足以检测）
            count: 0,
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        // 被直接命中的怪物
        let m1 = spawn_monster(&mut world, 100.0, 100.0, 100.0);
        // AoE 范围内的怪物（距离 50）
        let m2 = spawn_monster(&mut world, 150.0, 100.0, 100.0);
        // AoE 范围外的怪物（距离 200 > 100）
        let m3 = spawn_monster(&mut world, 300.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // m1 受直接命中
        assert!(world.get::<Health>(m1).unwrap().current < 100.0);
        // m2 在 AoE 范围内，应受伤
        assert!(
            world.get::<Health>(m2).unwrap().current < 100.0,
            "AoE 范围内怪物应受伤"
        );
        // m3 在 AoE 范围外，不受伤
        assert_eq!(
            world.get::<Health>(m3).unwrap().current,
            100.0,
            "AoE 范围外怪物不应受伤"
        );
    }

    /// StatusEffect 施加测试
    #[test]
    fn collision_applies_status_effects() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp
            .applied_effects
            .push((StatusEffectType::Poison, 5.0, 3.0));

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        let effects = world.get::<StatusEffects>(monster).unwrap();
        assert_eq!(effects.effects.len(), 1, "应施加 1 个状态效果");
        assert_eq!(effects.effects[0].effect_type, StatusEffectType::Poison);
        assert_eq!(effects.effects[0].remaining, 5.0);
        assert_eq!(effects.effects[0].magnitude, 3.0);
    }

    /// Weaken 增伤测试
    #[test]
    fn collision_weaken_amplifies_damage() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let proj_comp = default_projectile(10.0, 1);
        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        // 创建带 Weaken 状态的怪物
        let monster = world
            .spawn((
                Position { x: 100.0, y: 100.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
                MonsterAI {
                    state: AIState::Wandering,
                    target: None,
                    wander_target_x: 0.0,
                    wander_target_y: 0.0,
                    detection_radius: 200.0,
                },
                StatusEffects {
                    effects: vec![ActiveEffect {
                        effect_type: StatusEffectType::Weaken,
                        remaining: 5.0,
                        magnitude: 0.5, // +50% 伤害
                        anchor: None,
                        dot_timer: 0.0,
                    }],
                },
            ))
            .id();

        projectile_monster_collision(&mut world);

        // 10.0 * (1.0 + 0.5) = 15.0 伤害
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 85.0, "Weaken 应使伤害增加 50%: 10 * 1.5 = 15");
    }

    /// Mark 增伤测试
    #[test]
    fn collision_mark_amplifies_damage() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let proj_comp = default_projectile(20.0, 1);
        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        let monster = world
            .spawn((
                Position { x: 100.0, y: 100.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
                MonsterAI {
                    state: AIState::Wandering,
                    target: None,
                    wander_target_x: 0.0,
                    wander_target_y: 0.0,
                    detection_radius: 200.0,
                },
                StatusEffects {
                    effects: vec![ActiveEffect {
                        effect_type: StatusEffectType::Mark,
                        remaining: 5.0,
                        magnitude: 1.0, // +100% 伤害
                        anchor: None,
                        dot_timer: 0.0,
                    }],
                },
            ))
            .id();

        projectile_monster_collision(&mut world);

        // 20.0 * (1.0 + 1.0) = 40.0 伤害
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 60.0, "Mark 应使伤害增加 100%: 20 * 2.0 = 40");
    }

    /// DamageMod::FlatDamage 测试
    #[test]
    fn collision_flat_damage_mod() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.damage_mods.push(DamageModInstance {
            mod_type: DamageModType::FlatDamage,
            value: 5.0,
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));
        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 10 + 5 = 15 伤害
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 85.0, "FlatDamage 应额外增加 5 伤害");
    }

    /// DamageMod::DamagePercent 测试
    #[test]
    fn collision_damage_percent_mod() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.damage_mods.push(DamageModInstance {
            mod_type: DamageModType::DamagePercent,
            value: 0.5,
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));
        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // 10 * 1.5 = 15 伤害
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 85.0, "DamagePercent 50% 应使伤害变为 15");
    }

    /// DamageMod::BonusVsElite 测试
    #[test]
    fn collision_bonus_vs_elite() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.damage_mods.push(DamageModInstance {
            mod_type: DamageModType::BonusVsElite,
            value: 1.0, // +100% vs elite
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        // 精英怪物
        let monster = world
            .spawn((
                Position { x: 100.0, y: 100.0 },
                Health {
                    current: 100.0,
                    max: 100.0,
                },
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
                MonsterAI {
                    state: AIState::Wandering,
                    target: None,
                    wander_target_x: 0.0,
                    wander_target_y: 0.0,
                    detection_radius: 200.0,
                },
                EliteMarker,
            ))
            .id();

        projectile_monster_collision(&mut world);

        // 10 * (1 + 1) = 20
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(hp, 80.0, "BonusVsElite 应对精英造成双倍伤害");
    }

    /// DamageMod::Ramp 递增伤害测试
    #[test]
    fn collision_ramp_damage_increases_with_hits() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        proj_comp.pierce = 1;
        proj_comp.hits_remaining = 2;
        proj_comp.damage_mods.push(DamageModInstance {
            mod_type: DamageModType::Ramp,
            value: 0.5, // 每次命中 +50%
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));

        let m1 = spawn_monster(&mut world, 100.0, 100.0, 100.0);
        let m2 = spawn_monster(&mut world, 110.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        let hp1 = world.get::<Health>(m1).unwrap().current;
        let hp2 = world.get::<Health>(m2).unwrap().current;
        // 第一次命中：hit_count=0, damage = 10 * (1 + 0*0.5) = 10
        // 第二次命中：hit_count=1, damage = 10 * (1 + 1*0.5) = 15
        // 两个怪物中，一个受 10 伤害，一个受 15 伤害
        let dmg1 = 100.0 - hp1;
        let dmg2 = 100.0 - hp2;
        let mut damages = [dmg1, dmg2];
        damages.sort_by(|a, b| a.partial_cmp(b).unwrap());
        assert_eq!(damages[0], 10.0, "第一次命中应为 10 伤害");
        assert_eq!(damages[1], 15.0, "第二次命中（Ramp）应为 15 伤害");
    }

    /// CritBoost DamageMod 在暴击判定前应用
    #[test]
    fn collision_crit_boost_before_crit_roll() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(10.0, 1);
        // 基础 crit_chance=0.0，通过 CritBoost 加到 1.0
        proj_comp.crit_chance = 0.0;
        proj_comp.crit_multiplier = 2.0;
        proj_comp.damage_mods.push(DamageModInstance {
            mod_type: DamageModType::CritBoost,
            value: 1.0, // +1.0 暴击率 → 必暴击
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));
        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        // CritBoost 使 crit_chance = 1.0 → 必暴击 → 10 * 2.0 = 20
        let hp = world.get::<Health>(monster).unwrap().current;
        assert_eq!(
            hp, 80.0,
            "CritBoost 应在暴击判定前应用，使 crit_chance=1.0 触发暴击"
        );
    }

    /// Attach DoT 施加测试
    #[test]
    fn collision_attach_applies_dot() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());

        let mut proj_comp = default_projectile(50.0, 1);
        proj_comp.projectile_mods.push(ProjectileModInstance {
            mod_type: ProjectileModType::Attach,
            value: 3.0, // duration=3.0
            count: 0,
        });

        world.spawn((Position { x: 100.0, y: 100.0 }, proj_comp));
        let monster = spawn_monster(&mut world, 100.0, 100.0, 100.0);

        projectile_monster_collision(&mut world);

        let effects = world.get::<StatusEffects>(monster).unwrap();
        let poison = effects
            .effects
            .iter()
            .find(|e| e.effect_type == StatusEffectType::Poison);
        assert!(poison.is_some(), "Attach 应施加 Poison 效果");
        let poison = poison.unwrap();
        assert_eq!(poison.remaining, 3.0);
        // magnitude = damage * 0.2 = 50 * 0.2 = 10
        assert_eq!(poison.magnitude, 10.0);
    }
}
