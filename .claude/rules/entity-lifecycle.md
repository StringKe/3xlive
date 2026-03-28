# 实体生命周期规则

## 本地优先架构

所有实体由客户端 SimulationPlugin 本地创建和管理。无服务端同步，无 EntityMap。

## Spawn 规则

1. 玩家实体：`OnEnter(Playing)` 时由 `spawn_local_player` 创建，挂载 `LocalPlayerMarker`
2. 怪物实体：由 `wave_spawner_system` 按波次配置本地生成，挂载 `MonsterComp` + `MonsterAI`
3. 弹射体：由 `disc_fire_system` 本地生成，挂载 `ProjectileComp`
4. 掉落物：怪物死亡时由 `loot_drop_system` 本地生成，挂载 `LootDrop`

### Entity Marker 组件

| 实体类型 | Marker 组件                 |
| -------- | --------------------------- |
| 本地玩家 | `LocalPlayerMarker`         |
| 怪物     | `MonsterComp` + `MonsterAI` |
| 精英怪物 | `EliteClientMarker`         |
| Boss     | `BossClientMarker`          |
| 弹射体   | `ProjectileComp`            |
| 掉落物   | `LootDrop` + `LootMarker`   |

## Despawn 规则

1. 怪物：HP <= 0 时由 `death_system` despawn，触发 GameEvent::EntityKilled
2. 弹射体：`remaining_lifetime <= 0` 时 despawn
3. 掉落物：`despawn_timer <= 0` 或被拾取时 despawn
4. 使用 `commands.entity(e).despawn()`（单实体，不用 `despawn_recursive`）

## 渲染组件附加

渲染组件通过 `Added<T>` Query 自动附加：

- `attach_player_visual`：检测 `Added<LocalPlayerMarker>`
- `attach_monster_visual`：检测 `Added<MonsterComp>`
- `attach_projectile_visual`：检测 `Added<ProjectileComp>`
- `attach_loot_visual`：检测 `Added<LootDrop>`

## Run 生命周期

- `OnEnter(Playing)`：spawn 玩家、初始化 RunState、重置波次
- Playing 状态：完整游戏循环（移动、碰撞、伤害、波次、掉落）
- Run 结束（死亡/时间到）：结算 → 写入 MetaState → 触发同步上传
- `OnExit(Playing)`：清理所有游戏实体

## 新增实体类型检查

1. 在 game-core 定义组件
2. 在 SimulationPlugin 添加 spawn 逻辑
3. 在 RenderPlugin 添加 `attach_xxx_visual` 系统
4. 确认 despawn 条件和清理方式
