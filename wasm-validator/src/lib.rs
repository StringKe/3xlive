//! Headless WASM 回放验证器
//!
//! 接收 bincode 编码的 ReplayData，逐帧回放 InputFrame 调用 run_game_tick，
//! 返回 bincode 编码的 RunResult。

use wasm_bindgen::prelude::*;

use bevy_ecs::prelude::*;
use game_core::components::*;
use game_core::events::GameEventQueue;
use game_core::replay::ReplayData;
use game_core::resources::*;
use game_core::systems::environment::DataStormState;
use game_core::systems::high_risk_event::HighRiskEventManager;
use game_core::systems::DeltaTime;
use game_core::tick::run_game_tick;

/// 回放验证入口：接收 bincode 编码的 ReplayData，返回 bincode 编码的 RunResult
#[wasm_bindgen]
pub fn replay_verify(replay_bytes: &[u8]) -> Vec<u8> {
    let config = bincode::config::standard();

    // 反序列化 ReplayData
    let (replay, _): (ReplayData, _) = match bincode::decode_from_slice(replay_bytes, config) {
        Ok(r) => r,
        Err(_) => return Vec::new(),
    };

    // 创建 headless World
    let mut world = World::default();
    world.insert_resource(GameEventQueue::default());
    world.insert_resource(ChunkRegistry {
        seed: replay.seed,
        ..Default::default()
    });
    world.insert_resource(ChunkSpawner::default());
    world.insert_resource(WaveManager::default());
    world.insert_resource(DataStormState::default());
    world.insert_resource(HighRiskEventManager::default());
    world.insert_resource(DeltaTime(game_core::data::constants::TICK_DT));

    // spawn 玩家
    let player_entity = world
        .spawn((
            Player {
                id: 1,
                name: "Validator".into(),
            },
            Position { x: 0.0, y: 0.0 },
            Velocity { x: 0.0, y: 0.0 },
            Health {
                current: 100.0,
                max: 100.0,
            },
            Level {
                value: 1,
                xp: 0,
                xp_to_next: 100,
            },
            AimDirection { x: 1.0, y: 0.0 },
            DiscEquipped {
                disc: game_core::data::disc::make_starter_disc(),
                cooldown_timer: 0.0,
                active_timing: None,
                timing_state: TimingState::default(),
            },
            Inventory::new(2),
            ActiveBuffs::default(),
        ))
        .id();

    world.insert_resource(LocalPlayerEntity {
        entity: player_entity,
    });

    // 逐帧回放
    let player_speed = 200.0_f32;
    for frame in &replay.frames {
        // 应用输入
        if let Some(mut vel) = world.get_mut::<Velocity>(player_entity) {
            vel.x = frame.move_x * player_speed;
            vel.y = frame.move_y * player_speed;
        }
        // 执行游戏 tick
        run_game_tick(&mut world);

        // 检测玩家是否死亡
        if let Some(hp) = world.get::<Health>(player_entity) {
            if hp.current <= 0.0 {
                break;
            }
        } else {
            break;
        }
    }

    // 收集结果
    let survived_secs = replay.frames.len() as f32 * game_core::data::constants::TICK_DT;
    let (final_level, _) = world
        .get::<Level>(player_entity)
        .map(|l| (l.value, l.xp))
        .unwrap_or((1, 0));

    let result = game_core::events::RunResult {
        survived_secs,
        total_damage: 0.0, // TODO: 从事件累计
        kills_normal: 0,
        kills_elite: 0,
        kills_boss: 0,
        max_chunk_difficulty: 0.0,
        final_level,
    };

    bincode::encode_to_vec(&result, config).unwrap_or_default()
}
