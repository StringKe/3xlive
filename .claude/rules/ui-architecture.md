# UI 架构规则

## InputMode 资源

UI 与游戏交互分离的核心开关：

| 模式                                    | 行为                                  |
| --------------------------------------- | ------------------------------------- |
| `GameMode`                              | WASD 移动、鼠标瞄准正常运作           |
| `UiMode { locked_aim_x, locked_aim_y }` | 背包/编辑器可见，瞄准冻结在切换前位置 |

- `Tab` / `I` 键切换
- 必须 `.run_if(in_state(GameState::Playing))` 守卫

## 面板显隐规则

- inventory 和 disc_editor 监听 `InputMode` 变化
- `UiMode` → `Visibility::Visible`
- `GameMode` → `Visibility::Hidden`
- 使用 Visibility 切换，不 despawn，保持 UI 状态

## UI 层级

| 层级       | ZIndex            | 用途                             |
| ---------- | ----------------- | -------------------------------- |
| Background | -                 | 网格地面                         |
| Game       | -                 | 游戏实体和特效                   |
| HUD        | -                 | 信息面板（HP/XP/能量条、小地图） |
| Panel      | -                 | 按需弹出（背包、编辑器）         |
| Tooltip    | ZIndex(100)       | 悬浮提示                         |
| DragGhost  | GlobalZIndex(200) | 拖拽幽灵                         |
| Menu       | -                 | 菜单界面                         |

## 全局单例 UI 模式

Tooltip 和 DragGhost 使用 Startup 创建的全局单例节点：

- 初始 `Visibility::Hidden`
- 需要时修改内容并切换可见性
- 不动态 spawn/despawn

## 拖拽系统

- `DragState` Resource 持有当前拖拽数据
- 拖拽时移动 ghost 节点位置，不 spawn 新实体
- 拖拽操作只在 `UiMode` 下生效（`GameMode` 时直接 return）

## 背包 Slot 交互

- 使用 Bevy `Interaction` 组件检测 hover/pressed
- 各 slot 挂 `InventorySlot { index }` 组件标识位置
- 不依赖 cursor position 直接计算命中

## 新增 UI 元素检查

1. 明确所属层级和 ZIndex
2. 是否需要响应 InputMode 切换
3. 是否需要 GameState 守卫
4. 优先使用全局单例模式，避免频繁 spawn/despawn
