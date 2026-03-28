use bevy_ecs::prelude::*;

use crate::components::*;
use crate::systems::DeltaTime;

/// 高危事件类型
#[derive(Clone, Debug, PartialEq)]
pub enum HighRiskRewardType {
    EpicDisc,
    RareBuff,
    MassXp,
}

/// 高危事件组件
#[derive(Component, Clone, Debug)]
pub struct HighRiskEvent {
    pub x: f32,
    pub y: f32,
    pub reward_type: HighRiskRewardType,
    pub remaining_time: f32,
}

/// 高危事件管理器
#[derive(Resource)]
pub struct HighRiskEventManager {
    pub timer: f32,
    pub interval: f32,
    pub active_count: u32,
    pub max_active: u32,
}

impl Default for HighRiskEventManager {
    fn default() -> Self {
        Self {
            timer: 60.0,
            interval: 60.0,
            active_count: 0,
            max_active: 3,
        }
    }
}

/// 高危事件系统
pub fn high_risk_event_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    // 递减现有事件计时器
    let mut expired: Vec<Entity> = Vec::new();
    for (entity, mut event) in world
        .query::<(Entity, &mut HighRiskEvent)>()
        .iter_mut(world)
    {
        event.remaining_time -= dt;
        if event.remaining_time <= 0.0 {
            expired.push(entity);
        }
    }

    for entity in &expired {
        world.despawn(*entity);
    }

    let expired_count = expired.len() as u32;
    let mgr = world.resource_mut::<HighRiskEventManager>().into_inner();
    mgr.active_count = mgr.active_count.saturating_sub(expired_count);
    mgr.timer -= dt;

    if mgr.timer <= 0.0 && mgr.active_count < mgr.max_active {
        mgr.timer = mgr.interval;
        mgr.active_count += 1;

        // 随机位置（基于简单计数器）
        let counter = mgr.active_count;
        let x = ((counter as f32 * 137.5) % 2048.0) - 1024.0;
        let y = ((counter as f32 * 283.3) % 2048.0) - 1024.0;

        let reward_type = match counter % 3 {
            0 => HighRiskRewardType::EpicDisc,
            1 => HighRiskRewardType::RareBuff,
            _ => HighRiskRewardType::MassXp,
        };

        world.spawn((
            Position { x, y },
            HighRiskEvent {
                x,
                y,
                reward_type,
                remaining_time: 30.0,
            },
        ));
    }
}
