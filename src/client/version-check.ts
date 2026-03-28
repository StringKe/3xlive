// 版本更新检查：定时轮询 /_version.json，检测到新版本时显示更新提示
const CHECK_INTERVAL = 60_000; // 60 秒

interface VersionMeta {
  time: number;
  version: string;
}

let currentVersion: string | null = null;

async function fetchVersion(): Promise<VersionMeta | null> {
  try {
    const res = await fetch(`/_version.json?t=${Date.now()}`, { cache: "no-store" });
    if (!res.ok) return null;
    return (await res.json()) as VersionMeta;
  } catch {
    return null;
  }
}

function showUpdateBanner(): void {
  if (document.getElementById("update-banner")) return;

  const banner = document.createElement("div");
  banner.id = "update-banner";
  Object.assign(banner.style, {
    position: "fixed",
    top: "0",
    left: "0",
    width: "100%",
    padding: "10px 16px",
    background: "rgba(0, 200, 255, 0.15)",
    backdropFilter: "blur(8px)",
    color: "#0ff",
    fontFamily: "monospace",
    fontSize: "14px",
    textAlign: "center",
    zIndex: "99999",
    cursor: "pointer",
    borderBottom: "1px solid rgba(0, 255, 255, 0.3)",
  });

  const text = window.__i18n?.t("ui.update.available") ?? "New version available! Click to refresh";
  banner.textContent = text;
  banner.addEventListener("click", () => location.reload());
  document.body.appendChild(banner);
}

export function startVersionCheck(): void {
  // 开发环境不检查
  if (import.meta.env.DEV) return;

  const check = async () => {
    const meta = await fetchVersion();
    if (!meta) return;

    if (currentVersion === null) {
      // 首次获取，记录当前版本
      currentVersion = meta.version;
      return;
    }

    if (meta.version !== currentVersion) {
      showUpdateBanner();
    }
  };

  // 首次延迟 5 秒后检查，之后每 60 秒一次
  setTimeout(() => {
    void check();
    setInterval(check, CHECK_INTERVAL);
  }, 5_000);
}
