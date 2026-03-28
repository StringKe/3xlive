use bevy_ecs::prelude::*;
use bincode::{Decode, Encode};

use crate::ai::spawner::spawn_monster;
use crate::components::*;
use crate::data::constants::{
    MONSTER_COLLISION_RADIUS_SQUARE, MONSTER_DETECTION_RADIUS, MONSTER_RESPAWN_TIME,
};
use crate::resources::{ChunkRegistry, ChunkSpawnState, ChunkSpawner};
use crate::systems::DeltaTime;

// re-export 供外部通过 crate::chunk::CHUNK_SIZE / TILE_SIZE 引用
pub use crate::data::constants::{CHUNK_SIZE, TILE_SIZE};

/// Chunk 坐标
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Encode, Decode)]
pub struct ChunkCoord {
    pub x: i32,
    pub y: i32,
}

impl ChunkCoord {
    /// 从世界坐标计算所在 chunk 坐标
    pub fn from_world_pos(x: f32, y: f32) -> Self {
        Self {
            x: (x / CHUNK_SIZE).floor() as i32,
            y: (y / CHUNK_SIZE).floor() as i32,
        }
    }

    /// 返回自身及周围 8 个 chunk（共 9 个）
    pub fn neighbors(&self) -> Vec<ChunkCoord> {
        let mut result = Vec::with_capacity(9);
        for dy in -1..=1 {
            for dx in -1..=1 {
                result.push(ChunkCoord {
                    x: self.x + dx,
                    y: self.y + dy,
                });
            }
        }
        result
    }

    /// 返回 chunk 的世界坐标边界 (min_x, min_y, max_x, max_y)
    pub fn world_bounds(&self) -> (f32, f32, f32, f32) {
        let min_x = self.x as f32 * CHUNK_SIZE;
        let min_y = self.y as f32 * CHUNK_SIZE;
        (min_x, min_y, min_x + CHUNK_SIZE, min_y + CHUNK_SIZE)
    }
}

/// 生态区类型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum BiomeType {
    Cyan,
    Magenta,
    Amber,
    White,
}

/// Chunk 元数据
#[derive(Clone, Copy, Debug, Encode, Decode)]
pub struct ChunkData {
    pub difficulty: f32,
    pub biome: BiomeType,
}

/// 基于 seed 和坐标生成 chunk 数据（确定性哈希）
pub fn generate_chunk(coord: ChunkCoord, seed: u64) -> ChunkData {
    // 简单哈希：用 FNV-1a 风格混合 seed 和坐标
    let mut h = seed;
    h = h
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    h ^= coord.x as u64;
    h = h
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);
    h ^= coord.y as u64;
    h = h
        .wrapping_mul(6364136223846793005)
        .wrapping_add(1442695040888963407);

    // difficulty: 0.0 ~ 1.0
    let difficulty = ((h & 0xFFFF) as f32) / 0xFFFF as f32;

    // biome: 按高位分 4 区
    let biome = match (h >> 16) & 3 {
        0 => BiomeType::Cyan,
        1 => BiomeType::Magenta,
        2 => BiomeType::Amber,
        _ => BiomeType::White,
    };

    ChunkData { difficulty, biome }
}

/// Chunk 怪物刷新系统
pub fn chunk_spawn_system(world: &mut World) {
    let dt = world.resource::<DeltaTime>().0;

    // 收集当前 loaded chunks 及其 difficulty
    let loaded_chunks: Vec<(ChunkCoord, f32)> = {
        let registry = world.resource::<ChunkRegistry>();
        registry
            .loaded_chunks
            .iter()
            .map(|(coord, data)| (*coord, data.difficulty))
            .collect()
    };

    // 确保 ChunkSpawner 中每个 loaded chunk 都有 spawn state
    {
        let spawner = world.resource_mut::<ChunkSpawner>().into_inner();
        for (coord, difficulty) in &loaded_chunks {
            spawner.chunks.entry(*coord).or_insert_with(|| {
                let max_count = (2.0 + difficulty * 6.0).round() as u32;
                ChunkSpawnState {
                    current_count: 0,
                    max_count,
                    respawn_timer: 0.0,
                }
            });
        }
    }

    // 收集需要刷怪的 chunks
    struct SpawnRequest {
        coord: ChunkCoord,
        difficulty: f32,
    }
    let mut spawn_requests: Vec<SpawnRequest> = Vec::new();

    {
        let spawner = world.resource_mut::<ChunkSpawner>().into_inner();
        for (coord, difficulty) in &loaded_chunks {
            if let Some(state) = spawner.chunks.get_mut(coord) {
                if state.current_count < state.max_count {
                    if state.respawn_timer > 0.0 {
                        state.respawn_timer -= dt;
                    } else {
                        state.current_count += 1;
                        if state.current_count < state.max_count {
                            state.respawn_timer = MONSTER_RESPAWN_TIME;
                        }
                        spawn_requests.push(SpawnRequest {
                            coord: *coord,
                            difficulty: *difficulty,
                        });
                    }
                }
            }
        }
    }

    let seed = world.resource::<ChunkRegistry>().seed;

    for req in &spawn_requests {
        let (min_x, min_y, max_x, max_y) = req.coord.world_bounds();

        let hash = simple_hash(req.coord.x as u64, req.coord.y as u64, seed);
        let fx = (hash & 0xFFFF) as f32 / 0xFFFF as f32;
        let fy = ((hash >> 16) & 0xFFFF) as f32 / 0xFFFF as f32;
        let spawn_x = min_x + fx * (max_x - min_x);
        let spawn_y = min_y + fy * (max_y - min_y);

        let monster_type = (req.difficulty * 3.0).round().min(255.0) as u8;
        let damage = 5.0 + req.difficulty * 3.0;
        let hp = 20.0 + req.difficulty * 30.0;
        let collision_radius = MONSTER_COLLISION_RADIUS_SQUARE;

        spawn_monster(
            world,
            monster_type,
            spawn_x,
            spawn_y,
            hp,
            damage,
            MONSTER_DETECTION_RADIUS,
            collision_radius,
        );
    }
}

/// 怪物死亡时递减 chunk spawner 计数
pub fn on_monster_death(world: &mut World, monster_pos: (f32, f32)) {
    let coord = ChunkCoord::from_world_pos(monster_pos.0, monster_pos.1);
    let spawner = world.resource_mut::<ChunkSpawner>().into_inner();
    if let Some(state) = spawner.chunks.get_mut(&coord) {
        state.current_count = state.current_count.saturating_sub(1);
        if state.respawn_timer <= 0.0 {
            state.respawn_timer = MONSTER_RESPAWN_TIME;
        }
    }
}

/// 更新玩家 chunk 追踪（本地直接加载新 chunk）
pub fn update_chunks(world: &mut World) {
    let players: Vec<(u32, ChunkCoord)> = world
        .query::<(&Player, &Position)>()
        .iter(world)
        .map(|(p, pos)| (p.id, ChunkCoord::from_world_pos(pos.x, pos.y)))
        .collect();

    let seed = world.resource::<ChunkRegistry>().seed;

    for (pid, current_chunk) in &players {
        let visible = current_chunk.neighbors();
        let visible_set: std::collections::HashSet<ChunkCoord> = visible.iter().copied().collect();

        let registry = world.resource_mut::<ChunkRegistry>().into_inner();
        let prev = registry.player_chunks.entry(*pid).or_default();
        let entered: Vec<ChunkCoord> = visible_set.difference(prev).copied().collect();
        *prev = visible_set;

        for coord in entered {
            registry
                .loaded_chunks
                .entry(coord)
                .or_insert_with(|| generate_chunk(coord, seed));
        }
    }
}

/// 简单哈希函数（pub 供其他模块使用）
pub fn simple_hash(x: u64, y: u64, seed: u64) -> u64 {
    let mut h = seed;
    h = h.wrapping_mul(6364136223846793005).wrapping_add(x);
    h = h.wrapping_mul(6364136223846793005).wrapping_add(y);
    h ^ (h >> 17)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn from_world_pos_basic() {
        let c = ChunkCoord::from_world_pos(100.0, 200.0);
        assert_eq!(c, ChunkCoord { x: 0, y: 0 });

        let c = ChunkCoord::from_world_pos(1024.0, 2048.0);
        assert_eq!(c, ChunkCoord { x: 1, y: 2 });

        // 负坐标向下取整
        let c = ChunkCoord::from_world_pos(-1.0, -1.0);
        assert_eq!(c, ChunkCoord { x: -1, y: -1 });
    }

    #[test]
    fn neighbors_returns_9() {
        let c = ChunkCoord { x: 0, y: 0 };
        let n = c.neighbors();
        assert_eq!(n.len(), 9);
        assert!(n.contains(&c));
    }

    #[test]
    fn world_bounds_correct() {
        let c = ChunkCoord { x: 1, y: -1 };
        let (min_x, min_y, max_x, max_y) = c.world_bounds();
        assert_eq!(min_x, 1024.0);
        assert_eq!(min_y, -1024.0);
        assert_eq!(max_x, 2048.0);
        assert_eq!(max_y, 0.0);
    }

    #[test]
    fn generate_chunk_deterministic() {
        let coord = ChunkCoord { x: 5, y: -3 };
        let a = generate_chunk(coord, 42);
        let b = generate_chunk(coord, 42);
        assert_eq!(a.difficulty, b.difficulty);
        assert_eq!(a.biome, b.biome);
    }

    #[test]
    fn generate_chunk_varies_by_coord() {
        let seed = 12345;
        let results: Vec<ChunkData> = (0..20)
            .map(|i| generate_chunk(ChunkCoord { x: i, y: i * 7 }, seed))
            .collect();

        // 至少有一些不同的 difficulty 值
        let unique_difficulties: std::collections::HashSet<u32> = results
            .iter()
            .map(|d| (d.difficulty * 1000.0) as u32)
            .collect();
        assert!(unique_difficulties.len() > 1);
    }
}
