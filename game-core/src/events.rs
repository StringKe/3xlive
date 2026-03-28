use bevy_ecs::prelude::*;
use bincode::{Decode, Encode};

use crate::data::{FieldType, ReactionType, StatusEffectType};

/// 游戏事件枚举，系统间通信的统一渠道
///
/// derive Message 供客户端通过 MessageWriter/MessageReader 转发
#[derive(Clone, Debug, Message)]
pub enum GameEvent {
    EntityDamaged {
        entity: Entity,
        damage: f32,
        remaining_hp: f32,
    },
    EntityKilled {
        entity: Entity,
        killer: Option<Entity>,
    },
    ProjectileSpawned {
        entity: Entity,
        x: f32,
        y: f32,
        owner_id: u32,
    },
    ProjectileDespawned {
        entity: Entity,
    },
    MonsterSpawned {
        entity: Entity,
        monster_type: u8,
    },
    LootSpawned {
        entity: Entity,
        item_type: u8,
        x: f32,
        y: f32,
    },
    LootPickedUp {
        entity: Entity,
        picker: Entity,
        /// 被拾取的物品（Disc/Program 需要同步到客户端背包 UI）
        item: crate::components::LootItem,
    },
    LevelUp {
        entity: Entity,
        new_level: u32,
    },
    RunEnded {
        result: RunResult,
    },
    /// 场地效果生成
    FieldSpawned {
        entity: Entity,
        field_type: FieldType,
        x: f32,
        y: f32,
        radius: f32,
        duration: f32,
    },
    /// 元素反应触发
    ElementalReaction {
        entity: Entity,
        reaction: ReactionType,
        consumed: (StatusEffectType, StatusEffectType),
    },
}

/// 单局结果
#[derive(Clone, Debug, Encode, Decode)]
pub struct RunResult {
    pub survived_secs: f32,
    pub total_damage: f32,
    pub kills_normal: u32,
    pub kills_elite: u32,
    pub kills_boss: u32,
    pub max_chunk_difficulty: f32,
    pub final_level: u32,
}

/// 游戏事件队列，系统通过 world.resource_mut::<GameEventQueue>().push(event) 写入
#[derive(Resource, Default)]
pub struct GameEventQueue {
    events: Vec<GameEvent>,
}

impl GameEventQueue {
    pub fn push(&mut self, event: GameEvent) {
        self.events.push(event);
    }

    pub fn drain(&mut self) -> Vec<GameEvent> {
        std::mem::take(&mut self.events)
    }
}
