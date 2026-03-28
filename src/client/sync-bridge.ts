// HTTP 同步桥接，暴露 window.__sync 供 Rust WASM 调用
// 所有返回值为 Uint8Array 供 Rust bincode 解码

const API_BASE = "/api";

async function getTicket(): Promise<string> {
  // 从 auth bridge 获取 ticket
  if (window.__auth?.requestTicket) {
    const resp = await window.__auth.requestTicket();
    return (resp as { ticket?: string }).ticket ?? "";
  }
  return "";
}

async function getTurnstileToken(): Promise<string> {
  if (window.__turnstile?.getToken) {
    return window.__turnstile.getToken();
  }
  return "";
}

async function uploadResults(data: Uint8Array): Promise<Uint8Array> {
  const ticket = await getTicket();
  const turnstile = await getTurnstileToken();

  // Rust 侧传来的是 JSON 编码的 RunResult 字节，解码为对象后包装成 SyncBody
  let runResult: Record<string, unknown>;
  try {
    const jsonStr = new TextDecoder().decode(data);
    runResult = JSON.parse(jsonStr);
  } catch {
    // 无法解析时跳过上传
    return new Uint8Array(0);
  }

  const body = {
    run_results: [runResult],
    stat_increments: {
      kills:
        ((runResult.kills_normal as number) ?? 0) +
        ((runResult.kills_elite as number) ?? 0) +
        ((runResult.kills_boss as number) ?? 0),
      damage: (runResult.total_damage as number) ?? 0,
      runs: 1,
    },
  };

  const resp = await fetch(`${API_BASE}/sync`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${ticket}`,
      "X-Turnstile-Token": turnstile,
    },
    body: JSON.stringify(body),
  });
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}

async function pullState(): Promise<Uint8Array> {
  const ticket = await getTicket();
  const resp = await fetch(`${API_BASE}/pull`, {
    headers: { Authorization: `Bearer ${ticket}` },
  });
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}

async function submitArena(replay: Uint8Array): Promise<Uint8Array> {
  const ticket = await getTicket();
  const turnstile = await getTurnstileToken();
  const resp = await fetch(`${API_BASE}/arena/submit`, {
    method: "POST",
    headers: {
      "Content-Type": "application/octet-stream",
      Authorization: `Bearer ${ticket}`,
      "X-Turnstile-Token": turnstile,
    },
    body: replay as unknown as BodyInit,
  });
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}

async function getSeasonInfo(): Promise<Uint8Array> {
  const resp = await fetch(`${API_BASE}/season`);
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}

async function sendMail(data: Uint8Array): Promise<void> {
  const ticket = await getTicket();
  await fetch(`${API_BASE}/mail/send`, {
    method: "POST",
    headers: {
      "Content-Type": "application/octet-stream",
      Authorization: `Bearer ${ticket}`,
    },
    body: data as unknown as BodyInit,
  });
}

async function claimMail(mailId: number): Promise<Uint8Array> {
  const ticket = await getTicket();
  const resp = await fetch(`${API_BASE}/mail/claim`, {
    method: "POST",
    headers: {
      "Content-Type": "application/json",
      Authorization: `Bearer ${ticket}`,
    },
    body: JSON.stringify({ mail_id: mailId }),
  });
  const buf = await resp.arrayBuffer();
  return new Uint8Array(buf);
}

declare global {
  interface Window {
    __sync: typeof syncApi;
  }
}

const syncApi = { uploadResults, pullState, submitArena, getSeasonInfo, sendMail, claimMail };
window.__sync = syncApi;

console.log("[sync] HTTP sync bridge initialized");
