//! 怪物生成工具函数

use bevy_ecs::prelude::*;

use crate::components::*;
use crate::events::{GameEvent, GameEventQueue};

/// 统一怪物生成函数
#[allow(clippy::too_many_arguments)]
pub fn spawn_monster(
    world: &mut World,
    monster_type: u8,
    x: f32,
    y: f32,
    hp: f32,
    damage: f32,
    detection_radius: f32,
    collision_radius: f32,
) -> Entity {
    let entity = world
        .spawn((
            Position { x, y },
            Velocity { x: 0.0, y: 0.0 },
            Health {
                current: hp,
                max: hp,
            },
            MonsterComp {
                monster_type,
                damage,
            },
            MonsterAI {
                state: AIState::Chasing,
                target: None,
                wander_target_x: 0.0,
                wander_target_y: 0.0,
                detection_radius,
            },
            CollisionRadius(collision_radius),
            ContactAttackTimer { remaining: 0.0 },
        ))
        .id();

    world
        .resource_mut::<GameEventQueue>()
        .push(GameEvent::MonsterSpawned {
            entity,
            monster_type,
        });

    entity
}
