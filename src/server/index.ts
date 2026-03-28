import { Hono } from "hono";
import { cors } from "hono/cors";
import { trimTrailingSlash } from "hono/trailing-slash";
import { clerkMiddleware } from "@hono/clerk-auth";
import type { Env } from "../env.d.ts";
import { arenaRoutes } from "./routes/arena.ts";
import { authRoutes } from "./routes/auth.ts";
import { mailRoutes } from "./routes/mail.ts";
import { seasonRoutes } from "./routes/season.ts";
import { syncRoutes } from "./routes/sync.ts";

const app = new Hono<{ Bindings: Env }>();

app.use(trimTrailingSlash());
app.use("/api/*", cors());
app.use("/api/*", clerkMiddleware());

app.route("/api/auth", authRoutes);
app.route("/api", syncRoutes);
app.route("/api", seasonRoutes);
app.route("/api", arenaRoutes);
app.route("/api", mailRoutes);
app.get("/api/health", (c) => c.json({ status: "ok", ts: Date.now() }));

export default app;
