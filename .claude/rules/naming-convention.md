# 命名规范

## 代码语言

- 代码注释和文档使用简体中文
- 类名、方法名、变量名、枚举值保持英文
- locale key 使用英文点分路径：`ui.xxx.yyy`

## 战斗系统命名

Disc/Program 体系（Tron/Neon 主题）：

| 名称             | 语义                        |
| ---------------- | --------------------------- |
| Disc             | 武器载体，Tron 经典身份飞碟 |
| DiscQuality      | 飞碟品质等级                |
| DiscEquipped     | 已装备的飞碟组件            |
| DiscEditor       | UI 编辑器面板               |
| disc_engine      | 射击执行引擎                |
| disc_fire_system | 开火系统                    |
| Program          | 装载在 Disc 上的能力程序    |
| ProgramType      | 程序类型枚举                |
| ProgramParams    | 程序参数                    |

### 协议消息

| 消息          | 含义                 |
| ------------- | -------------------- |
| InsertProgram | 将 Program 装入 Disc |
| RemoveProgram | 从 Disc 卸载 Program |
| SwapDisc      | 切换装备的 Disc      |

### 客户端类型

| 类型          | 含义                |
| ------------- | ------------------- |
| ClientDisc    | 客户端 Disc 数据    |
| ClientProgram | 客户端 Program 数据 |

### locale key

| key 前缀           | 用途           |
| ------------------ | -------------- |
| ui.disc_editor.\*  | 编辑器面板文本 |
| ui.program_type.\* | 程序类型名称   |

## 实体命名

| 实体类型 | 英文         | 说明 |
| -------- | ------------ | ---- |
| 本地玩家 | LocalPlayer  |      |
| 远程玩家 | RemotePlayer |      |
| 怪物     | Monster      |      |
| 弹射体   | Projectile   |      |
| 掉落物   | Loot         |      |
| Buff     | Buff         |      |

## UI 层级命名

| 层级       | 说明                         |
| ---------- | ---------------------------- |
| Background | 网格地面，始终渲染           |
| Game       | 游戏实体和特效，Playing 状态 |
| HUD        | 信息面板，Playing 状态       |
| Panel      | 按需弹出（背包、编辑器）     |
| Menu       | 菜单界面，Menu 状态          |
