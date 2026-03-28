use bincode::{Decode, Encode};
use serde::{Deserialize, Serialize};

use crate::meta::milestone::MilestoneDefinition;

/// 游戏全局配置，RON 加载 + bincode 运行时传输
#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct GameConfig {
    pub discs: DiscTable,
    pub programs: ProgramTable,
    pub monsters: MonsterTable,
    pub loot: LootTable,
    pub waves: WaveConfig,
    #[serde(default)]
    pub milestones: Vec<MilestoneDefinition>,
}

// ─── DiscTable ───

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DiscTable {
    pub templates: Vec<DiscTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct DiscTemplate {
    pub id: String,
    pub quality: String,
    pub slots: u8,
    pub base_fire_rate: f32,
    pub base_spread: f32,
}

// ─── ProgramTable ───

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ProgramTable {
    pub templates: Vec<ProgramTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct ProgramTemplate {
    pub id: String,
    pub program_type: String,
    pub damage: Option<f32>,
    pub speed: Option<f32>,
    pub lifetime: Option<f32>,
    pub size: Option<f32>,
    pub mod_type: Option<String>,
    pub count: Option<u32>,
    pub trigger_type: Option<String>,
    pub payload_slots: Option<u8>,
    pub effect_type: Option<String>,
    pub duration: Option<f32>,
    pub magnitude: Option<f32>,
}

// ─── MonsterTable ───

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct MonsterTable {
    pub templates: Vec<MonsterTemplate>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct MonsterTemplate {
    pub id: String,
    pub hp: f32,
    pub speed: f32,
    pub damage: f32,
    pub xp_reward: u32,
    pub detection_radius: f32,
    pub traits: Vec<String>,
}

// ─── LootTable ───

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LootTable {
    pub entries: Vec<LootEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LootEntry {
    pub monster_id: String,
    pub drops: Vec<LootDropEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct LootDropEntry {
    pub item_id: String,
    pub weight: f32,
    pub min_count: u32,
    pub max_count: u32,
}

// ─── WaveConfig ───

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct WaveConfig {
    pub waves: Vec<WaveEntry>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Encode, Decode)]
pub struct WaveEntry {
    pub time_start: f32,
    pub time_end: f32,
    pub monster_ids: Vec<String>,
    pub spawn_rate: f32,
    pub max_count: u32,
}
