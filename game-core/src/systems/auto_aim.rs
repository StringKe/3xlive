use bevy_ecs::prelude::*;

use crate::components::*;
use crate::data::{Disc, ProgramParams};

/// 自动瞄准系统：为每个玩家找到射程内最近的怪物，更新 AimDirection
pub fn auto_aim_system(world: &mut World) {
    let players: Vec<(Entity, f32, f32, f32)> = world
        .query::<(Entity, &Position, &DiscEquipped)>()
        .iter(world)
        .filter(|(e, _, _)| world.get::<Player>(*e).is_some())
        .map(|(e, pos, disc)| {
            let range = calc_max_range(&disc.disc);
            (e, pos.x, pos.y, range)
        })
        .collect();

    let targets: Vec<(f32, f32)> = world
        .query_filtered::<&Position, With<MonsterComp>>()
        .iter(world)
        .map(|p| (p.x, p.y))
        .collect();

    for (entity, px, py, range) in players {
        let range_sq = range * range;
        let mut best: Option<(f32, f32, f32)> = None;
        for &(tx, ty) in &targets {
            let dx = tx - px;
            let dy = ty - py;
            let dist_sq = dx * dx + dy * dy;
            if dist_sq <= range_sq && (best.is_none() || dist_sq < best.unwrap().0) {
                best = Some((dist_sq, dx, dy));
            }
        }
        if let Some((_, dx, dy)) = best {
            let len = (dx * dx + dy * dy).sqrt();
            if len > 0.0 {
                if let Some(mut aim) = world.get_mut::<AimDirection>(entity) {
                    aim.x = dx / len;
                    aim.y = dy / len;
                }
            }
            // 有目标：移除标记，允许开火
            world.entity_mut(entity).remove::<AutoAimNoTarget>();
        } else {
            // 无目标：插入标记，阻止开火
            world.entity_mut(entity).insert(AutoAimNoTarget);
        }
    }
}

/// 计算 Disc 上所有 Projectile Program 的最大射程（speed * lifetime）
fn calc_max_range(disc: &Disc) -> f32 {
    disc.slots
        .iter()
        .filter_map(|s| s.as_ref())
        .filter_map(|p| match &p.params {
            ProgramParams::Projectile {
                speed, lifetime, ..
            } => Some(speed * lifetime),
            _ => None,
        })
        .fold(0.0_f32, f32::max)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::components::{
        AIState, AimDirection, CollisionRadius, DiscEquipped, Health, MonsterAI, MonsterComp,
        Player, Position, Velocity,
    };
    use crate::data::disc::make_starter_disc;
    use bevy_ecs::world::World;

    /// 辅助函数：创建带完整组件的玩家实体
    fn spawn_player(world: &mut World, x: f32, y: f32, aim_x: f32, aim_y: f32) -> Entity {
        world
            .spawn((
                Player {
                    id: 1,
                    name: "test".into(),
                },
                Position { x, y },
                Velocity { x: 0.0, y: 0.0 },
                AimDirection { x: aim_x, y: aim_y },
                DiscEquipped {
                    disc: make_starter_disc(),
                    cooldown_timer: 0.0,
                    active_timing: None,
                    timing_state: TimingState::default(),
                },
            ))
            .id()
    }

    /// 辅助函数：创建带完整组件的怪物实体
    fn spawn_monster(world: &mut World, x: f32, y: f32) -> Entity {
        world
            .spawn((
                MonsterComp {
                    monster_type: 0,
                    damage: 5.0,
                },
                Position { x, y },
                Velocity { x: 0.0, y: 0.0 },
                Health {
                    current: 50.0,
                    max: 50.0,
                },
                CollisionRadius(24.0),
                MonsterAI {
                    state: AIState::Wandering,
                    target: None,
                    wander_target_x: 0.0,
                    wander_target_y: 0.0,
                    detection_radius: 300.0,
                },
            ))
            .id()
    }

    /// (a) 有怪物在射程内时 AimDirection 指向怪物
    #[test]
    fn aim_at_monster_in_range() {
        let mut world = World::new();
        // 玩家在原点，初始瞄准右方
        let player = spawn_player(&mut world, 0.0, 0.0, 1.0, 0.0);
        // 怪物在正上方 200px（远小于射程 1000px）
        spawn_monster(&mut world, 0.0, 200.0);

        auto_aim_system(&mut world);

        let aim = world.get::<AimDirection>(player).unwrap();
        // 应指向正上方 (0, 1)
        assert!(
            (aim.x - 0.0).abs() < 1e-5,
            "aim.x = {}, expected 0.0",
            aim.x
        );
        assert!(
            (aim.y - 1.0).abs() < 1e-5,
            "aim.y = {}, expected 1.0",
            aim.y
        );
    }

    /// (b) 无怪物时 AimDirection 不变（保持初始值）
    #[test]
    fn aim_unchanged_when_no_monsters() {
        let mut world = World::new();
        let player = spawn_player(&mut world, 0.0, 0.0, 0.6, 0.8);

        auto_aim_system(&mut world);

        let aim = world.get::<AimDirection>(player).unwrap();
        assert!(
            (aim.x - 0.6).abs() < 1e-5,
            "aim.x = {}, expected 0.6",
            aim.x
        );
        assert!(
            (aim.y - 0.8).abs() < 1e-5,
            "aim.y = {}, expected 0.8",
            aim.y
        );
    }

    /// (c) 多个怪物时选最近的
    #[test]
    fn aim_at_closest_monster() {
        let mut world = World::new();
        let player = spawn_player(&mut world, 0.0, 0.0, 1.0, 0.0);
        // 远处怪物 (500, 0)
        spawn_monster(&mut world, 500.0, 0.0);
        // 近处怪物 (100, 0)
        spawn_monster(&mut world, 100.0, 0.0);
        // 中距怪物 (0, 300)
        spawn_monster(&mut world, 0.0, 300.0);

        auto_aim_system(&mut world);

        let aim = world.get::<AimDirection>(player).unwrap();
        // 应指向最近的 (100, 0)，即正右方 (1, 0)
        assert!(
            (aim.x - 1.0).abs() < 1e-5,
            "aim.x = {}, expected 1.0",
            aim.x
        );
        assert!(
            (aim.y - 0.0).abs() < 1e-5,
            "aim.y = {}, expected 0.0",
            aim.y
        );
    }

    /// (d) 怪物超出射程时不瞄准（AimDirection 不变）
    #[test]
    fn aim_unchanged_when_monster_out_of_range() {
        let mut world = World::new();
        // 初始瞄准左方
        let player = spawn_player(&mut world, 0.0, 0.0, -1.0, 0.0);
        // 怪物在 1500px 外（射程 = 500 speed * 2 lifetime = 1000px）
        spawn_monster(&mut world, 1500.0, 0.0);

        auto_aim_system(&mut world);

        let aim = world.get::<AimDirection>(player).unwrap();
        assert!(
            (aim.x - (-1.0)).abs() < 1e-5,
            "aim.x = {}, expected -1.0",
            aim.x
        );
        assert!(
            (aim.y - 0.0).abs() < 1e-5,
            "aim.y = {}, expected 0.0",
            aim.y
        );
    }
}
