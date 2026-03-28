//! 怪物 AI 系统：VS-Style 永久追击最近玩家 + Boids 分离力防扎堆

use bevy_ecs::prelude::Entity;
use bevy_ecs::world::World;

use crate::components::*;
use crate::data::constants::{MONSTER_SEPARATION_RADIUS, MONSTER_SEPARATION_WEIGHT, MONSTER_SPEED};
use crate::data::StatusEffectType;

/// 全局递增计数器，用于 Confuse 伪随机方向
static mut AI_TICK_COUNTER: u32 = 0;

/// VS-Style 追击系统：所有怪物始终朝最近玩家移动，Boids 分离力防止扎堆
/// 状态效果影响：Fear→逃离最近玩家，Confuse→随机方向，Blind→检测半径缩小
pub fn monster_chase_system(world: &mut World) {
    // WASM 单线程安全
    let tick_count = unsafe {
        AI_TICK_COUNTER = AI_TICK_COUNTER.wrapping_add(1);
        AI_TICK_COUNTER
    };

    // 收集所有玩家位置
    let players: Vec<(u32, f32, f32)> = world
        .query::<(&Player, &Position)>()
        .iter(world)
        .map(|(p, pos)| (p.id, pos.x, pos.y))
        .collect();

    if players.is_empty() {
        return;
    }

    // 收集所有怪物位置（只读快照，用于分离力计算）
    let monster_positions: Vec<(f32, f32)> = world
        .query::<(&MonsterComp, &Position)>()
        .iter(world)
        .map(|(_, pos)| (pos.x, pos.y))
        .collect();

    let sep_r_sq = MONSTER_SEPARATION_RADIUS * MONSTER_SEPARATION_RADIUS;

    for (entity, mut ai, pos, mut vel, status) in world
        .query::<(
            Entity,
            &mut MonsterAI,
            &Position,
            &mut Velocity,
            Option<&StatusEffects>,
        )>()
        .iter_mut(world)
    {
        let my_x = pos.x;
        let my_y = pos.y;

        // 检查状态效果
        let mut has_fear = false;
        let mut has_confuse = false;
        let mut blind_magnitude = 0.0f32;

        if let Some(effects) = status {
            for eff in &effects.effects {
                match eff.effect_type {
                    StatusEffectType::Fear => {
                        has_fear = true;
                    }
                    StatusEffectType::Confuse => {
                        has_confuse = true;
                    }
                    StatusEffectType::Blind => {
                        // 取最强致盲效果
                        blind_magnitude = blind_magnitude.max(eff.magnitude);
                    }
                    _ => {}
                }
            }
        }

        // Confuse：AI 状态设为 Wandering，方向随机
        if has_confuse {
            ai.state = AIState::Wandering;
            ai.target = None;

            // 伪随机方向
            let seed = entity
                .index_u32()
                .wrapping_mul(2654435761)
                .wrapping_add(tick_count);
            let angle = (seed as f32 / u32::MAX as f32) * std::f32::consts::TAU;
            vel.x = angle.cos() * MONSTER_SPEED;
            vel.y = angle.sin() * MONSTER_SPEED;
            continue;
        }

        // Blind：缩小检测半径
        let effective_detection_radius = if blind_magnitude > 0.0 {
            ai.detection_radius * (1.0 - blind_magnitude).max(0.0)
        } else {
            ai.detection_radius
        };
        let effective_det_sq = effective_detection_radius * effective_detection_radius;

        // 找最近玩家（VS-Style：无 Blind 时无限追击，有 Blind 时受检测半径限制）
        let mut closest_id: Option<u32> = None;
        let mut closest_dist_sq = f32::MAX;

        for &(pid, px, py) in &players {
            let dx = px - my_x;
            let dy = py - my_y;
            let dist_sq = dx * dx + dy * dy;
            let in_range = blind_magnitude <= 0.0 || dist_sq <= effective_det_sq;
            if dist_sq < closest_dist_sq && in_range {
                closest_dist_sq = dist_sq;
                closest_id = Some(pid);
            }
        }

        // Fear：强制逃离最近玩家（不受 Blind 检测限制，找绝对最近）
        if has_fear {
            // Fear 时无视 Blind 检测限制，找绝对最近的玩家来逃离
            let mut fear_closest_id: Option<u32> = None;
            let mut fear_closest_dist_sq = f32::MAX;
            for &(pid, px, py) in &players {
                let dx = px - my_x;
                let dy = py - my_y;
                let dist_sq = dx * dx + dy * dy;
                if dist_sq < fear_closest_dist_sq {
                    fear_closest_dist_sq = dist_sq;
                    fear_closest_id = Some(pid);
                }
            }

            if let Some(pid) = fear_closest_id {
                ai.state = AIState::Fleeing;
                ai.target = Some(pid);

                let (_, tx, ty) = players.iter().find(|(id, _, _)| *id == pid).unwrap();
                let dx = tx - my_x;
                let dy = ty - my_y;
                let dist = (dx * dx + dy * dy).sqrt();

                // 逃离方向 = 追击方向取反
                let (flee_x, flee_y) = if dist > 1.0 {
                    (-dx / dist, -dy / dist)
                } else {
                    (1.0, 0.0) // 重叠时给一个默认逃离方向
                };

                vel.x = flee_x * MONSTER_SPEED;
                vel.y = flee_y * MONSTER_SPEED;
            }
            continue;
        }

        if let Some(pid) = closest_id {
            ai.state = AIState::Chasing;
            ai.target = Some(pid);

            // 追击方向（单位向量）
            let (_, tx, ty) = players.iter().find(|(id, _, _)| *id == pid).unwrap();
            let dx = tx - my_x;
            let dy = ty - my_y;
            let dist = (dx * dx + dy * dy).sqrt();

            let (chase_x, chase_y) = if dist > 1.0 {
                (dx / dist, dy / dist)
            } else {
                (0.0, 0.0)
            };

            // Boids 分离力：远离附近怪物，力度随距离反比衰减
            // dist_sq > 0.001 自动跳过自身（自身距离 = 0）
            let mut sep_x = 0.0f32;
            let mut sep_y = 0.0f32;
            for &(ox, oy) in &monster_positions {
                let rx = my_x - ox;
                let ry = my_y - oy;
                let d_sq = rx * rx + ry * ry;
                if d_sq > 0.001 && d_sq < sep_r_sq {
                    let d = d_sq.sqrt();
                    let force = (MONSTER_SEPARATION_RADIUS - d) / MONSTER_SEPARATION_RADIUS;
                    sep_x += rx / d * force;
                    sep_y += ry / d * force;
                }
            }

            // 混合追击 + 分离，归一化到 MONSTER_SPEED
            let blend_x = chase_x + sep_x * MONSTER_SEPARATION_WEIGHT;
            let blend_y = chase_y + sep_y * MONSTER_SEPARATION_WEIGHT;
            let blend_len = (blend_x * blend_x + blend_y * blend_y).sqrt();

            if blend_len > 0.001 {
                vel.x = blend_x / blend_len * MONSTER_SPEED;
                vel.y = blend_y / blend_len * MONSTER_SPEED;
            }
        } else {
            // Blind 导致没发现玩家，切换为 Wandering
            ai.state = AIState::Wandering;
            ai.target = None;
        }
    }
}
