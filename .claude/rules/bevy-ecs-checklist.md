# Bevy ECS 开发检查清单

## Plugin

- [ ] `impl Plugin for XxxPlugin { fn build(&self, app: &mut App) }` 实现完整
- [ ] 在 `main.rs` 或父 Plugin 的 `build` 中注册 `app.add_plugins(XxxPlugin)`
- [ ] 内部通过 `app.add_systems`、`app.add_message`、`app.init_resource` 注册
- [ ] 有依赖顺序时用 `.before()` / `.after()` 约束

## System

- [ ] 指定正确 Schedule：`Update`（每帧）、`FixedUpdate`（固定步长）、`Startup`（启动一次）
- [ ] 游戏逻辑 System 添加 `.run_if(in_state(GameState::Playing))` 守卫
- [ ] `OnEnter(State)` / `OnExit(State)` 用于状态转换时的一次性操作
- [ ] 读写同一 Component 的 System 之间有明确顺序约束
- [ ] 高频 System 避免堆内存分配（Vec、String）
- [ ] 参数超过 5 个时拆分或用 SystemParam 聚合

## GameState 守卫规则

```
Menu 状态：菜单 UI + 认证轮询 + 同步轮询
Playing 状态：所有游戏逻辑、HUD、特效、输入处理
```

**始终运行**：摄像机跟随、网格 shader 更新、网格跟随摄像机
**Playing 限定**：实体渲染、特效、HUD 更新、输入、背包/编辑器、拖拽、印记系统

## Component

- [ ] `#[derive(Component)]`
- [ ] despawn 使用 `commands.entity(e).despawn_recursive()`（有子实体时）
- [ ] 删除 Component 用 `commands.entity(e).remove::<T>()`
- [ ] 跨帧临时状态用 Component，非局部变量
- [ ] Observer / Trigger：在 Plugin.build 中 `app.add_observer`

## Resource

- [ ] `#[derive(Resource)]`
- [ ] 启动时初始化：`app.insert_resource()` 或 `app.init_resource::<T>()`
- [ ] 可选 Resource 用 `Option<Res<T>>`
- [ ] 不存储大量 Entity 引用（跨帧可能失效）

## Message（Bevy 0.18 Event→Message 重命名）

- [ ] `#[derive(Event)]`
- [ ] 注册：`app.add_message::<XxxEvent>()`
- [ ] 发送 `MessageWriter<T>`，读取 `MessageReader<T>`
- [ ] Message 2 帧后清除，跨帧持久化用 Resource 或 Component

## WASM 兼容性（game-client）

- [ ] 禁止 `std::thread::spawn`
- [ ] 禁止 `std::sync::Mutex` / `RwLock`（用 `RefCell` / `Cell`）
- [ ] 禁止阻塞 IO（`std::fs`、`std::net`）
- [ ] 禁止 `std::time::Instant`（用 `web_sys::window().performance().now()`）
- [ ] 依赖 crate 确认支持 `wasm32-unknown-unknown`
- [ ] 异步状态桥接使用 `WasmShared<T>` 模式（`Rc<RefCell<Option<T>>>` + unsafe Send/Sync）

## 编译检查

```bash
cargo build -p game-core
cargo build -p game-client --target wasm32-unknown-unknown
cargo build -p wasm-validator --target wasm32-unknown-unknown
cargo test
cargo clippy -- -D warnings
cargo fmt --check
```
