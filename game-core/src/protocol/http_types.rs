//! HTTP 请求/响应类型定义（Phase 5-6 使用）

use bincode::{Decode, Encode};

use crate::events::RunResult;

/// 同步请求：客户端上传局结果和统计增量
#[derive(Clone, Debug, Encode, Decode)]
pub struct SyncRequest {
    pub run_results: Vec<RunResult>,
    pub stat_increments: StatIncrement,
}

/// 统计增量
#[derive(Clone, Debug, Default, Encode, Decode)]
pub struct StatIncrement {
    pub kills: u32,
    pub damage: f32,
    pub runs: u32,
}

/// 同步响应
#[derive(Clone, Debug, Encode, Decode)]
pub struct SyncResponse {
    pub success: bool,
    pub new_trust_score: i32,
}

/// 拉取响应：服务端下发的权威状态
#[derive(Clone, Debug, Encode, Decode)]
pub struct PullResponse {
    pub meta_unlocks: Vec<u32>,
    pub season_id: u32,
    pub season_seed: u64,
}

/// 竞技场提交请求
#[derive(Clone, Debug, Encode, Decode)]
pub struct ArenaSubmitRequest {
    pub replay_data: Vec<u8>,
    pub claimed_damage: f64,
}

/// 竞技场提交响应
#[derive(Clone, Debug, Encode, Decode)]
pub struct ArenaSubmitResponse {
    pub verified: bool,
    pub actual_damage: f64,
    pub rank: u32,
}
