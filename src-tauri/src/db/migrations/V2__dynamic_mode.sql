-- Dynamic mode tables for multi-account support

-- Token storage (encrypted)
CREATE TABLE token_stores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  encrypted_token BLOB NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Platform configurations
CREATE TABLE platform_configs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  base_url TEXT NOT NULL,
  token_id INTEGER NOT NULL REFERENCES token_stores(id) ON DELETE CASCADE,
  mcp_runner_name TEXT,
  user_name TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (platform, base_url)
);

-- Indexes
CREATE INDEX idx_platform_configs_platform ON platform_configs(platform);
CREATE INDEX idx_token_stores_platform ON token_stores(platform);
