# 项目架构与工具链

Rust + Bevy ECS 本地优先 WASM 架构，Tron/Neon 赛博朋克荧光视觉风格。单人 Roguelite（Vampire Survivors 风格）。

- **客户端**：Rust/Bevy（game-client/），编译为 WASM，本地完整游戏逻辑 + HDR Bloom 霓虹几何体渲染
- **共享层**：game-core/，ECS 组件、数据定义、AI 系统、碰撞检测、波次管理
- **验证器**：wasm-validator/，轻量 WASM 模块，用于服务端回放验证
- **桥接层**：TypeScript（src/），CF Workers API + 平台能力（i18n、auth、sync、storage、turnstile）

## 视觉风格

Tron/Neon 赛博朋克：深色背景、高对比度霓虹几何体、HDR Bloom 自发光、发光网格地面。
实体用几何 Mesh2d 表示（六边形、三角、菱形），颜色以 Cyan 为主色调。
详见 `.claude/rules/visual-style.md`。

## Cargo Workspace 结构

```
game-core/          # 共享 ECS 类型、组件、AI、碰撞、波次、数据配置
game-client/        # Bevy App 客户端，编译为 WASM（本地完整游戏逻辑）
  assets/fonts/     # 多语言字体（en.ttf / zh-CN.otf / ja.otf / ko.otf）
wasm-validator/     # 回放验证器，cdylib → wasm32-unknown-unknown
src/
  client/           # 浏览器客户端入口
    main.ts         # 入口：i18n 初始化 → 版本检查 → WASM 客户端加载
    auth-bridge.ts  # 认证桥接（Clerk、window.__auth）
    i18n-bridge.ts  # 多语言初始化（locale 检测、翻译加载、window.__i18n）
    sync-bridge.ts  # HTTP 同步桥接（window.__sync）
    turnstile-bridge.ts # CF Turnstile 反机器人验证（window.__turnstile）
    version-check.ts # 版本检查（轮询 /_version.json）
  server/           # CF Workers 服务端
    index.ts        # Workers 入口（Hono）
    services/       # ticket.ts、player.ts 等服务
    routes/         # HTTP 路由（sync.ts、arena.ts）
  wasm-client/      # wasm-bindgen 客户端输出（gitignored）
  env.d.ts          # 环境类型声明
public/
  locales/          # 翻译 JSON（en.json / zh-CN.json / ja.json / ko.json）
  assets/fonts/     # 符号链接 → game-client/assets/fonts/（Vite 静态服务）
data/               # RON 配置文件（default.ron）
```

## 构建命令

```bash
make wasm-client      # 编译客户端 WASM + wasm-bindgen → src/wasm-client/（release）
make wasm-all         # 编译客户端 WASM（release，thin LTO）
make wasm-dev         # 编译客户端 WASM（debug，快速迭代，含 dev_log）
make wasm-validator   # 编译回放验证器 WASM（release）
make dev              # 启动 Vite 开发服务器（需先 make wasm-dev）
make build            # 生产构建（wasm-all + vp build）
make deploy           # 部署到 CF Workers
make check            # 格式化 + lint + 类型检查
make clean            # 清理编译产物
```

Cargo 命令：

```bash
cargo build                          # 编译所有 workspace crate（native）
cargo test                           # 运行所有测试
cargo clippy -- -D warnings          # lint
cargo fmt                            # 格式化
```

## TypeScript 工具链：Vite+

服务端 TypeScript 使用 Vite+，全局 CLI 命令为 `vp`。

核心规则：

- **禁止直接使用 pnpm/npm/yarn**，所有包管理操作通过 `vp` 执行
- **禁止直接安装 vitest、oxlint、oxfmt、tsdown**，这些工具已内置于 vite-plus
- **禁止使用 `vp vitest` 或 `vp oxlint`**，正确命令是 `vp test` 和 `vp lint`
- **所有模块从 `vite-plus` 导入**：`import { defineConfig } from 'vite-plus'`
- **一次性二进制命令用 `vp dlx`**，不用 `npx`

## 代码规范

- 所有代码注释和文档使用简体中文
- 技术术语、类名、方法名、变量名保持英文

## 客户端 Plugin 架构

| Plugin           | 职责                                                  |
| ---------------- | ----------------------------------------------------- |
| MenuPlugin       | 主菜单 UI、认证状态轮询                               |
| MenuPanelsPlugin | 邮箱、赛季目标面板（egui）                            |
| SimulationPlugin | 本地游戏逻辑：移动、碰撞、伤害、波次刷新、掉落        |
| SyncPlugin       | HTTP 同步：上传战绩、拉取 MetaState、定时后台同步     |
| InputPlugin      | 键盘 WASD + 鼠标瞄准 → LocalInput                     |
| RenderPlugin     | 摄像机、网格 shader、实体形状、特效渲染               |
| UiPlugin         | HUD + InventoryPlugin + DiscEditorPlugin + DragPlugin |
| ArenaPlugin      | DPS Arena 练习场（60 秒计时、伤害统计）               |
| ImprintPlugin    | 世界印记（死亡标记、路标放置、脉冲动画）              |
| AudioPlugin      | 音效系统                                              |

### GameState 状态机

```rust
enum GameState { Menu, Playing }
```

- 游戏逻辑 System 必须添加 `.run_if(in_state(GameState::Playing))`
- 始终运行：摄像机跟随、网格 shader
- HUD 通过 `OnEnter(Playing)` / `OnExit(Playing)` 控制 `Visibility`

## 平台层与游戏层边界

游戏层（Rust/Bevy WASM）负责完整游戏逻辑和渲染（本地优先）。平台能力由 JS 层提供，游戏层通过 `js_bridge` 消费。

JS 桥接命名空间：

- `window.__auth`：认证（requestTicket / openSignIn / getAuthState / signOut / auth_state_version）
- `window.__i18n`：多语言（locale / fontPath / t / setLocale）
- `window.__sync`：HTTP 同步（uploadResults / pullState / submitArena / getSeasonInfo / sendMail / claimMail）
- `window.__storage`：IndexedDB 异步存储（get / set / delete / keys）
- `window.__turnstile`：CF Turnstile 反机器人验证（getToken）

Rust 侧通过 `wasm_bindgen(js_namespace = __xxx)` 绑定，定义在 `game-client/src/js_bridge.rs`。

## 多语言系统

- JS 层（`src/i18n-bridge.ts`）负责 locale 检测、翻译 JSON 加载、`window.__i18n` 挂载
- 支持语言：en / zh-CN / ja / ko
- 字体按语言加载：`game-client/assets/fonts/{locale}.otf`（NotoSans 系列）
- 翻译 key 通过 `js_bridge::t("ui.xxx")` 在 Rust 侧消费
- 切换语言 = `localStorage.setItem("locale", newLocale)` + `location.reload()`

## 战斗系统命名

Disc/Program 体系（Tron/Neon 主题）：

| 名称        | 含义                      |
| ----------- | ------------------------- |
| Disc        | 武器载体（Tron 身份飞碟） |
| Program     | 装载在 Disc 上的能力程序  |
| DiscEditor  | 编辑器 UI                 |
| ProgramType | 程序类型                  |

## Bevy ECS 开发规范

- Plugin 粒度：每个功能域一个 Plugin，在 `App::add_plugins` 中注册
- System 调度：明确使用 `Update`、`FixedUpdate`、`Startup` 等 Schedule
- Component 必须 `#[derive(Component)]`，Resource 必须 `#[derive(Resource)]`
- WASM 禁止使用 `std::thread`、`std::sync::Mutex`、阻塞 IO
- 持久化序列化使用 `bincode` v2（Encode/Decode），与 game-core 共享类型
- AssetServer 使用 `AssetMetaCheck::Never` 避免 .meta 404
- WASM 异步状态使用 `WasmShared<T>`（`Rc<RefCell<Option<T>>>` + unsafe Send/Sync）模式桥接到 Bevy Resource

## 构建优化

- Dev：自身代码 opt-level=0，第三方依赖 opt-level=2，line-tables-only debug
- Release：opt-level="z"（最小体积），thin LTO，codegen-units=4，strip=symbols，panic=abort
- `.cargo/config.toml` 配置 jobs=32 并行前端编译

## 开始任务前检查清单

- [ ] 拉取远程变更后执行 `vp install`
- [ ] `make wasm-all` 编译通过
- [ ] `cargo clippy -- -D warnings` 无 lint 警告
- [ ] `cargo test` 全部通过
- [ ] 修改 game-core 后同时检查 game-client 是否需要更新
- [ ] 修改服务端 TS 代码后执行 `vp check` 验证
