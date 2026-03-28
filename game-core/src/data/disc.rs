use bincode::{Decode, Encode};

use super::program::Program;

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct Disc {
    pub quality: DiscQuality,
    pub slots: Vec<Option<Program>>,
    pub base_fire_rate: f32,
    pub base_spread: f32,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Encode, Decode, serde::Serialize, serde::Deserialize,
)]
pub enum DiscQuality {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
}

impl DiscQuality {
    pub fn slot_count(&self) -> usize {
        match self {
            DiscQuality::Common => 4,
            DiscQuality::Uncommon => 5,
            DiscQuality::Rare => 6,
            DiscQuality::Epic => 7,
            DiscQuality::Legendary => 8,
        }
    }
}

/// 创建初始 Disc：指定品质，slot 0 装载基础 Projectile Program
pub fn make_starter_disc_with_quality(quality: DiscQuality) -> Disc {
    let slot_count = quality.slot_count();
    let mut slots = vec![None; slot_count];

    slots[0] = Some(Program {
        program_id: 1,
        program_type: super::program::ProgramType::Projectile,
        params: super::program::ProgramParams::Projectile {
            damage: 10.0,
            speed: 500.0,
            lifetime: 2.0,
            size: 4.0,
            pierce: 0,
            crit_chance: 0.0,
            crit_multiplier: 2.0,
            knockback: 0.0,
        },
    });

    Disc {
        quality,
        slots,
        base_fire_rate: 6.0,
        base_spread: 0.0,
    }
}

/// 创建初始 Disc：Common 品质，slot 0 装载基础 Projectile Program
pub fn make_starter_disc() -> Disc {
    let quality = DiscQuality::Common;
    let slot_count = quality.slot_count();
    let mut slots = vec![None; slot_count];

    // 基础弹射体 Program：10 damage, 500 speed, 3 fire rate
    slots[0] = Some(Program {
        program_id: 1,
        program_type: super::program::ProgramType::Projectile,
        params: super::program::ProgramParams::Projectile {
            damage: 10.0,
            speed: 500.0,
            lifetime: 2.0,
            size: 4.0,
            pierce: 0,
            crit_chance: 0.0,
            crit_multiplier: 2.0,
            knockback: 0.0,
        },
    });

    Disc {
        quality,
        slots,
        base_fire_rate: 6.0,
        base_spread: 0.0,
    }
}
