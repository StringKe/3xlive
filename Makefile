.PHONY: help dev build check check-fix clean deploy wasm-client wasm-all wasm-client-dev wasm-dev wasm-validator

WASM_TARGET := wasm32-unknown-unknown
# 并行 cargo 前端作业数（利用多核）
CARGO_JOBS := $(shell nproc 2>/dev/null || sysctl -n hw.ncpu 2>/dev/null || echo 8)

help:
	@echo "=== 3xlive 开发命令 ==="
	@echo ""
	@echo "WASM 编译："
	@echo "  make wasm-client       编译 Rust 客户端 WASM + wasm-bindgen（release）"
	@echo "  make wasm-all          编译客户端 WASM（release）"
	@echo "  make wasm-dev          编译客户端 WASM（debug，快速迭代）"
	@echo ""
	@echo "服务端 + 开发服务器："
	@echo "  make dev               启动 Vite 开发服务器（HMR + 本地 workerd）"
	@echo "  make build             生产构建"
	@echo "  make deploy            部署到 Cloudflare Workers"
	@echo ""
	@echo "代码质量："
	@echo "  make check             格式化 + lint + 类型检查"
	@echo "  make check-fix         检查并自动修复"
	@echo ""
	@echo "其他："
	@echo "  make clean             清理编译产物"

# 服务端 + 开发服务器
dev:
	vp dev

build: wasm-all
	vp build

deploy: build
	vp run deploy

# 代码质量
check:
	vp check

check-fix:
	vp check --fix

# 清理
clean:
	rm -rf dist .vite src/wasm-client
	cargo clean

# ─── Rust WASM 构建（release，用于部署） ───

wasm-client:
	cargo build -p game-client --target $(WASM_TARGET) --release -j $(CARGO_JOBS)
	wasm-bindgen --out-dir src/wasm-client --out-name wasm-client --target web target/$(WASM_TARGET)/release/game-client.wasm

# 编译客户端（release）
wasm-all: wasm-client

# ─── Rust WASM 构建（debug，开发快速迭代） ───

wasm-client-dev:
	cargo build -p game-client --target $(WASM_TARGET) --features dev_log -j $(CARGO_JOBS)
	wasm-bindgen --out-dir src/wasm-client --out-name wasm-client --target web target/$(WASM_TARGET)/debug/game-client.wasm

# 编译客户端（debug，启用 dev_log）
wasm-dev: wasm-client-dev

# ─── WASM 验证器（回放验证用） ───

wasm-validator:
	cargo build -p wasm-validator --target $(WASM_TARGET) --release -j $(CARGO_JOBS)
	wasm-bindgen --out-dir src/wasm-validator --out-name wasm-validator --target web target/$(WASM_TARGET)/release/wasm_validator.wasm
