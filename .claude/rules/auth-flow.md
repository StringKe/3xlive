# 认证流程规则

## 双轨身份识别

Clerk 用户优先，fallback 到 FingerprintJS，再 fallback 到 localStorage 随机 UUID。

| 场景           | player_id 来源                           | 数据库操作         |
| -------------- | ---------------------------------------- | ------------------ |
| Clerk 回访     | clerk_user_id 查 DB                      | UPDATE fingerprint |
| 访客升级 Clerk | 原子迁移（batch INSERT + DELETE 旧记录） | 迁移               |
| Clerk 首次     | clerk_user_id                            | INSERT             |
| 访客回访       | fingerprint 查 DB                        | 无                 |
| 访客首次       | fingerprint                              | INSERT             |

## Ticket 机制

- 有效期 24 小时，HMAC-SHA256 签名
- 格式：`base64(payload).base64(sig)`
- 客户端通过 `window.__auth.requestTicket()` 获取
- HTTP 同步请求时携带 ticket 验证身份

## JS 桥接规则

- 所有 auth 函数通过 `wasm_bindgen(js_namespace = __auth)` 绑定
- 函数名使用 `js_name = camelCase` 映射
- `stateVersion` 版本号轮询检测 auth 状态变化，避免游戏帧内直接调 Clerk API

### `window.__auth` 接口

| 方法              | 用途                |
| ----------------- | ------------------- |
| `requestTicket()` | 请求认证 ticket     |
| `openSignIn()`    | 打开 Clerk 登录弹窗 |
| `getAuthState()`  | 获取当前认证状态    |
| `signOut()`       | 登出                |

## Clerk 主题配置

与 Tron/Neon 风格保持一致：

- colorPrimary: `#00e5ff`
- colorBackground: `#080c16`
- colorText: `#e0e0e0`

## 禁止事项

- Rust 侧禁止直接调用 Clerk SDK 或浏览器 API
- 禁止在游戏帧循环中发起 HTTP 认证请求
- 禁止硬编码 ticket 密钥到客户端代码
