# 游戏玩法设计规则

## 核心玩法

Tron/Neon 风格的单人 Roguelite 生存游戏（Vampire Survivors 风格）。玩家操控 Disc（飞碟武器）击杀怪物、收集 Program（能力卡）、升级强化、在波次中生存。本地优先架构，所有游戏逻辑在客户端 WASM 中运行。

## 武器系统（Disc + Program）

### Disc（飞碟载体）

5 档品质：Common / Uncommon / Rare / Epic / Legendary

核心属性：slots（4-8）、base_fire_rate、base_energy、base_energy_regen、base_spread

### Program 类型

| 类型          | 说明                                                       |
| ------------- | ---------------------------------------------------------- |
| Projectile    | 弹射体，含 damage/speed/lifetime/size                      |
| TrajectoryMod | 轨迹修改：Homing/Orbit/Spiral/Wave/Boomerang               |
| QuantityMod   | 多重施法：double/triple                                    |
| Trigger       | 条件触发：OnHit/OnKill/OnExpire/OnPierce，含 payload_slots |
| StatusEffect  | 状态效果：Poison/Freeze/Burn/Slow，含 duration + magnitude |

## 怪物系统

6 种基础怪物：slime / bat / skeleton / ghost / golem / demon

AI 状态机：`Wandering` → `Chasing`（进入 detection_radius）

精英（EliteMarker）：1.3x 缩放 + 脉冲动画
Boss（BossMarker）：3 种攻击模式（AreaAoe / Barrage / Charge），呼吸发光效果

## 碰撞半径

| 对象       | 半径                                |
| ---------- | ----------------------------------- |
| 弹射体命中 | 12px                                |
| 怪物接触   | 24px                                |
| 玩家命中   | 16px                                |
| 拾取       | 32px（可被 PickupRadius Buff 扩大） |

## 掉落系统

掉落物类型：DiscDrop / ProgramDrop / BuffDrop / XpOrb

权重驱动：强怪掉落 Disc 概率更高，XpOrb 最常见。Buff 同类取最高 magnitude，刷新持续时间（不叠加）。

## 等级与背包

- 等级上限：20 级
- 背包容量：初始 2 格，每升 1 级扩 1 格，上限 12 格
- 公式：`min(2 + (level - 1), 12)`

## 波次系统

6 个 Wave Entry，0~600 秒递增，spawn_rate 从 2.0 增至 5.0，max_count 从 20 增至 50。时间区间有重叠（前波未结束新波已开始）。

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

## MetaState（跨 Run 进度）

- 通过 IndexedDB 本地持久化
- 包含：解锁 Disc/Program 收藏、赛季进度、邮箱
- HTTP 同步到服务端（定时 + Run 结算后立即）

## 新增玩法要素检查

1. 数值定义在 `data/default.ron` 配置文件
2. 组件定义在 game-core
3. 逻辑在 game-client SimulationPlugin 中执行
4. 视觉表现在 game-client RenderPlugin 中实现
5. 必须符合 Tron/Neon 赛博朋克风格
