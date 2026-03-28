import { Hono } from "hono";
import type { Env } from "../../env.d.ts";
import { verifyTicket } from "../services/ticket.ts";

export const mailRoutes = new Hono<{ Bindings: Env }>();

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

// 每赛季发送邮件上限
const MAIL_SEND_LIMIT_PER_SEASON = 3;

// 发送邮件请求体
interface MailSendBody {
  to_id: string;
  item_type: string;
  item_data: string;
}

// 领取邮件请求体
interface MailClaimBody {
  mail_id: number;
}

// POST /mail/send - 发送邮件
mailRoutes.post("/mail/send", async (c) => {
  const playerId = await authenticate(c.req.header("Authorization"), c.env.SESSION_SECRET);
  if (!playerId) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  const body = await c.req.json<MailSendBody>();
  const db = c.env.DB;
  const kv = c.env.KV;

  // 读取当前赛季 id
  const season = (await kv.get("season:current", "json")) as { id: string } | null;
  const seasonId = season?.id ?? "season_0";

  // 检查本赛季发送次数限制
  const countRow = await db
    .prepare("SELECT COUNT(*) as cnt FROM mailbox WHERE from_id = ? AND season_id = ?")
    .bind(playerId, seasonId)
    .first<{ cnt: number }>();

  if ((countRow?.cnt ?? 0) >= MAIL_SEND_LIMIT_PER_SEASON) {
    return c.json({ error: "Send limit reached for this season" }, 429);
  }

  // 插入邮件
  await db
    .prepare(
      `INSERT INTO mailbox (from_id, to_id, season_id, item_type, item_data, claimed)
       VALUES (?, ?, ?, ?, ?, 0)`,
    )
    .bind(playerId, body.to_id, seasonId, body.item_type, body.item_data)
    .run();

  return c.json({ success: true });
});

// POST /mail/claim - 领取邮件
mailRoutes.post("/mail/claim", async (c) => {
  const playerId = await authenticate(c.req.header("Authorization"), c.env.SESSION_SECRET);
  if (!playerId) {
    return c.json({ error: "Unauthorized" }, 401);
  }

  const body = await c.req.json<MailClaimBody>();
  const db = c.env.DB;

  // 查询邮件并验证归属
  const mail = await db
    .prepare("SELECT id, to_id, item_type, item_data, claimed FROM mailbox WHERE id = ?")
    .bind(body.mail_id)
    .first<{ id: number; to_id: string; item_type: string; item_data: string; claimed: number }>();

  if (!mail) {
    return c.json({ error: "Mail not found" }, 404);
  }
  if (mail.to_id !== playerId) {
    return c.json({ error: "Mail does not belong to you" }, 403);
  }
  if (mail.claimed === 1) {
    return c.json({ error: "Mail already claimed" }, 409);
  }

  // 标记为已领取
  await db.prepare("UPDATE mailbox SET claimed = 1 WHERE id = ?").bind(body.mail_id).run();

  return c.json({
    success: true,
    item_type: mail.item_type,
    item_data: mail.item_data,
  });
});
