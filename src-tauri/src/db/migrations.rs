pub const INITIAL_MIGRATION: &str = r#"
-- app_settings table (singleton)
CREATE TABLE IF NOT EXISTS app_settings (
    id INTEGER PRIMARY KEY CHECK (id = 1),
    worktree_base_path TEXT NOT NULL DEFAULT '~/.local-code-agent/worktrees',
    default_base_branch TEXT NOT NULL DEFAULT 'main',
    agent_timeout_minutes INTEGER NOT NULL DEFAULT 30,
    sync_interval_minutes INTEGER NOT NULL DEFAULT 10,
    grpc_server_url TEXT NOT NULL DEFAULT 'http://localhost:9000',
    locale TEXT NOT NULL DEFAULT 'en',
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Insert default settings if not exists
INSERT OR IGNORE INTO app_settings (id) VALUES (1);

-- repositories table
CREATE TABLE IF NOT EXISTS repositories (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    mcp_server_name TEXT NOT NULL,
    platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
    base_url TEXT NOT NULL,
    name TEXT NOT NULL,
    url TEXT NOT NULL,
    owner TEXT NOT NULL,
    repo_name TEXT NOT NULL,
    local_path TEXT,
    last_synced_at TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now')),
    UNIQUE (mcp_server_name, owner, repo_name)
);

-- agent_jobs table
CREATE TABLE IF NOT EXISTS agent_jobs (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    repository_id INTEGER NOT NULL REFERENCES repositories(id) ON DELETE CASCADE,
    issue_number INTEGER NOT NULL,
    jobworkerp_job_id TEXT NOT NULL,
    status TEXT NOT NULL CHECK (status IN (
        'Pending', 'PreparingWorkspace', 'FetchingIssue',
        'RunningAgent', 'CreatingPR', 'PrCreated',
        'Merged', 'Completed', 'Failed', 'Cancelled'
    )),
    worktree_path TEXT,
    branch_name TEXT,
    pr_number INTEGER,
    error_message TEXT,
    created_at TEXT NOT NULL DEFAULT (datetime('now')),
    updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- Indexes
CREATE INDEX IF NOT EXISTS idx_repositories_mcp_server ON repositories(mcp_server_name);
CREATE INDEX IF NOT EXISTS idx_agent_jobs_repository ON agent_jobs(repository_id);
CREATE INDEX IF NOT EXISTS idx_agent_jobs_status ON agent_jobs(status);
CREATE INDEX IF NOT EXISTS idx_agent_jobs_jobworkerp_id ON agent_jobs(jobworkerp_job_id);
"#;
