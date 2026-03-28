use bevy_ecs::prelude::*;
use std::collections::HashSet;

use crate::chunk::ChunkCoord;
use crate::components::*;
use crate::data::constants::*;
use crate::events::{GameEvent, GameEventQueue};
use crate::resources::WaveManager;
use crate::systems::DeltaTime;

use super::collision::apply_damage;

/// 数据风暴状态
#[derive(Resource, Default)]
pub struct DataStormState {
    pub affected_chunks: HashSet<ChunkCoord>,
    pub expansion_timer: f32,
    pub damage_per_sec: f32,
}

impl DataStormState {
    pub fn new() -> Self {
        Self {
            affected_chunks: HashSet::new(),
            expansion_timer: 0.0,
            damage_per_sec: DATA_STORM_DAMAGE,
        }
    }
}

/// 数据风暴系统：根据存活时间逐步标记外围 chunk
pub fn data_storm_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;
    let wave_timer = world.resource::<WaveManager>().wave_timer;
    let wave_number = world.resource::<WaveManager>().wave_number;

    // 推算已过时间：wave_number * 30s + (30 - wave_timer)
    let elapsed = wave_number as f32 * WAVE_INTERVAL + (WAVE_INTERVAL - wave_timer);

    // 300 秒后开始数据风暴
    if elapsed < DATA_STORM_START_TIME {
        return;
    }

    let storm = world.resource_mut::<DataStormState>().into_inner();
    storm.expansion_timer -= dt;

    if storm.expansion_timer <= 0.0 {
        storm.expansion_timer = DATA_STORM_EXPANSION_INTERVAL;

        // 确定当前应影响的最大距离（从原点开始，由外向内）
        let max_ring = ((elapsed - DATA_STORM_START_TIME) / DATA_STORM_EXPANSION_RATE) as i32
            + DATA_STORM_INITIAL_RING;
        let min_ring = max_ring
            .saturating_sub(((elapsed - DATA_STORM_START_TIME) / DATA_STORM_SHRINK_RATE) as i32);

        // 标记外围 chunk
        for x in -max_ring..=max_ring {
            for y in -max_ring..=max_ring {
                let dist = x.abs().max(y.abs());
                if dist >= min_ring {
                    storm.affected_chunks.insert(ChunkCoord { x, y });
                }
            }
        }
    }

    // 对风暴区内的玩家造成伤害
    let damage_per_sec = world.resource::<DataStormState>().damage_per_sec;
    let affected = world.resource::<DataStormState>().affected_chunks.clone();

    let players: Vec<(Entity, ChunkCoord)> = world
        .query::<(Entity, &Position, &Player)>()
        .iter(world)
        .map(|(e, pos, _)| (e, ChunkCoord::from_world_pos(pos.x, pos.y)))
        .collect();

    for (entity, chunk) in players {
        if affected.contains(&chunk) {
            let remaining_hp = apply_damage(world, entity, damage_per_sec * dt);
            world
                .resource_mut::<GameEventQueue>()
                .push(GameEvent::EntityDamaged {
                    entity,
                    damage: damage_per_sec * dt,
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
}
