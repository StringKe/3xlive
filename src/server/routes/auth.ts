import { getAuth } from "@hono/clerk-auth";
import { Hono } from "hono";
import type { Env } from "../../env.d.ts";
import { enterPlayer } from "../services/player.ts";
import { signTicket } from "../services/ticket.ts";

export const authRoutes = new Hono<{ Bindings: Env }>();

// POST /enter - 统一入口（访客 + Clerk 用户）
authRoutes.post("/enter", async (c) => {
  const body = await c.req.json<{ fingerprint: string }>();

  const auth = getAuth(c);
  const clerkUserId = auth?.userId ?? undefined;
  const clerkDisplayName = clerkUserId
    ? (((auth as Record<string, unknown>)?.sessionClaims as Record<string, unknown>)?.name as
        | string
        | undefined)
    : undefined;

  let result: { id: string; displayName: string; isNew: boolean };
  try {
    result = await enterPlayer(c.env.DB, body.fingerprint, clerkUserId, clerkDisplayName);
  } catch (e) {
    if (e instanceof Error && e.message === "Invalid fingerprint") {
      return c.json({ error: "Invalid fingerprint" }, 400);
    }
    throw e;
  }

  const ticket = await signTicket(result.id, c.env.SESSION_SECRET);

  // 读取 meta_unlocks 和赛季信息
  const playerRow = await c.env.DB.prepare("SELECT meta_unlocks FROM players WHERE id=?")
    .bind(result.id)
    .first<{ meta_unlocks: string }>();
  const metaUnlocks = playerRow?.meta_unlocks ? JSON.parse(playerRow.meta_unlocks) : {};

  const seasonRaw = await c.env.KV.get("season:current");
  const seasonInfo = seasonRaw ? JSON.parse(seasonRaw) : null;

  return c.json({
    playerId: result.id,
    displayName: result.displayName,
    ticket,
    isNew: result.isNew,
    metaUnlocks,
    seasonInfo,
  });
});
