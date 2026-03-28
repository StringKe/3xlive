# GameState 状态守卫规则

## 状态定义

```rust
enum GameState { Menu, Playing }
```

## 守卫规则

### 始终运行（无状态守卫）

- `camera_follow_system` - 摄像机跟随
- `update_grid_uniforms` - 网格 shader 参数更新
- `grid_follow_camera` - 网格跟随摄像机

### Menu 状态

- `menu_ui_system` - 主菜单 UI（egui）
- `menu_panels_system` - 邮箱/目标面板（egui）
- `poll_auth_state` - 认证状态轮询
- `sync_poll_systems` - HTTP 同步轮询

### Playing 状态（必须 `.run_if(in_state(GameState::Playing))`）

- 所有 SimulationPlugin 系统（移动、碰撞、伤害、波次、掉落、死亡）
- 所有 RenderPlugin 游戏特效（entity visuals, damage numbers, death fragments 等）
- 所有 HUD 更新系统（HP/XP/能量条, 小地图, 通知）
- 所有输入处理系统（keyboard, mouse）
- 背包、编辑器、拖拽系统
- ImprintPlugin 系统（死亡标记、路标放置、脉冲动画）

### ArenaState 独立状态机

Arena 有独立的 `ArenaState { Inactive, Building, Fighting, Result }` 状态机，与 GameState 正交。

### HUD 可见性控制

- `Startup`：HudRoot 初始 `Visibility::Hidden`
- `OnEnter(Playing)`：设为 `Visibility::Inherited`
- `OnExit(Playing)`：设为 `Visibility::Hidden`

## 新增 System 检查

每个新 System 必须明确归属：

1. 属于哪个状态？
2. 是否需要 `run_if` 守卫？
3. 与其他 System 有顺序依赖吗？
