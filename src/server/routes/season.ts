import { Hono } from "hono";
import type { Env } from "../../env.d.ts";

export const seasonRoutes = new Hono<{ Bindings: Env }>();

// 赛季数据结构
interface SeasonData {
  id: string;
  seed: number;
  goals: Record<string, number>;
  progress: Record<string, number>;
  remaining_days: number;
}

// 默认赛季（KV 中无数据时的 fallback）
function defaultSeason(): SeasonData {
  return {
    id: "season_0",
    seed: 42,
    goals: { kills: 1000, damage: 500000, runs: 50 },
    progress: {},
    remaining_days: 30,
  };
}

// GET /season - 获取当前赛季信息
seasonRoutes.get("/season", async (c) => {
  const kv = c.env.KV;
  const data = await kv.get("season:current", "json");

  if (!data) {
    return c.json(defaultSeason());
  }

  return c.json(data as SeasonData);
});
