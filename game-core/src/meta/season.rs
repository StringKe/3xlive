//! 赛季定义

use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use super::milestone::UnlockReward;

/// 赛季定义
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct SeasonDefinition {
    pub id: u32,
    pub seed: u64,
    pub start_time: u64,
    pub end_time: u64,
    pub global_goals: Vec<SeasonGoal>,
}

/// 赛季全局目标
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub struct SeasonGoal {
    pub goal_type: SeasonGoalType,
    pub target: f64,
    pub reward: UnlockReward,
}

/// 赛季目标类型
#[derive(Clone, Debug, Serialize, Deserialize, Encode, Decode)]
pub enum SeasonGoalType {
    TotalKills,
    TotalDamage,
    BossKills,
}
