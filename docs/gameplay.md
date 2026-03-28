# 游戏系统

## 核心循环

击杀怪物 → 拾取经验宝石 → 升级 → 获得更强 Disc/Program → 挑战更强怪物/Boss

## 视觉风格

Tron/Neon 赛博朋克：深色背景上的荧光几何体，HDR Bloom 自发光，高对比度霓虹线条。

- 玩家 = 发光六边形（Cyan HDR）
- 怪物 = 三角/方块（绿/紫）
- 精英 = 菱形（金色 HDR + 脉冲）
- Boss = 大号六边形（红色 HDR + 呼吸）
- 掉落物 = 旋转 + 呼吸缩放的小几何体
- 弹射体 = 高亮粒子
- 地面 = 网格 shader（Tron 风格发光网格线）

## 战斗

### Disc 系统（武器载体）

Disc = 基础属性 + Program 槽位。

基础属性：

- 射速（fire_rate）
- 能量容量（energy）
- 能量回复（energy_regen）
- 散射角度（spread）

品质决定槽位数：Common(4) → Uncommon(5) → Rare(6) → Epic(7) → Legendary(8)

### Program 类型（5 种）

| ProgramType   | 功能                                               |
| ------------- | -------------------------------------------------- |
| Projectile    | 生成弹射体（伤害、速度、大小、生命周期）           |
| TrajectoryMod | 改变弹道（Homing、Orbit、Spiral、Wave、Boomerang） |
| QuantityMod   | 齐射（一次射击多颗）                               |
| Trigger       | 触发器（OnHit、OnKill、OnExpire、OnPierce）        |
| StatusEffect  | 状态效果（Poison、Freeze、Burn、Slow）             |

### Disc 执行引擎

按槽位顺序遍历：

1. 遇到 Projectile → 结合所有 pending mods 生成弹体
2. TrajectoryMod / QuantityMod / StatusEffect → 累积到 pending 队列
3. Trigger → 收集后续 N 张 Program 作为 payload，附加到弹体

### 弹射体

由 Disc 开火产生，有伤害、速度、生命周期、尺寸属性。碰撞后触发命中反馈。

### 状态效果

- Poison / Burn：DoT 伤害
- Freeze / Slow：减速

## 怪物

- 6 种基础怪物：slime / bat / skeleton / ghost / golem / demon
- AI 状态机：Wandering → Chasing（进入 detection_radius）
- 波次系统：6 个 Wave Entry，0~600 秒递增
- Boss：高 HP，攻击模式（Area AOE / Barrage / Charge），呼吸发光效果
- 精英：增强属性，金色脉冲特效，1.3x 缩放

## HUD

- HP 条（红色）
- XP 条（黄色）
- 能量条（青色）
- 等级显示（Lv.N）
- 小地图（显示周围怪物 + Boss）
- Buff 栏（最多 8 个）
- 通知系统（顶部，淡入淡出）

## 背包与装备

- 背包容量：初始 2 格，每升 1 级扩 1 格，上限 12 格
- 等级上限：20 级
- Disc 编辑器：槽位网格 + 属性面板
- 拖拽系统：背包 ↔ Disc 槽位之间拖放
- 拖出面板区域丢弃物品

## 掉落系统

掉落物类型：DiscDrop / ProgramDrop / BuffDrop / XpOrb

权重驱动：强怪掉落 Disc 概率更高，XpOrb 最常见。Buff 同类取最高 magnitude，刷新持续时间（不叠加）。

## Run 生命周期

- Run 时长：600 秒（10 分钟）
- 死亡即结算（无复活）
- 结算数据：击杀数、存活时间、最终等级、获得物品
- 结算后通过 HTTP 同步上传服务端

## 世界印记系统

- 死亡标记：玩家死亡时自动创建，持久化到 IndexedDB
- 路标：V 键放置（每局最多 3 个），青色光柱视觉效果
- 印记类型：DeathMark / Waypoint / LootCache / DangerZone

## DPS Arena

- 60 秒计时练习场
- 固定血量目标在 (200, 0)
- 实时 DPS 统计
- Arena 结果可提交（需 Turnstile 验证）

## Buff 系统

9 种 Buff：MaxHealth、MoveSpeed、EnergyRegen、DamageBoost、ExtraSlot、FireRate、SpreadReduction、PickupRadius、XpBoost。

同类 Buff 自动合并（取最高值，刷新计时器）。

## MetaState（跨 Run 进度）

- 通过 IndexedDB 本地持久化
- 包含：解锁 Disc/Program 收藏、赛季进度、邮箱
- HTTP 同步到服务端（定时 + Run 结算后立即）

## 多语言

简体中文、English、日本語、한국어
