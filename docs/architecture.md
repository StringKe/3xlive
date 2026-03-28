# 架构

## 总览

3X Live 采用 Rust + Bevy 0.18 ECS 本地优先 WASM 架构。视觉风格为 Tron/Neon 赛博朋克：高对比度霓虹几何体、HDR Bloom 发光、深色背景上的荧光线条。

所有游戏逻辑在客户端 WASM 中运行，服务端仅负责数据持久化和验证。

```
浏览器
  ├── game-client (Rust/Bevy 0.18 → WASM, wasm-bindgen)
  │     Bevy App，Tron/Neon 风格渲染
  │     HDR + Bloom，几何体 Mesh2d
  │     本地完整游戏逻辑
  ├── JS 平台层 (i18n, auth, sync, storage, turnstile)
  └── Clerk / Guest 认证

Cloudflare Workers
  ├── Hono 路由 (HTTP API)
  └── D1 SQLite (玩家数据)
```

## Cargo Workspace

```
game-core/          # 共享层：ECS 组件、数据配置、AI、碰撞、波次
game-client/        # 客户端：Bevy App → WASM，Tron/Neon 渲染，本地完整游戏逻辑
wasm-validator/     # 回放验证器：cdylib → wasm32-unknown-unknown
```

三个 crate 通过 `[workspace.dependencies]` 统一依赖版本。game-core 定义所有共享类型。

## 客户端 (game-client)

Bevy 0.18 App，10 个 Plugin 组成完整客户端：

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

### 状态机

```rust
enum GameState { Menu, Playing }
```

- Menu：主菜单，认证按钮，开始游戏
- Playing：游戏主循环，所有游戏系统激活

### 渲染管线

Tron/Neon 视觉核心：

- HDR + Bloom（intensity=0.3）：所有实体自发光
- 实体用几何 Mesh2d 表示：六边形（玩家）、三角/方块（怪物）、菱形（精英）
- 颜色方案：Cyan HDR（本地玩家）、Red HDR（Boss）、Gold HDR（精英）
- 网格地面：GridMaterial 自定义 shader，随摄像机滚动
- 特效：命中闪光、伤害数字上浮、死亡碎片散射、掉落物旋转呼吸、精英脉冲、Boss 呼吸发光

### 平台桥接

游戏层通过 `js_bridge.rs` 消费 JS 平台能力，禁止直接访问 DOM/localStorage/fetch：

- `window.__auth`：requestTicket / openSignIn / getAuthState / signOut
- `window.__i18n`：locale / fontPath / t / setLocale
- `window.__sync`：uploadResults / pullState / submitArena / getSeasonInfo / sendMail / claimMail
- `window.__storage`：get / set / delete / keys
- `window.__turnstile`：getToken

## 共享层 (game-core)

| 模块       | 内容                                                                             |
| ---------- | -------------------------------------------------------------------------------- |
| components | Position, Velocity, Health, Level, Player, DiscEquipped, Inventory, MonsterAI 等 |
| data       | Disc, Program, ProgramType, DiscQuality, GameConfig, RON 加载                    |
| systems    | movement_system, energy_regen_system                                             |

## 平台层 (src/)

```
src/
  client/
    main.ts           # 入口：initI18n → startVersionCheck → WASM 加载
    auth-bridge.ts     # Clerk 集成 + Fingerprint → window.__auth
    i18n-bridge.ts     # locale 检测 + 翻译 JSON → window.__i18n
    sync-bridge.ts     # HTTP 同步桥接 → window.__sync
    turnstile-bridge.ts # CF Turnstile → window.__turnstile
    version-check.ts   # 轮询 /_version.json，显示更新提示
  server/
    index.ts           # Hono Workers 入口
    services/          # ticket, player 等服务
    routes/            # HTTP 路由
```

## 数据同步

本地优先架构，HTTP 同步：

- Run 结算后上传（bincode 编码）
- 后台每 300 秒自动同步
- 启动时拉取一次 MetaState
- 反作弊：CF Turnstile + wasm-validator 回放验证

## 认证

双轨认证：

- Clerk：正式账号，JWT 验证
- Guest：FingerprintJS deviceId + HMAC ticket，无需注册即玩

## 多语言

JS 层（i18n-bridge.ts）负责 locale 检测和翻译 JSON 加载，Rust 侧通过 `js_bridge::t("key")` 消费。

- 支持：en / zh-CN / ja / ko
- 字体按语言加载：`game-client/assets/fonts/{locale}.otf`（NotoSans 系列）
- 切换语言 = localStorage 写入 + location.reload()
