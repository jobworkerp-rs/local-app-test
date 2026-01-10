# Local Code Agent Service フロントエンド技術仕様書

## 概要

本文書は、Local Code Agent Service のフロントエンドアプリケーションの技術仕様を定義する。
**Tauri v2** を使用したデスクトップアプリケーションとして実装し、バックエンドとして jobworkerp-rs を利用する。

### 関連文書

- PRD: `local-code-agent-service-prd.md` - サービス要件定義
- 技術統合仕様: `local-code-agent-jobworkerp-integration.md` - jobworkerp-rs統合の技術詳細
- jobworkerp-rs: https://github.com/jobworkerp-rs/jobworkerp-rs - バックエンドジョブワーカーシステム

### アーキテクチャ選定理由

| 検討オプション | 評価 | 採用 |
|---------------|------|------|
| **Tauri v2** | セキュア（Rustバックエンド）、軽量、クロスプラットフォーム | ✅ |
| BFF + SPA | 別プロセス管理が必要、ポート衝突リスク | - |
| Electron | バンドルサイズ大、メモリ消費大 | - |
| 純粋SPA | トークン暗号化・ファイル操作のセキュリティリスク | - |

**Tauriを選択した理由**:
- **セキュリティ**: トークン暗号化・SQLite操作をRustバックエンドで安全に処理
- **軽量**: Electronの1/10以下のバンドルサイズ
- **Rust統合**: jobworkerp-rsと同じRustエコシステム、gRPCネイティブクライアント利用可能
- **クロスプラットフォーム**: Windows/macOS/Linux対応

### MCPサーバー運用モード

GitHub/Gitea MCPサーバーは**サーバー起動時にトークンを設定**する仕様であり、リクエストごとの動的なトークン変更はサポートされない。

| モード | 説明 | 本アプリでのトークン管理 |
|-------|------|------------------------|
| **静的設定モード** | `mcp-settings.toml`でMCPサーバーとトークンを事前設定 | 不要（jobworkerp-rs側で管理） |
| **動的設定モード** | `RunnerService.Create`で実行時にMCPサーバーを登録 | 必要（SQLite + 暗号化） |

> **実装要件**: 両モードとも実装必須。ユーザーは運用状況に応じてどちらのモードも使用可能。

**静的設定モードの場合**:
- 本アプリはトークン管理不要
- `PlatformConfig`・`TokenStore`テーブルは不使用
- リポジトリ登録時はMCPサーバー名・URL・ローカルパスのみ管理
- セットアップが簡単、単一アカウント運用に適する

**動的設定モードの場合**:
- 本アプリがトークンを暗号化して管理
- `RunnerService.Create`経由でMCPサーバーを動的登録
- 複数アカウント・複数プラットフォームの切り替えが可能
- 柔軟な運用が可能、マルチアカウント運用に適する

---

## 1. アーキテクチャ概要

```
┌─────────────────────────────────────────────────────────────────┐
│                    Tauri Desktop Application                     │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────────────────────────────────────────────────┐│
│  │              WebView (Vite 7 + React 19)                     ││
│  │  ┌─────────────┐  ┌─────────────┐  ┌─────────────────────┐  ││
│  │  │  Routes     │  │  Components │  │  Hooks              │  ││
│  │  │  (TanStack) │  │  (shadcn)   │  │  (useJob, useRepo)  │  ││
│  │  └─────────────┘  └─────────────┘  └─────────────────────┘  ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │  State: TanStack Query + Zustand                        │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │  Tauri IPC Bridge (@tauri-apps/api)                     │││
│  │  │  - invoke('command_name', args)                         │││
│  │  │  - listen('event_name', callback)                       │││
│  │  └─────────────────────────────────────────────────────────┘││
│  └──────────────────────────┬──────────────────────────────────┘│
│                             │ IPC (JSON-RPC over WebView)       │
│  ┌──────────────────────────▼──────────────────────────────────┐│
│  │              Tauri Rust Backend (src-tauri/)                 ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │  Commands (Tauri Commands)                              │││
│  │  │  - platform::create, platform::list, platform::delete   │││
│  │  │  - repository::add, repository::sync                    │││
│  │  │  - agent::start, agent::cancel, agent::status           │││
│  │  │  - job::stream (Event Streaming)                        │││
│  │  └─────────────────────────────────────────────────────────┘││
│  │  ┌─────────────────┐  ┌─────────────────────────────────┐   ││
│  │  │  SQLite         │  │  gRPC Client (tonic)            │   ││
│  │  │  (rusqlite)     │  │  - JobService                   │   ││
│  │  │  - Encrypted    │  │  - JobResultService             │   ││
│  │  │    token store  │  │  - WorkerService                │   ││
│  │  └─────────────────┘  └─────────────────────────────────┘   ││
│  │  ┌─────────────────────────────────────────────────────────┐││
│  │  │  Crypto (ring/aes-gcm)                                  │││
│  │  │  - Token encryption/decryption                          │││
│  │  │  - Keychain integration (optional)                      │││
│  │  └─────────────────────────────────────────────────────────┘││
│  └─────────────────────────────────────────────────────────────┘│
└────────────────────────────────────────────────────────────────┬┘
                                                                 │
                                                                 │ gRPC (native, not gRPC-Web)
                                                                 ▼
┌─────────────────────────────────────────────────────────────────┐
│                      jobworkerp-rs Backend                       │
│  - JobService (Enqueue, EnqueueForStream, Delete)               │
│  - JobResultService (Listen, ListenStream)                      │
│  - JobProcessingStatusService (Find, FindByCondition)           │
│  - WorkerService, RunnerService                                 │
└─────────────────────────────────────────────────────────────────┘
```

### Tauri v2 アーキテクチャの利点

| 層 | 技術 | 利点 |
|----|------|------|
| **WebView** | Vite 7 + React 19 | 高速HMR、型安全ルーティング |
| **IPC** | Tauri Commands | 型安全、非同期、ストリーミング対応 |
| **Backend** | Rust | メモリ安全、高性能、gRPCネイティブ |
| **Storage** | rusqlite | ネイティブSQLite、暗号化対応 |
| **gRPC** | tonic | HTTP/2ネイティブ、ストリーミング完全対応 |

---

## 2. 技術スタック

### 2.1 デスクトップアプリケーション基盤

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| デスクトップフレームワーク | **Tauri v2** | 軽量、セキュア、Rust統合 |
| WebViewビルド | **Vite 7.x** | 高速HMR、ESM対応 |
| ルーティング | **TanStack Router** | 型安全ルーティング、TanStack Query統合 |
| フロントエンド言語 | **TypeScript 5.x** | 型安全性 |
| バックエンド言語 | **Rust (stable)** | メモリ安全、gRPCネイティブ |
| パッケージマネージャ | **pnpm** (frontend) / **cargo** (backend) | 高速、効率的 |

**Tauri v2の特徴**:
- **マルチウィンドウ**: 複数ウィンドウ対応
- **モバイル対応**: iOS/Android対応（将来拡張可能）
- **プラグインシステム**: 拡張可能なアーキテクチャ
- **セキュリティ**: CSP、ケイパビリティベースのパーミッション
- **IPC**: 型安全な非同期コマンド、イベントストリーミング

**Viteバージョン選定**:
- 最新安定版（7.x）を使用
- **Node.js要件**: 20.19以上または22.12以上
- **Vitest互換性**: Vitest 3.x

### 2.2 Tauri Rust バックエンド

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| gRPCクライアント | **tonic 0.12+** | HTTP/2ネイティブ、ストリーミング完全対応 |
| SQLite | **rusqlite + r2d2** | ネイティブ、コネクションプール |
| 暗号化 | **aes-gcm / ring** | 高性能、セキュア |
| 非同期ランタイム | **tokio** | Tauriデフォルト |
| シリアライズ | **serde + serde_json** | Tauri IPC標準 |
| キーチェーン | **keyring** | OS標準の認証情報管理（オプション） |
| Proto生成 | **tonic-build + prost** | jobworkerp-rsと同じツールチェーン |

### 2.3 UI/スタイリング

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| UIライブラリ | **shadcn/ui** | カスタマイズ可能、Tailwind CSS統合、アクセシビリティ |
| スタイリング | **Tailwind CSS 4.x** | ユーティリティファースト、ビルド最適化 |
| アイコン | **Lucide React** | 軽量、一貫したデザイン |
| ダークモード | **Zustand** + localStorage | ライブラリ依存を減らす |

### 2.4 状態管理・データフェッチ

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| サーバー状態 | **TanStack Query v5** | キャッシュ、再検証、楽観的更新 |
| ルーター統合 | **TanStack Router** | loaderによるデータプリフェッチ、Search Params型安全管理 |
| クライアント状態 | **Zustand** | 軽量、TypeScript親和性 |
| フォーム | **TanStack Form + Zod** | TanStackエコシステム統一、バリデーション |

### 2.5 Tauri IPC通信

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| コマンド呼び出し | **@tauri-apps/api invoke()** | 型安全、非同期 |
| イベント購読 | **@tauri-apps/api listen()** | ストリーミングデータ受信 |
| gRPC (Rust側) | **tonic** | HTTP/2ネイティブ、ストリーミング完全対応 |

**アーキテクチャ**: WebViewはTauri IPCを介してRustバックエンドを呼び出し、RustバックエンドがネイティブgRPCでjobworkerp-rsと通信する。gRPC-Webは使用しない。

### 2.6 ローカルデータベース

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| DB | **SQLite** | 軽量、サーバーレス、PRDのデータモデルと互換 |
| Rust実装 | **rusqlite + r2d2** | ネイティブ、コネクションプール |
| マイグレーション | **refinery** | Rust標準、SQLファイルベース |

**注意**: SQLiteはTauri Rustバックエンドで操作し、WebView側からは直接アクセスしない。

### 2.7 ストリーミング・リアルタイム

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| Tauri Event | **emit() / listen()** | WebViewへのプッシュ通知 |
| gRPCストリーミング | **tonic Streaming<T>** | jobworkerp-rsの`ListenStream`対応 |
| パターン | **Rust→Event→WebView** | gRPCストリームをTauriイベントに変換 |

### 2.8 テスト

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| フロントエンドユニット | **Vitest 3.x** | 高速、ESM対応、Vite 7互換 |
| Rustユニット | **cargo test** | 標準テストフレームワーク |
| コンポーネントテスト | **Testing Library** | アクセシビリティ重視 |
| E2Eテスト | **Playwright + Tauri Driver** | デスクトップアプリ対応 |

### 2.9 ビルド・開発ツール

| カテゴリ | 技術 | 理由 |
|---------|------|------|
| ビルド | **Vite 7.x** | 高速HMR、ESM対応 |
| Lint | **ESLint + Biome** | 一貫性、パフォーマンス |
| フォーマット | **Biome** | 高速、ESLint統合 |
| 型チェック | **tsc + @tanstack/router-plugin/vite** | ルート型自動生成 |
| テスト | **Vitest 3.x** | Vite 7互換、高速 |

---

## 3. jobworkerp-rs gRPC API 連携仕様

> **注記**: gRPC APIの詳細仕様は `docs/local-code-agent-jobworkerp-integration.md` を参照。
> 本セクションでは、Tauriアプリケーションからの利用パターンに焦点を当てる。

### 3.1 主要サービス

jobworkerp-rsは以下のgRPCサービスを公開する。Tauri Rustバックエンドからtonic経由でネイティブgRPCとして利用する（gRPC-Webは使用しない）。

#### 3.1.1 JobService（ジョブ実行）

```protobuf
service JobService {
  // ジョブをキューに追加（非同期）
  rpc Enqueue(JobRequest) returns (CreateJobResponse);

  // ジョブをキューに追加し、結果をストリーミング受信
  rpc EnqueueForStream(JobRequest) returns (stream ResultOutputItem);

  // ジョブの削除/キャンセル
  // - PENDING: キューから削除（JobResult作成なし）
  // - RUNNING/WAIT_RESULT: キャンセル、JobResultにCANCELLED状態で記録
  rpc Delete(JobId) returns (SuccessResponse);

  // ジョブ検索（DB/Redisキュー利用時のみ）
  rpc Find(JobId) returns (OptionalJobResponse);
  rpc FindList(FindListRequest) returns (stream Job);
  rpc FindQueueList(FindQueueListRequest) returns (stream JobAndStatus);
}

message JobRequest {
  oneof worker {
    WorkerId worker_id = 1;
    string worker_name = 2;
  }
  bytes args = 3;                    // Protobufシリアライズ済み引数
  optional string uniq_key = 4;      // 重複防止キー
  optional int64 run_after_time = 5; // 遅延実行（Unix ms）
  optional Priority priority = 6;    // HIGH/MEDIUM/LOW
  optional uint64 timeout = 7;       // タイムアウト（ms）
  optional string using = 8;         // MCP/Pluginランナーのメソッド指定
}

message CreateJobResponse {
  JobId id = 1;
  optional JobResult result = 2;     // DIRECT応答時のみ
}
```

#### 3.1.2 JobResultService（ジョブ結果）

```protobuf
service JobResultService {
  // 結果をポーリング（タイムアウトまで待機）
  rpc Listen(ListenRequest) returns (JobResult);

  // 結果をストリーミング受信
  rpc ListenStream(ListenRequest) returns (stream ResultOutputItem);

  // 特定ワーカーの全結果をストリーミング
  rpc ListenByWorker(ListenByWorkerRequest) returns (stream JobResult);

  // 結果検索（フィルタ・ソート対応）
  rpc FindListBy(FindJobResultListRequest) returns (stream JobResult);
  rpc CountBy(CountJobResultRequest) returns (CountResponse);
  rpc DeleteBulk(DeleteJobResultBulkRequest) returns (DeleteJobResultBulkResponse);
}

message ResultOutputItem {
  oneof item {
    bytes data = 1;           // データチャンク（例: LLMトークン）
    Trailer end = 2;          // ストリーム終了マーカー
    bytes final_collected = 3; // STREAMING_TYPE_INTERNAL用最終結果
  }
}
```

#### 3.1.3 JobProcessingStatusService（ジョブステータス監視）

```protobuf
service JobProcessingStatusService {
  // 単一ジョブのステータス取得
  rpc Find(JobId) returns (OptionalJobProcessingStatusResponse);

  // 全ジョブのステータスストリーミング
  rpc FindAll(Empty) returns (stream JobProcessingStatusResponse);

  // 条件付き検索（RDBインデックス有効時のみ）
  rpc FindByCondition(FindJobProcessingStatusRequest)
      returns (stream JobProcessingStatusDetailResponse);
}

enum JobProcessingStatus {
  UNKNOWN = 0;
  PENDING = 1;      // キュー待機中
  RUNNING = 2;      // 実行中
  WAIT_RESULT = 3;  // 結果処理待ち
  CANCELLING = 4;   // キャンセル中
}
```

#### 3.1.4 FunctionService / FunctionSetService について

**FunctionService**はJSON形式でのやり取りを前提としたフロントエンド向けラッパー層であり、
Protobufを直接扱えないクライアント向けに設計されています。Tauri Rustバックエンドからは
`JobService`/`JobResultService`/`WorkerService`を直接利用するため、
本アプリケーションでは`FunctionService`は基本的に使用しません。

**FunctionSetService**はLLMツール呼び出し用のメタデータ管理サービスです。
ワークフローを拡張してLLM機能（ツール定義、関数セット管理等）を統合する場合に利用を検討します。

**将来の拡張時の利用ケース**:
- LLMワークフローでのツール定義管理
- エージェントへのカスタムツールセット提供
- 動的なツール追加・削除

### 3.2 ストリーミングパターン

#### 3.2.1 ワークフロー実行のストリーミング

```typescript
// ワークフロー実行時のストリーミングイベント
interface WorkflowEvent {
  // ストリーミングジョブ開始（LLM_CHAT等）
  streaming_job_started?: JobStartedEvent;
  streaming_job_completed?: JobCompletedEvent;

  // 通常ジョブ開始
  job_started?: JobStartedEvent;
  job_completed?: JobCompletedEvent;

  // タスク開始/完了（ForTask, SwitchTask等）
  task_started?: TaskStartedEvent;
  task_completed?: TaskCompletedEvent;

  // ストリーミングデータ（LLMトークン等）
  streaming_data?: StreamingDataEvent;
}

interface JobStartedEvent {
  job_id: JobId;
  runner_name: string;           // "LLM_CHAT", "COMMAND"等
  worker_name?: string;          // ワーカー名（一時ワーカーはnull）
  position: string;              // JSON Pointer形式: "/tasks/0/do/1"
}

interface StreamingDataEvent {
  job_id: JobId;
  data: Uint8Array;              // UTF-8テキストまたはProtobuf
}
```

#### 3.2.2 Tauriアーキテクチャでのストリーミング処理

Tauriアーキテクチャでは、gRPCストリーミングはRustバックエンドで処理し、WebViewへはTauriイベントとして通知する。

**アーキテクチャ**:
```
jobworkerp-rs (gRPC Stream) → Tauri Rust Backend → Tauri Event → WebView
```

**React Hookでの受信パターン**（セクション7.3で詳述）:
```typescript
// Tauriイベントベースのストリーミング受信
function useJobStream(jobId: string) {
  const [chunks, setChunks] = useState<Uint8Array[]>([]);
  const [status, setStatus] = useState<'idle' | 'streaming' | 'completed' | 'error'>('idle');

  useEffect(() => {
    // Tauriイベントを購読（セクション7.2-7.3参照）
    const unlisten = listenJobStream(jobId, (event) => {
      switch (event.type) {
        case 'Data':
          setStatus('streaming');
          setChunks(prev => [...prev, new Uint8Array(event.data)]);
          break;
        case 'FinalCollected':
          setChunks([new Uint8Array(event.data)]);
          setStatus('completed');
          break;
        case 'End':
          setStatus('completed');
          break;
      }
    });

    return () => { unlisten.then(fn => fn()); };
  }, [jobId]);

  return { chunks, status };
}
```

### 3.3 認証

jobworkerp-rsはヘッダーベースの認証を使用:

```
環境変数 AUTH_TOKEN:
  - 設定あり: 全リクエストに 'jobworkerp-auth' ヘッダー必須
  - 設定なし: 認証なし

ヘッダー形式:
  'jobworkerp-auth: <token-value>'
```

Tauri Rustバックエンドでの実装:

```rust
// src-tauri/src/grpc/client.rs
impl JobworkerpClient {
    pub fn new(url: &str) -> Result<Self, AppError> {
        let channel = Channel::from_shared(url.to_string())?.connect_lazy();
        let auth_token = std::env::var("JOBWORKERP_AUTH_TOKEN").ok();
        Ok(Self { channel, auth_token })
    }

    fn add_auth_header<T>(&self, mut request: tonic::Request<T>) -> tonic::Request<T> {
        if let Some(token) = &self.auth_token {
            request.metadata_mut()
                .insert("jobworkerp-auth", token.parse().unwrap());
        }
        request
    }
}
```

---

## 4. ローカルデータベーススキーマ

本サービスで管理するデータモデルをSQLiteスキーマとして定義する。

> **注記**: 静的設定モードでは、トークン管理は`mcp-settings.toml`で行うため、`platform_configs`と`token_stores`テーブルは使用しない。動的設定モードでは両テーブルが必要となる。

### 4.1 テーブル定義

#### 基本テーブル（両モード共通）

```sql
-- アプリケーション設定
CREATE TABLE app_settings (
  id INTEGER PRIMARY KEY CHECK (id = 1),  -- シングルトン
  worktree_base_path TEXT NOT NULL DEFAULT '~/.local-code-agent/worktrees',
  default_base_branch TEXT NOT NULL DEFAULT 'main',
  agent_timeout_minutes INTEGER NOT NULL DEFAULT 30,
  sync_interval_minutes INTEGER NOT NULL DEFAULT 10,
  grpc_server_url TEXT NOT NULL DEFAULT 'http://localhost:9000',  -- jobworkerp-rs接続先
  locale TEXT NOT NULL DEFAULT 'en',  -- UI言語設定
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- 初期設定を挿入
INSERT INTO app_settings (id) VALUES (1);

-- リポジトリ
CREATE TABLE repositories (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  mcp_server_name TEXT NOT NULL,  -- MCPサーバー名 (github/gitea)
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  base_url TEXT NOT NULL,  -- サービスURL (github.com または Gitea URL)
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

-- エージェントジョブ
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

-- インデックス（共通）
CREATE INDEX idx_agent_jobs_repository ON agent_jobs(repository_id);
CREATE INDEX idx_agent_jobs_status ON agent_jobs(status);
CREATE INDEX idx_agent_jobs_jobworkerp_id ON agent_jobs(jobworkerp_job_id);
CREATE INDEX idx_repositories_mcp_server ON repositories(mcp_server_name);
```

#### 動的設定モード用テーブル

動的設定モードで以下のテーブルを使用する。

```sql
-- プラットフォーム設定（GitHub/Gitea）- 動的設定モード
CREATE TABLE platform_configs (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  base_url TEXT NOT NULL,
  token_id INTEGER NOT NULL REFERENCES token_stores(id),
  mcp_runner_name TEXT,  -- RunnerService.Createで登録したランナー名
  user_name TEXT,
  created_at TEXT NOT NULL DEFAULT (datetime('now')),
  updated_at TEXT NOT NULL DEFAULT (datetime('now')),
  UNIQUE (platform, base_url)
);

-- トークン保存（暗号化）- 動的設定モード
CREATE TABLE token_stores (
  id INTEGER PRIMARY KEY AUTOINCREMENT,
  platform TEXT NOT NULL CHECK (platform IN ('GitHub', 'Gitea')),
  encrypted_token BLOB NOT NULL,
  created_at TEXT NOT NULL DEFAULT (datetime('now'))
);

-- インデックス（動的設定モード用）
CREATE INDEX idx_platform_configs_platform ON platform_configs(platform);
```

### 4.2 TypeScript型定義（参考）

> **注記**: 実際のDB操作はTauri Rustバックエンド（rusqlite）で行う。
> 以下はWebView側での型定義の参考として記載。Tauriコマンドのレスポンス型として使用する。

```typescript
// types/database.ts - WebView側の型定義（Tauriコマンドのレスポンス型として使用）

// === プラットフォーム種別 ===

export type Platform = 'GitHub' | 'Gitea';

// === AgentJobステータス ===

export type AgentJobStatus =
  | 'Pending'
  | 'PreparingWorkspace'
  | 'FetchingIssue'
  | 'RunningAgent'
  | 'CreatingPR'
  | 'PrCreated'
  | 'Merged'
  | 'Completed'
  | 'Failed'
  | 'Cancelled';

// === 基本テーブル（両モード共通） ===

/** アプリケーション設定 */
export interface AppSettings {
  id: number;
  worktreeBasePath: string;
  defaultBaseBranch: string;
  agentTimeoutMinutes: number;
  syncIntervalMinutes: number;
  grpcServerUrl: string;       // jobworkerp-rs gRPC接続先URL
  locale: string;              // UI言語設定（'en', 'ja'等）
  createdAt: string;
  updatedAt: string;
}

/** リポジトリ情報 */
export interface Repository {
  id: number;
  mcpServerName: string;
  platform: Platform;
  baseUrl: string;
  name: string;
  url: string;
  owner: string;
  repoName: string;
  localPath: string | null;
  lastSyncedAt: string | null;
  createdAt: string;
  updatedAt: string;
}

/** エージェントジョブ */
export interface AgentJob {
  id: number;
  repositoryId: number;
  issueNumber: number;
  jobworkerpJobId: string;
  status: AgentJobStatus;
  worktreePath: string | null;
  branchName: string | null;
  prNumber: number | null;
  errorMessage: string | null;
  createdAt: string;
  updatedAt: string;
}

// === 動的設定モード用テーブル ===

/** トークン保存（暗号化）- 動的設定モード */
export interface TokenStore {
  id: number;
  platform: Platform;
  // encryptedToken はWebView側には公開しない（セキュリティ上の理由）
  createdAt: string;
}

/** プラットフォーム設定 - 動的設定モード */
export interface PlatformConfig {
  id: number;
  platform: Platform;
  baseUrl: string;
  tokenId: number;
  mcpRunnerName: string | null;  // RunnerService.Createで登録したランナー名
  userName: string | null;
  createdAt: string;
  updatedAt: string;
}

// === API用リクエスト/レスポンス型 ===

/** リポジトリ追加リクエスト */
export interface AddRepositoryRequest {
  mcpServerName: string;
  platform: Platform;
  baseUrl: string;
  owner: string;
  repoName: string;
  localPath?: string;
}

/** エージェント実行リクエスト */
export interface StartAgentRequest {
  repositoryId: number;
  issueNumber: number;
  issueTitle: string;
}

/** エージェント実行レスポンス */
export interface StartAgentResponse {
  jobId: number;
  jobworkerpJobId: string;
}

/** プラットフォーム作成リクエスト（動的設定モード） */
export interface CreatePlatformRequest {
  platform: Platform;
  baseUrl: string;
  token: string;
}

/** MCPサーバー（Runner）動的作成リクエスト */
export interface CreateMcpRunnerRequest {
  platform: "GitHub" | "Gitea";
  name: string;           // MCPサーバー識別名（ユーザー指定）
  url: string;            // GitHub: "https://github.com"(デフォルト), Gitea: "https://gitea.example.com"
  token: string;          // Personal Access Token
}

/** MCPサーバー情報 */
export interface McpServerInfo {
  name: string;
  description: string | null;
  transport: string;      // "stdio" | "sse"
}
```

---

## 5. ディレクトリ構成

```
local-code-agent/
├── .github/
│   └── workflows/
│       ├── ci.yml
│       └── release.yml
├── src/                          # WebView (TypeScript/React)
│   ├── main.tsx                  # エントリポイント
│   ├── routes/                   # TanStack Router ファイルベースルーティング
│   │   ├── __root.tsx            # ルートレイアウト
│   │   ├── index.tsx             # / ダッシュボード
│   │   ├── repositories.tsx      # /repositories レイアウト + リポジトリ一覧
│   │   ├── repositories/
│   │   │   ├── $repoId.tsx       # /repositories/$repoId 詳細
│   │   │   └── $repoId/
│   │   │       ├── issues.tsx    # /repositories/$repoId/issues
│   │   │       └── pulls.tsx     # /repositories/$repoId/pulls
│   │   ├── jobs/
│   │   │   ├── index.tsx         # /jobs 一覧
│   │   │   └── $jobId.tsx        # /jobs/$jobId 詳細（ストリーミング）
│   │   └── settings.tsx          # /settings
│   ├── components/
│   │   ├── ui/                   # shadcn/ui
│   │   ├── layout/
│   │   │   ├── header.tsx
│   │   │   ├── sidebar.tsx
│   │   │   └── index.ts
│   │   ├── repositories/
│   │   │   ├── repo-card.tsx
│   │   │   └── repo-selector.tsx
│   │   ├── issues/
│   │   │   ├── issue-list.tsx
│   │   │   ├── issue-card.tsx
│   │   │   └── related-pr-warning.tsx
│   │   └── jobs/
│   │       ├── job-status-badge.tsx
│   │       ├── job-progress.tsx
│   │       ├── job-stream-viewer.tsx
│   │       └── job-cancel-button.tsx
│   ├── hooks/
│   │   ├── use-job-stream.ts     # Tauriイベントベースストリーミング
│   │   ├── use-job-status.ts     # ステータスポーリング
│   │   ├── use-repository.ts
│   │   └── use-tauri.ts          # Tauri invoke/listen ラッパー
│   ├── lib/
│   │   ├── tauri/
│   │   │   ├── commands.ts       # Tauriコマンド型定義・呼び出し
│   │   │   ├── events.ts         # Tauriイベント型定義・購読
│   │   │   └── types.ts          # Rust⇔TS共有型定義
│   │   ├── query/
│   │   │   ├── query-client.ts   # TanStack Query設定
│   │   │   ├── repositories.ts   # リポジトリクエリ
│   │   │   ├── issues.ts         # Issueクエリ
│   │   │   └── jobs.ts           # ジョブクエリ
│   │   └── utils.ts
│   ├── stores/
│   │   ├── ui-store.ts           # UIステート（Zustand）
│   │   └── preferences-store.ts
│   ├── types/
│   │   └── models.ts             # 統合型定義
│   ├── routeTree.gen.ts          # TanStack Router自動生成
│   └── config/
│       └── env.ts
├── src-tauri/                    # Tauri Rust バックエンド
│   ├── Cargo.toml                # Rust依存関係
│   ├── build.rs                  # tonic-build (Proto生成)
│   ├── tauri.conf.json           # Tauri設定
│   ├── capabilities/             # Tauri v2 パーミッション設定
│   │   └── default.json
│   ├── src/
│   │   ├── main.rs               # エントリポイント
│   │   ├── lib.rs                # Tauriアプリ初期化
│   │   ├── commands/             # Tauriコマンド定義
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs     # 接続確認
│   │   │   ├── mcp.rs            # MCP管理（list, check, create_runner）
│   │   │   ├── repositories.rs   # repository::add, sync, list
│   │   │   ├── issues.rs         # issue一覧・詳細
│   │   │   ├── pulls.rs          # PR一覧・関連PR検出
│   │   │   ├── jobs.rs           # job一覧・詳細
│   │   │   └── settings.rs       # 設定取得・更新
│   │   ├── grpc/                 # gRPCクライアント (tonic)
│   │   │   ├── mod.rs
│   │   │   ├── client.rs         # gRPC接続管理、MCP呼び出し
│   │   │   └── generated/        # tonic-build生成コード
│   │   │       ├── jobworkerp.data.rs
│   │   │       └── jobworkerp.service.rs
│   │   ├── db/                   # SQLite操作 (rusqlite)
│   │   │   ├── mod.rs
│   │   │   ├── connection.rs     # コネクションプール
│   │   │   ├── models.rs         # データモデル定義
│   │   │   ├── queries.rs        # DBクエリ関数
│   │   │   └── migrations.rs     # SQLマイグレーション
│   │   ├── crypto/               # 暗号化 (aes-gcm)
│   │   │   ├── mod.rs
│   │   │   └── token.rs          # トークン暗号化/復号
│   │   ├── error.rs              # エラー型定義
│   │   └── state.rs              # Tauri State管理
│   └── proto/                    # jobworkerp-rsからコピー
│       └── protobuf/
│           └── jobworkerp/
├── public/
├── tests/
│   ├── unit/                     # Vitestユニットテスト
│   ├── integration/              # 統合テスト
│   └── e2e/                      # Playwright E2Eテスト
├── index.html                    # Viteエントリ
├── vite.config.ts
├── tailwind.config.ts
├── tsconfig.json
├── package.json
└── README.md
```

> **注記**: 現在の実装ではプラットフォーム専用ページ（/platforms）は未実装。
> MCPサーバーの動的登録はリポジトリ登録フォーム内で直接行う設計としている。

### 5.1 Tauri固有ファイル

#### tauri.conf.json（主要設定）

```json
{
  "$schema": "https://schema.tauri.app/config/2",
  "productName": "Local Code Agent",
  "version": "0.1.0",
  "identifier": "com.local-code-agent",
  "build": {
    "frontendDist": "../dist",
    "devUrl": "http://localhost:1420",
    "beforeDevCommand": "pnpm dev",
    "beforeBuildCommand": "pnpm build"
  },
  "app": {
    "windows": [
      {
        "title": "Local Code Agent",
        "width": 1200,
        "height": 800,
        "resizable": true,
        "fullscreen": false
      }
    ],
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  },
  "bundle": {
    "active": true,
    "targets": "all",
    "icon": ["icons/icon.png"]
  }
}
```

#### Cargo.toml（Rust依存関係）

```toml
[package]
name = "local-code-agent"
version = "0.1.0"
edition = "2021"

[lib]
name = "local_code_agent_lib"
crate-type = ["staticlib", "cdylib", "rlib"]

[build-dependencies]
tauri-build = { version = "2", features = [] }
tonic-build = "0.12"

[dependencies]
tauri = { version = "2", features = ["devtools"] }
tauri-plugin-opener = "2"
serde = { version = "1", features = ["derive"] }
serde_json = "1"
tokio = { version = "1", features = ["full"] }
tonic = "0.12"
prost = "0.13"
prost-types = "0.13"
rusqlite = { version = "0.32", features = ["bundled"] }
r2d2 = "0.8"
r2d2_sqlite = "0.25"
aes-gcm = "0.10"
rand = "0.9"
keyring = "3"
hex = "0.4"
thiserror = "2"
tracing = "0.1"
tracing-subscriber = "0.3"
refinery = { version = "0.8", features = ["rusqlite"] }
anyhow = "1"
chrono = { version = "0.4", features = ["serde"] }
futures = "0.3"
async-trait = "0.1"
directories = "5"
```

---

## 6. コンポーネント設計

### 6.1 ページ構成

| パス | ルートファイル | 説明 |
|------|---------------|------|
| `/` | `routes/index.tsx` | 概要表示、最近のジョブ、クイックアクション |
| `/platforms` | `routes/platforms/index.tsx` | プラットフォーム設定一覧 |
| `/platforms/new` | `routes/platforms/new.tsx` | 新規プラットフォーム追加 |
| `/repositories` | `routes/repositories/index.tsx` | リポジトリ一覧、統計 |
| `/repositories/$id` | `routes/repositories/$id/index.tsx` | リポジトリ詳細、Issue/PR概要 |
| `/repositories/$id/issues` | `routes/repositories/$id/issues.tsx` | Issue一覧、エージェント実行 |
| `/repositories/$id/pulls` | `routes/repositories/$id/pulls.tsx` | PR一覧 |
| `/jobs` | `routes/jobs/index.tsx` | 全ジョブ一覧 |
| `/jobs/$id` | `routes/jobs/$id.tsx` | ジョブ詳細、ストリーミング表示 |
| `/settings` | `routes/settings.tsx` | アプリ設定 |

### 6.2 TanStack Router ルート定義

#### ルートレイアウト（__root.tsx）

```typescript
// routes/__root.tsx
import { createRootRouteWithContext, Outlet } from '@tanstack/react-router';
import { QueryClient } from '@tanstack/react-query';
import { Header } from '@/components/layout/header';
import { Sidebar } from '@/components/layout/sidebar';

interface RouterContext {
  queryClient: QueryClient;
}

export const Route = createRootRouteWithContext<RouterContext>()({
  component: RootLayout,
});

function RootLayout() {
  return (
    <div className="flex h-screen">
      <Sidebar />
      <div className="flex flex-1 flex-col">
        <Header />
        <main className="flex-1 overflow-auto p-6">
          <Outlet />
        </main>
      </div>
    </div>
  );
}
```

#### Issue一覧（型安全なパラメータとSearch Params）

```typescript
// routes/repositories/$id/issues.tsx
import { createFileRoute } from '@tanstack/react-router';
import { z } from 'zod';
import { issuesQueryOptions } from '@/lib/query/issues';
import { IssueList } from '@/components/issues/issue-list';

// Search Paramsのスキーマ定義
const issueSearchSchema = z.object({
  status: z.enum(['open', 'closed', 'all']).default('open'),
  label: z.string().optional(),
  assignee: z.string().optional(),
  page: z.number().default(1),
});

export const Route = createFileRoute('/repositories/$id/issues')({
  // Search Paramsのバリデーション
  validateSearch: issueSearchSchema,

  // データプリフェッチ（TanStack Query統合）
  loader: async ({ context: { queryClient }, params, search }) => {
    await queryClient.ensureQueryData(
      issuesQueryOptions(Number(params.id), search)
    );
  },

  component: IssuesPage,
});

function IssuesPage() {
  // パラメータは完全に型安全
  const { id } = Route.useParams();
  const search = Route.useSearch();

  return (
    <IssueList
      repositoryId={Number(id)}
      status={search.status}
      label={search.label}
      assignee={search.assignee}
      page={search.page}
    />
  );
}
```

#### ジョブ詳細（ストリーミング対応）

```typescript
// routes/jobs/$id.tsx
import { createFileRoute } from '@tanstack/react-router';
import { jobQueryOptions, jobStatusQueryOptions } from '@/lib/query/jobs';
import { JobStreamViewer } from '@/components/jobs/job-stream-viewer';
import { JobCancelButton } from '@/components/jobs/job-cancel-button';

export const Route = createFileRoute('/jobs/$id')({
  loader: async ({ context: { queryClient }, params }) => {
    const jobId = params.id;
    // 初期データをプリフェッチ
    await Promise.all([
      queryClient.ensureQueryData(jobQueryOptions(jobId)),
      queryClient.ensureQueryData(jobStatusQueryOptions(jobId)),
    ]);
  },

  component: JobDetailPage,
});

function JobDetailPage() {
  const { id } = Route.useParams();

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <h1 className="text-2xl font-bold">Job #{id}</h1>
        <JobCancelButton jobId={id} />
      </div>
      <JobStreamViewer jobId={id} />
    </div>
  );
}
```

#### リポジトリ登録フォーム（MCPサーバー選択/新規作成拡張）

```typescript
// routes/repositories.tsx (RepositoryFormコンポーネント部分)
import { useState, type FormEvent } from "react";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { McpServerInfo, CreateMcpRunnerRequest } from "@/types/models";

interface RepositoryFormProps {
  mcpServers: McpServerInfo[];
  onSuccess: () => void;
}

function RepositoryForm({ mcpServers, onSuccess }: RepositoryFormProps) {
  const queryClient = useQueryClient();
  // MCPサーバー選択状態: 既存サーバー名 or "new"（新規作成）
  const [mcpSelection, setMcpSelection] = useState<string>("");
  // 新規MCPサーバー作成用フォームデータ
  const [newMcpData, setNewMcpData] = useState<CreateMcpRunnerRequest>({
    platform: "GitHub",
    name: "",
    url: "https://github.com",
    token: "",
  });

  // MCPサーバー動的作成Mutation
  const createMcpMutation = useMutation({
    mutationFn: (request: CreateMcpRunnerRequest) =>
      invoke<McpServerInfo>("mcp_create_runner", {
        platform: request.platform,
        name: request.name,
        url: request.url,
        token: request.token,
      }),
    onSuccess: (newServer) => {
      // MCPサーバー一覧を再取得
      queryClient.invalidateQueries({ queryKey: ["mcp-servers"] });
      // 新規作成したサーバーを選択状態にする
      setMcpSelection(newServer.name);
    },
  });

  // プラットフォーム変更時のデフォルトURL設定
  const handlePlatformChange = (platform: "GitHub" | "Gitea") => {
    setNewMcpData({
      ...newMcpData,
      platform,
      url: platform === "GitHub" ? "https://github.com" : "",
    });
  };

  return (
    <form className="border border-slate-200 dark:border-slate-700 rounded-lg p-6 mb-6">
      {/* MCPサーバー選択 */}
      <div className="mb-4">
        <label className="block text-sm font-medium mb-1">MCPサーバー</label>
        <select
          value={mcpSelection}
          onChange={(e) => setMcpSelection(e.target.value)}
          className="w-full p-2 border rounded"
          required
        >
          <option value="">選択してください</option>
          {mcpServers.map((server) => (
            <option key={server.name} value={server.name}>
              {server.name}
              {server.description ? ` - ${server.description}` : ""}
            </option>
          ))}
          <option value="new">+ 新規MCPサーバー作成</option>
        </select>
      </div>

      {/* 新規MCPサーバー作成フォーム（"new"選択時のみ表示） */}
      {mcpSelection === "new" && (
        <div className="border border-blue-200 dark:border-blue-800 rounded-lg p-4 mb-4 bg-blue-50 dark:bg-blue-900/30">
          <h3 className="text-lg font-semibold mb-3">新規MCPサーバー作成</h3>

          {/* プラットフォーム選択 */}
          <div className="mb-3">
            <label className="block text-sm font-medium mb-1">プラットフォーム</label>
            <select
              value={newMcpData.platform}
              onChange={(e) => handlePlatformChange(e.target.value as "GitHub" | "Gitea")}
              className="w-full p-2 border rounded"
            >
              <option value="GitHub">GitHub</option>
              <option value="Gitea">Gitea</option>
            </select>
          </div>

          {/* サーバー識別名 */}
          <div className="mb-3">
            <label className="block text-sm font-medium mb-1">サーバー識別名</label>
            <input
              type="text"
              value={newMcpData.name}
              onChange={(e) => setNewMcpData({ ...newMcpData, name: e.target.value })}
              placeholder="my-github-server"
              className="w-full p-2 border rounded"
              required
            />
          </div>

          {/* URL */}
          <div className="mb-3">
            <label className="block text-sm font-medium mb-1">
              URL
              {newMcpData.platform === "GitHub" && (
                <span className="text-gray-500 ml-2">(GitHub Enterpriseの場合は変更)</span>
              )}
            </label>
            <input
              type="url"
              value={newMcpData.url}
              onChange={(e) => setNewMcpData({ ...newMcpData, url: e.target.value })}
              placeholder={newMcpData.platform === "GitHub" ? "https://github.com" : "https://gitea.example.com"}
              className="w-full p-2 border rounded"
              required
            />
          </div>

          {/* Personal Access Token */}
          <div className="mb-3">
            <label className="block text-sm font-medium mb-1">Personal Access Token</label>
            <input
              type="password"
              value={newMcpData.token}
              onChange={(e) => setNewMcpData({ ...newMcpData, token: e.target.value })}
              placeholder="ghp_xxxx... / gitea_token_xxxx..."
              className="w-full p-2 border rounded"
              required
            />
          </div>

          <button
            type="button"
            onClick={() => createMcpMutation.mutate(newMcpData)}
            disabled={createMcpMutation.isPending || !newMcpData.name || !newMcpData.token}
            className="px-4 py-2 bg-blue-600 text-white rounded hover:bg-blue-700 disabled:opacity-50"
          >
            {createMcpMutation.isPending ? "作成中..." : "MCPサーバー作成"}
          </button>

          {createMcpMutation.isError && (
            <p className="text-red-600 mt-2">エラー: {String(createMcpMutation.error)}</p>
          )}
        </div>
      )}

      {/* 以下、既存のリポジトリ登録フィールド（owner, repo_name, local_path等） */}
      {/* ... */}
    </form>
  );
}
```

### 6.3 TanStack Query クエリ定義

```typescript
// lib/query/issues.ts
import { queryOptions } from '@tanstack/react-query';
import { fetchIssues } from '@/lib/grpc/client';

export const issuesQueryOptions = (
  repositoryId: number,
  filters: { status?: string; label?: string; assignee?: string; page?: number }
) =>
  queryOptions({
    queryKey: ['issues', repositoryId, filters],
    queryFn: () => fetchIssues(repositoryId, filters),
    staleTime: 5 * 60 * 1000, // 5分
  });

// lib/query/jobs.ts
import { queryOptions } from '@tanstack/react-query';
import { fetchJob, fetchJobStatus } from '@/lib/grpc/client';

export const jobQueryOptions = (jobId: string) =>
  queryOptions({
    queryKey: ['job', jobId],
    queryFn: () => fetchJob(jobId),
  });

export const jobStatusQueryOptions = (jobId: string) =>
  queryOptions({
    queryKey: ['jobStatus', jobId],
    queryFn: () => fetchJobStatus(jobId),
    refetchInterval: (query) => {
      const data = query.state.data;
      // 完了/失敗ならポーリング停止
      if (data?.status === 'Completed' || data?.status === 'Failed') {
        return false;
      }
      return 2000;
    },
  });
```

### 6.4 主要コンポーネント

#### JobStreamViewer（ストリーミング表示）

```typescript
// components/jobs/job-stream-viewer.tsx

import { useJobStream } from '@/hooks/use-job-stream';
import { ScrollArea } from '@/components/ui/scroll-area';
import { Badge } from '@/components/ui/badge';

interface JobStreamViewerProps {
  jobId: string;
}

export function JobStreamViewer({ jobId }: JobStreamViewerProps) {
  const { chunks, status, error } = useJobStream(jobId);

  // UTF-8デコード
  const text = useMemo(() => {
    const decoder = new TextDecoder();
    return chunks.map(chunk => decoder.decode(chunk)).join('');
  }, [chunks]);

  return (
    <div className="rounded-lg border">
      <div className="flex items-center justify-between border-b p-3">
        <h3 className="font-semibold">Agent Output</h3>
        <Badge variant={
          status === 'streaming' ? 'default' :
          status === 'completed' ? 'success' :
          status === 'error' ? 'destructive' : 'secondary'
        }>
          {status}
        </Badge>
      </div>
      <ScrollArea className="h-[400px] p-4">
        <pre className="whitespace-pre-wrap font-mono text-sm">
          {text}
          {status === 'streaming' && <span className="animate-pulse">▋</span>}
        </pre>
      </ScrollArea>
      {error && (
        <div className="border-t bg-destructive/10 p-3 text-destructive">
          {error.message}
        </div>
      )}
    </div>
  );
}
```

#### IssueList（Issue一覧）

```typescript
// components/issues/issue-list.tsx
import { useState } from 'react';
import { useIssues, useRelatedPRs } from '@/hooks/use-issues';
import { useStartAgent } from '@/hooks/use-agent';
import { IssueCard } from './issue-card';
import { RelatedPRWarning } from './related-pr-warning';
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog';

interface IssueListProps {
  repositoryId: number;
  platformConfig: PlatformConfig;
}

export function IssueList({ repositoryId, platformConfig }: IssueListProps) {
  const { data: issues, isLoading } = useIssues(repositoryId);
  const [selectedIssue, setSelectedIssue] = useState<Issue | null>(null);
  const { data: relatedPRs } = useRelatedPRs(repositoryId, selectedIssue?.number);
  const startAgent = useStartAgent();

  const handleRunAgent = async (issue: Issue) => {
    setSelectedIssue(issue);
  };

  const confirmRunAgent = async () => {
    if (!selectedIssue) return;

    await startAgent.mutateAsync({
      repositoryId,
      issueNumber: selectedIssue.number,
      issueTitle: selectedIssue.title,
    });

    setSelectedIssue(null);
  };

  return (
    <>
      <div className="space-y-4">
        {issues?.map(issue => (
          <IssueCard
            key={issue.number}
            issue={issue}
            onRunAgent={() => handleRunAgent(issue)}
          />
        ))}
      </div>

      <AlertDialog open={!!selectedIssue} onOpenChange={() => setSelectedIssue(null)}>
        <AlertDialogContent>
          <AlertDialogHeader>
            <AlertDialogTitle>
              Run Agent for Issue #{selectedIssue?.number}?
            </AlertDialogTitle>
            <AlertDialogDescription>
              {selectedIssue?.title}
            </AlertDialogDescription>
          </AlertDialogHeader>

          {relatedPRs && relatedPRs.length > 0 && (
            <RelatedPRWarning prs={relatedPRs} />
          )}

          <AlertDialogFooter>
            <AlertDialogCancel>Cancel</AlertDialogCancel>
            <AlertDialogAction onClick={confirmRunAgent}>
              Start Agent
            </AlertDialogAction>
          </AlertDialogFooter>
        </AlertDialogContent>
      </AlertDialog>
    </>
  );
}
```

---

## 7. API設計（Tauri Commands）

TauriアーキテクチャではWebViewからRustバックエンドをTauri Commandsで呼び出す。
gRPC通信とSQLite操作はすべてRustバックエンドで行う。

### 7.1 Tauriコマンド定義（Rust側）

#### MCPサーバー管理（両モード共通）

```rust
// src-tauri/src/commands/mcp.rs
use crate::grpc::client::JobworkerpClient;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct McpServerInfo {
    pub name: String,
    pub description: Option<String>,
    pub transport: String,  // "stdio" | "sse"
}

/// 設定済みMCPサーバー一覧を取得（jobworkerp-rsから）
#[tauri::command]
pub async fn mcp_list_servers(
    grpc: State<'_, JobworkerpClient>,
) -> Result<Vec<McpServerInfo>, AppError> {
    // RunnerService経由でMCP_SERVERタイプのランナー一覧を取得
    grpc.list_mcp_servers().await
}

/// MCPサーバーの接続確認
#[tauri::command]
pub async fn mcp_check_connection(
    server_name: String,
    grpc: State<'_, JobworkerpClient>,
) -> Result<bool, AppError> {
    grpc.check_mcp_server(&server_name).await
}

/// MCPサーバー経由でアクセス可能なリポジトリ一覧を取得
#[tauri::command]
pub async fn mcp_list_repositories(
    server_name: String,
    grpc: State<'_, JobworkerpClient>,
) -> Result<Vec<RepositoryInfo>, AppError> {
    // MCPサーバーのget_my_user_info + list_user_reposツールを呼び出し
    grpc.list_repositories_via_mcp(&server_name).await
}

/// GitHub/Gitea MCPサーバー（Runner）を動的登録
/// TOML定義は内部でplatformに応じて自動生成（URLからscheme/hostを抽出）
///
/// Docker実行形式:
/// - GitHub: `docker run ghcr.io/github/github-mcp-server` + GITHUB_PERSONAL_ACCESS_TOKEN, GITHUB_HOST（Enterprise時のみ）
/// - Gitea: `docker run docker.gitea.com/gitea-mcp-server` + GITEA_ACCESS_TOKEN, GITEA_HOST, GITEA_INSECURE（http時のみ）
#[tauri::command]
pub async fn mcp_create_runner(
    grpc: State<'_, Arc<JobworkerpClient>>,
    platform: String,     // "GitHub" or "Gitea"
    name: String,         // MCPサーバー識別名
    url: String,          // URL (https://github.com, https://gitea.example.com)
    token: String,        // Personal Access Token
) -> Result<McpServerInfo, AppError> {
    // platform に応じてTOML定義を内部生成（URLからscheme/hostを抽出）
    let definition = match platform.as_str() {
        "GitHub" => github_mcp_toml(&url, &token)?,
        "Gitea" => gitea_mcp_toml(&url, &token)?,
        _ => return Err(AppError::InvalidInput(format!("Unsupported platform: {}", platform))),
    };

    // gRPC経由でRunner登録
    grpc.create_runner(&name, &definition).await?;

    Ok(McpServerInfo {
        name,
        description: Some(format!("{} MCP Server", platform)),
        transport: "stdio".to_string(),
    })
}

// --- 以下、JobworkerpClient内の実装詳細 ---

impl JobworkerpClient {
    /// MCPサーバー経由でリポジトリ一覧を取得
    ///
    /// 処理フロー:
    /// 1. MCPサーバーに対応するワーカーを検索
    /// 2. ジョブを投入してMCPツール（list_user_repos等）を呼び出し
    /// 3. 結果をパースしてRepositoryInfo一覧を返却
    pub async fn list_repositories_via_mcp(&self, server_name: &str) -> Result<Vec<RepositoryInfo>, AppError> {
        // 1. ワーカー検索（MCPサーバー名でワーカーを特定）
        let worker = self.find_worker_by_name(server_name).await?
            .ok_or(AppError::NotFound(format!("MCP server worker '{}' not found", server_name)))?;

        // 2. MCPツール呼び出し用の引数を構築
        // GitHub MCP: list_user_repos, Gitea MCP: list_repos
        let tool_name = match server_name {
            s if s.contains("github") => "list_user_repos",
            s if s.contains("gitea") => "list_repos",
            _ => "list_user_repos", // デフォルト
        };

        // 3. ジョブを投入してMCPツールを実行
        let args = serde_json::json!({
            "affiliation": "owner,collaborator"  // GitHub用パラメータ
        });

        let request = proto::JobRequest {
            worker: Some(proto::job_request::Worker::WorkerName(server_name.to_string())),
            args: serde_json::to_vec(&args)?,
            using: Some(tool_name.to_string()),  // MCPツール名を指定
            ..Default::default()
        };

        // 4. 同期実行（結果を待機）
        let response = self.enqueue_and_wait(request).await?;

        // 5. 結果をパース
        let repos: Vec<RepositoryInfo> = serde_json::from_slice(&response.output)?;
        Ok(repos)
    }
}
```

#### プラットフォーム管理（動的設定モード）

> **注記**: 静的設定モードでは、以下のプラットフォーム管理コマンドは使用しない。
> `mcp-settings.toml`でMCPサーバーが事前設定されている場合、トークン管理は不要となる。

```rust
// src-tauri/src/commands/platform.rs
use crate::db::repositories::platform::PlatformRepository;
use crate::crypto::token::TokenCrypto;
use crate::grpc::client::JobworkerpClient;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use tauri::State;

#[derive(Debug, Serialize, Deserialize)]
pub struct PlatformConfig {
    pub id: i64,
    pub platform: String,
    pub base_url: String,
    pub mcp_runner_name: Option<String>,  // 動的登録したMCPサーバー名
    pub user_name: Option<String>,
    pub created_at: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePlatformRequest {
    pub platform: String,  // "GitHub" | "Gitea"
    pub base_url: String,
    pub token: String,
}

/// 動的設定モード: プラットフォーム作成（MCPサーバーも動的登録）
#[tauri::command]
pub async fn platform_create(
    request: CreatePlatformRequest,
    db: State<'_, crate::state::DbPool>,
    crypto: State<'_, TokenCrypto>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<PlatformConfig, AppError> {
    // 1. Validate token with GitHub/Gitea API
    let user_info = validate_token(&request.platform, &request.base_url, &request.token).await?;

    // 2. Encrypt token for local storage
    let encrypted_token = crypto.encrypt(&request.token)?;

    // 3. Register MCP server dynamically via RunnerService.Create
    let mcp_runner_name = format!("{}-{}", request.platform.to_lowercase(), user_info.login);
    grpc.create_mcp_runner(&mcp_runner_name, &request.platform, &request.base_url, &request.token).await?;

    // 4. Store in SQLite
    let conn = db.get()?;
    let repo = PlatformRepository::new(&conn);
    let config = repo.create(
        &request.platform,
        &request.base_url,
        &encrypted_token,
        &mcp_runner_name,
        &user_info.login
    )?;

    Ok(config)
}

#[tauri::command]
pub async fn platform_list(
    db: State<'_, crate::state::DbPool>,
) -> Result<Vec<PlatformConfig>, AppError> {
    let conn = db.get()?;
    let repo = PlatformRepository::new(&conn);
    repo.list()
}

#[tauri::command]
pub async fn platform_delete(
    id: i64,
    db: State<'_, crate::state::DbPool>,
) -> Result<(), AppError> {
    let conn = db.get()?;
    let repo = PlatformRepository::new(&conn);
    repo.delete(id)
}
```

#### エージェント実行

```rust
// src-tauri/src/commands/agent.rs
use crate::db::repositories::agent_job::AgentJobRepository;
use crate::grpc::client::JobworkerpClient;
use crate::error::AppError;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Emitter, State};

#[derive(Debug, Deserialize)]
pub struct StartAgentRequest {
    pub repository_id: i64,
    pub issue_number: i64,
    pub issue_title: String,
}

#[derive(Debug, Serialize)]
pub struct StartAgentResponse {
    pub job_id: i64,
    pub jobworkerp_job_id: String,
}

#[tauri::command]
pub async fn agent_start(
    request: StartAgentRequest,
    app: AppHandle,
    db: State<'_, crate::state::DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<StartAgentResponse, AppError> {
    let conn = db.get()?;

    // Get repository info
    let repo_info = crate::db::repositories::repository::RepositoryRepository::new(&conn)
        .find_by_id(request.repository_id)?
        .ok_or(AppError::NotFound("Repository not found".into()))?;

    // Prepare workflow arguments
    let workflow_args = serde_json::json!({
        "owner": repo_info.owner,
        "repo": repo_info.repo_name,
        "issue_number": request.issue_number,
        "issue_title": request.issue_title,
        "base_branch": "main",
        "local_repo_path": repo_info.local_path,
    });

    // Call jobworkerp-rs via gRPC
    let jobworkerp_job_id = grpc.enqueue_job("code-agent-workflow", &workflow_args).await?;

    // Store in local DB
    let agent_repo = AgentJobRepository::new(&conn);
    let job = agent_repo.create(
        request.repository_id,
        request.issue_number,
        &jobworkerp_job_id,
        "Pending",
    )?;

    // Start streaming in background task
    let app_clone = app.clone();
    let grpc_clone = grpc.inner().clone();
    let job_id_clone = jobworkerp_job_id.clone();
    tokio::spawn(async move {
        if let Err(e) = stream_job_results(app_clone, grpc_clone, job_id_clone).await {
            tracing::error!("Stream error: {:?}", e);
        }
    });

    Ok(StartAgentResponse {
        job_id: job.id,
        jobworkerp_job_id,
    })
}

/// gRPCストリームをTauriイベントに変換
async fn stream_job_results(
    app: AppHandle,
    grpc: JobworkerpClient,
    job_id: String,
) -> Result<(), AppError> {
    let mut stream = grpc.listen_stream(&job_id).await?;

    while let Some(item) = stream.message().await? {
        match item.item {
            Some(proto::result_output_item::Item::Data(data)) => {
                // Emit streaming data to WebView
                app.emit(&format!("job-stream-{}", job_id), StreamEvent::Data {
                    data: data.to_vec(),
                })?;
            }
            Some(proto::result_output_item::Item::End(_)) => {
                app.emit(&format!("job-stream-{}", job_id), StreamEvent::End)?;
                break;
            }
            Some(proto::result_output_item::Item::FinalCollected(data)) => {
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

#[tauri::command]
pub async fn agent_cancel(
    jobworkerp_job_id: String,
    db: State<'_, crate::state::DbPool>,
    grpc: State<'_, JobworkerpClient>,
) -> Result<(), AppError> {
    // Cancel via gRPC
    grpc.delete_job(&jobworkerp_job_id).await?;

    // Update local DB
    let conn = db.get()?;
    AgentJobRepository::new(&conn).update_status(&jobworkerp_job_id, "Cancelled")?;

    Ok(())
}

#[derive(Debug, Clone, Serialize)]
#[serde(tag = "type")]
pub enum StreamEvent {
    Data { data: Vec<u8> },
    End,
    FinalCollected { data: Vec<u8> },
}
```

### 7.2 TypeScript側コマンド呼び出し

#### Tauriコマンドラッパー

```typescript
// src/lib/tauri/commands.ts
import { invoke } from '@tauri-apps/api/core';

// Types matching Rust structs
export interface PlatformConfig {
  id: number;
  platform: 'GitHub' | 'Gitea';
  base_url: string;
  user_name?: string;
  created_at: string;
}

export interface CreatePlatformRequest {
  platform: 'GitHub' | 'Gitea';
  base_url: string;
  token: string;
}

export interface StartAgentRequest {
  repository_id: number;
  issue_number: number;
  issue_title: string;
}

export interface StartAgentResponse {
  job_id: number;
  jobworkerp_job_id: string;
}

// Platform commands
export const platformCommands = {
  create: (request: CreatePlatformRequest) =>
    invoke<PlatformConfig>('platform_create', { request }),

  list: () =>
    invoke<PlatformConfig[]>('platform_list'),

  delete: (id: number) =>
    invoke<void>('platform_delete', { id }),
};

// Agent commands
export const agentCommands = {
  start: (request: StartAgentRequest) =>
    invoke<StartAgentResponse>('agent_start', { request }),

  cancel: (jobworkerpJobId: string) =>
    invoke<void>('agent_cancel', { jobworkerpJobId }),
};
```

#### Tauriイベント購読

```typescript
// src/lib/tauri/events.ts
import { listen, UnlistenFn } from '@tauri-apps/api/event';

export interface StreamDataEvent {
  type: 'Data';
  data: number[];  // Rust Vec<u8> is received as number[]
}

export interface StreamEndEvent {
  type: 'End';
}

export interface StreamFinalCollectedEvent {
  type: 'FinalCollected';
  data: number[];
}

export type StreamEvent = StreamDataEvent | StreamEndEvent | StreamFinalCollectedEvent;

export function listenJobStream(
  jobId: string,
  callback: (event: StreamEvent) => void
): Promise<UnlistenFn> {
  return listen<StreamEvent>(`job-stream-${jobId}`, (event) => {
    callback(event.payload);
  });
}
```

### 7.3 React Hooks（Tauriイベントベース）

```typescript
// src/hooks/use-job-stream.ts
import { useState, useEffect, useCallback } from 'react';
import { listenJobStream, StreamEvent } from '@/lib/tauri/events';

type StreamStatus = 'idle' | 'connecting' | 'streaming' | 'completed' | 'error';

// Default: maxChunks=1000 to prevent memory exhaustion on long-running agents
const DEFAULT_MAX_CHUNKS = 1000;

export function useJobStream(jobId: string, maxChunks = DEFAULT_MAX_CHUNKS) {
  const [chunks, setChunks] = useState<Uint8Array[]>([]);
  const [status, setStatus] = useState<StreamStatus>('idle');
  const [error, setError] = useState<Error | null>(null);

  useEffect(() => {
    if (!jobId) return;

    setStatus('connecting');
    setChunks([]);
    setError(null);

    let unlisten: (() => void) | undefined;

    listenJobStream(jobId, (event) => {
      switch (event.type) {
        case 'Data':
          setStatus('streaming');
          // Memory protection: keep only the most recent maxChunks entries
          setChunks(prev => {
            const newChunks = [...prev, new Uint8Array(event.data)];
            return newChunks.slice(-maxChunks);
          });
          break;
        case 'FinalCollected':
          setChunks([new Uint8Array(event.data)]);
          setStatus('completed');
          break;
        case 'End':
          setStatus('completed');
          break;
      }
    })
      .then(fn => { unlisten = fn; })
      .catch(err => {
        setError(err instanceof Error ? err : new Error('Failed to listen'));
        setStatus('error');
      });

    return () => {
      unlisten?.();
    };
  }, [jobId, maxChunks]);

  return { chunks, status, error };
}
```

**メモリ管理に関する注意**:

大量のストリーミングデータを受信する場合、チャンクの蓄積によるメモリ問題が発生する可能性があります。
長時間実行されるエージェントタスクでは、以下の対策を検討してください:

```typescript
// メモリ制限付きバージョン
export function useJobStreamWithLimit(jobId: string, maxChunks = 1000) {
  const [chunks, setChunks] = useState<Uint8Array[]>([]);
  // ... 省略 ...

  // Data受信時: 古いチャンクを削除してメモリを制限
  case 'Data':
    setChunks(prev => {
      const newChunks = [...prev, new Uint8Array(event.data)];
      // 最新のmaxChunks件のみ保持
      return newChunks.slice(-maxChunks);
    });
    break;
}

// または、テキスト蓄積版（より効率的）
export function useJobStreamText(jobId: string) {
  const [text, setText] = useState<string>('');
  const decoder = useMemo(() => new TextDecoder(), []);
  // ... 省略 ...

  case 'Data':
    setText(prev => prev + decoder.decode(new Uint8Array(event.data)));
    break;
}
```

### 7.4 TanStack Queryとの統合

```typescript
// src/lib/query/platforms.ts
import { queryOptions, useMutation, useQueryClient } from '@tanstack/react-query';
import { platformCommands, CreatePlatformRequest } from '@/lib/tauri/commands';

export const platformsQueryOptions = queryOptions({
  queryKey: ['platforms'],
  queryFn: () => platformCommands.list(),
  staleTime: 5 * 60 * 1000,
});

export function useCreatePlatform() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: CreatePlatformRequest) => platformCommands.create(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['platforms'] });
    },
  });
}

export function useDeletePlatform() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (id: number) => platformCommands.delete(id),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['platforms'] });
    },
  });
}

// src/lib/query/agent.ts
import { useMutation, useQueryClient } from '@tanstack/react-query';
import { agentCommands, StartAgentRequest } from '@/lib/tauri/commands';

export function useStartAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (request: StartAgentRequest) => agentCommands.start(request),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agentJobs'] });
    },
  });
}

export function useCancelAgent() {
  const queryClient = useQueryClient();

  return useMutation({
    mutationFn: (jobworkerpJobId: string) => agentCommands.cancel(jobworkerpJobId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ['agentJobs'] });
    },
  });
}
```

---

## 8. セキュリティ要件

Tauriアーキテクチャにより、セキュリティ上の重要な操作はすべてRustバックエンドで行う。

### 8.1 トークン管理（Rust実装）

```rust
// src-tauri/src/crypto/token.rs
use aes_gcm::{
    aead::{Aead, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use rand::RngCore;
use thiserror::Error;

const NONCE_SIZE: usize = 12;
const KEY_SIZE: usize = 32;

#[derive(Error, Debug)]
pub enum CryptoError {
    #[error("Encryption failed")]
    EncryptionFailed,
    #[error("Decryption failed")]
    DecryptionFailed,
    #[error("Invalid data format")]
    InvalidFormat,
}

pub struct TokenCrypto {
    cipher: Aes256Gcm,
}

impl TokenCrypto {
    /// Create from environment variable or generate new key
    pub fn new() -> Result<Self, CryptoError> {
        let key = Self::get_or_generate_key()?;
        let cipher = Aes256Gcm::new_from_slice(&key)
            .map_err(|_| CryptoError::EncryptionFailed)?;
        Ok(Self { cipher })
    }

    /// Get key from keychain or generate and store new one
    /// Falls back to file-based storage if keychain is unavailable
    fn get_or_generate_key() -> Result<[u8; KEY_SIZE], CryptoError> {
        // Try keychain first
        match keyring::Entry::new("local-code-agent", "encryption-key") {
            Ok(entry) => {
                match entry.get_password() {
                    Ok(key_hex) => {
                        // Decode existing key
                        let key = hex::decode(&key_hex)
                            .map_err(|_| CryptoError::InvalidFormat)?;
                        let mut arr = [0u8; KEY_SIZE];
                        arr.copy_from_slice(&key);
                        return Ok(arr);
                    }
                    Err(_) => {
                        // Generate and store new key
                        let mut key = [0u8; KEY_SIZE];
                        OsRng.fill_bytes(&mut key);
                        let key_hex = hex::encode(&key);
                        if entry.set_password(&key_hex).is_ok() {
                            return Ok(key);
                        }
                        // Fall through to file-based storage
                    }
                }
            }
            Err(_) => {
                // Keychain unavailable, fall through to file-based storage
            }
        }

        // Fallback: file-based key storage (less secure)
        tracing::warn!(
            "Keychain unavailable, falling back to file-based key storage. \
             This is less secure than keychain storage."
        );
        Self::get_or_generate_key_from_file()
    }

    /// Fallback: store encryption key in application data directory
    /// Note: This is less secure than keychain storage
    fn get_or_generate_key_from_file() -> Result<[u8; KEY_SIZE], CryptoError> {
        let key_path = dirs::data_local_dir()
            .ok_or(CryptoError::EncryptionFailed)?
            .join("local-code-agent")
            .join(".encryption_key");

        if key_path.exists() {
            let key_hex = std::fs::read_to_string(&key_path)
                .map_err(|_| CryptoError::EncryptionFailed)?;
            let key = hex::decode(key_hex.trim())
                .map_err(|_| CryptoError::InvalidFormat)?;
            let mut arr = [0u8; KEY_SIZE];
            arr.copy_from_slice(&key);
            Ok(arr)
        } else {
            // Generate and store new key
            if let Some(parent) = key_path.parent() {
                std::fs::create_dir_all(parent)
                    .map_err(|_| CryptoError::EncryptionFailed)?;
            }
            let mut key = [0u8; KEY_SIZE];
            OsRng.fill_bytes(&mut key);
            let key_hex = hex::encode(&key);
            std::fs::write(&key_path, &key_hex)
                .map_err(|_| CryptoError::EncryptionFailed)?;

            // Set restrictive permissions (Unix only)
            #[cfg(unix)]
            {
                use std::os::unix::fs::PermissionsExt;
                std::fs::set_permissions(&key_path, std::fs::Permissions::from_mode(0o600))
                    .map_err(|_| CryptoError::EncryptionFailed)?;
            }

            Ok(key)
        }
    }

    pub fn encrypt(&self, plaintext: &str) -> Result<Vec<u8>, CryptoError> {
        let mut nonce_bytes = [0u8; NONCE_SIZE];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = self.cipher
            .encrypt(nonce, plaintext.as_bytes())
            .map_err(|_| CryptoError::EncryptionFailed)?;

        // Prepend nonce to ciphertext
        let mut result = nonce_bytes.to_vec();
        result.extend(ciphertext);
        Ok(result)
    }

    pub fn decrypt(&self, encrypted: &[u8]) -> Result<String, CryptoError> {
        if encrypted.len() < NONCE_SIZE {
            return Err(CryptoError::InvalidFormat);
        }

        let (nonce_bytes, ciphertext) = encrypted.split_at(NONCE_SIZE);
        let nonce = Nonce::from_slice(nonce_bytes);

        let plaintext = self.cipher
            .decrypt(nonce, ciphertext)
            .map_err(|_| CryptoError::DecryptionFailed)?;

        String::from_utf8(plaintext)
            .map_err(|_| CryptoError::DecryptionFailed)
    }
}
```

### 8.2 Tauri Capabilities（パーミッション設定）

Tauri v2ではケイパビリティベースのパーミッションシステムを使用:

```json
// src-tauri/capabilities/default.json
{
  "$schema": "https://schema.tauri.app/config/2/capabilities",
  "identifier": "default",
  "description": "Default capabilities for Local Code Agent",
  "windows": ["main"],
  "permissions": [
    "core:default",
    "shell:allow-open",
    {
      "identifier": "fs:allow-read",
      "allow": [
        { "path": "$APPDATA/**" },
        { "path": "$HOME/.local-code-agent/**" }
      ]
    },
    {
      "identifier": "fs:allow-write",
      "allow": [
        { "path": "$APPDATA/**" },
        { "path": "$HOME/.local-code-agent/**" }
      ]
    }
  ]
}
```

### 8.3 Content Security Policy

```json
// src-tauri/tauri.conf.json (抜粋)
{
  "app": {
    "security": {
      "csp": "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'"
    }
  }
}
```

> **Note**: `style-src 'unsafe-inline'` はTauriフレームワークの制約上必要です。
> Tauriは内部的にインラインスタイルを動的に注入するため、この設定がないとアプリケーションが正常に動作しません。
> セキュリティリスクは、XSSによるスタイル注入攻撃に限定されますが、`script-src` は厳格に制限しているため、
> スクリプト実行による攻撃は防止されています。

### 8.4 セキュリティ上の利点

| 項目 | SPA (gRPC-Web) | Tauri |
|------|----------------|-------|
| トークン保存 | ブラウザStorage（漏洩リスク） | OS Keychain（暗号化） |
| 暗号化キー | JS環境（抽出可能） | Rust + Keychain（保護） |
| SQLite | sql.js (WASM, メモリ上) | rusqlite（ファイルシステム、安全） |
| gRPC通信 | gRPC-Web (プロキシ必要) | tonic (ネイティブHTTP/2) |
| ファイルアクセス | 制限あり（ブラウザサンドボックス） | ケイパビリティで制御 |

### 8.5 セキュリティベストプラクティス

1. **トークンは絶対にWebViewに渡さない**: gRPC呼び出しはRustバックエンドで行う
2. **機密データはTauri Stateで管理**: WebViewからはコマンド経由でのみアクセス
3. **ケイパビリティの最小化**: 必要なパーミッションのみ許可
4. **CSPの厳格化**: 外部スクリプト・接続を制限
5. **エラーメッセージの情報漏洩防止**: 内部エラーはログのみ、ユーザーには一般化したメッセージ

### 8.6 エラー型定義

```rust
// src-tauri/src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("Not found: {0}")]
    NotFound(String),

    #[error("gRPC error: {0}")]
    GrpcError(String),

    #[error("Database error: {0}")]
    DbError(String),

    #[error("Invalid input: {0}")]
    InvalidInput(String),

    #[error("Authentication error: {0}")]
    AuthError(String),

    #[error("Crypto error: {0}")]
    CryptoError(#[from] crate::crypto::token::CryptoError),

    #[error("Internal error")]
    InternalError,
}

// Tauri IPC用のシリアライズ実装
impl serde::Serialize for AppError {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        // ユーザーに見せるメッセージは一般化（情報漏洩防止）
        let user_message = match self {
            AppError::NotFound(_) => "リソースが見つかりません",
            AppError::GrpcError(_) => "バックエンドとの通信に失敗しました",
            AppError::DbError(_) => "データベースエラーが発生しました",
            AppError::InvalidInput(msg) => msg.as_str(),
            AppError::AuthError(_) => "認証に失敗しました",
            AppError::CryptoError(_) => "暗号化処理に失敗しました",
            AppError::InternalError => "内部エラーが発生しました",
        };
        serializer.serialize_str(user_message)
    }
}

// gRPC Status からの変換
impl From<tonic::Status> for AppError {
    fn from(status: tonic::Status) -> Self {
        tracing::error!("gRPC error: {:?}", status);
        AppError::GrpcError(status.message().to_string())
    }
}

// rusqlite からの変換
impl From<rusqlite::Error> for AppError {
    fn from(err: rusqlite::Error) -> Self {
        tracing::error!("Database error: {:?}", err);
        AppError::DbError(err.to_string())
    }
}

// r2d2 からの変換
impl From<r2d2::Error> for AppError {
    fn from(err: r2d2::Error) -> Self {
        tracing::error!("Connection pool error: {:?}", err);
        AppError::DbError(err.to_string())
    }
}

// serde_json からの変換
impl From<serde_json::Error> for AppError {
    fn from(err: serde_json::Error) -> Self {
        tracing::error!("JSON error: {:?}", err);
        AppError::InvalidInput(format!("Invalid JSON: {}", err))
    }
}
```

---

## 9. パフォーマンス要件

### 9.1 キャッシュ戦略

```typescript
// hooks/use-issues.ts
export function useIssues(repositoryId: number) {
  return useQuery({
    queryKey: ['issues', repositoryId],
    queryFn: () => fetchIssues(repositoryId),
    staleTime: 5 * 60 * 1000,     // 5分間は再フェッチしない
    gcTime: 30 * 60 * 1000,       // 30分間キャッシュ保持
    refetchOnWindowFocus: false,
  });
}

// hooks/use-job-status.ts
export function useJobStatus(jobId: string) {
  return useQuery({
    queryKey: ['jobStatus', jobId],
    queryFn: () => fetchJobStatus(jobId),
    refetchInterval: (data) => {
      // 完了/失敗状態ならポーリング停止
      if (data?.status === 'Completed' || data?.status === 'Failed') {
        return false;
      }
      return 2000; // 2秒ごとにポーリング
    },
  });
}
```

### 9.2 コード分割（Vite）

```typescript
// routes/jobs/$id.tsx
import { lazy, Suspense } from 'react';
import { Skeleton } from '@/components/ui/skeleton';

// Viteの動的インポートによるコード分割
const JobStreamViewer = lazy(() =>
  import('@/components/jobs/job-stream-viewer').then(mod => ({ default: mod.JobStreamViewer }))
);

function JobDetailPage() {
  const { id } = Route.useParams();

  return (
    <Suspense fallback={<Skeleton className="h-[400px] w-full" />}>
      <JobStreamViewer jobId={id} />
    </Suspense>
  );
}
```

---

## 10. テスト戦略

### 10.1 ユニットテスト

```typescript
// tests/unit/hooks/use-job-stream.test.ts
import { renderHook, waitFor } from '@testing-library/react';
import { vi, describe, it, expect } from 'vitest';
import { useJobStream } from '@/hooks/use-job-stream';

// Mock gRPC client
vi.mock('@/lib/grpc/client', () => ({
  createJobResultServiceClient: () => ({
    listenStream: vi.fn(() => ({
      responses: (async function* () {
        yield { item: { oneofKind: 'data', data: new TextEncoder().encode('Hello') } };
        yield { item: { oneofKind: 'data', data: new TextEncoder().encode(' World') } };
        yield { item: { oneofKind: 'end', end: { metadata: {} } } };
      })(),
    })),
  }),
}));

describe('useJobStream', () => {
  it('should stream and accumulate data chunks', async () => {
    const { result } = renderHook(() => useJobStream('123', '456'));

    await waitFor(() => {
      expect(result.current.status).toBe('completed');
    });

    const decoder = new TextDecoder();
    const text = result.current.chunks.map(c => decoder.decode(c)).join('');
    expect(text).toBe('Hello World');
  });
});
```

### 10.2 E2Eテスト

```typescript
// tests/e2e/agent-workflow.spec.ts
import { test, expect } from '@playwright/test';

test.describe('Agent Workflow', () => {
  test('should run agent for an issue and show progress', async ({ page }) => {
    // Setup: Add platform and repository
    await page.goto('/platforms/new');
    await page.fill('[name="baseUrl"]', 'https://github.com');
    await page.fill('[name="token"]', process.env.TEST_GITHUB_TOKEN!);
    await page.click('button[type="submit"]');

    // Navigate to issues
    await page.goto('/repositories/1/issues');

    // Start agent
    await page.click('[data-testid="run-agent-1"]');
    await page.click('[data-testid="confirm-run-agent"]');

    // Verify redirect to job page
    await expect(page).toHaveURL(/\/jobs\/\d+/);

    // Wait for streaming to start
    await expect(page.locator('[data-testid="job-status"]')).toContainText('streaming');

    // Wait for completion (with timeout)
    await expect(page.locator('[data-testid="job-status"]')).toContainText('completed', {
      timeout: 120000,
    });
  });
});
```

---

## 11. 開発環境セットアップ

### 11.1 必要条件

- **Node.js**: 20.19+ または 22.12+（Vite 7必須）
- **pnpm**: 9+
- **Rust**: stable (1.75+)
- **Tauri CLI**: `cargo install tauri-cli`
- **Protocol Buffers**: protoc (tonic-build用)
- **jobworkerp-rs**: ローカル起動

#### OS別追加要件

**Linux (Ubuntu/Debian)**:
```bash
sudo apt update
sudo apt install libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev librsvg2-dev
```

**macOS**: Xcode Command Line Tools
```bash
xcode-select --install
```

**Windows**: WebView2 (Windows 10/11では標準搭載)

### 11.2 セットアップ手順

```bash
# リポジトリクローン
git clone https://github.com/your-org/local-code-agent.git
cd local-code-agent

# フロントエンド依存関係インストール
pnpm install

# Rust依存関係インストール（自動）
cd src-tauri && cargo build && cd ..

# 開発サーバー起動（Tauri + Vite同時起動）
pnpm run tauri dev
```

### 11.3 環境変数

```bash
# .env.local (WebView用 - VITE_プレフィックス必須)
VITE_APP_NAME=Local Code Agent

# src-tauri/.env (Rust用 - dotenvで読み込み)
JOBWORKERP_GRPC_URL=http://localhost:9000
JOBWORKERP_AUTH_TOKEN=your-auth-token  # 設定されている場合
LOG_LEVEL=debug
```

### 11.4 Vite + Tauri 設定

```typescript
// vite.config.ts
import { defineConfig } from 'vite';
import react from '@vitejs/plugin-react';
import { TanStackRouterVite } from '@tanstack/router-plugin/vite';
import path from 'path';

// Tauri開発時はINTERNAL_TAURI_CONFIGが設定される
const isTauri = !!process.env.TAURI_ENV_PLATFORM;

export default defineConfig({
  plugins: [
    TanStackRouterVite(),
    react(),
  ],
  resolve: {
    alias: {
      '@': path.resolve(__dirname, './src'),
    },
  },

  // Tauri環境でのViteサーバー設定
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    // Tauri開発時はlocalhostのみ
    host: isTauri ? 'localhost' : '0.0.0.0',
    watch: {
      // Tauriのsrc-tauriディレクトリは監視対象外
      ignored: ['**/src-tauri/**'],
    },
  },

  // Tauriビルド設定
  build: {
    // Tauri用ビルドターゲット
    target: isTauri
      ? ['es2021', 'chrome100', 'safari15']
      : 'baseline-widely-available',
    // Tauri開発時はソースマップ有効
    sourcemap: !!process.env.TAURI_ENV_DEBUG,
    rollupOptions: {
      output: {
        manualChunks: {
          'tanstack': ['@tanstack/react-router', '@tanstack/react-query'],
          'ui': ['lucide-react'],
        },
      },
    },
  },

  // 環境変数プレフィックス
  envPrefix: ['VITE_', 'TAURI_'],
});
```

### 11.5 package.json

> **注記**: 以下のバージョンは実装時点で最新の安定版に調整すること。特にVite/Vitest/Tailwind CSSは頻繁にメジャーバージョンアップがあるため注意。

```json
{
  "name": "local-code-agent",
  "version": "0.1.0",
  "type": "module",
  "scripts": {
    "dev": "vite",
    "build": "tsc -b && vite build",
    "preview": "vite preview",
    "tauri": "tauri",
    "tauri:dev": "tauri dev",
    "tauri:build": "tauri build",
    "test": "vitest",
    "test:e2e": "playwright test",
    "lint": "biome check .",
    "format": "biome format --write ."
  },
  "dependencies": {
    "@tauri-apps/api": "^2.0.0",
    "@tauri-apps/plugin-shell": "^2.0.0",
    "@tanstack/react-query": "^5.60.0",
    "@tanstack/react-router": "^1.95.0",
    "@tanstack/react-form": "~0.40.0",
    "lucide-react": "^0.470.0",
    "react": "^19.0.0",
    "react-dom": "^19.0.0",
    "tailwind-merge": "^2.6.0",
    "zod": "^3.24.0",
    "zustand": "^5.0.0"
  },
  "devDependencies": {
    "@biomejs/biome": "^1.9.0",
    "@playwright/test": "^1.49.0",
    "@tauri-apps/cli": "^2.0.0",
    "@tanstack/router-plugin": "^1.146.0",
    "@testing-library/react": "^16.1.0",
    "@types/react": "^19.1.0",
    "@types/react-dom": "^19.1.0",
    "@vitejs/plugin-react": "^4.6.0",
    "autoprefixer": "^10.4.23",
    "postcss": "^8.5.6",
    "tailwindcss": "^4.1.0",
    "typescript": "^5.8.0",
    "vite": "^7.0.0",
    "vitest": "^4.0.0"
  },
  "engines": {
    "node": ">=20.19.0 || >=22.12.0"
  }
}
```

**バージョン選定方針**:
- `@tanstack/react-form`: 0.x系は破壊的変更が頻繁なため、パッチバージョンのみ許可（`~`）
- `vite`: 最新安定版（7.x）を使用
- `vitest`: viteメジャーバージョン-3を使用（vite 7.x には vitest 4.x）
- `tailwindcss`: 4.x安定版を使用

### 11.6 Rust Proto生成設定

```rust
// src-tauri/build.rs
fn main() {
    // Tauri build
    tauri_build::build();

    // tonic-build for gRPC
    // Note: FunctionService is a JSON-based wrapper for frontends, not needed for Rust backend
    let proto_files = [
        "proto/protobuf/jobworkerp/service/job.proto",
        "proto/protobuf/jobworkerp/service/job_result.proto",
        "proto/protobuf/jobworkerp/service/worker.proto",
    ];

    let includes = ["proto/protobuf"];

    tonic_build::configure()
        .build_server(false)  // クライアントのみ
        .out_dir("src/grpc/proto")
        .compile_protos(&proto_files, &includes)
        .expect("Failed to compile protos");
}
```

---

## 12. デプロイメント

### 12.1 開発ビルド

```bash
# 開発モード起動（ホットリロード有効）
pnpm run tauri:dev
```

### 12.2 プロダクションビルド

```bash
# デスクトップアプリビルド
pnpm run tauri:build
```

ビルド成果物の出力先:
- **Linux**: `src-tauri/target/release/bundle/deb/`, `appimage/`
- **macOS**: `src-tauri/target/release/bundle/dmg/`, `macos/`
- **Windows**: `src-tauri/target/release/bundle/msi/`, `nsis/`

### 12.3 クロスプラットフォームビルド

GitHub Actionsでのマルチプラットフォームビルド:

```yaml
# .github/workflows/release.yml
name: Release

on:
  push:
    tags:
      - 'v*'

jobs:
  release:
    strategy:
      fail-fast: false
      matrix:
        platform: [macos-latest, ubuntu-22.04, windows-latest]

    runs-on: ${{ matrix.platform }}
    steps:
      - uses: actions/checkout@v4

      - name: Setup Node
        uses: actions/setup-node@v4
        with:
          node-version: '22'

      - name: Setup pnpm
        uses: pnpm/action-setup@v3
        with:
          version: 9

      - name: Install Rust
        uses: dtolnay/rust-toolchain@stable

      - name: Install Linux dependencies
        if: matrix.platform == 'ubuntu-22.04'
        run: |
          sudo apt-get update
          sudo apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libappindicator3-dev

      - name: Install frontend dependencies
        run: pnpm install

      - name: Build Tauri app
        uses: tauri-apps/tauri-action@v0
        env:
          GITHUB_TOKEN: ${{ secrets.GITHUB_TOKEN }}
        with:
          tagName: v__VERSION__
          releaseName: 'Local Code Agent v__VERSION__'
          releaseBody: 'See the assets to download this version.'
          releaseDraft: true
          prerelease: false
```

### 12.4 自動更新（オプション）

Tauri v2の自動更新機能を使用する場合:

```json
// src-tauri/tauri.conf.json
{
  "plugins": {
    "updater": {
      "active": true,
      "endpoints": [
        "https://releases.local-code-agent.example.com/{{target}}/{{arch}}/{{current_version}}"
      ],
      "pubkey": "YOUR_PUBLIC_KEY"
    }
  }
}
```

---

## 13. 国際化（i18n）

### 13.1 技術選定

**採用**: Paraglide JS (@inlang/paraglide-js)

| 項目 | 値 |
|------|-----|
| バンドルサイズ | ~3-5 kB |
| 型安全性 | コンパイル時検証 |
| 対応言語 | 英語（en）、日本語（ja） |

**選定理由**:
- バンドルサイズ最小（Tauriデスクトップアプリに最適）
- TypeScript型安全（翻訳キーと引数のコンパイル時検証）
- TanStack Router公式統合例が存在
- Viteプラグイン対応

### 13.2 ディレクトリ構造

```
src/
├── paraglide/              # 自動生成（.gitignore対象）
│   ├── messages.js
│   └── runtime.js
├── messages/               # 翻訳ファイル
│   ├── en.json             # 英語（デフォルト）
│   └── ja.json             # 日本語
└── components/
    └── LanguageSwitcher.tsx
project.inlang/
└── settings.json           # Paraglide設定
```

### 13.3 設定ファイル

**project.inlang/settings.json**:
```json
{
  "$schema": "https://inlang.com/schema/project-settings",
  "sourceLanguageTag": "en",
  "languageTags": ["en", "ja"],
  "modules": [
    "https://cdn.jsdelivr.net/npm/@inlang/message-lint-rule-empty-pattern@latest/dist/index.js",
    "https://cdn.jsdelivr.net/npm/@inlang/message-lint-rule-missing-translation@latest/dist/index.js",
    "https://cdn.jsdelivr.net/npm/@inlang/plugin-message-format@latest/dist/index.js",
    "https://cdn.jsdelivr.net/npm/@inlang/plugin-m-function-matcher@latest/dist/index.js"
  ],
  "plugin.inlang.messageFormat": {
    "pathPattern": "./src/messages/{languageTag}.json"
  }
}
```

**vite.config.ts追加**:
```typescript
import { paraglideVitePlugin } from "@inlang/paraglide-js";

export default defineConfig(async () => ({
  plugins: [
    paraglideVitePlugin({
      project: "./project.inlang",
      outdir: "./src/paraglide",
    }),
    TanStackRouterVite(),
    react(),
  ],
  // ...
}));
```

### 13.4 翻訳ファイル形式

**src/messages/en.json**:
```json
{
  "app_title": "Local Code Agent",
  "nav_repositories": "Repositories",
  "nav_jobs": "Jobs",
  "nav_settings": "Settings",
  "button_save": "Save",
  "button_cancel": "Cancel",
  "status_pending": "Pending",
  "status_running": "Running",
  "status_completed": "Completed",
  "status_failed": "Failed"
}
```

**src/messages/ja.json**:
```json
{
  "app_title": "Local Code Agent",
  "nav_repositories": "リポジトリ",
  "nav_jobs": "ジョブ",
  "nav_settings": "設定",
  "button_save": "保存",
  "button_cancel": "キャンセル",
  "status_pending": "待機中",
  "status_running": "実行中",
  "status_completed": "完了",
  "status_failed": "失敗"
}
```

### 13.5 使用方法

```typescript
import * as m from '@/paraglide/messages';
import { setLanguageTag, languageTag } from '@/paraglide/runtime';

// 翻訳テキスト取得（型安全）
const title = m.app_title();
const saveButton = m.button_save();

// 言語切り替え
setLanguageTag('ja');

// 現在の言語取得
const currentLang = languageTag();
```

### 13.6 言語設定永続化

```typescript
// Tauri Store APIで永続化
import { Store } from '@tauri-apps/plugin-store';

const store = await Store.load('.settings.json');

// 保存
await store.set('locale', 'ja');
await store.save();

// 読み込み
const savedLocale = await store.get<string>('locale');
```

**言語検出優先順位**:
1. 保存済み設定（Tauri Store）
2. システム言語（navigator.language）
3. デフォルト言語（en）

### 13.7 LanguageSwitcherコンポーネント

```typescript
// src/components/LanguageSwitcher.tsx
import { languageTag, setLanguageTag, availableLanguageTags } from '@/paraglide/runtime';

const LANGUAGE_NAMES: Record<string, string> = {
  en: 'English',
  ja: '日本語',
};

export function LanguageSwitcher() {
  const currentLang = languageTag();

  const handleChange = async (lang: string) => {
    setLanguageTag(lang as 'en' | 'ja');
    // Tauri Storeに保存
    const store = await Store.load('.settings.json');
    await store.set('locale', lang);
    await store.save();
  };

  return (
    <select value={currentLang} onChange={(e) => handleChange(e.target.value)}>
      {availableLanguageTags.map((lang) => (
        <option key={lang} value={lang}>
          {LANGUAGE_NAMES[lang]}
        </option>
      ))}
    </select>
  );
}
```

---

## 14. 今後の拡張

### Phase 1（MVP）
- プラットフォーム設定
- リポジトリ管理
- Issue一覧表示
- エージェント実行（基本）
- ジョブステータス表示

### Phase 2
- ストリーミング出力表示
- 関連PR検出・警告
- Issue コメント分析
- PR完了状態の同期

### Phase 3
- ローカルLLM対応
- カスタムワークフロー
- 複数リポジトリ一括操作
- 通知機能

---

## 付録A: Tauri State管理

```rust
// src-tauri/src/state.rs
use r2d2::{Pool, PooledConnection};
use r2d2_sqlite::SqliteConnectionManager;
use std::sync::Arc;
use crate::crypto::token::TokenCrypto;
use crate::grpc::client::JobworkerpClient;

pub type DbPool = Pool<SqliteConnectionManager>;
pub type DbConnection = PooledConnection<SqliteConnectionManager>;

pub struct AppState {
    pub db: DbPool,
    pub crypto: TokenCrypto,
    pub grpc: Arc<JobworkerpClient>,
}

impl AppState {
    pub fn new(db_path: &str, grpc_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        // SQLite connection pool with foreign key enforcement
        let manager = SqliteConnectionManager::file(db_path)
            .with_init(|conn| {
                // Enable foreign key constraints (SQLite requires explicit enable)
                conn.execute_batch("PRAGMA foreign_keys = ON;")?;
                Ok(())
            });
        let db = Pool::new(manager)?;

        // Token encryption
        let crypto = TokenCrypto::new()?;

        // gRPC client
        let grpc = Arc::new(JobworkerpClient::new(grpc_url)?);

        Ok(Self { db, crypto, grpc })
    }
}
```

---

## 付録B: Tauri gRPCクライアント

```rust
// src-tauri/src/grpc/client.rs
use tonic::transport::Channel;
use crate::error::AppError;

// Generated from proto
mod proto {
    tonic::include_proto!("jobworkerp.service");
    tonic::include_proto!("jobworkerp.data");
}

use proto::job_service_client::JobServiceClient;
use proto::job_result_service_client::JobResultServiceClient;
use proto::worker_service_client::WorkerServiceClient;

#[derive(Clone)]
pub struct JobworkerpClient {
    channel: Channel,
    auth_token: Option<String>,
}

impl JobworkerpClient {
    pub fn new(url: &str) -> Result<Self, AppError> {
        let channel = Channel::from_shared(url.to_string())
            .map_err(|e| AppError::GrpcError(e.to_string()))?
            .connect_lazy();

        let auth_token = std::env::var("JOBWORKERP_AUTH_TOKEN").ok();

        Ok(Self { channel, auth_token })
    }

    fn job_client(&self) -> JobServiceClient<Channel> {
        JobServiceClient::new(self.channel.clone())
    }

    fn result_client(&self) -> JobResultServiceClient<Channel> {
        JobResultServiceClient::new(self.channel.clone())
    }

    fn worker_client(&self) -> WorkerServiceClient<Channel> {
        WorkerServiceClient::new(self.channel.clone())
    }

    pub async fn enqueue_job(
        &self,
        worker_name: &str,
        args: &serde_json::Value,
    ) -> Result<String, AppError> {
        let mut client = self.job_client();

        let request = proto::JobRequest {
            worker: Some(proto::job_request::Worker::WorkerName(worker_name.to_string())),
            args: serde_json::to_vec(args)?,
            ..Default::default()
        };

        let mut req = tonic::Request::new(request);
        if let Some(token) = &self.auth_token {
            req.metadata_mut()
                .insert("jobworkerp-auth", token.parse().unwrap());
        }

        let response = client.enqueue(req).await?;
        let job_id = response.into_inner().id
            .ok_or(AppError::GrpcError("No job ID returned".into()))?;

        Ok(job_id.value.to_string())
    }

    pub async fn listen_stream(
        &self,
        job_id: &str,
    ) -> Result<tonic::Streaming<proto::ResultOutputItem>, AppError> {
        let mut client = self.result_client();

        let request = proto::ListenRequest {
            job_id: Some(proto::JobId {
                value: job_id.parse().map_err(|_| AppError::InvalidInput("Invalid job ID".into()))?,
            }),
            ..Default::default()
        };

        let mut req = tonic::Request::new(request);
        if let Some(token) = &self.auth_token {
            req.metadata_mut()
                .insert("jobworkerp-auth", token.parse().unwrap());
        }

        let response = client.listen_stream(req).await?;
        Ok(response.into_inner())
    }

    pub async fn delete_job(&self, job_id: &str) -> Result<(), AppError> {
        let mut client = self.job_client();

        let request = proto::JobId {
            value: job_id.parse().map_err(|_| AppError::InvalidInput("Invalid job ID".into()))?,
        };

        let mut req = tonic::Request::new(request);
        if let Some(token) = &self.auth_token {
            req.metadata_mut()
                .insert("jobworkerp-auth", token.parse().unwrap());
        }

        client.delete(req).await?;
        Ok(())
    }
}
```

---

## 付録C: TanStack Router + Query 統合設定

```typescript
// src/main.tsx
import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import { RouterProvider, createRouter } from '@tanstack/react-router';
import { QueryClient, QueryClientProvider } from '@tanstack/react-query';
import { routeTree } from './routeTree.gen';

// Query Client
const queryClient = new QueryClient({
  defaultOptions: {
    queries: {
      staleTime: 1000 * 60 * 5, // 5分
      retry: 1,
    },
  },
});

// Router with Query Context
const router = createRouter({
  routeTree,
  context: {
    queryClient,
  },
  defaultPreload: 'intent',
  defaultPreloadStaleTime: 0,
});

// Type safety for router
declare module '@tanstack/react-router' {
  interface Register {
    router: typeof router;
  }
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <QueryClientProvider client={queryClient}>
      <RouterProvider router={router} />
    </QueryClientProvider>
  </StrictMode>
);
```
