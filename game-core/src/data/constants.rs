// ─── Tick ───

/// 游戏逻辑 Tick 频率（Hz），Bevy 0.15 FixedUpdate 默认值
pub const TICK_RATE_HZ: f32 = 64.0;
/// 每 tick 时间步长（秒）
pub const TICK_DT: f32 = 1.0 / TICK_RATE_HZ;

// ─── DoT ───

/// DoT 效果最小 tick 间隔（秒）：每 200ms 触发一次伤害，不再每帧触发
pub const DOT_TICK_INTERVAL: f32 = 0.2;
/// DoT 最小单次伤害（防止 magnitude 太小导致 0 伤害）
pub const DOT_MIN_DAMAGE: f32 = 1.0;

// ─── 玩家 ───

/// 玩家移动速度（像素/秒）
pub const PLAYER_SPEED: f32 = 160.0;
/// 玩家碰撞半径（像素）
pub const PLAYER_HIT_RADIUS: f32 = 16.0;

// ─── 怪物 ───

/// 怪物追击/游荡移动速度（像素/秒）
pub const MONSTER_SPEED: f32 = 90.0;
/// 怪物到达目标的判定距离（像素）
pub const MONSTER_ARRIVE_THRESHOLD: f32 = 16.0;
/// 怪物默认检测半径（像素）
pub const MONSTER_DETECTION_RADIUS: f32 = 512.0;
/// 怪物接触碰撞半径（像素）
pub const MONSTER_CONTACT_RADIUS: f32 = 24.0;
/// 怪物重生冷却时间（秒）
pub const MONSTER_RESPAWN_TIME: f32 = 30.0;
/// 怪物接触攻击冷却（秒）
pub const MONSTER_ATTACK_COOLDOWN: f32 = 1.0;
/// 玩家受击无敌帧时长（秒），短暂安全窗口防止密集围拢瞬杀
pub const PLAYER_CONTACT_IFRAMES: f32 = 0.15;
/// 怪物生成最小间距（像素）
pub const SPAWN_MIN_SPACING: f32 = 64.0;
/// 怪物分离力作用半径（像素），形成包围圈而非堆叠
pub const MONSTER_SEPARATION_RADIUS: f32 = 64.0;
/// 怪物分离力权重，>1 让分离力足够散开怪物群
pub const MONSTER_SEPARATION_WEIGHT: f32 = 1.2;
/// 基础怪物 HP
pub const BASE_MONSTER_HP: f32 = 30.0;
/// 基础怪物伤害
pub const BASE_MONSTER_DAMAGE: f32 = 5.0;

// ─── 怪物碰撞半径（匹配客户端视觉，内缩 1%）───

/// 精英碰撞半径（菱形 18x18 * 1.3 缩放）
pub const ELITE_COLLISION_RADIUS: f32 = 11.7 * 0.99;
/// 普通怪物碰撞半径：三角形（circumradius 14.0）
pub const MONSTER_COLLISION_RADIUS_TRIANGLE: f32 = 14.0 * 0.99;
/// 普通怪物碰撞半径：方块（22x22 half-side）
pub const MONSTER_COLLISION_RADIUS_SQUARE: f32 = 11.0 * 0.99;
/// Boss 碰撞半径（六边形 16 * 3.0 缩放）
pub const BOSS_COLLISION_RADIUS: f32 = 48.0 * 0.99;

// ─── 精英 ───

/// 精英怪物类型标识
pub const ELITE_MONSTER_TYPE: u8 = 10;
/// 精英 HP 倍率
pub const ELITE_HP_MULT: f32 = 3.0;
/// 精英伤害倍率
pub const ELITE_DAMAGE_MULT: f32 = 2.0;

// ─── Boss ───

/// Boss 怪物类型标识
pub const BOSS_MONSTER_TYPE: u8 = 20;
/// Boss HP 倍率
pub const BOSS_HP_MULT: f32 = 20.0;
/// Boss 伤害倍率
pub const BOSS_DAMAGE_MULT: f32 = 3.0;
/// Boss 检测半径（像素）
pub const BOSS_DETECTION_RADIUS: f32 = 768.0;
/// Boss 生成间隔（秒）
pub const BOSS_SPAWN_INTERVAL: f32 = 300.0;
/// Boss 生成距玩家偏移（像素）
pub const BOSS_SPAWN_OFFSET: f32 = 500.0;
/// Boss Barrage 弹射体速度（像素/秒）
pub const BOSS_BARRAGE_SPEED: f32 = 200.0;
/// Boss Barrage 弹射体数量
pub const BOSS_BARRAGE_COUNT: u32 = 5;
/// Boss Barrage 弹射体生命周期（秒）
pub const BOSS_BARRAGE_LIFETIME: f32 = 3.0;
/// Boss Barrage 弹射体碰撞大小（像素）
pub const BOSS_BARRAGE_SIZE: f32 = 12.0;
/// Boss Charge 冲锋速度（像素/秒）
pub const BOSS_CHARGE_SPEED: f32 = 350.0;

// ─── Boss 攻击配置 ───

/// Boss AreaAoe：冷却（秒）、伤害、范围半径
pub const BOSS_AOE_COOLDOWN: f32 = 5.0;
pub const BOSS_AOE_DAMAGE: f32 = 30.0;
pub const BOSS_AOE_RADIUS: f32 = 200.0;
/// Boss Barrage：冷却（秒）、伤害、扇形半角（弧度）
pub const BOSS_BARRAGE_COOLDOWN: f32 = 3.0;
pub const BOSS_BARRAGE_DAMAGE: f32 = 15.0;
pub const BOSS_BARRAGE_SPREAD: f32 = 0.5;
/// Boss Charge：冷却（秒）、伤害、冲锋宽度
pub const BOSS_CHARGE_COOLDOWN: f32 = 8.0;
pub const BOSS_CHARGE_DAMAGE: f32 = 40.0;
pub const BOSS_CHARGE_WIDTH: f32 = 48.0;

// ─── 波次 ───

/// 波次刷新间隔（秒）：高频少量，保证玩家移动时持续遇怪
pub const WAVE_INTERVAL: f32 = 2.5;
/// 每波基础怪物数量
pub const WAVE_BASE_COUNT: f32 = 8.0;
/// 每波数量增长系数（wave_number * 此值）
pub const WAVE_COUNT_GROWTH: f32 = 0.15;
/// 场上怪物数量上限
pub const MONSTER_MAX_COUNT: u32 = 300;
/// 精英出现间隔（每 N 波出一次精英）
pub const ELITE_SPAWN_WAVE_INTERVAL: u32 = 5;
/// 怪物生成半径：最小（像素），刚好超出屏幕可见范围
pub const SPAWN_RADIUS_MIN: f32 = 500.0;
/// 怪物生成半径：最大（像素），不会太远确保玩家移动时能遇到
pub const SPAWN_RADIUS_MAX: f32 = 900.0;

// ─── 弹射体 ───

/// 弹射体命中碰撞半径（像素）
pub const PROJECTILE_HIT_RADIUS: f32 = 12.0;
/// 场上弹射体数量上限
pub const MAX_PROJECTILES: u32 = 200;
/// 场上场地效果数量上限
pub const MAX_FIELD_EFFECTS: u32 = 20;

// ─── 掉落 ───

/// 掉落物 despawn 时间（秒）
pub const LOOT_DESPAWN_TIME: f32 = 30.0;
/// 拾取半径（像素），足够大让玩家不需要精确走到掉落物位置
pub const PICKUP_RADIUS: f32 = 80.0;
/// XP 磁铁吸附速度（像素/秒），无限距离，始终追踪最近玩家
pub const XP_MAGNET_SPEED: f32 = 800.0;
/// XP 球基础数量
pub const XP_ORB_BASE: f32 = 10.0;
/// XP 球难度系数
pub const XP_ORB_DIFFICULTY_MULT: f32 = 15.0;
/// Disc 掉落基础概率
pub const DISC_DROP_BASE_CHANCE: f32 = 0.20;
/// Disc 掉落难度增长
pub const DISC_DROP_DIFFICULTY_BONUS: f32 = 0.05;
/// Program 掉落基础概率
pub const PROGRAM_DROP_BASE_CHANCE: f32 = 0.50;
/// Program 掉落难度增长
pub const PROGRAM_DROP_DIFFICULTY_BONUS: f32 = 0.08;
/// Buff 掉落概率
pub const BUFF_DROP_CHANCE: f32 = 0.10;

// ─── 数据风暴 ───

/// 数据风暴伤害（HP/秒）
pub const DATA_STORM_DAMAGE: f32 = 10.0;
/// 数据风暴开始时间（秒）
pub const DATA_STORM_START_TIME: f32 = 300.0;
/// 数据风暴扩展间隔（秒）
pub const DATA_STORM_EXPANSION_INTERVAL: f32 = 10.0;
/// 数据风暴初始圈数偏移
pub const DATA_STORM_INITIAL_RING: i32 = 4;
/// 数据风暴扩展速率周期（秒）
pub const DATA_STORM_EXPANSION_RATE: f32 = 30.0;
/// 数据风暴内缩速率周期（秒）
pub const DATA_STORM_SHRINK_RATE: f32 = 10.0;

// ─── 背包 ───

/// 背包初始容量
pub const INVENTORY_INITIAL_SLOTS: usize = 24;
/// 背包最大容量
pub const INVENTORY_MAX_SLOTS: usize = 128;

// ─── 等级 ───

/// 等级上限
pub const DEFAULT_LEVEL_CAP: u32 = 20;
/// 升级所需经验基础值
pub const XP_PER_LEVEL_BASE: u32 = 100;

// ─── Chunk ───

/// Chunk 大小（像素）
pub const CHUNK_SIZE: f32 = 1024.0;
/// Tile 大小（像素）
pub const TILE_SIZE: f32 = 96.0;

// ─── entity_type 常量（用于客户端渲染层区分实体类型）───

pub const ENTITY_TYPE_PLAYER: u8 = 0;
pub const ENTITY_TYPE_REMOTE_PLAYER: u8 = 1;
pub const ENTITY_TYPE_MONSTER: u8 = 2;
pub const ENTITY_TYPE_PROJECTILE: u8 = 3;
pub const ENTITY_TYPE_LOOT: u8 = 4;
pub const ENTITY_TYPE_LOOT_DISC: u8 = 4;
pub const ENTITY_TYPE_LOOT_PROGRAM: u8 = 5;
pub const ENTITY_TYPE_LOOT_BUFF: u8 = 6;
pub const ENTITY_TYPE_ELITE: u8 = 7;
pub const ENTITY_TYPE_BOSS: u8 = 8;
