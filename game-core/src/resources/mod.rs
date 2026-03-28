// 共享 Resource 定义

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;

use crate::chunk::{ChunkCoord, ChunkData};
use crate::data::constants::BOSS_SPAWN_INTERVAL;

/// 本地玩家实体引用
#[derive(Resource)]
pub struct LocalPlayerEntity {
    pub entity: Entity,
}

/// Chunk 注册表
#[derive(Resource)]
pub struct ChunkRegistry {
    pub loaded_chunks: HashMap<ChunkCoord, ChunkData>,
    pub player_chunks: HashMap<u32, HashSet<ChunkCoord>>,
    pub seed: u64,
}

impl Default for ChunkRegistry {
    fn default() -> Self {
        Self {
            loaded_chunks: HashMap::new(),
            player_chunks: HashMap::new(),
            seed: 42,
        }
    }
}

/// 单个 Chunk 的刷怪状态
#[derive(Clone, Debug)]
pub struct ChunkSpawnState {
    pub current_count: u32,
    pub max_count: u32,
    pub respawn_timer: f32,
}

/// Chunk 刷怪管理器
#[derive(Resource, Default)]
pub struct ChunkSpawner {
    pub chunks: HashMap<ChunkCoord, ChunkSpawnState>,
}

/// 波次管理器
#[derive(Resource)]
pub struct WaveManager {
    pub wave_timer: f32,
    pub wave_number: u32,
    pub boss_timer: f32,
    /// 每次刷怪递增，混入伪随机种子使每波位置不同
    pub spawn_seed: u32,
}

impl Default for WaveManager {
    fn default() -> Self {
        Self {
            wave_timer: 0.0, // 首波立即触发
            wave_number: 0,
            boss_timer: BOSS_SPAWN_INTERVAL,
            spawn_seed: 0,
        }
    }
}

/// 游戏配置资源
#[derive(Resource)]
pub struct GameConfigRes {
    pub config: crate::data::GameConfig,
}

/// 视口尺寸（像素），用于动态计算怪物生成距离
#[derive(Resource, Clone)]
pub struct ViewportSize {
    pub width: f32,
    pub height: f32,
}

impl Default for ViewportSize {
    fn default() -> Self {
        // 默认 1920x1080，确保无视口信息时生成距离刚好在屏幕外
        Self {
            width: 1920.0,
            height: 1080.0,
        }
    }
}

impl ViewportSize {
    /// 屏幕半对角线长度（像素）
    pub fn half_diagonal(&self) -> f32 {
        let hw = self.width / 2.0;
        let hh = self.height / 2.0;
        (hw * hw + hh * hh).sqrt()
    }
}
