const TICKET_TTL = 86400; // 24 小时，单位秒

export async function signTicket(playerId: string, secret: string): Promise<string> {
  const payload = btoa(
    JSON.stringify({ pid: playerId, exp: Math.floor(Date.now() / 1000) + TICKET_TTL }),
  );
  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["sign"],
  );
  const sig = await crypto.subtle.sign("HMAC", key, new TextEncoder().encode(payload));
  return `${payload}.${btoa(String.fromCharCode(...new Uint8Array(sig)))}`;
}

export async function verifyTicket(
  ticket: string,
  secret: string,
): Promise<{ pid: string } | null> {
  const parts = ticket.split(".");
  if (parts.length !== 2) return null;
  const [payload, sig] = parts;
  if (!payload || !sig) return null;

  const key = await crypto.subtle.importKey(
    "raw",
    new TextEncoder().encode(secret),
    { name: "HMAC", hash: "SHA-256" },
    false,
    ["verify"],
  );
  let sigBytes: ArrayBuffer;
  let data: { pid: string; exp: number };
  try {
    const decoded = Uint8Array.from(atob(sig), (c) => c.charCodeAt(0));
    sigBytes = decoded.buffer;
    data = JSON.parse(atob(payload)) as { pid: string; exp: number };
  } catch {
    return null;
  }

  const valid = await crypto.subtle.verify(
    "HMAC",
    key,
    sigBytes,
    new TextEncoder().encode(payload),
  );
  if (!valid) return null;

  return data.exp < Math.floor(Date.now() / 1000) ? null : { pid: data.pid };
}
