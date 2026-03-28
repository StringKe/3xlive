use bincode::{Decode, Encode};

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub struct Program {
    pub program_id: u32,
    pub program_type: ProgramType,
    pub params: ProgramParams,
}

#[derive(
    Clone, Copy, Debug, PartialEq, Eq, Hash, Encode, Decode, serde::Serialize, serde::Deserialize,
)]
pub enum ProgramType {
    Projectile,
    TrajectoryMod,
    QuantityMod,
    Trigger,
    StatusEffect,
    FormationMod,
    TimingMod,
    DamageMod,
    ProjectileMod,
    FieldMod,
}

#[derive(Clone, Debug, PartialEq, Encode, Decode)]
pub enum ProgramParams {
    Projectile {
        damage: f32,
        speed: f32,
        lifetime: f32,
        size: f32,
        /// 穿透次数（0 = 不穿透，命中后销毁）
        pierce: u8,
        /// 暴击概率（0.0 ~ 1.0）
        crit_chance: f32,
        /// 暴击伤害倍率
        crit_multiplier: f32,
        /// 击退力度（像素/秒）
        knockback: f32,
    },
    TrajectoryMod {
        mod_type: TrajectoryType,
    },
    QuantityMod {
        count: u32,
    },
    Trigger {
        trigger_type: TriggerType,
        payload_slots: u8,
    },
    StatusEffect {
        effect_type: StatusEffectType,
        duration: f32,
        magnitude: f32,
    },
    FormationMod {
        formation: FormationType,
    },
    TimingMod {
        timing: TimingType,
    },
    DamageMod {
        mod_type: DamageModType,
        value: f32,
    },
    ProjectileMod {
        mod_type: ProjectileModType,
        value: f32,
        count: u8,
    },
    FieldMod {
        field_type: FieldType,
        radius: f32,
        duration: f32,
        magnitude: f32,
    },
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum TrajectoryType {
    Homing,
    Orbit,
    Spiral,
    Wave,
    Boomerang,
    /// 碰壁/碰敌后反弹改变方向
    Ricochet,
    /// 锯齿形左右交替折线
    Zigzag,
    /// 飞行中逐渐加速
    Accelerate,
    /// 飞行中逐渐减速
    Decelerate,
    /// 受重力影响的抛物线
    Gravity,
    /// 发射时随机散布多方向
    Scatter,
    /// 飞行一段距离后瞬移到目标位置
    Teleport,
    /// 围绕锁定目标公转
    OrbitTarget,
    /// 垂直方向正弦波动
    SineVertical,
    /// 钟摆式左右摆动
    Pendulum,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum TriggerType {
    OnHit,
    OnKill,
    OnExpire,
    OnPierce,
    /// 暴击时触发
    OnCrit,
    /// 命中 Boss 时触发
    OnBossHit,
    /// 自身低血量时触发
    OnLowHp,
    /// 周期性自动触发
    Periodic,
    /// 闪避时触发
    OnDodge,
    /// 施加状态效果时触发
    OnStatusApply,
    /// 连续击杀时触发
    OnMultiKill,
    /// 满血时触发
    OnFullHp,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum StatusEffectType {
    Poison,
    Freeze,
    Burn,
    Slow,
    /// 削弱：降低目标伤害输出
    Weaken,
    /// 感电：周期性小额伤害 + 短暂眩晕
    Shock,
    /// 流血：移动时额外受伤
    Bleed,
    /// 诅咒：受到的治疗效果降低
    Curse,
    /// 标记：受到的伤害增加
    Mark,
    /// 恐惧：强制远离施法者
    Fear,
    /// 腐蚀：持续降低护甲
    Corrode,
    /// 混乱：移动方向随机偏移
    Confuse,
    /// 致盲：缩小检测/瞄准范围
    Blind,
    /// 引力井：持续拉向效果中心
    GravityWell,
    /// 末日：倒计时结束时造成大量伤害
    Doom,
}

/// 阵型类型：控制多发弹射体的空间排列
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum FormationType {
    /// 扇形散射
    Fan,
    /// 平行排列
    Parallel,
    /// 十字形
    Cross,
    /// 星形放射
    Star,
    /// 环形均匀分布
    Ring,
    /// 横排墙
    Wall,
    /// 锥形聚拢
    Cone,
    /// 随机方向
    Random,
    /// 螺旋阵型
    SpiralForm,
    /// 背射（向后方发射）
    Backshot,
}

/// 时序类型：控制弹射体的发射节奏
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum TimingType {
    /// 瞬间齐射
    Burst,
    /// 依次快速连发
    Salvo,
    /// 延迟发射
    Delayed,
    /// 交替轮流
    Alternating,
    /// 蓄力后发射
    Charge,
    /// 转管加速连射
    Gatling,
    /// 心跳节奏（快快慢）
    Heartbeat,
    /// 双连击
    DoubleTap,
}

/// 伤害修正类型：被动修改伤害计算
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum DamageModType {
    /// 生命偷取
    Lifesteal,
    /// 护甲穿透
    ArmorPen,
    /// 对精英额外伤害
    BonusVsElite,
    /// 对 Boss 额外伤害
    BonusVsBoss,
    /// 斩杀：低血量目标额外伤害
    Execute,
    /// 固定伤害加成
    FlatDamage,
    /// 百分比伤害加成
    DamagePercent,
    /// 暴击率提升
    CritBoost,
    /// 暴击伤害提升
    CritDamage,
    /// 溢出伤害转移给附近敌人
    Overkill,
    /// 连续命中递增伤害
    Ramp,
    /// 玻璃大炮：大幅增伤但降低自身防御
    GlassCannon,
}

/// 弹射体行为修正类型
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum ProjectileModType {
    /// 链式弹跳到附近敌人
    Chain,
    /// 命中后分裂为多个子弹
    Split,
    /// 命中后爆炸造成范围伤害
    Explode,
    /// 飞到最远距离后返回
    Return,
    /// 飞行中逐渐增大
    GrowSize,
    /// 飞行中逐渐缩小
    ShrinkSize,
    /// 销毁时碎裂为小弹片
    Fragment,
    /// 碰壁反弹
    Bounce,
    /// 穿透敌人（不消耗穿透次数）
    Ghost,
    /// 附着在敌人身上持续伤害
    Attach,
    /// 飞行中复制自身
    Clone,
    /// 命中后环绕目标公转
    OrbitHit,
}

/// 场地效果类型：在地面创建持续区域
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum FieldType {
    /// 伤害区域
    DamageZone,
    /// 减速场
    SlowField,
    /// 治疗区域
    HealZone,
    /// 引力牵引
    GravityPull,
    /// 地雷阵
    MineField,
    /// 护盾区域
    Shield,
    /// 信标（增益光环）
    Beacon,
    /// 漩涡（持续拉入并伤害）
    Vortex,
}

/// 元素反应类型：两种状态效果同时存在时触发
#[derive(Clone, Copy, Debug, PartialEq, Eq, Encode, Decode)]
pub enum ReactionType {
    /// 冰+电 → 粉碎：大范围伤害
    Shatter,
    /// 火+冰 → 融化：伤害倍增
    Melt,
    /// 毒+火 → 毒雾：范围 DoT
    Toxic,
    /// 血+毒 → 脓毒：持续伤害加速
    Sepsis,
    /// 咒+弱 → 撕裂：护甲归零
    Shred,
    /// 减速+引力 → 黑洞：强吸引+持续伤害
    BlackHole,
    /// 混乱+恐惧 → 恐慌：范围恐惧
    Panic,
}
