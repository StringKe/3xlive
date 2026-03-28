import { Hono } from "hono";
import type { Env } from "../../env.d.ts";
import { verifyTicket } from "../services/ticket.ts";

export const arenaRoutes = new Hono<{ Bindings: Env }>();

// 从 Authorization header 提取并验证 ticket
async function authenticate(
  authHeader: string | undefined,
  secret: string,
): Promise<string | null> {
  if (!authHeader?.startsWith("Bearer ")) return null;
  const token = authHeader.slice(7);
  const result = await verifyTicket(token, secret);
  return result?.pid ?? null;
}

// 竞技场提交请求体
interface ArenaSubmitBody {
  replay_data: string; // base64 编码的回放数据
  claimed_damage: number;
}

// 竞技场排行榜条目
interface ArenaLeaderboardEntry {
  player_id: string;
  damage: number;
  verified: number;
  created_at: string;
}

// Turnstile 验证
const TURNSTILE_VERIFY_URL = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

async function verifyTurnstile(token: string, secret: string): Promise<boolean> {
  if (!token || !secret) return true;
  const resp = await fetch(TURNSTILE_VERIFY_URL, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: `secret=${encodeURIComponent(secret)}&response=${encodeURIComponent(token)}`,
  });
  const data = (await resp.json()) as { success: boolean };
  return data.success;
}

// POST /arena/submit - 提交竞技场结果
arenaRoutes.post("/arena/submit", async (c) => {
  const playerId = await authenticate(c.req.header("Authorization"), c.env.SESSION_SECRET);
  if (!playerId) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  // Turnstile 验证
  const turnstileToken = c.req.header("X-Turnstile-Token") ?? "";
  const turnstileSecret = c.env.TURNSTILE_SECRET ?? "";
  if (turnstileSecret) {
    const valid = await verifyTurnstile(turnstileToken, turnstileSecret);
    if (!valid) {
      return c.json({ error: "Turnstile verification failed" }, 403);
    }
  }

  const body = await c.req.json<ArenaSubmitBody>();
  const db = c.env.DB;
  const kv = c.env.KV;

  // 读取当前赛季 id
  const season = (await kv.get("season:current", "json")) as { id: string } | null;
  const seasonId = season?.id ?? "season_0";

  // 插入竞技场结果（verified=0，等待 WASM 回放验证）
  // TODO: 接入 WASM 回放验证，校验 claimed_damage 与实际伤害是否一致
  await db
    .prepare(
      `INSERT INTO arena_results (player_id, season_id, replay_data, damage, verified)
       VALUES (?, ?, ?, ?, 0)`,
    )
    .bind(playerId, seasonId, body.replay_data, body.claimed_damage)
    .run();

  return c.json({ verified: false, actual_damage: 0, rank: 0 });
});

// GET /arena/leaderboard - 获取竞技场排行榜
arenaRoutes.get("/arena/leaderboard", async (c) => {
  const db = c.env.DB;
  const kv = c.env.KV;

  // 读取当前赛季
  const season = (await kv.get("season:current", "json")) as { id: string } | null;
  const seasonId = season?.id ?? "season_0";

  // 查询已验证的结果，按伤害降序排列，取 Top 50
  const rows = await db
    .prepare(
      `SELECT player_id, damage, verified, created_at
       FROM arena_results
       WHERE verified = 1 AND season_id = ?
       ORDER BY damage DESC
       LIMIT 50`,
    )
    .bind(seasonId)
    .all<ArenaLeaderboardEntry>();

  return c.json({ season_id: seasonId, results: rows.results });
});
