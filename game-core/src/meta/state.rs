//! Meta 运行时状态

use std::collections::{HashMap, HashSet};

use bevy_ecs::prelude::*;

use crate::data::disc::DiscQuality;
use crate::data::program::ProgramType;

/// Meta 运行时状态 Resource
#[derive(Resource, Clone, Debug)]
pub struct MetaState {
    /// 已解锁的 Program 类型
    pub unlocked_program_types: HashSet<ProgramType>,
    /// 已解锁的 Program 等级上限（type → max_level）
    pub unlocked_program_levels: HashMap<ProgramType, u32>,
    /// 已解锁的 Disc 品质选项
    pub unlocked_disc_qualities: HashSet<DiscQuality>,
    /// 里程碑进度（milestone_id → current_progress）
    pub milestone_progress: HashMap<u32, f32>,
    /// 已完成的里程碑 ID
    pub completed_milestones: HashSet<u32>,
}

impl Default for MetaState {
    fn default() -> Self {
        let mut unlocked_program_types = HashSet::new();
        unlocked_program_types.insert(ProgramType::Projectile);
        unlocked_program_types.insert(ProgramType::TrajectoryMod);
        unlocked_program_types.insert(ProgramType::QuantityMod);
        unlocked_program_types.insert(ProgramType::StatusEffect);

        let mut unlocked_disc_qualities = HashSet::new();
        unlocked_disc_qualities.insert(DiscQuality::Common);

        Self {
            unlocked_program_types,
            unlocked_program_levels: HashMap::new(),
            unlocked_disc_qualities,
            milestone_progress: HashMap::new(),
            completed_milestones: HashSet::new(),
        }
    }
}
