//! 协议模块
//!
//! Phase 2 清理：C2S/S2C 实时消息协议已删除（不再需要 WebSocket 二进制协议）。
//! ENTITY_TYPE_* 常量已迁移到 data/constants.rs。
//! http_types.rs 定义 Phase 5-6 的 HTTP 请求/响应类型。

pub mod http_types;

/// 从 data::constants 重导出 entity_type 常量，保持向后兼容
pub use crate::data::constants::{
    ENTITY_TYPE_BOSS, ENTITY_TYPE_ELITE, ENTITY_TYPE_LOOT, ENTITY_TYPE_LOOT_BUFF,
    ENTITY_TYPE_LOOT_DISC, ENTITY_TYPE_LOOT_PROGRAM, ENTITY_TYPE_MONSTER, ENTITY_TYPE_PLAYER,
    ENTITY_TYPE_PROJECTILE, ENTITY_TYPE_REMOTE_PLAYER,
};
