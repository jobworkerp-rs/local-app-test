# Local Code Agent Service 実装計画

本ドキュメントは、Local Code Agent Serviceの実装計画を定義する。

## 関連文書

- PRD: `local-code-agent-service-prd.md`
- 技術統合仕様: `local-code-agent-jobworkerp-integration.md`
- フロントエンド技術仕様: `local-code-agent-frontend-tech-spec.md`

---

## 1. 実装可能性評価

### 1.1 評価結果: 実装着手可能

以下の条件が満たされていることを確認した：

| 項目 | 状態 | 備考 |
|------|------|------|
| PRD完成度 | ✅ | ユーザーストーリー、機能仕様が明確 |
| 技術仕様完成度 | ✅ | jobworkerp-rs APIとの整合性確認済み |
| フロントエンド仕様完成度 | ✅ | Tauri v2アーキテクチャ、コンポーネント設計が明確 |
| jobworkerp-rs API | ✅ | 必要なgRPC APIがすべて利用可能 |
| 依存技術の成熟度 | ✅ | Tauri v2、Vite 7、React 19はすべて安定版 |

### 1.2 レビュー指摘事項の解決策

| 指摘事項 | 解決策 | 実装フェーズ |
|---------|--------|-------------|
| WorkflowRunArgsの仕様不完全 | 実際のProto定義を直接参照して実装 | Phase 1 |
| CreateRunnerRequestパラメータ名不整合 | PRDの記述（`definition`フィールド）を正とする | Phase 1 |
| TypeScript型定義の混乱 | drizzle-orm記述を削除し、純粋なinterface定義に変更 | Phase 1 |
| モード判定フローの明確化 | 初回起動時にMCPサーバー存在確認で自動判定 | Phase 1 |

---

## 2. フェーズ構成

### 2.1 フェーズ概要

```text
Phase 0: 環境構築・プロジェクト初期化 ✅ 完了
    ↓
Phase 1: コアインフラ ✅ 完了
    - Tauri Rustバックエンド基盤
    - SQLite + マイグレーション
    - gRPCクライアント
    ↓
Phase 2: 静的設定モード実装 ✅ 完了
    - MCPサーバー接続
    - リポジトリ管理
    - Issue/PR一覧表示
    ↓
Phase 3: エージェント実行 🚧 未着手
    - ワークフロー定義
    - ジョブ投入・ストリーミング
    - PR作成
    ↓
Phase 4: 動的設定モード（一部実装済み）
    - トークン暗号化 ✅ 実装済（crypto/token.rs）
    - MCPサーバー動的登録 ✅ 実装済（mcp_create_runner）
    ↓
Phase 5: UI/UX完成 🚧 一部実装済み
    - 全画面実装 ✅ 基本画面完了
    - エラーハンドリング 🚧 基本実装
    - テスト 🚧 未着手
```

> **現在の状況**: Phase 2完了、Phase 3（エージェント実行）着手準備中。Phase 4のMCPサーバー動的登録は先行実装済み。

### 2.2 各フェーズの詳細

---

## Phase 0: 環境構築・プロジェクト初期化

### 目標
- 開発環境の構築
- プロジェクト骨格の作成
- CI/CD基盤の準備

### タスク

#### P0-1: 開発環境準備
```bash
# 必要ツールのインストール確認
node --version  # >= 20.19.0 or >= 22.12.0
pnpm --version  # >= 9.0.0
rustc --version # stable >= 1.75
cargo install tauri-cli
```

#### P0-2: プロジェクト初期化
```bash
# Tauriプロジェクト作成
pnpm create tauri-app local-code-agent --template react-ts

# ディレクトリ構造整備
mkdir -p src/{routes,components,hooks,lib,stores,types}
mkdir -p src-tauri/src/{commands,grpc,db,crypto}
```

#### P0-3: 依存関係設定
- `package.json`: React 19、TanStack Router/Query、shadcn/ui、@inlang/paraglide-js
- `Cargo.toml`: tonic、rusqlite、aes-gcm
- `project.inlang/settings.json`: i18n設定ファイル作成
- `src/messages/`: 初期翻訳ファイル（en.json, ja.json）作成

#### P0-4: Proto定義のコピーとビルド設定
```bash
# jobworkerp-rsからProto定義をコピー
cp -r ../proto/protobuf src-tauri/proto/

# build.rsでtonic-build設定
```

### 成果物
- [x] 起動可能なTauriアプリケーション（空のウィンドウ）
- [x] Proto生成が動作すること
- [x] CI基本設定（lint、型チェック）

---

## Phase 1: コアインフラ

### 目標
- Tauri Rustバックエンドの基盤構築
- SQLiteデータベース接続
- jobworkerp-rsへのgRPC接続

### タスク

#### P1-1: Tauri State管理
```rust
// src-tauri/src/state.rs
pub struct AppState {
    pub db: DbPool,
    pub crypto: TokenCrypto,
    pub grpc: Arc<JobworkerpClient>,
}
```

#### P1-2: SQLiteセットアップ
```rust
// src-tauri/src/db/connection.rs
// refinery マイグレーション設定
// V1__initial.sql 作成
```

**マイグレーションSQL（静的設定モード用）:**
```sql
-- app_settings テーブル
CREATE TABLE app_settings (
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

-- repositories テーブル
CREATE TABLE repositories (
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

-- agent_jobs テーブル
CREATE TABLE agent_jobs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  repository_id INTEGER NOT NULL REFERENCES repositories(id),
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

-- インデックス
CREATE INDEX idx_repositories_mcp_server ON repositories(mcp_server_name);
CREATE INDEX idx_agent_jobs_repository ON agent_jobs(repository_id);
CREATE INDEX idx_agent_jobs_status ON agent_jobs(status);
CREATE INDEX idx_agent_jobs_jobworkerp_id ON agent_jobs(jobworkerp_job_id);

-- 初期設定
INSERT INTO app_settings (id) VALUES (1);
```

#### P1-3: gRPCクライアント基盤
```rust
// src-tauri/src/grpc/client.rs
pub struct JobworkerpClient {
    channel: Channel,
    auth_token: Option<String>,
}

impl JobworkerpClient {
    pub fn new(url: &str) -> Result<Self, AppError>;
    pub async fn check_connection(&self) -> Result<bool, AppError>;
}
```

#### P1-4: エラー型定義
```rust
// src-tauri/src/error.rs
#[derive(Error, Debug)]
pub enum AppError {
    NotFound(String),
    GrpcError(String),
    DbError(String),
    // ...
}
```

#### P1-5: 基本Tauriコマンド
```rust
// 接続確認コマンド
#[tauri::command]
pub async fn check_jobworkerp_connection(
    grpc: State<'_, JobworkerpClient>,
) -> Result<bool, AppError>;

// 設定取得コマンド
#[tauri::command]
pub async fn get_app_settings(
    db: State<'_, DbPool>,
) -> Result<AppSettings, AppError>;
```

### 成果物
- [x] SQLiteマイグレーション動作
- [x] jobworkerp-rsへの接続確認成功
- [x] Tauri Commandsの基本パターン確立

---

## Phase 2: 静的設定モード実装

### 目標
- mcp-settings.tomlで事前設定されたMCPサーバーの利用
- リポジトリ管理機能
- Issue/PR一覧表示

### タスク

#### P2-1: MCPサーバー管理コマンド
```rust
// src-tauri/src/commands/mcp.rs

/// 設定済みMCPサーバー一覧を取得
#[tauri::command]
pub async fn mcp_list_servers(
    grpc: State<'_, JobworkerpClient>,
) -> Result<Vec<McpServerInfo>, AppError>;

/// MCPサーバーの接続確認
#[tauri::command]
pub async fn mcp_check_connection(
    server_name: String,
    grpc: State<'_, JobworkerpClient>,
) -> Result<bool, AppError>;
```

**実装詳細:**
- `RunnerService.FindListBy`でrunner_type=MCP_SERVERのランナー一覧を取得
- 接続確認は`tools/list`呼び出しで検証

#### P2-2: リポジトリ管理コマンド
```rust
// src-tauri/src/commands/repository.rs

#[tauri::command]
pub async fn repository_add(
    request: AddRepositoryRequest,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<Repository, AppError>;

#[tauri::command]
pub async fn repository_list(
    db: State<'_, DbPool>,
) -> Result<Vec<Repository>, AppError>;

#[tauri::command]
pub async fn repository_sync(
    id: i64,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<Repository, AppError>;
```

#### P2-3: Issue/PR取得コマンド
```rust
// src-tauri/src/commands/issue.rs

#[tauri::command]
pub async fn issue_list(
    repository_id: i64,
    filters: IssueFilters,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<Vec<Issue>, AppError>;

#[tauri::command]
pub async fn pr_list(
    repository_id: i64,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<Vec<PullRequest>, AppError>;

#[tauri::command]
pub async fn find_related_prs(
    repository_id: i64,
    issue_number: i64,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<Vec<PullRequest>, AppError>;
```

#### P2-4: gRPC Runner/Worker管理メソッド

```rust
// src-tauri/src/grpc/client.rs に追加

impl JobworkerpClient {
    // Runner管理
    pub async fn find_runner_by_name(&self, name: &str) -> Result<Option<data::Runner>, AppError>;
    pub async fn create_runner(&self, data: data::RunnerData) -> Result<i64, AppError>;

    // Worker管理
    pub async fn find_worker_by_name(&self, name: &str) -> Result<Option<data::Worker>, AppError>;
    pub async fn create_worker(&self, data: data::WorkerData) -> Result<i64, AppError>;

    // 自動プロビジョニング（核心）
    /// MCPツール呼び出し時にWorkerが存在しない場合、自動的に作成する
    /// 1. Worker を名前で検索 → 存在すれば返す
    /// 2. Runner を名前で検索 → 存在しなければエラー（Runnerは事前登録必須）
    /// 3. Worker を作成（Runner IDを参照、Runner名と同一名）
    /// 4. 作成したWorkerを返す
    pub async fn ensure_mcp_worker(&self, mcp_server_name: &str) -> Result<data::Worker, AppError>;
}
```

#### P2-5: MCPサーバー（Runner）動的登録コマンド

```rust
// src-tauri/src/commands/mcp.rs に追加

/// GitHub/Gitea MCPサーバー（Runner）を動的登録
/// TOML定義は内部でplatformに応じて自動生成（URLからscheme/hostを抽出）
#[tauri::command]
pub async fn mcp_create_runner(
    grpc: State<'_, Arc<JobworkerpClient>>,
    platform: String,     // "GitHub" or "Gitea"
    name: String,         // MCPサーバー識別名
    url: String,          // URL (https://github.com, https://gitea.example.com)
    token: String,        // Personal Access Token
) -> Result<McpServerInfo, AppError>;
```

**TOML生成（Docker実行形式）:**
- GitHub: `docker run ghcr.io/github/github-mcp-server` + GITHUB_PERSONAL_ACCESS_TOKEN, GITHUB_HOST（Enterprise時のみ）
- Gitea: `docker run docker.gitea.com/gitea-mcp-server` + GITEA_ACCESS_TOKEN, GITEA_HOST, GITEA_INSECURE（http時のみ）

#### P2-6: リポジトリ登録フォーム拡張

リポジトリ登録時にMCPサーバーの選択/新規作成を可能にする。

**UIフロー:**
1. MCPサーバー選択ドロップダウン（既存の`mcp_list_servers`結果を表示）
2. 「新規MCPサーバー作成」オプション選択時にフォームを展開
   - プラットフォーム選択（GitHub / Gitea）
   - サーバー識別名
   - URL（GitHubはデフォルト`https://github.com`、GitHub Enterpriseは変更可、Giteaは必須入力）
   - Personal Access Token

**MCP呼び出し実装パターン:**
```rust
impl JobworkerpClient {
    pub async fn call_mcp_tool<T: DeserializeOwned>(
        &self,
        server_name: &str,
        tool_name: &str,
        args: serde_json::Value,
    ) -> Result<T, AppError> {
        // 1. Workerを確保（存在しなければ自動作成）
        let worker = self.ensure_mcp_worker(server_name).await?;

        // 2. ジョブを投入
        let request = proto::JobRequest {
            worker: Some(proto::job_request::Worker::WorkerName(
                worker.data.name.clone()
            )),
            args: serde_json::to_vec(&args)?,
            using: Some(tool_name.to_string()),
            ..Default::default()
        };

        // 3. 結果を待機
        let response = self.enqueue_and_wait(request).await?;

        // 4. デシリアライズ
        let result: T = serde_json::from_slice(&response.output)?;
        Ok(result)
    }
}
```

#### P2-7: フロントエンド基本画面

**TanStack Router設定:**
```typescript
// src/routes/__root.tsx
export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
});

// src/routes/index.tsx (ダッシュボード)
// src/routes/repositories/index.tsx
// src/routes/repositories/$id/issues.tsx
```

**shadcn/uiセットアップ:**
```bash
pnpm dlx shadcn@latest init
pnpm dlx shadcn@latest add button card dialog table badge
```

### 成果物
- [x] MCPサーバー一覧表示（`mcp_list_servers`コマンド実装済）
- [x] MCPサーバー接続確認（`mcp_check_connection`コマンド実装済）
- [x] Runner/Worker管理メソッド（`find_runner_by_name`, `find_worker_by_name`, `create_runner`, `create_worker`実装済）
- [x] Worker自動作成機能（`ensure_mcp_worker`実装済）
- [x] MCPサーバー（Runner）動的登録機能（`mcp_create_runner`コマンド実装済）
- [x] MCP呼び出し実装（`call_mcp_tool`メソッド実装済）
- [x] リポジトリ登録・一覧表示（`create_repository`, `list_repositories`, `get_repository`, `delete_repository`実装済）
- [x] Issue一覧表示（`list_issues`, `get_issue`実装済、stateフィルター対応）
- [x] PR一覧表示（`list_pulls`実装済）
- [x] 関連PR検出・警告表示（`find_related_prs`実装済、title/body/branch正規表現マッチ）
- [x] リポジトリ登録フォーム拡張（MCPサーバー選択/新規作成UI実装済 - `src/routes/repositories.tsx`）
- [x] フロントエンド基本画面（TanStack Router、ダッシュボード、リポジトリ一覧/詳細、設定画面）

---

## Phase 3: エージェント実行

### 目標
- Claude Codeによるコード生成ワークフロー
- ジョブ投入とストリーミング表示
- PR作成

### タスク

#### P3-1: エージェントワークフロー定義
```yaml
# workflows/code-agent-workflow.yaml
document:
  dsl: "1.0.0"
  namespace: "local-code-agent"
  name: "code-agent-workflow"
  version: "1.0.0"

input:
  schema:
    document:
      type: object
      properties:
        owner: { type: string }
        repo: { type: string }
        issue_number: { type: integer }
        issue_title: { type: string }
        base_branch: { type: string, default: "main" }
        worktree_base_path: { type: string }
        local_repo_path: { type: string }
        mcp_server: { type: string }
      required:
        - owner
        - repo
        - issue_number
        - issue_title
        - worktree_base_path
        - local_repo_path
        - mcp_server

do:
  # 1. ブランチ名・パス決定
  - determineBranchName:
      set:
        branch_name: "${\"issue-\" + (.issue_number | tostring)}"
        worktree_path: "${.worktree_base_path + \"/issue-\" + (.issue_number | tostring)}"

  # 2. メイン処理（エラーハンドリング付き）
  - mainProcessWithErrorHandling:
      try:
        - checkExistingBranch:
            # ... ブランチ存在確認
        - createWorktree:
            # ... worktree作成
        - fetchIssue:
            # ... Issue情報取得
        - fetchIssueComments:
            # ... コメント取得
        - generatePrompt:
            # ... プロンプト生成
        - writePromptFile:
            # ... ファイル書き出し
        - runAgent:
            # ... Claude Code実行
        - pushChanges:
            # ... git push
        - createPR:
            # ... PR作成
        - cleanup:
            # ... worktree削除
      catch:
        as: error
        do:
          - cleanupOnError:
              # ... エラー時クリーンアップ
          - raiseError:
              # ... エラー再送出

output:
  schema:
    document:
      type: object
      properties:
        status: { type: string }
        pr_number: { type: integer }
        pr_url: { type: string }
```

#### P3-2: エージェント実行コマンド
```rust
// src-tauri/src/commands/agent.rs

#[tauri::command]
pub async fn agent_start(
    request: StartAgentRequest,
    app: AppHandle,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<StartAgentResponse, AppError> {
    // 1. リポジトリ情報取得
    let repo_info = get_repository(&db, request.repository_id)?;

    // 2. AppSettings取得（worktree_base_path等）
    let settings = get_app_settings(&db)?;

    // 3. ワークフロー引数構築
    let workflow_args = build_workflow_args(&repo_info, &settings, &request);

    // 4. ワークフロー実行（EnqueueForStream）
    let job_id = grpc.enqueue_workflow_stream(
        "code-agent-workflow",
        &workflow_args,
    ).await?;

    // 5. ローカルDBに記録
    let agent_job = create_agent_job(&db, &request, &job_id)?;

    // 6. バックグラウンドでストリーミング開始
    spawn_stream_listener(app, grpc, job_id.clone());

    Ok(StartAgentResponse {
        job_id: agent_job.id,
        jobworkerp_job_id: job_id,
    })
}
```

#### P3-3: ストリーミングイベント変換
```rust
async fn stream_job_results(
    app: AppHandle,
    grpc: JobworkerpClient,
    job_id: String,
) -> Result<(), AppError> {
    let mut stream = grpc.listen_stream(&job_id).await?;

    while let Some(item) = stream.message().await? {
        match item.item {
            Some(proto::result_output_item::Item::Data(data)) => {
                // ストリーミングデータをTauriイベントとして発火
                app.emit(&format!("job-stream-{}", job_id), StreamEvent::Data {
                    data: data.to_vec(),
                })?;
            }
            Some(proto::result_output_item::Item::End(trailer)) => {
                app.emit(&format!("job-stream-{}", job_id), StreamEvent::End)?;

                // 完了時にローカルDBを更新
                update_job_status(&job_id, "PrCreated")?;
                break;
            }
            Some(proto::result_output_item::Item::FinalCollected(data)) => {
                // ワークフロー最終結果をパース
                let result: WorkflowResult = serde_json::from_slice(&data)?;

                if result.status == "success" {
                    update_job_with_pr(&job_id, result.pr_number, result.pr_url)?;
                } else {
                    update_job_error(&job_id, &result.error)?;
                }

                app.emit(&format!("job-stream-{}", job_id), StreamEvent::FinalCollected {
                    data: data.to_vec(),
                })?;
                break;
            }
            None => {}
        }
    }

    Ok(())
}
```

#### P3-4: ジョブキャンセル
```rust
#[tauri::command]
pub async fn agent_cancel(
    jobworkerp_job_id: String,
    db: State<'_, DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<(), AppError> {
    // 1. gRPCでキャンセル
    grpc.delete_job(&jobworkerp_job_id).await?;

    // 2. ローカルDB更新
    update_job_status(&jobworkerp_job_id, "Cancelled")?;

    Ok(())
}
```

#### P3-5: フロントエンドストリーミング表示
```typescript
// src/hooks/use-job-stream.ts
export function useJobStream(jobId: string) {
  const [chunks, setChunks] = useState<Uint8Array[]>([]);
  const [status, setStatus] = useState<StreamStatus>('idle');

  useEffect(() => {
    const unlisten = listenJobStream(jobId, (event) => {
      switch (event.type) {
        case 'Data':
          setStatus('streaming');
          setChunks(prev => [...prev, new Uint8Array(event.data)].slice(-1000));
          break;
        case 'End':
        case 'FinalCollected':
          setStatus('completed');
          break;
      }
    });

    return () => { unlisten.then(fn => fn()); };
  }, [jobId]);

  return { chunks, status };
}
```

### 成果物
- [ ] ワークフロー定義ファイル
- [ ] エージェント実行→PR作成の一連のフロー動作
- [ ] ストリーミング出力のリアルタイム表示
- [ ] ジョブキャンセル機能

---

## Phase 4: 動的設定モード

### 目標
- トークンの暗号化保存
- MCPサーバーの動的登録
- マルチアカウント対応

### タスク

#### P4-1: SQLiteマイグレーション追加
```sql
-- V2__dynamic_mode.sql
CREATE TABLE token_stores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  encrypted_token BLOB NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

CREATE TABLE platform_configs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  base_url TEXT NOT NULL,
  token_id INTEGER NOT NULL REFERENCES token_stores(id),
  mcp_runner_name TEXT,
  user_name TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (platform, base_url)
);

CREATE INDEX idx_platform_configs_platform ON platform_configs(platform);
```

#### P4-2: トークン暗号化モジュール
```rust
// src-tauri/src/crypto/token.rs
pub struct TokenCrypto {
    cipher: Aes256Gcm,
}

impl TokenCrypto {
    pub fn new() -> Result<Self, CryptoError>;
    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, CryptoError>;
    pub fn decrypt(&self, encrypted: &[u8]) -> Result<String, CryptoError>;
}
```

**鍵管理:**
1. OS Keychain（優先）
2. ファイルベース（フォールバック）

#### P4-3: プラットフォーム管理コマンド
```rust
#[tauri::command]
pub async fn platform_create(
    request: CreatePlatformRequest,
    db: State<'_, DbPool>,
    crypto: State<'_, TokenCrypto>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<PlatformConfig, AppError> {
    // 1. トークン検証
    let user_info = validate_token(&request).await?;

    // 2. トークン暗号化
    let encrypted = crypto.encrypt(&request.token)?;

    // 3. MCPサーバー動的登録
    let mcp_runner_name = format!("{}-{}",
        request.platform.to_lowercase(),
        user_info.login
    );

    grpc.create_mcp_runner(&CreateRunnerRequest {
        name: mcp_runner_name.clone(),
        description: format!("{} MCP Server for {}", request.platform, user_info.login),
        runner_type: RunnerType::MCP_SERVER,
        definition: build_mcp_toml(&request),
    }).await?;

    // 4. DBに保存
    let config = save_platform_config(&db, &request, &encrypted, &mcp_runner_name)?;

    Ok(config)
}
```

**MCP TOML生成:**
```rust
fn build_mcp_toml(request: &CreatePlatformRequest) -> String {
    match request.platform.as_str() {
        "GitHub" => format!(r#"
name = "{}"
description = "GitHub MCP Server"
transport = "stdio"
command = "npx"
args = ["-y", "@modelcontextprotocol/server-github"]
envs = {{ GITHUB_PERSONAL_ACCESS_TOKEN = "{}" }}
"#,
            request.platform.to_lowercase(),
            request.token
        ),
        "Gitea" => format!(r#"
name = "{}"
description = "Gitea MCP Server"
transport = "stdio"
command = "gitea-mcp-server"
envs = {{ GITEA_TOKEN = "{}", GITEA_URL = "{}" }}
"#,
            request.platform.to_lowercase(),
            request.token,
            request.base_url
        ),
        _ => panic!("Unknown platform"),
    }
}
```

#### P4-4: モード判定ロジック
```rust
// src-tauri/src/mode.rs

pub enum AppMode {
    Static,   // mcp-settings.tomlで事前設定
    Dynamic,  // アプリで動的管理
    Mixed,    // 両方併用
}

pub async fn detect_app_mode(
    db: &DbPool,
    grpc: &JobworkerpClient,
) -> Result<AppMode, AppError> {
    // 1. 静的設定されたMCPサーバーを確認
    let static_servers = grpc.list_mcp_servers().await?;
    let has_github_or_gitea = static_servers.iter()
        .any(|s| s.name.contains("github") || s.name.contains("gitea"));

    // 2. 動的設定されたプラットフォームを確認
    let conn = db.get()?;
    let platform_count: i32 = conn.query_row(
        "SELECT COUNT(*) FROM platform_configs",
        [],
        |row| row.get(0),
    )?;

    match (has_github_or_gitea, platform_count > 0) {
        (true, false) => Ok(AppMode::Static),
        (false, true) => Ok(AppMode::Dynamic),
        (true, true) => Ok(AppMode::Mixed),
        (false, false) => Ok(AppMode::Static), // 初回起動、設定ガイドへ
    }
}
```

### 成果物
- [ ] トークン暗号化・復号
- [ ] プラットフォーム設定画面
- [ ] MCPサーバー動的登録
- [ ] モード自動判定

---

## Phase 5: UI/UX完成

### 目標
- 全画面の実装完了
- エラーハンドリングの充実
- テスト

### タスク

#### P5-1: 残り画面実装
- [ ] ダッシュボード（最近のジョブ、クイックアクション）
- [ ] 設定画面（worktree_base_path、タイムアウト設定）
- [ ] ジョブ一覧画面
- [ ] ジョブ詳細画面（ストリーミング表示）
- [ ] プラットフォーム設定画面（動的モード）

#### P5-2: エラーハンドリング強化
```typescript
// src/components/error-boundary.tsx
export function ErrorBoundary({ children }: { children: React.ReactNode }) {
  return (
    <QueryErrorResetBoundary>
      {({ reset }) => (
        <ReactErrorBoundary
          onReset={reset}
          fallbackRender={({ error, resetErrorBoundary }) => (
            <ErrorFallback error={error} onReset={resetErrorBoundary} />
          )}
        >
          {children}
        </ReactErrorBoundary>
      )}
    </QueryErrorResetBoundary>
  );
}
```

#### P5-3: テスト実装
```bash
# Rustユニットテスト
cargo test --package local-code-agent

# フロントエンドユニットテスト
pnpm run test

# E2Eテスト
pnpm run test:e2e
```

**テストカバレッジ目標:**
- Rustバックエンド: 70%以上
- フロントエンド: 60%以上
- E2E: 主要フロー（リポジトリ登録→エージェント実行→完了確認）

#### P5-4: ドキュメント更新
- [ ] README.md作成
- [ ] 開発者ガイド
- [ ] ユーザーガイド

#### P5-5: 国際化（i18n）
- [ ] 全UIテキストの翻訳キー化
- [ ] LanguageSwitcherコンポーネント実装
- [ ] 言語設定永続化（Tauri Store）
- [ ] 翻訳レビュー・修正

### 成果物
- [ ] 全機能が動作するアプリケーション
- [ ] テストスイート
- [ ] ドキュメント

---

## 3. 技術的決定事項

### 3.1 実装時に確定する事項

| 項目 | 選択肢 | 決定時期 |
|------|--------|---------|
| Viteバージョン | 7.x or 6.x | Phase 0 |
| Tailwind CSSバージョン | 4.x or 3.x | Phase 0 |
| 暗号化アルゴリズム | AES-256-GCM | 確定 |
| 鍵管理 | OS Keychain優先、ファイルフォールバック | 確定 |

### 3.2 アーキテクチャ決定

```
┌─────────────────────────────────────────────────────────┐
│                     WebView (React)                      │
│  ┌─────────────────────────────────────────────────────┐│
│  │  TanStack Router + Query + shadcn/ui                 ││
│  └──────────────────────┬──────────────────────────────┘│
│                         │ Tauri IPC                      │
│  ┌──────────────────────▼──────────────────────────────┐│
│  │              Tauri Rust Backend                      ││
│  │  ┌───────────────┐ ┌───────────────┐ ┌────────────┐ ││
│  │  │ Commands      │ │ gRPC Client   │ │ SQLite     │ ││
│  │  │ (Tauri)       │ │ (tonic)       │ │ (rusqlite) │ ││
│  │  └───────────────┘ └───────┬───────┘ └────────────┘ ││
│  └────────────────────────────│─────────────────────────┘│
└───────────────────────────────│──────────────────────────┘
                                │ gRPC (native)
                                ▼
                    ┌───────────────────────┐
                    │    jobworkerp-rs      │
                    │  - WORKFLOW Runner    │
                    │  - MCP_SERVER Runner  │
                    │  - COMMAND Runner     │
                    └───────────────────────┘
```

---

## 4. リスクと対策

| リスク | 影響度 | 対策 |
|--------|--------|------|
| Claude Code認証の有効期限切れ | 高 | エラー検出時に再認証ガイドを表示 |
| jobworkerp-rs接続断 | 高 | 指数バックオフでリトライ、接続状態表示 |
| 大規模リポジトリでのworktree作成遅延 | 中 | 進捗表示、タイムアウト設定 |
| MCPサーバーのAPI変更 | 中 | 動的にtools/listを取得してツール名を確認 |
| Tauri v2の破壊的変更 | 低 | 安定版リリースを使用 |

---

## 5. マイルストーン

| マイルストーン | 内容 | 想定工数 |
|---------------|------|---------|
| M1: 環境構築完了 | Phase 0完了 | 1-2日 |
| M2: 接続確認成功 | Phase 1完了、jobworkerp-rsと通信可能 | 3-5日 |
| M3: Issue表示 | Phase 2完了、リポジトリ・Issue一覧表示 | 5-7日 |
| M4: エージェント動作 | Phase 3完了、PR作成まで動作 | 7-10日 |
| M5: MVP完成 | Phase 4-5完了、全機能動作 | 3-10日 |

**合計想定工数: 19-34日**

---

## 6. 開始前チェックリスト

実装開始前に以下を確認すること：

- [ ] jobworkerp-rsが起動していること
- [ ] mcp-settings.tomlにGitHub/Gitea MCPサーバーが設定されていること（静的モード検証用）
- [ ] Claude Code認証が完了していること
- [ ] Node.js 20.19+またはNode.js 22.12+がインストールされていること
- [ ] Rust stable 1.75+がインストールされていること
- [ ] Tauri CLIがインストールされていること（`cargo install tauri-cli`）

---

## 付録A: ファイル構成チェックリスト

### Tauri Rust Backend
- [ ] `src-tauri/Cargo.toml`
- [ ] `src-tauri/tauri.conf.json`
- [ ] `src-tauri/capabilities/default.json`
- [ ] `src-tauri/build.rs`
- [ ] `src-tauri/src/main.rs`
- [ ] `src-tauri/src/lib.rs`
- [ ] `src-tauri/src/state.rs`
- [ ] `src-tauri/src/error.rs`
- [ ] `src-tauri/src/commands/mod.rs`
- [ ] `src-tauri/src/commands/mcp.rs`
- [ ] `src-tauri/src/commands/repository.rs`
- [ ] `src-tauri/src/commands/issue.rs`
- [ ] `src-tauri/src/commands/agent.rs`
- [ ] `src-tauri/src/commands/platform.rs`
- [ ] `src-tauri/src/grpc/mod.rs`
- [ ] `src-tauri/src/grpc/client.rs`
- [ ] `src-tauri/src/db/mod.rs`
- [ ] `src-tauri/src/db/connection.rs`
- [ ] `src-tauri/src/db/migrations/V1__initial.sql`
- [ ] `src-tauri/src/db/migrations/V2__dynamic_mode.sql`
- [ ] `src-tauri/src/crypto/mod.rs`
- [ ] `src-tauri/src/crypto/token.rs`

### Frontend
- [ ] `package.json`
- [ ] `vite.config.ts`
- [ ] `tailwind.config.ts`
- [ ] `tsconfig.json`
- [ ] `src/main.tsx`
- [ ] `src/App.tsx`
- [ ] `src/routes/__root.tsx`
- [ ] `src/routes/index.tsx`
- [ ] `src/routes/repositories/index.tsx`
- [ ] `src/routes/repositories/$id/index.tsx`
- [ ] `src/routes/repositories/$id/issues.tsx`
- [ ] `src/routes/jobs/index.tsx`
- [ ] `src/routes/jobs/$id.tsx`
- [ ] `src/routes/settings.tsx`
- [ ] `src/lib/tauri/commands.ts`
- [ ] `src/lib/tauri/events.ts`
- [ ] `src/hooks/use-job-stream.ts`

### Workflow
- [ ] `workflows/code-agent-workflow.yaml`
