pub mod auto_aim;
pub mod buff;
pub mod collision;
pub mod elemental_reaction;
pub mod environment;
pub mod field_effect;
pub mod high_risk_event;
pub mod imprint;
pub mod projectile;
pub mod status_effect;
pub mod trajectory;

use bevy_ecs::prelude::*;

use crate::components::{Position, Velocity};

/// 帧间隔时间资源，由宿主（game-server / game-client）每帧写入
#[derive(Resource, Debug, Clone, Copy)]
pub struct DeltaTime(pub f32);

/// 移动系统：根据 Velocity 更新 Position
pub fn movement_system(mut query: Query<(&Velocity, &mut Position)>, dt: Res<DeltaTime>) {
    for (vel, mut pos) in query.iter_mut() {
        pos.x += vel.x * dt.0;
        pos.y += vel.y * dt.0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn movement_updates_position() {
        let mut world = World::default();
        world.insert_resource(DeltaTime(1.0 / 60.0));
        world.spawn((
            Position { x: 100.0, y: 200.0 },
            Velocity { x: 60.0, y: -120.0 },
        ));

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let mut q = world.query::<&Position>();
        let pos = q.single(&world).unwrap();
        assert!((pos.x - 101.0).abs() < 0.001);
        assert!((pos.y - 198.0).abs() < 0.001);
    }

    #[test]
    fn movement_zero_velocity() {
        let mut world = World::default();
        world.insert_resource(DeltaTime(0.016));
        world.spawn((Position { x: 50.0, y: 50.0 }, Velocity { x: 0.0, y: 0.0 }));

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let mut q = world.query::<&Position>();
        let pos = q.single(&world).unwrap();
        assert!((pos.x - 50.0).abs() < 0.001);
        assert!((pos.y - 50.0).abs() < 0.001);
    }

    #[test]
    fn movement_multiple_entities() {
        let mut world = World::default();
        world.insert_resource(DeltaTime(1.0));
        world.spawn((Position { x: 0.0, y: 0.0 }, Velocity { x: 10.0, y: 5.0 }));
        world.spawn((
            Position { x: 100.0, y: 100.0 },
            Velocity { x: -3.0, y: 7.0 },
        ));

        let mut schedule = Schedule::default();
        schedule.add_systems(movement_system);
        schedule.run(&mut world);

        let mut q = world.query::<&Position>();
        let mut positions: Vec<(f32, f32)> = q.iter(&world).map(|p| (p.x, p.y)).collect();
        positions.sort_by(|a, b| a.0.partial_cmp(&b.0).unwrap());

        assert!((positions[0].0 - 10.0).abs() < 0.001);
        assert!((positions[0].1 - 5.0).abs() < 0.001);
        assert!((positions[1].0 - 97.0).abs() < 0.001);
        assert!((positions[1].1 - 107.0).abs() < 0.001);
    }
}
