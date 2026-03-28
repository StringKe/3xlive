//! 世界印记类型定义
//!
//! 玩家死亡、放置路标等事件产生的持久化标记，
//! 在其他玩家的世界中异步展示。

use bincode::{Decode, Encode};

/// 印记类型
#[derive(Clone, Debug, PartialEq, Eq, Encode, Decode)]
pub enum ImprintType {
    /// 死亡标记
    DeathMark,
    /// 危险区域
    DangerZone,
    /// 路标
    Waypoint,
    /// 宝箱缓存
    LootCache,
}

/// 世界印记数据（用于网络传输和 IndexedDB 持久化）
#[derive(Clone, Debug, Encode, Decode)]
pub struct WorldImprint {
    pub imprint_type: ImprintType,
    pub x: f32,
    pub y: f32,
    pub creator_id: u32,
    pub data: String,
    /// 创建时间戳（毫秒）
    pub timestamp: u64,
}
