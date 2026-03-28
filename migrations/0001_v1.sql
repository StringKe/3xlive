-- v1: 完整 schema（匹配 TypeScript 代码实际引用）

-- 玩家主表
CREATE TABLE IF NOT EXISTS players (
  id TEXT PRIMARY KEY,
  fingerprint TEXT NOT NULL,
  clerk_user_id TEXT,
  display_name TEXT NOT NULL DEFAULT 'Player',
  meta_unlocks TEXT DEFAULT '{}',
  trust_score INTEGER DEFAULT 50,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  last_seen_at TEXT NOT NULL DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_players_clerk ON players(clerk_user_id);
CREATE INDEX IF NOT EXISTS idx_players_fingerprint ON players(fingerprint);

-- 单局结果
CREATE TABLE IF NOT EXISTS run_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  player_id TEXT NOT NULL,
  survived_secs REAL NOT NULL DEFAULT 0,
  total_damage REAL NOT NULL DEFAULT 0,
  kills_normal INTEGER NOT NULL DEFAULT 0,
  kills_elite INTEGER NOT NULL DEFAULT 0,
  kills_boss INTEGER NOT NULL DEFAULT 0,
  max_chunk_difficulty REAL NOT NULL DEFAULT 0,
  final_level INTEGER NOT NULL DEFAULT 1,
  needs_replay_verification INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_run_results_player ON run_results(player_id);

-- 玩家累计统计
CREATE TABLE IF NOT EXISTS player_stats (
  player_id TEXT PRIMARY KEY,
  kills INTEGER DEFAULT 0,
  damage REAL DEFAULT 0,
  runs INTEGER DEFAULT 0,
  avg_survival REAL DEFAULT 0
);

-- 竞技场结果
CREATE TABLE IF NOT EXISTS arena_results (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  player_id TEXT NOT NULL,
  season_id INTEGER NOT NULL DEFAULT 0,
  damage REAL NOT NULL DEFAULT 0,
  replay_data BLOB,
  verified INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_arena_results_season ON arena_results(season_id, verified, damage);

-- 邮箱
CREATE TABLE IF NOT EXISTS mailbox (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  from_id TEXT,
  to_id TEXT NOT NULL,
  season_id INTEGER NOT NULL DEFAULT 0,
  item_type TEXT,
  item_data TEXT,
  claimed INTEGER DEFAULT 0,
  created_at TEXT DEFAULT (datetime('now'))
);
CREATE INDEX IF NOT EXISTS idx_mailbox_to ON mailbox(to_id, claimed);
