export interface Env {
  ASSETS: Fetcher;
  DB: D1Database;
  KV: KVNamespace;
  CLERK_SECRET_KEY: string;
  CLERK_PUBLISHABLE_KEY: string;
  SESSION_SECRET: string;
  TURNSTILE_SECRET: string;
}

interface ImportMetaEnv {
  readonly VITE_CLERK_PUBLISHABLE_KEY: string;
}

interface ImportMeta {
  readonly env: ImportMetaEnv;
}
