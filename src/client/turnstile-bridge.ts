// Cloudflare Turnstile 桥接
// 隐式模式嵌入，暴露 window.__turnstile 供 Rust WASM 调用

const SITE_KEY = "0x4AAAAAACwJpkcRitMESqAp";

let cachedToken: string | null = null;
let tokenExpiry = 0;
const TOKEN_LIFETIME_MS = 240_000; // 4 分钟（Turnstile token 有效期 5 分钟，提前刷新）

async function getToken(): Promise<string> {
  const now = Date.now();
  if (cachedToken && now < tokenExpiry) {
    return cachedToken;
  }

  return new Promise<string>((resolve, reject) => {
    // 确保 Turnstile script 已加载
    if (typeof (window as unknown as Record<string, unknown>).turnstile === "undefined") {
      resolve("");
      return;
    }

    const turnstile = (window as unknown as Record<string, unknown>).turnstile as {
      render: (container: string | HTMLElement, options: Record<string, unknown>) => string;
      remove: (widgetId: string) => void;
    };

    // 创建隐藏容器
    let container = document.getElementById("turnstile-container");
    if (!container) {
      container = document.createElement("div");
      container.id = "turnstile-container";
      container.style.display = "none";
      document.body.appendChild(container);
    }

    const widgetId = turnstile.render(container, {
      sitekey: SITE_KEY,
      callback: (token: string) => {
        cachedToken = token;
        tokenExpiry = Date.now() + TOKEN_LIFETIME_MS;
        turnstile.remove(widgetId);
        resolve(token);
      },
      "error-callback": () => {
        turnstile.remove(widgetId);
        reject(new Error("Turnstile verification failed"));
      },
      size: "compact",
      execution: "execute",
    });
  }).catch(() => "");
}

declare global {
  interface Window {
    __turnstile: typeof turnstileApi;
  }
}

const turnstileApi = { getToken };
window.__turnstile = turnstileApi;

console.log("[turnstile] Turnstile bridge initialized");
