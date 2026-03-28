import type { AuthState } from "./types/auth.d.ts";
import type { Clerk as ClerkType } from "@clerk/clerk-js/no-rhc";

declare global {
  interface Window {
    __internal_ClerkUICtor?: unknown;
  }
}

const FALLBACK_FP_KEY = "__3xlive_fallback_fp";

async function getFingerprint(): Promise<string> {
  try {
    const { default: FingerprintJS } = await import("@fingerprintjs/fingerprintjs");
    const fp = await FingerprintJS.load();
    const result = await fp.get();
    return result.visitorId;
  } catch {
    let stored = localStorage.getItem(FALLBACK_FP_KEY);
    if (!stored) {
      stored = crypto.randomUUID().replace(/-/g, "");
      localStorage.setItem(FALLBACK_FP_KEY, stored);
    }
    return stored;
  }
}

// Clerk 初始化
const meta = document.querySelector<HTMLMetaElement>('meta[name="clerk-key"]');
const key = meta?.content;

let clerk: ClerkType | null = null;

let cachedAuthState: AuthState = {
  isClerkSignedIn: false,
  displayName: "",
  avatarUrl: null,
  stateVersion: 0,
};

function updateAuthState() {
  _stateVersion++;
  cachedAuthState = {
    isClerkSignedIn: !!clerk?.user,
    displayName: clerk?.user?.fullName ?? clerk?.user?.username ?? "",
    avatarUrl: clerk?.user?.imageUrl ?? null,
    stateVersion: _stateVersion,
  };
}

// 挂载 window.__auth
let _stateVersion = 0;

window.__auth = {
  stateVersion() {
    return _stateVersion;
  },

  async requestTicket() {
    try {
      const fp = await getFingerprint();
      const headers: Record<string, string> = { "Content-Type": "application/json" };
      if (clerk?.session) {
        headers["Authorization"] = `Bearer ${await clerk.session.getToken()}`;
      }
      const res = await fetch("/api/auth/enter", {
        method: "POST",
        headers,
        body: JSON.stringify({ fingerprint: fp }),
      });
      if (!res.ok) {
        const text = await res.text();
        console.error(`[auth-bridge] /api/auth/enter ${res.status}: ${text}`);
        return { error: `HTTP ${res.status}` };
      }
      return res.json();
    } catch (e) {
      console.error("[auth-bridge] requestTicket failed:", e);
      return { error: String(e) };
    }
  },

  openSignIn() {
    if (clerk?.user) return; // 已登录，跳过
    clerk?.openSignIn();
  },

  openUserProfile() {
    clerk?.openUserProfile();
  },

  getAuthState() {
    return cachedAuthState;
  },

  async signOut() {
    await clerk?.signOut();
  },
};

// 从 publishable key 推导 Frontend API 域名，加载 Clerk UI bundle
async function loadClerkUI(publishableKey: string): Promise<void> {
  const clerkDomain = atob(publishableKey.split("_")[2]).slice(0, -1);
  return new Promise((resolve, reject) => {
    const script = document.createElement("script");
    script.src = `https://${clerkDomain}/npm/@clerk/ui@1/dist/ui.browser.js`;
    script.async = true;
    script.crossOrigin = "anonymous";
    script.onload = () => resolve();
    script.onerror = () => reject(new Error("Failed to load @clerk/ui bundle"));
    document.head.appendChild(script);
  });
}

// 延迟加载 Clerk（核心 + UI bundle）
if (key) {
  const [{ Clerk }] = await Promise.all([import("@clerk/clerk-js/no-rhc"), loadClerkUI(key)]);
  const instance = new Clerk(key);
  instance
    .load({
      // @clerk/ui CDN bundle 设置 window.__internal_ClerkUICtor
      // eslint-disable-next-line @typescript-eslint/no-explicit-any
      ui: { ClerkUI: window.__internal_ClerkUICtor as any },
      appearance: {
        baseTheme: undefined,
        variables: {
          colorPrimary: "#00e5ff",
        },
      },
    })
    .then(() => {
      clerk = instance;
      updateAuthState();

      // 监听 session/user 变化
      clerk.addListener((event) => {
        if (event.session !== undefined || event.user !== undefined) {
          updateAuthState();
        }
      });
    })
    .catch((err) => {
      console.warn("[auth-bridge] Clerk load failed:", err);
    });
}
