//! 弹射体轨迹修正系统
//!
//! 根据 ProjectileComp.trajectory_mods 中的 TrajectoryType 修改弹射体速度方向。

use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::constants::TICK_DT;
use crate::data::TrajectoryType;

/// 全局 tick 计数器，用于 Scatter 伪随机种子
static mut TRAJECTORY_TICK: u32 = 0;

/// 弹射体轨迹修正 tick 系统
pub fn trajectory_tick_system(world: &mut World) {
    let tick_count = unsafe {
        TRAJECTORY_TICK = TRAJECTORY_TICK.wrapping_add(1);
        TRAJECTORY_TICK
    };

    // 收集活着的怪物位置（带 Health > 0），用于 Homing 追踪
    let alive_monsters: Vec<(f32, f32)> = world
        .query::<(&Position, &Health, &MonsterComp)>()
        .iter(world)
        .filter(|(_, hp, _)| hp.current > 0.0)
        .map(|(pos, _, _)| (pos.x, pos.y))
        .collect();

    // 收集弹射体数据
    struct ProjData {
        entity: Entity,
        px: f32,
        py: f32,
        vx: f32,
        vy: f32,
        lifetime: f32,
        remaining: f32,
        trajectory_mods: Vec<TrajectoryType>,
        /// Teleport 触发时的位置偏移量
        teleport_dx: f32,
        teleport_dy: f32,
    }

    let projs: Vec<ProjData> = world
        .query::<(Entity, &Position, &Velocity, &ProjectileComp)>()
        .iter(world)
        .filter(|(_, _, _, proj)| proj.remaining_lifetime > 0.0 && !proj.trajectory_mods.is_empty())
        .map(|(entity, pos, vel, proj)| ProjData {
            entity,
            px: pos.x,
            py: pos.y,
            vx: vel.x,
            vy: vel.y,
            lifetime: proj.lifetime,
            remaining: proj.remaining_lifetime,
            trajectory_mods: proj.trajectory_mods.clone(),
            teleport_dx: 0.0,
            teleport_dy: 0.0,
        })
        .collect();

    // 对每个弹射体应用轨迹修正（可变遍历，Teleport 需要写入位置偏移）
    let mut projs = projs;
    for proj in &mut projs {
        let elapsed = proj.lifetime - proj.remaining;
        let mut new_vx = proj.vx;
        let mut new_vy = proj.vy;
        let speed = (new_vx * new_vx + new_vy * new_vy).sqrt();
        let entity_index = proj.entity.index_u32();
        let px = proj.px;
        let py = proj.py;
        let init_vx = proj.vx;
        let init_vy = proj.vy;
        let lifetime = proj.lifetime;

        for traj in proj.trajectory_mods.clone().iter() {
            match traj {
                TrajectoryType::Homing => {
                    // 寻找最近的活着的怪物
                    if let Some((tx, ty)) = find_nearest(&alive_monsters, px, py) {
                        let dx = tx - px;
                        let dy = ty - py;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 1.0 {
                            // 目标方向
                            let target_angle = dy.atan2(dx);
                            let current_angle = new_vy.atan2(new_vx);
                            // 转向率 3.0 rad/s * TICK_DT
                            let max_turn = 3.0 * TICK_DT;
                            let diff = angle_diff(current_angle, target_angle);
                            let turn = diff.clamp(-max_turn, max_turn);
                            let new_angle = current_angle + turn;
                            new_vx = new_angle.cos() * speed;
                            new_vy = new_angle.sin() * speed;
                        }
                    }
                }
                TrajectoryType::Spiral => {
                    // 每 tick 旋转 2.0 rad/s
                    let angle = 2.0 * TICK_DT;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    let rotated_vx = new_vx * cos_a - new_vy * sin_a;
                    let rotated_vy = new_vx * sin_a + new_vy * cos_a;
                    new_vx = rotated_vx;
                    new_vy = rotated_vy;
                }
                TrajectoryType::Wave => {
                    // 正弦横向偏移：freq=6.0, amplitude=100.0
                    let freq = 6.0;
                    let amplitude = 100.0;
                    // 计算横向方向（垂直于速度方向）
                    if speed > 1.0 {
                        let perp_x = -new_vy / speed;
                        let perp_y = new_vx / speed;
                        let wave_offset = (elapsed * freq).cos() * amplitude * freq * TICK_DT;
                        new_vx += perp_x * wave_offset;
                        new_vy += perp_y * wave_offset;
                    }
                }
                TrajectoryType::Boomerang => {
                    // 前半段正常飞行，后半段反转方向
                    let half = lifetime * 0.5;
                    if elapsed > half {
                        // 已进入后半段：反转速度方向
                        // 只在刚进入后半段的瞬间反转一次（通过检查上一帧是否还在前半段）
                        let prev_elapsed = elapsed - TICK_DT;
                        if prev_elapsed <= half {
                            new_vx = -new_vx;
                            new_vy = -new_vy;
                        }
                    }
                }
                TrajectoryType::Zigzag => {
                    // 每 0.2s 交替偏转 +/-45 度
                    let period = 0.2;
                    let current_phase = (elapsed / period) as u32;
                    let prev_phase = ((elapsed - TICK_DT).max(0.0) / period) as u32;
                    if current_phase != prev_phase {
                        // 相位切换，偏转方向
                        let angle = if current_phase.is_multiple_of(2) {
                            std::f32::consts::FRAC_PI_4
                        } else {
                            -std::f32::consts::FRAC_PI_4
                        };
                        let cos_a = angle.cos();
                        let sin_a = angle.sin();
                        let rotated_vx = new_vx * cos_a - new_vy * sin_a;
                        let rotated_vy = new_vx * sin_a + new_vy * cos_a;
                        new_vx = rotated_vx;
                        new_vy = rotated_vy;
                    }
                }
                TrajectoryType::Accelerate => {
                    // 每 tick speed *= 1.02
                    new_vx *= 1.02;
                    new_vy *= 1.02;
                }
                TrajectoryType::Decelerate => {
                    // 每 tick speed *= 0.98，最低速度 50
                    let new_speed = (speed * 0.98).max(50.0);
                    if speed > 1.0 {
                        let ratio = new_speed / speed;
                        new_vx *= ratio;
                        new_vy *= ratio;
                    }
                }
                TrajectoryType::Gravity => {
                    // 向下弯曲（vy -= gravity * TICK_DT）
                    let gravity = 300.0;
                    new_vy -= gravity * TICK_DT;
                }
                TrajectoryType::Scatter => {
                    // 每 tick 随机微扰方向 +/-0.1 rad
                    let seed = entity_index
                        .wrapping_mul(2654435761)
                        .wrapping_add(tick_count);
                    let noise = (seed as f32 / u32::MAX as f32) * 0.2 - 0.1; // [-0.1, 0.1] rad
                    let cos_a = noise.cos();
                    let sin_a = noise.sin();
                    let rotated_vx = new_vx * cos_a - new_vy * sin_a;
                    let rotated_vy = new_vx * sin_a + new_vy * cos_a;
                    new_vx = rotated_vx;
                    new_vy = rotated_vy;
                }
                TrajectoryType::Orbit => {
                    // 围绕发射点旋转：速度方向每 tick 旋转 4.0 rad/s（比 Spiral 更快），
                    // 形成紧密圆周运动
                    let angle = 4.0 * TICK_DT;
                    let cos_a = angle.cos();
                    let sin_a = angle.sin();
                    let rotated_vx = new_vx * cos_a - new_vy * sin_a;
                    let rotated_vy = new_vx * sin_a + new_vy * cos_a;
                    new_vx = rotated_vx;
                    new_vy = rotated_vy;
                }
                TrajectoryType::Ricochet => {
                    // 每 0.3s 随机反转 x 或 y 方向，模拟反弹效果
                    let period = 0.3;
                    let current_phase = (elapsed / period) as u32;
                    let prev_phase = ((elapsed - TICK_DT).max(0.0) / period) as u32;
                    if current_phase != prev_phase {
                        // 用 entity index + phase 做伪随机决定反转哪个轴
                        let seed = entity_index
                            .wrapping_mul(2654435761)
                            .wrapping_add(current_phase);
                        if seed.is_multiple_of(2) {
                            new_vx = -new_vx;
                        } else {
                            new_vy = -new_vy;
                        }
                    }
                }
                TrajectoryType::Teleport => {
                    // 每 0.5s 瞬移到当前位置的随机方向 50px 处
                    let period = 0.5;
                    let current_phase = (elapsed / period) as u32;
                    let prev_phase = ((elapsed - TICK_DT).max(0.0) / period) as u32;
                    if current_phase != prev_phase && current_phase > 0 {
                        // 用 entity index + phase 做伪随机方向
                        let seed = entity_index
                            .wrapping_mul(2654435761)
                            .wrapping_add(current_phase);
                        let angle = (seed as f32 / u32::MAX as f32) * std::f32::consts::TAU;
                        let teleport_dist = 50.0;
                        proj.teleport_dx += angle.cos() * teleport_dist;
                        proj.teleport_dy += angle.sin() * teleport_dist;
                    }
                }
                TrajectoryType::OrbitTarget => {
                    // 围绕最近怪物做圆周运动：速度方向设为垂直于指向目标的方向
                    if let Some((tx, ty)) = find_nearest(&alive_monsters, px, py) {
                        let dx = tx - px;
                        let dy = ty - py;
                        let dist = (dx * dx + dy * dy).sqrt();
                        if dist > 1.0 {
                            // 切线方向（逆时针垂直于径向）
                            let tangent_x = -dy / dist;
                            let tangent_y = dx / dist;
                            new_vx = tangent_x * speed;
                            new_vy = tangent_y * speed;
                            // 施加微弱向心力保持轨道稳定
                            let centripetal = speed * 0.05;
                            new_vx += (dx / dist) * centripetal;
                            new_vy += (dy / dist) * centripetal;
                        }
                    }
                }
                TrajectoryType::SineVertical => {
                    // 垂直方向正弦偏移：在速度的垂直分量上叠加正弦波
                    let freq = 10.0;
                    let amplitude = 100.0;
                    if speed > 1.0 {
                        let perp_x = -new_vy / speed;
                        let perp_y = new_vx / speed;
                        let wave_offset = (elapsed * freq).sin() * amplitude * TICK_DT;
                        new_vx += perp_x * wave_offset;
                        new_vy += perp_y * wave_offset;
                    }
                }
                TrajectoryType::Pendulum => {
                    // 钟摆式摆动：速度方向角 = 基准角 + sin(elapsed * 3.0) * 0.8 rad
                    // 基准角用初始速度方向（发射时刻的 vx/vy）
                    let base_angle = init_vy.atan2(init_vx);
                    let swing = (elapsed * 3.0).sin() * 0.8;
                    let new_angle = base_angle + swing;
                    new_vx = new_angle.cos() * speed;
                    new_vy = new_angle.sin() * speed;
                }
            }
        }

        // 写回速度
        let entity = proj.entity;
        if let Some(mut vel) = world.get_mut::<Velocity>(entity) {
            vel.x = new_vx;
            vel.y = new_vy;
        }

        // Teleport 位置偏移写回
        if proj.teleport_dx != 0.0 || proj.teleport_dy != 0.0 {
            if let Some(mut pos) = world.get_mut::<Position>(entity) {
                pos.x += proj.teleport_dx;
                pos.y += proj.teleport_dy;
            }
        }
    }
}

/// 查找最近的目标位置
fn find_nearest(targets: &[(f32, f32)], px: f32, py: f32) -> Option<(f32, f32)> {
    targets
        .iter()
        .map(|(tx, ty)| {
            let dx = tx - px;
            let dy = ty - py;
            (dx * dx + dy * dy, *tx, *ty)
        })
        .min_by(|a, b| a.0.partial_cmp(&b.0).unwrap_or(std::cmp::Ordering::Equal))
        .map(|(_, tx, ty)| (tx, ty))
}

/// 计算两个角度之间的最短差值（考虑环绕），结果在 [-PI, PI]
fn angle_diff(from: f32, to: f32) -> f32 {
    let mut diff = to - from;
    while diff > std::f32::consts::PI {
        diff -= std::f32::consts::TAU;
    }
    while diff < -std::f32::consts::PI {
        diff += std::f32::consts::TAU;
    }
    diff
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::events::GameEventQueue;

    /// Homing 追踪测试：弹射体在怪物方向偏转
    #[test]
    fn homing_turns_toward_monster() {
        let mut world = World::new();
        world.insert_resource(GameEventQueue::default());
        world.insert_resource(crate::systems::DeltaTime(TICK_DT));

        // 弹射体在 (0, 0)，向右飞 (speed=100, 0)
        let proj_entity = world
            .spawn((
                Position { x: 0.0, y: 0.0 },
                Velocity { x: 100.0, y: 0.0 },
                ProjectileComp {
                    damage: 10.0,
                    lifetime: 2.0,
                    remaining_lifetime: 1.5,
                    size: 4.0,
                    owner_id: 1,
                    pierce: 0,
                    hits_remaining: 1,
                    crit_chance: 0.0,
                    crit_multiplier: 1.0,
                    knockback: 0.0,
                    damage_mods: Vec::new(),
                    projectile_mods: Vec::new(),
                    field_mods: Vec::new(),
                    applied_effects: Vec::new(),
                    trajectory_mods: vec![TrajectoryType::Homing],
                    hit_entities: Vec::new(),
                    on_hit: None,
                    on_kill: None,
                    on_expire: None,
                    on_crit: None,
                    on_boss_hit: None,
                    on_periodic: None,
                },
            ))
            .id();

        // 怪物在 (0, 100)，弹射体应向上偏转
        world.spawn((
            Position { x: 0.0, y: 100.0 },
            Health {
                current: 50.0,
                max: 50.0,
            },
            MonsterComp {
                monster_type: 1,
                damage: 5.0,
            },
        ));

        trajectory_tick_system(&mut world);

        let vel = world.get::<Velocity>(proj_entity).unwrap();
        // 速度 y 分量应增加（朝怪物方向偏转）
        assert!(vel.y > 0.0, "Homing 应使 vy 正向增加，实际 vy={}", vel.y);
        // 速度大小应基本不变（转向不改速度大小）
        let speed = (vel.x * vel.x + vel.y * vel.y).sqrt();
        assert!(
            (speed - 100.0).abs() < 1.0,
            "速度大小应基本不变，实际 speed={}",
            speed
        );
    }
}
