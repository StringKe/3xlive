// 入口：Loading UI + 资源预加载 + WASM 初始化
import { loadAssetManifest, resolveAsset } from "./asset-resolver";
import { initI18n } from "./i18n-bridge";
import "./auth-bridge";
import "./storage-bridge";
import "./sync-bridge";
import "./turnstile-bridge";
import { startVersionCheck } from "./version-check";
import init from "../wasm-client/wasm-client.js";

// ─── Loading UI 控制 ───

function updateLoading(text: string, progress: number) {
  const el = document.getElementById("loading-text");
  const bar = document.getElementById("loading-bar");
  if (el) el.textContent = text;
  if (bar) bar.style.width = `${Math.min(100, Math.round(progress))}%`;
}

function removeLoading() {
  const screen = document.getElementById("loading-screen");
  if (screen) {
    screen.style.transition = "opacity 0.3s";
    screen.style.opacity = "0";
    setTimeout(() => screen.remove(), 300);
  }
}

// ─── 资源预加载 ───

async function preloadAssets(): Promise<void> {
  updateLoading("Loading manifest...", 5);
  await loadAssetManifest();

  updateLoading("Loading translations...", 15);
  await initI18n();

  // 预加载字体文件（确保 Bevy 请求时命中浏览器缓存）
  updateLoading("Loading fonts...", 30);
  const fontPath = window.__i18n?.fontPath();
  if (fontPath) {
    try {
      await fetch(fontPath);
    } catch {
      // 字体加载失败不阻断
    }
  }

  // 预加载 WASM 二进制（最大文件，浏览器缓存后 init() 直接命中）
  updateLoading("Loading game engine...", 50);
  try {
    const wasmUrl = resolveAsset("/src/wasm-client/wasm-client_bg.wasm");
    await fetch(wasmUrl);
  } catch {
    // WASM 预加载失败不阻断，init() 会重新请求
  }

  updateLoading("Initializing...", 80);
}

// ─── Canvas 事件防护 ───

function setupCanvas() {
  const canvas = document.getElementById("game-canvas") as HTMLCanvasElement | null;
  if (!canvas) return;

  // touchmove 阻止页面滚动
  canvas.addEventListener("touchmove", (e) => e.preventDefault(), { passive: false });
  // 键盘：阻止空格/方向键等滚动页面
  canvas.addEventListener("keydown", (e) => {
    const keys = ["Space", "ArrowUp", "ArrowDown", "ArrowLeft", "ArrowRight", "Tab"];
    if (keys.includes(e.code)) e.preventDefault();
  });
  // 右键菜单
  canvas.addEventListener("contextmenu", (e) => e.preventDefault());
}

// ─── 启动流程 ───

await preloadAssets();

startVersionCheck();
setupCanvas();

updateLoading("Starting game...", 95);

// WASM 客户端加载（Bevy 渲染引擎）
// winit 用 throw 实现事件循环控制流，init() 会 reject 但游戏正常运行
init()
  .then(() => {
    removeLoading();
  })
  .catch((e: unknown) => {
    if (e instanceof Error && e.message.includes("Using exceptions for control flow")) {
      console.log("[wasm] game-client 事件循环已启动");
      removeLoading();
    } else {
      console.error("[wasm] game-client 加载失败:", e);
      updateLoading("Failed to load game", 0);
    }
  });
