//! 本地存档数据结构（IndexedDB 持久化）

use bincode::{Decode, Encode};

/// 本地存档
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct SaveData {
    pub meta_unlocks: MetaUnlocks,
    pub milestone_progress: MilestoneProgress,
    pub season_stats: SeasonStats,
    pub current_season_id: u32,
}

/// Meta 解锁状态
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct MetaUnlocks {
    /// 已解锁的 program type ID 列表
    pub unlocked_program_types: Vec<u32>,
    /// 已解锁的 disc quality 等级（0=Common .. 4=Legendary）
    pub unlocked_disc_qualities: Vec<u8>,
}

/// 里程碑进度
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct MilestoneProgress {
    /// (milestone_id, current_progress) 列表
    pub entries: Vec<(u32, f32)>,
}

/// 赛季统计
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct SeasonStats {
    pub total_kills: u32,
    pub total_damage: f32,
    pub total_runs: u32,
}
