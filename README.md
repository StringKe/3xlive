# 3X Live

Vampire Survivors 风格的单人 Roguelite 生存游戏，打开浏览器即玩。本地优先架构。

**[3xlive.app](https://3xlive.app)**

## 技术栈

| 层         | 选型                                             |
| ---------- | ------------------------------------------------ |
| 游戏客户端 | Rust + Bevy 0.18 ECS，编译为 WASM (wasm-bindgen) |
| 共享层     | game-core (ECS 组件、数据定义)                   |
| 验证器     | wasm-validator (回放验证 WASM)                   |
| 渲染风格   | Tron/Neon 赛博朋克                               |
| UI 框架    | bevy_egui 0.39 (egui)                            |
| 平台层     | TypeScript (CF Workers 入口、i18n、auth 桥接)    |
| 序列化     | bincode v2                                       |
| 认证       | Clerk + Guest (FingerprintJS)                    |
| 数据库     | Cloudflare D1                                    |
| 构建工具   | Vite+、Cargo workspace、wasm-bindgen             |

## 项目结构

```
game-core/              共享 ECS 类型、组件、数据配置
game-client/            Bevy App 客户端 → WASM（本地完整游戏逻辑）
wasm-validator/         回放验证器 → WASM cdylib
src/
├── client/             浏览器入口 (i18n、auth 桥接)
├── server/             CF Workers (路由、服务)
public/
├── locales/            翻译 JSON (en / zh-CN / ja / ko)
data/                   RON 配置文件
```

## 命令

```bash
make wasm-dev           # 编译客户端 WASM (debug，快速迭代)
make wasm-all           # 编译客户端 WASM (release，thin LTO)
make dev                # 启动 Vite 开发服务器
make build              # 生产构建 (wasm-all + vp build)
make deploy             # 部署到 CF Workers
make check              # 格式化 + lint + 类型检查
cargo test              # 运行 Rust 测试
cargo clippy            # Rust lint
```

## 文档

- [架构](docs/architecture.md)
- [游戏系统](docs/gameplay.md)
- [开发指南](docs/development.md)

## 部署

推送到 `main` 分支自动部署到 [3xlive.app](https://3xlive.app)。

```bash
cp .env.example .env    # 填入 CLOUDFLARE_API_TOKEN
make deploy
```

## License

Private
