# 平台层与游戏层职责边界

## 核心原则

游戏层（Rust/Bevy WASM）负责完整游戏逻辑和渲染（本地优先）。平台能力由 JS 层提供，游戏层通过 `js_bridge` 消费结果。

## 平台层（JS/TypeScript）

| 能力       | 实现                                         | 桥接                 |
| ---------- | -------------------------------------------- | -------------------- |
| 认证       | Clerk 初始化、登录弹窗、session、ticket 请求 | `window.__auth`      |
| 多语言     | locale 检测、翻译 JSON 加载、翻译查询        | `window.__i18n`      |
| HTTP 同步  | fetch API 调用服务端 REST 端点               | `window.__sync`      |
| 存储       | IndexedDB 异步读写                           | `window.__storage`   |
| 反机器人   | CF Turnstile 无感验证                        | `window.__turnstile` |
| 版本检查   | 轮询 `/_version.json`，显示更新提示          | 纯 JS，独立运行      |
| 浏览器 API | 指纹采集、页面 reload、URL 操作              | 通过桥接间接访问     |

## 游戏层（Rust/Bevy WASM）

| 能力       | 说明                                               |
| ---------- | -------------------------------------------------- |
| 渲染       | Tron/Neon 风格几何体、HDR Bloom、Grid shader、特效 |
| 游戏逻辑   | ECS 系统、状态机、碰撞、伤害、波次、AI             |
| 输入处理   | 键盘 WASD、鼠标瞄准                                |
| 数据持久化 | bincode 编解码、MetaState 管理、Run 结算           |
| UI         | HUD、背包、法杖编辑器（Bevy UI / egui）            |

## 桥接定义

所有 JS 能力通过 `window.__xxx` 命名空间暴露。Rust 侧用 `wasm_bindgen(js_namespace = __xxx)` 绑定，定义在 `game-client/src/js_bridge.rs`。

### window.\_\_auth

```
requestTicket() → Promise<TicketResponse>
openSignIn()
openUserProfile()
getAuthState() → AuthState
signOut() → Promise
stateVersion() → u32
```

### window.\_\_i18n

```
locale() → String
fontPath() → String
t(key: &str) → String
setLocale(locale: &str)
```

### window.\_\_sync

```
uploadResults(data: Uint8Array) → Promise<JsValue>
pullState() → Promise<JsValue>
submitArena(replay: Uint8Array) → Promise<JsValue>
getSeasonInfo() → Promise<JsValue>
sendMail(data: Uint8Array) → Promise<void>
claimMail(mail_id: number) → Promise<JsValue>
```

### window.\_\_storage

```
get(key: string) → Promise<Uint8Array | null>
set(key: string, value: Uint8Array) → Promise<void>
delete(key: string) → Promise<void>
keys() → Promise<string[]>
```

### window.\_\_turnstile

```
getToken() → Promise<string>
```

## 禁止事项

- 游戏层禁止直接使用 `web_sys::WebSocket`、`web_sys::Window`、`web_sys::Location` 等浏览器 API
- 游戏层禁止直接操作 DOM、localStorage、fetch
- 游戏层禁止包含翻译文本内容（通过 key 查询）
- 游戏层禁止包含语言检测逻辑
- JS 层禁止包含游戏逻辑（ECS、物理、渲染）
- JS 层禁止直接操作 Bevy 内部状态
- JS 层禁止包含 bincode 编解码逻辑
