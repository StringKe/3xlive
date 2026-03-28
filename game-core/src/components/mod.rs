use bevy_ecs::component::Component;
use bincode::{Decode, Encode};

use crate::data::constants::{INVENTORY_INITIAL_SLOTS, INVENTORY_MAX_SLOTS};
use crate::data::{Disc, FieldType, Program, StatusEffectType, TimingType, TrajectoryType};
use crate::disc_engine::{DamageModInstance, FieldModInstance, ProjectileModInstance};

// ─── 怪物 AI 组件 ───

/// AI 状态枚举
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum AIState {
    /// 随机游荡
    Wandering,
    /// 追击玩家
    Chasing,
    /// 逃跑
    Fleeing,
}

/// 怪物 AI 组件
#[derive(Component, Clone, Debug, Encode, Decode)]
pub struct MonsterAI {
    pub state: AIState,
    /// 追击目标的 player_id
    pub target: Option<u32>,
    /// 游荡目标坐标
    pub wander_target_x: f32,
    pub wander_target_y: f32,
    /// 检测半径（像素）
    pub detection_radius: f32,
}

/// 世界坐标位置
#[derive(Component, Clone, Copy, Debug, Encode, Decode)]
pub struct Position {
    pub x: f32,
    pub y: f32,
}

/// 速度向量
#[derive(Component, Clone, Copy, Debug, Encode, Decode)]
pub struct Velocity {
    pub x: f32,
    pub y: f32,
}

/// 玩家标识
#[derive(Component, Clone, Debug, Encode, Decode)]
pub struct Player {
    pub id: u32,
    pub name: String,
}

/// 瞄准方向（单位向量）
#[derive(Component, Clone, Copy, Debug, Encode, Decode)]
pub struct AimDirection {
    pub x: f32,
    pub y: f32,
}

/// 生命值
#[derive(Component, Clone, Copy, Debug, Encode, Decode)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

/// 等级与经验
#[derive(Component, Clone, Copy, Debug, Encode, Decode)]
pub struct Level {
    pub value: u32,
    pub xp: u32,
    pub xp_to_next: u32,
}

/// 实体类型标记
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum EntityType {
    LocalPlayer,
    RemotePlayer,
    Monster,
    Projectile,
    Loot,
    Buff,
}

/// 弹射体组件
#[derive(Component, Clone, Debug)]
pub struct ProjectileComp {
    pub damage: f32,
    pub lifetime: f32,
    pub remaining_lifetime: f32,
    pub size: f32,
    pub owner_id: u32,
    /// 穿透次数
    pub pierce: u8,
    /// 剩余命中次数（初始值 = pierce + 1）
    pub hits_remaining: u8,
    /// 暴击率
    pub crit_chance: f32,
    /// 暴击倍率
    pub crit_multiplier: f32,
    /// 击退力度
    pub knockback: f32,
    /// 伤害修饰列表
    pub damage_mods: Vec<DamageModInstance>,
    /// 弹射体修饰列表
    pub projectile_mods: Vec<ProjectileModInstance>,
    /// 场地修饰列表
    pub field_mods: Vec<FieldModInstance>,
    /// 命中时施加的状态效果 (type, duration, magnitude)
    pub applied_effects: Vec<(StatusEffectType, f32, f32)>,
    /// 轨迹修正列表
    pub trajectory_mods: Vec<TrajectoryType>,
    /// 已命中的实体列表（pierce 用）
    pub hit_entities: Vec<bevy_ecs::entity::Entity>,
    /// Trigger payload：命中时触发
    pub on_hit: Option<Vec<Program>>,
    /// Trigger payload：击杀时触发
    pub on_kill: Option<Vec<Program>>,
    /// Trigger payload：过期时触发
    pub on_expire: Option<Vec<Program>>,
    /// Trigger payload：暴击时触发
    pub on_crit: Option<Vec<Program>>,
    /// Trigger payload：命中 Boss 时触发
    pub on_boss_hit: Option<Vec<Program>>,
    /// Trigger payload：周期性触发
    pub on_periodic: Option<Vec<Program>>,
}

/// 怪物组件
#[derive(Component, Clone, Debug, Encode, Decode)]
pub struct MonsterComp {
    pub monster_type: u8,
    pub damage: f32,
}

/// 自动瞄准无目标标记：射程内无敌人时插入，阻止 disc_fire_system 开火
#[derive(Component)]
pub struct AutoAimNoTarget;

/// 单个状态效果实例
#[derive(Clone, Debug)]
pub struct ActiveEffect {
    pub effect_type: StatusEffectType,
    pub remaining: f32,
    pub magnitude: f32,
    /// GravityWell 锚点坐标
    pub anchor: Option<(f32, f32)>,
    /// DoT tick 计时器：累积到 DOT_TICK_INTERVAL 时触发一次伤害
    pub dot_timer: f32,
}

/// 状态效果容器组件
#[derive(Component, Clone, Debug, Default)]
pub struct StatusEffects {
    pub effects: Vec<ActiveEffect>,
}

/// Disc 开火时序状态（Burst/Gatling 等 TimingMod 使用）
#[derive(Clone, Debug)]
pub struct TimingState {
    /// Burst 连发计数（连发 3 次后进入长冷却）
    pub burst_count: u8,
    /// Gatling 冷却倍率（每次开火后递减，最低 0.1）
    pub gatling_cooldown_mult: f32,
}

impl Default for TimingState {
    fn default() -> Self {
        Self {
            burst_count: 0,
            gatling_cooldown_mult: 1.0,
        }
    }
}

/// Disc 装备组件
#[derive(Component, Clone, Debug)]
pub struct DiscEquipped {
    pub disc: Disc,
    pub cooldown_timer: f32,
    /// 当前 disc 的 TimingType（缓存，换 disc 时更新）
    pub active_timing: Option<TimingType>,
    /// TimingMod 运行时状态
    pub timing_state: TimingState,
}

// ─── 掉落系统组件 ───

/// 掉落物组件
#[derive(Component, Clone, Debug, Encode, Decode)]
pub struct LootDrop {
    pub item: LootItem,
    pub despawn_timer: f32,
}

/// 掉落物类型
#[derive(Clone, Debug, Encode, Decode)]
pub enum LootItem {
    DiscDrop(Disc),
    ProgramDrop(Program),
    BuffDrop(BuffType),
    XpOrb(u32),
}

/// Buff 类型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum BuffType {
    MaxHealth,
    MoveSpeed,
    DamageBoost,
    ExtraSlot,
    FireRate,
    SpreadReduction,
    PickupRadius,
    XpBoost,
}

/// 背包组件
#[derive(Component, Clone, Debug)]
pub struct Inventory {
    pub slots: Vec<Option<InventoryItem>>,
    pub max_slots: usize,
}

/// 背包物品
#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum InventoryItem {
    Disc(Disc),
    Program(Program),
}

impl Inventory {
    pub fn new(max_slots: usize) -> Self {
        Self {
            slots: vec![None; max_slots],
            max_slots,
        }
    }

    /// 尝试添加物品到第一个空位，成功返回 slot index
    pub fn try_add(&mut self, item: InventoryItem) -> Option<usize> {
        for (i, slot) in self.slots.iter_mut().enumerate() {
            if slot.is_none() {
                *slot = Some(item);
                return Some(i);
            }
        }
        None
    }

    /// 根据等级扩展背包容量：2 + (level - 1)，最大 12
    pub fn expand_for_level(&mut self, level: u32) {
        let new_max =
            (INVENTORY_INITIAL_SLOTS + level.saturating_sub(1) as usize).min(INVENTORY_MAX_SLOTS);
        if new_max > self.max_slots {
            self.slots.resize(new_max, None);
            self.max_slots = new_max;
        }
    }
}

// ─── 精英与 Boss 标记 ───

/// 精英怪物标记
#[derive(Component, Clone, Copy, Debug)]
pub struct EliteMarker;

/// Boss 标记
#[derive(Component, Clone, Copy, Debug)]
pub struct BossMarker;

/// 碰撞半径（用于碰撞检测）
#[derive(Component)]
pub struct CollisionRadius(pub f32);

/// 怪物接触攻击冷却计时器
#[derive(Component)]
pub struct ContactAttackTimer {
    pub remaining: f32,
}

/// 玩家受击无敌帧（iframes），受到接触伤害后短暂无敌
#[derive(Component, Default)]
pub struct ContactIframes {
    pub remaining: f32,
}

/// Boss 攻击模式类型
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BossPattern {
    /// 以 Boss 为中心的圆形伤害区域
    AreaAoe,
    /// 朝目标扇形发射多个弹射体
    Barrage,
    /// 朝目标直线冲锋，路径上造成伤害
    Charge,
}

/// 单个 Boss 攻击
#[derive(Clone, Debug)]
pub struct BossAttack {
    pub pattern_type: BossPattern,
    pub cooldown: f32,
    pub cooldown_timer: f32,
    pub damage: f32,
    /// AreaAoe: 半径, Barrage: 扩散角度, Charge: 冲锋宽度
    pub radius: f32,
}

/// Boss 攻击模式组件
#[derive(Component, Clone, Debug)]
pub struct BossAttackPattern {
    pub patterns: Vec<BossAttack>,
}

// ─── 场地效果组件 ───

/// 场地效果组件：在地面创建持续区域
#[derive(Component, Clone, Debug)]
pub struct FieldEffect {
    pub field_type: FieldType,
    pub radius: f32,
    pub remaining_duration: f32,
    pub magnitude: f32,
    pub owner_id: u32,
}

// ─── 经验宝石磁吸 ───

/// 经验宝石磁吸标记：自动追踪最近玩家
#[derive(Component, Clone, Debug, Encode, Decode)]
pub struct XpMagnet {
    /// 追踪速度（像素/秒）
    pub speed: f32,
}

// ─── 限时房间组件 ───

/// 缩圈状态
#[derive(Clone, Debug, Encode, Decode)]
pub struct ShrinkZoneState {
    pub center_x: f32,
    pub center_y: f32,
    pub current_radius: f32,
    pub target_radius: f32,
    pub shrink_speed: f32,
    /// 圈外每秒伤害
    pub damage_per_sec: f32,
}

// ─── Buff 系统组件 ───

/// 单个 buff 实例
#[derive(Clone, Debug)]
pub struct BuffInstance {
    pub buff_type: BuffType,
    pub magnitude: f32,
    pub remaining: f32,
}

/// 活跃 buff 容器组件
#[derive(Component, Clone, Debug, Default)]
pub struct ActiveBuffs {
    pub buffs: Vec<BuffInstance>,
}

impl ActiveBuffs {
    /// 添加 buff，同类取最高值（刷新持续时间）
    pub fn add(&mut self, buff_type: BuffType, magnitude: f32, duration: f32) {
        if let Some(existing) = self.buffs.iter_mut().find(|b| b.buff_type == buff_type) {
            if magnitude >= existing.magnitude {
                existing.magnitude = magnitude;
                existing.remaining = duration;
            } else {
                existing.remaining = existing.remaining.max(duration);
            }
        } else {
            self.buffs.push(BuffInstance {
                buff_type,
                magnitude,
                remaining: duration,
            });
        }
    }

    /// 获取指定类型 buff 的 magnitude，无则返回 0
    pub fn get_magnitude(&self, buff_type: BuffType) -> f32 {
        self.buffs
            .iter()
            .filter(|b| b.buff_type == buff_type)
            .map(|b| b.magnitude)
            .fold(0.0_f32, f32::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bevy_ecs::world::World;

    /// 6B.5: 弹射体命中目标 → Health 减少
    #[test]
    fn projectile_hit_reduces_health() {
        let mut world = World::new();

        let monster = world
            .spawn((
                Position { x: 100.0, y: 100.0 },
                Health {
                    current: 50.0,
                    max: 50.0,
                },
                MonsterComp {
                    monster_type: 1,
                    damage: 5.0,
                },
            ))
            .id();

        // 模拟 apply_damage（与 game-server 中相同逻辑）
        let damage = 10.0_f32;
        if let Some(mut hp) = world.get_mut::<Health>(monster) {
            hp.current = (hp.current - damage).max(0.0);
        }

        let hp = world.get::<Health>(monster).unwrap();
        assert_eq!(hp.current, 40.0);
        assert_eq!(hp.max, 50.0);
    }

    /// Health 不会低于 0
    #[test]
    fn health_does_not_go_negative() {
        let mut world = World::new();
        let entity = world
            .spawn(Health {
                current: 5.0,
                max: 100.0,
            })
            .id();

        if let Some(mut hp) = world.get_mut::<Health>(entity) {
            hp.current = (hp.current - 50.0).max(0.0);
        }

        let hp = world.get::<Health>(entity).unwrap();
        assert_eq!(hp.current, 0.0);
    }
}
