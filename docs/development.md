# 开发指南

## 环境准备

```bash
# Rust 工具链（1.94+ stable）
rustup target add wasm32-unknown-unknown
cargo install wasm-bindgen-cli

# JS 依赖（使用 vp，禁止直接使用 pnpm/npm/yarn）
vp install
```

## 开发流程

```bash
# 1. 编译客户端 WASM（debug 模式，快速迭代）
make wasm-dev

# 2. 启动开发服务器（Vite + 本地 workerd）
make dev

# 3. 代码验证
make check              # 格式化 + lint + 类型检查（TS 侧）
cargo clippy -- -D warnings  # Rust lint
cargo test              # Rust 测试
cargo fmt --check       # Rust 格式化检查
```

## 构建模式

### Debug（开发）

```bash
make wasm-dev    # opt-level=0，第三方依赖 opt-level=2
```

- 自身代码不优化，编译快
- 第三方依赖开启优化（bevy 不优化会极慢）
- 保留行号调试信息（line-tables-only）

### Release（生产）

```bash
make wasm-all    # opt-level="z"，thin LTO
make build       # wasm-all + vp build
```

- 最小体积优化（opt-level="z"）
- thin LTO（体积接近 fat LTO，编译快数倍）
- 4 个并行 codegen 单元
- 剥离符号表 + panic=abort

### 多核编译

- `.cargo/config.toml`：`jobs = 32`
- Makefile：`-j $(CARGO_JOBS)`（自动检测 CPU 核心数）
- CI 使用 `CARGO_BUILD_JOBS=4`

## 部署

```bash
make deploy    # wasm-all + vp build + wrangler deploy
```

CI 自动部署（push to main）：编译 WASM → check → build → D1 migrations → wrangler deploy

## 新增内容检查清单

### 新增 UI 文本

1. 在 4 个 locale JSON 文件（`public/locales/`）中添加 key
2. Rust 侧通过 `js_bridge::t("key")` 查询

### 新增 ECS 组件

1. 定义在 game-core（`#[derive(Component)]` 或 `#[derive(Resource)]`）
2. 客户端使用共享类型

### 新增 Plugin

1. 实现 `impl Plugin for XxxPlugin`
2. 在 main.rs 的 `add_plugins` 中注册
3. System 使用正确的 Schedule 和 `run_if` 状态守卫

详细 ECS 检查清单见 `.claude/rules/bevy-ecs-checklist.md`。

## 基础设施

| 服务               | 用途                   |
| ------------------ | ---------------------- |
| Cloudflare Workers | HTTP API 运行时        |
| D1                 | 玩家数据               |
| Clerk              | 用户认证               |
| FingerprintJS      | 设备指纹（Guest 模式） |

## TypeScript 工具链

使用 Vite+，全局 CLI 命令为 `vp`。

- 禁止直接使用 pnpm/npm/yarn
- 禁止直接安装 vitest/oxlint/oxfmt/tsdown（已内置）
- 正确命令：`vp test`、`vp lint`（非 `vp vitest`）
- 所有模块从 `vite-plus` 导入
- 一次性二进制命令用 `vp dlx`

## 目录结构

```
game-core/              # 共享 ECS 类型、组件、数据配置
game-client/            # 客户端 Bevy App WASM（本地完整游戏逻辑）
  assets/fonts/         # 多语言字体（NotoSans 系列）
wasm-validator/         # 回放验证器 WASM
src/
  client/               # JS 平台层（入口、认证、多语言、同步、版本检查）
  server/               # CF Workers（Hono 路由、服务）
  wasm-client/          # wasm-bindgen 客户端输出（gitignored）
public/
  locales/              # 翻译 JSON（en / zh-CN / ja / ko）
  assets/fonts/         # 符号链接 → game-client/assets/fonts/
data/                   # RON 配置文件
docs/                   # 项目文档
  sdlc/                 # SDLC 产出（spec.md / plan.md）
```
