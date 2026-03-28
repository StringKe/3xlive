import { Hono } from "hono";
import type { Env } from "../../env.d.ts";
import { verifyTicket } from "../services/ticket.ts";

export const syncRoutes = new Hono<{ Bindings: Env }>();

// 从 Authorization header 提取并验证 ticket，返回 player id
async function authenticate(
  authHeader: string | undefined,
  secret: string,
): Promise<string | null> {
  if (!authHeader?.startsWith("Bearer ")) return null;
  const token = authHeader.slice(7);
  const result = await verifyTicket(token, secret);
  return result?.pid ?? null;
}

// 单局结果类型
interface RunResult {
  survived_secs: number;
  total_damage: number;
  kills_normal: number;
  kills_elite: number;
  kills_boss: number;
  max_chunk_difficulty: number;
  final_level: number;
}

// 同步请求体
interface SyncBody {
  run_results: RunResult[];
  stat_increments: {
    kills: number;
    damage: number;
    runs: number;
  };
}

// L1 范围检查阈值
const L1_MAX_SURVIVED_SECS = 3600;
const L1_MAX_TOTAL_DAMAGE = 1_000_000;

// L2 统计异常检测所需最小样本数
const L2_MIN_SAMPLES = 5;
// L2 标准差倍数阈值
const L2_STDDEV_MULTIPLIER = 3;

// 信任分变动值
const TRUST_DELTA_PASS = 1;
const TRUST_DELTA_L1_FAIL = -20;
const TRUST_DELTA_L2_ANOMALY = -5;

// L3 抽样率（基于信任分）
const L3_SAMPLE_RATE_HIGH = 0.05; // trust > 80
const L3_SAMPLE_RATE_MID = 0.2; // 50-80
const L3_SAMPLE_RATE_LOW = 0.5; // < 50

// Turnstile 验证 URL
const TURNSTILE_VERIFY_URL = "https://challenges.cloudflare.com/turnstile/v0/siteverify";

async function verifyTurnstile(token: string, secret: string): Promise<boolean> {
  if (!token || !secret) return true; // 未配置时跳过
  const resp = await fetch(TURNSTILE_VERIFY_URL, {
    method: "POST",
    headers: { "Content-Type": "application/x-www-form-urlencoded" },
    body: `secret=${encodeURIComponent(secret)}&response=${encodeURIComponent(token)}`,
  });
  const data = (await resp.json()) as { success: boolean };
  return data.success;
}

function shouldSampleL3(trustScore: number): boolean {
  const rate =
    trustScore > 80
      ? L3_SAMPLE_RATE_HIGH
      : trustScore >= 50
        ? L3_SAMPLE_RATE_MID
        : L3_SAMPLE_RATE_LOW;
  return Math.random() < rate;
}

// POST /sync - 接收客户端同步数据
syncRoutes.post("/sync", async (c) => {
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

  const body = await c.req.json<SyncBody>();
  const db = c.env.DB;

  let trustDelta = TRUST_DELTA_PASS;

  // L1 范围检查
  const l1Failed = body.run_results.some(
    (r) => r.survived_secs >= L1_MAX_SURVIVED_SECS || r.total_damage >= L1_MAX_TOTAL_DAMAGE,
  );
  if (l1Failed) {
    trustDelta = TRUST_DELTA_L1_FAIL;
  }

  // L2 统计异常检测（仅在 L1 通过时执行）
  if (!l1Failed && body.run_results.length > 0) {
    const stats = await db
      .prepare(
        "SELECT total_damage FROM run_results WHERE player_id = ? ORDER BY created_at DESC LIMIT 100",
      )
      .bind(playerId)
      .all<{ total_damage: number }>();

    if (stats.results.length >= L2_MIN_SAMPLES) {
      const damages = stats.results.map((r) => r.total_damage);
      const mean = damages.reduce((a, b) => a + b, 0) / damages.length;
      const variance = damages.reduce((a, b) => a + (b - mean) ** 2, 0) / damages.length;
      const stddev = Math.sqrt(variance);
      const threshold = mean + L2_STDDEV_MULTIPLIER * stddev;

      const hasAnomaly = body.run_results.some((r) => r.total_damage > threshold);
      if (hasAnomaly) {
        trustDelta = TRUST_DELTA_L2_ANOMALY;
      }
    }
  }

  // 更新信任分
  await db
    .prepare("UPDATE players SET trust_score = MAX(0, MIN(100, trust_score + ?)) WHERE id = ?")
    .bind(trustDelta, playerId)
    .run();

  // 插入单局结果
  const insertStmt = db.prepare(
    `INSERT INTO run_results (player_id, survived_secs, total_damage, kills_normal, kills_elite, kills_boss, max_chunk_difficulty, final_level)
     VALUES (?, ?, ?, ?, ?, ?, ?, ?)`,
  );
  if (body.run_results.length > 0) {
    await db.batch(
      body.run_results.map((r) =>
        insertStmt.bind(
          playerId,
          r.survived_secs,
          r.total_damage,
          r.kills_normal,
          r.kills_elite,
          r.kills_boss,
          r.max_chunk_difficulty,
          r.final_level,
        ),
      ),
    );
  }

  // 更新玩家累计统计（UPSERT：新玩家自动创建记录）
  const inc = body.stat_increments;
  const totalSurvivedSecs = body.run_results.reduce((sum, r) => sum + r.survived_secs, 0);
  await db
    .prepare(
      `INSERT INTO player_stats (player_id, kills, damage, runs, avg_survival)
       VALUES (?, ?, ?, ?, ?)
       ON CONFLICT(player_id) DO UPDATE SET
         kills = kills + excluded.kills,
         damage = damage + excluded.damage,
         runs = runs + excluded.runs,
         avg_survival = CASE WHEN runs + excluded.runs > 0
           THEN ((avg_survival * runs) + ?) / (runs + excluded.runs)
           ELSE avg_survival END`,
    )
    .bind(playerId, inc.kills, inc.damage, inc.runs, totalSurvivedSecs, totalSurvivedSecs)
    .run();

  // 查询最新信任分
  const player = await db
    .prepare("SELECT trust_score FROM players WHERE id = ?")
    .bind(playerId)
    .first<{ trust_score: number }>();

  // L3 抽样决策
  const trustScore = player?.trust_score ?? 50;
  const needsReplayVerification = shouldSampleL3(trustScore);
  if (needsReplayVerification && body.run_results.length > 0) {
    // 标记最新 run 需要回放验证
    await db
      .prepare(
        "UPDATE run_results SET needs_replay_verification = 1 WHERE player_id = ? ORDER BY created_at DESC LIMIT 1",
      )
      .bind(playerId)
      .run();
  }

  return c.json({
    success: true,
    new_trust_score: trustScore,
    needs_replay_verification: needsReplayVerification,
  });
});

// GET /pull - 拉取服务端权威状态
syncRoutes.get("/pull", async (c) => {
  const playerId = await authenticate(c.req.header("Authorization"), c.env.SESSION_SECRET);
  if (!playerId) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  const db = c.env.DB;
  const kv = c.env.KV;

  // 并行查询各项数据
  const [playerRow, mailRows, seasonProgress, leaderboardRows] = await Promise.all([
    // 玩家元数据与解锁信息
    db
      .prepare("SELECT meta_unlocks FROM players WHERE id = ?")
      .bind(playerId)
      .first<{ meta_unlocks: string | null }>(),

    // 未领取的邮件
    db
      .prepare(
        "SELECT id, from_id, item_type, item_data, created_at FROM mailbox WHERE to_id = ? AND claimed = 0",
      )
      .bind(playerId)
      .all<{
        id: number;
        from_id: string;
        item_type: string;
        item_data: string;
        created_at: string;
      }>(),

    // 赛季进度（KV）
    kv.get(`season_progress:${playerId}`, "json"),

    // 排行榜 Top 50
    db
      .prepare(
        "SELECT player_id, kills, damage, runs, avg_survival FROM player_stats ORDER BY kills DESC LIMIT 50",
      )
      .all<{
        player_id: string;
        kills: number;
        damage: number;
        runs: number;
        avg_survival: number;
      }>(),
  ]);

  return c.json({
    meta_unlocks: playerRow?.meta_unlocks ? JSON.parse(playerRow.meta_unlocks) : {},
    mailbox: mailRows.results,
    season_progress: seasonProgress ?? {},
    leaderboard: leaderboardRows.results,
  });
});
