# 构建工具链规则

## TypeScript 侧：Vite+

全局 CLI 命令为 `vp`。

- **禁止直接使用 pnpm/npm/yarn**，所有包管理操作通过 `vp` 执行
- **禁止直接安装 vitest/oxlint/oxfmt/tsdown**，已内置于 vite-plus
- 正确命令：`vp test`、`vp lint`（非 `vp vitest`、`vp oxlint`）
- 所有模块从 `vite-plus` 导入：`import { defineConfig } from 'vite-plus'`
- 一次性二进制命令用 `vp dlx`，不用 `npx`

## Rust 侧：Cargo Workspace

- 所有依赖版本在 `[workspace.dependencies]` 中统一管理
- 子 crate 使用 `xxx.workspace = true` 引用
- 新增依赖时先检查是否已在 workspace 中定义

## WASM 构建

- `make wasm-all`：编译客户端 WASM（release，thin LTO）
- Debug 用 `make wasm-dev`，Release 用 `make wasm-all`
- wasm-bindgen 输出到 `src/wasm-client/`（gitignored）

## 构建 Profile

### Dev（快速迭代）

- 自身代码 opt-level=0
- 第三方依赖 opt-level=2（bevy 不优化会极慢）
- debug = "line-tables-only"

### Release（最小体积）

- opt-level="z"
- thin LTO（体积接近 fat LTO，编译快数倍）
- codegen-units=4
- strip=symbols, panic=abort

## 并行编译

- `.cargo/config.toml`：jobs = 32
- Makefile：`-j $(CARGO_JOBS)`（自动检测 CPU 核心数）
- CI：`CARGO_BUILD_JOBS=4`

## 验证流程

```bash
# Rust
cargo clippy -- -D warnings
cargo test
cargo fmt --check

# TypeScript
vp check    # 格式化 + lint + 类型检查
```
