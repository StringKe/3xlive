# 数据同步协议规范

## 架构

本地优先。游戏逻辑完全在客户端 WASM 中运行，服务端仅负责数据持久化和验证。

## 序列化

- 所有持久化数据使用 `bincode` v2（Encode/Decode trait）
- HTTP 同步传输使用 bincode 编码的二进制载荷
- 翻译文件使用 JSON

## HTTP 同步端点

| 端点                   | 方法 | 用途              | 反作弊     |
| ---------------------- | ---- | ----------------- | ---------- |
| `/api/sync/upload`     | POST | 上传 Run 结算数据 | Turnstile  |
| `/api/sync/pull`       | GET  | 拉取 MetaState    | Auth token |
| `/api/arena/submit`    | POST | 提交 Arena 成绩   | Turnstile  |
| `/api/sync/season`     | GET  | 获取赛季信息      | Auth token |
| `/api/sync/mail/send`  | POST | 发送邮件          | Auth token |
| `/api/sync/mail/claim` | POST | 领取邮件附件      | Auth token |

## JS 桥接

所有 HTTP 请求通过 `window.__sync` 桥接（`src/client/sync-bridge.ts`）。

Rust 侧通过 `wasm_bindgen(js_namespace = __sync)` 绑定异步函数，使用 `WasmShared<T>` 模式将 async 结果桥接到 Bevy Resource 系统。

## 同步策略

- Run 结算后立即上传
- 后台每 300 秒自动同步
- 启动时拉取一次 MetaState
- SyncManager Resource 控制 `is_syncing` 状态，防止重复请求

## 反作弊

- CF Turnstile 无感验证：uploadResults 和 submitArena 需携带 Turnstile token
- L3 采样验证：根据 trust_score 概率性要求回放验证（5%/20%/50%）
- wasm-validator 用于服务端回放重跑验证

## 数据类型

关键持久化类型定义在 game-core：

- `SaveData`：Run 结算数据
- `MetaState`：跨 Run 进度状态
- `WorldImprint`：世界印记数据

所有类型 derive `bincode::Encode` + `bincode::Decode`。
