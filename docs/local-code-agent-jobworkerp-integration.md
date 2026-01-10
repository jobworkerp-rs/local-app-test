# Local Code Agent Service - jobworkerp-rs 技術統合仕様

本文書は、Local Code Agent ServiceがjobworkerpRrsをバックエンドとして利用する際の技術的な前提条件と統合仕様を定義する。PRDの要件が実現可能であることを確認するための技術資料である。

## 関連文書

- PRD: `local-code-agent-service-prd.md`
- フロントエンド技術仕様: `local-code-agent-frontend-tech-spec.md`

---

## 1. jobworkerp-rs 概要

### 1.1 アーキテクチャ

jobworkerp-rsは、gRPCベースのジョブワーカーシステムである。本サービスでは以下のコンポーネントを利用する。

```
┌─────────────────────────────────────────────────────────────────┐
│                      jobworkerp-rs                               │
├─────────────────────────────────────────────────────────────────┤
│  ┌─────────────────┐  ┌─────────────────┐  ┌─────────────────┐ │
│  │  WORKFLOW       │  │   MCP_SERVER    │  │   COMMAND       │ │
│  │  Runner         │  │   Runner        │  │   Runner        │ │
│  │                 │  │                 │  │                 │ │
│  │  - マルチステップ │  │  - github MCP   │  │  - シェルコマンド│ │
│  │  - 状態管理     │  │  - gitea MCP    │  │  - git操作      │ │
│  │  - エラー処理   │  │  - filesystem   │  │  - claude CLI   │ │
│  └─────────────────┘  └─────────────────┘  └─────────────────┘ │
└─────────────────────────────────────────────────────────────────┘
```

### 1.2 利用するgRPCサービス

| サービス | 用途 | 主要API |
|---------|------|--------|
| JobService | ジョブ投入・キャンセル | `Enqueue`, `EnqueueForStream`, `Delete` |
| JobResultService | 結果取得 | `Listen`, `ListenStream`, `FindListBy` |
| JobProcessingStatusService | ステータス監視 | `Find`, `FindByCondition` |
| RunnerService | MCP動的登録・管理 | `Create`, `Delete`, `FindByName`, `FindListBy` |
| WorkerService | ワーカー管理 | `Create`, `Update`, `FindByName`, `FindList` |

### 1.3 認証

```
環境変数 AUTH_TOKEN:
  - 設定あり: 全リクエストに 'jobworkerp-auth' ヘッダー必須
  - 設定なし: 認証なし

ヘッダー形式:
  'jobworkerp-auth: <token-value>'
```

---

## 2. 利用するRunner

### 2.1 WORKFLOW Runner

マルチステップのジョブを定義・実行するためのランナー。Serverless Workflow DSL v1.0.0ベースの構文を使用。

#### ジョブ投入

```
JobService.EnqueueForStream(
  worker_name: "<workflow-worker-name>",
  args: WorkflowRunArgs {
    workflow_url: "file:///path/to/workflow.yaml",  // または workflow_data
    input: "{...JSON...}",
  }
) -> stream ResultOutputItem
```

**入力パラメータ**:
- `workflow_url`: ワークフロー定義のファイルパスまたはURL
- `workflow_data`: ワークフロー定義のYAML/JSON文字列（`workflow_url`と排他）
- `input`: ワークフローへの入力データ（JSON文字列）

#### ResultOutputItemの構造

ストリーミング結果は以下の3種類のバリエーションを持つ。

```
message ResultOutputItem {
  oneof item {
    bytes data = 1;            // ストリーミングデータチャンク
    Trailer end = 2;           // ストリーム終了マーカー
    bytes final_collected = 3; // 複合ジョブの最終結果
  }
}
```

**処理パターン**:
1. `data`: 中間データ（LLMトークン等）を受信
2. `final_collected`: ワークフロー全体の最終結果
3. `end`: ストリーム終了（メタデータ含む）

### 2.2 COMMAND Runner

シェルコマンドを実行するランナー。

#### 引数仕様

```
CommandArgs {
  command: string,            // 実行コマンド
  args: [string],             // コマンド引数
  with_memory_monitoring: bool, // メモリ監視フラグ（オプション）
}
```

**with_memory_monitoring**:
- `true`に設定すると、コマンド実行中のメモリ使用量を100ms間隔で監視
- 結果に`max_memory_usage_kb`フィールドが含まれる
- 長時間実行されるエージェントタスクのリソース監視に有用

**制約事項**:
- `workdir`オプションはサポートされない。ディレクトリ変更はシェルコマンド内で`cd`を使用
- `envs`オプションはサポートされない。環境変数設定はシェルコマンド内で行う

#### 使用例

```yaml
run:
  runner:
    name: COMMAND
    arguments:
      command: "sh"
      args: "${[\"-c\", \"cd \" + $worktree_path + \" && git status\"]}"
```

### 2.3 MCP_SERVER Runner

MCPサーバを介してツールを実行するランナー。

#### MCPサーバー認証アーキテクチャ

GitHub/Gitea MCPサーバーは**サーバー起動時にトークンを設定**する仕様であり、リクエストごとの動的なトークン変更はサポートされない。

**認証の仕組み**:
- **GitHub MCP**: 環境変数`GITHUB_PERSONAL_ACCESS_TOKEN`でトークンを設定
- **Gitea MCP**: 環境変数`GITEA_ACCESS_TOKEN`および`GITEA_URL`でトークンとURLを設定

**運用モード**:

| モード | 説明 | MCPサーバー登録 |
|-------|------|----------------|
| **静的設定モード** | `mcp-settings.toml`で事前設定 | jobworkerp-rs起動時に読み込み |
| **動的設定モード** | `RunnerService.Create`で実行時登録 | クライアントからAPIで登録 |

> **実装要件**: 両モードとも実装必須。ユーザーは運用状況に応じてどちらのモードも使用可能。

**参考リンク**:
- GitHub MCP Server: <https://github.com/github/github-mcp-server>
- Gitea MCP Server: <https://gitea.com/gitea/gitea-mcp> (Docker image: `docker.gitea.com/gitea-mcp-server`)

**HOST環境変数の違い**:

| プラットフォーム | 環境変数 | 値の形式 | 例 |
|-----------------|---------|---------|-----|
| GitHub Enterprise | `GITHUB_HOST` | ホスト名のみ | `github.example.com` |
| Gitea | `GITEA_HOST` | 完全なURL（スキーム含む） | `https://gitea.example.com` |

> **注意**: GitHub MCPの`GITHUB_HOST`はホスト名のみを指定するのに対し、Gitea MCPの`GITEA_HOST`（または`GITEA_URL`）は`https://`を含む完全なURLを指定する。この違いを間違えるとAPI接続に失敗する。

#### MCPサーバ設定フォーマット（静的設定モード）

MCPサーバはjobworkerp-rs起動時に`mcp-settings.toml`から読み込まれる。

**GitHub MCP Server（Docker実行形式・推奨）**:
```toml
[[server]]
name = "github"
description = "github server mcp"
transport = "stdio"
command = "docker"
args = [
  "run",
  "-i",
  "--rm",
  "-e",
  "GITHUB_PERSONAL_ACCESS_TOKEN",
  "ghcr.io/github/github-mcp-server"
]
envs = { GITHUB_PERSONAL_ACCESS_TOKEN = "github_pat_xxxxx" }
```

**Gitea MCP Server（Docker実行形式）**:
```toml
[[server]]
name = "gitea"
description = "gitea mcp server"
transport = "stdio"
command = "docker"
args = [
  "run",
  "-i",
  "--rm",
  "-e",
  "GITEA_ACCESS_TOKEN",
  "docker.gitea.com/gitea-mcp-server"
]
envs = { GITEA_ACCESS_TOKEN = "xxx" }
```

**Gitea MCP Server（SSE トランスポート）**:
```toml
[[server]]
name = "gitea-sse"
description = "gitea server mcp"
transport = "sse"
url = "http://gitea-mcp.example.com:8080/sse"

[server.headers]
Authorization = "Bearer xxxxx"
```

**重要な注意事項**:
- フィールド名は`envs`（複数形）を使用する。`env`は不可
- 環境変数の動的展開（`${VAR}`形式）はサポートされない
- トークン等の機密情報はファイルに直接記載されるため、適切な権限管理が必要
- トークンはサーバー起動時に設定され、実行時に変更できない
- Docker実行形式が推奨（環境の依存性が少ない）

#### MCPサーバ動的登録（動的設定モード）

複数アカウントや異なるプラットフォームを切り替える場合、`RunnerService.Create`でMCPサーバーを動的に登録できる。

**本サービスでの動的登録TOML生成（Docker実行形式）**:

GitHub:
```toml
[[server]]
name = "my-github"
description = "GitHub MCP Server"
transport = "stdio"
command = "docker"
args = [
  "run",
  "-i",
  "--rm",
  "-e",
  "GITHUB_PERSONAL_ACCESS_TOKEN",
  "ghcr.io/github/github-mcp-server"
]
envs = { GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_xxxx" }
```

GitHub Enterprise（GITHUB_HOST追加）:
```toml
[[server]]
name = "my-ghes"
description = "GitHub MCP Server"
transport = "stdio"
command = "docker"
args = [
  "run",
  "-i",
  "--rm",
  "-e",
  "GITHUB_PERSONAL_ACCESS_TOKEN",
  "-e",
  "GITHUB_HOST",
  "ghcr.io/github/github-mcp-server"
]
envs = { GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_xxxx", GITHUB_HOST = "https://github.example.com" }
```

Gitea:
```toml
[[server]]
name = "my-gitea"
description = "Gitea MCP Server"
transport = "stdio"
command = "docker"
args = [
  "run",
  "-i",
  "--rm",
  "-e",
  "GITEA_ACCESS_TOKEN",
  "docker.gitea.com/gitea-mcp-server"
]
envs = { GITEA_ACCESS_TOKEN = "xxx", GITEA_HOST = "https://gitea.example.com" }
```

**gRPC API呼び出し**:
```
RunnerService.Create(
  name: "my-github",
  description: "GitHub MCP Server",
  runner_type: MCP_SERVER,
  definition: "<TOML文字列>"  // 上記フォーマットのTOML文字列
)
```

**MCPサーバーの削除**:

プラットフォーム設定削除時は `RunnerService.Delete` でMCPサーバーも削除する。

```
RunnerService.Delete(runner_id)
```

**注意事項**:
- 動的登録されたMCPサーバーの設定はjobworkerp-rsのRDB（SQLite/MySQL）に永続化される
- jobworkerp-rs再起動時、RDBからMCPサーバー設定が自動的に読み込まれる
- 本サービス側では`PlatformConfig.mcp_runner_name`とjobworkerp-rs側のRunner名の関連付けを保持する
- トークンは本サービス側で暗号化して保存し、jobworkerp-rs側のRDBにも設定として保存される

#### ワークフローからの呼び出し

```yaml
run:
  runner:
    name: "${.mcp_server}"  # "github" または "gitea" 等
    using: "issue_read"     # MCPツール名（GitHub MCP v1.0.0+）
    arguments:
      owner: "${.owner}"
      repo: "${.repo}"
      issue_number: "${.issue_number}"
```

> **注意**: GitHub MCP Server v1.0.0以降、ツール名が変更されています。
> 旧: `get_issue` → 新: `issue_read`

---

## 3. ワークフロー定義仕様

### 3.1 DSL概要

Serverless Workflow DSL v1.0.0をベースとし、jobworkerp-rs固有の拡張を含む。

```yaml
document:
  dsl: "1.0.0"
  namespace: "namespace"
  name: "workflow-name"
  version: "1.0.0"

input:
  schema:
    document:
      type: object
      properties:
        # 入力スキーマ定義

do:
  - taskName:
      # タスク定義

output:
  schema:
    document:
      # 出力スキーマ定義
```

### 3.2 変数展開構文

#### jq構文 (`${...}`)

値全体を括る。部分的な埋め込みは不可。

> **実装詳細**: jq構文は [jaq](https://github.com/01mf02/jaq) ライブラリを使用して実装されている。jaqはjqのRust実装であり、大部分のjqフィルタをサポートするが、一部差異がある。

```yaml
# 入力データのフィールドにアクセス
value: "${.field_name}"

# コンテキスト変数にアクセス（set/exportで設定）
value: "${$context_variable}"

# 文字列連結
value: "${\"prefix-\" + .field_name}"

# 配列構築
value: "${[\"arg1\", \"arg2\", .dynamic_arg]}"
```

**jaq固有の関数（jqとの差異）**:

| jq構文 | jaq構文 | 説明 |
|--------|---------|------|
| `@base64` | なし | jaqでは `encode_base64` / `decode_base64` を使用 |
| `@uri` | なし | jaqでは `encode_uri` / `decode_uri` を使用 |
| `@html` | なし | jaqでは `escape_html` / `unescape_html` を使用 |
| `@csv` | なし | jaqでは `escape_csv` を使用 |
| `@tsv` | なし | jaqでは `escape_tsv` を使用 |
| `@sh` | なし | jaqでは `escape_sh` を使用 |

**例: base64エンコード**:
```yaml
# jq形式（非サポート）
# value: "${$prompt | @base64}"

# jaq形式（正しい）
value: "${$prompt | encode_base64}"
```

#### Liquid構文 (`$${...}`)

テンプレート展開用。値全体を括る。

```yaml
prompt: |
  $${
  ## Issue #{{ issue_number }}: {{ issue_title }}

  {{ issue_body }}

  {% for comment in comments %}
  {{ comment.body }}
  {% endfor %}
  }
```

### 3.3 タスク種別

#### set タスク

コンテキスト変数を設定する。

```yaml
- determineBranchName:
    set:
      branch_name: "${\"issue-\" + (.issue_number | tostring)}"
      worktree_path: "${.worktree_base_path + \"/issue-\" + (.issue_number | tostring)}"
```

#### run タスク

ランナーを実行する。

```yaml
- fetchIssue:
    run:
      runner:
        name: "${.mcp_server}"
        using: "issue_read"  # GitHub MCP v1.0.0+
        arguments:
          owner: "${.owner}"
          repo: "${.repo}"
          issue_number: "${.issue_number}"
    export:
      as:
        issue_body: "${.body}"
        issue_labels: "${.labels}"
```

**export.as の構文**: object型（key-value形式）

#### try-catch タスク

エラーハンドリング。

```yaml
- runWithErrorHandling:
    try:
      - mainTask:
          run:
            runner:
              name: COMMAND
              arguments:
                command: "..."
                args: ["..."]
          timeout:
            after:
              minutes: 10
    catch:
      as: error
      do:
        - cleanupTask:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "..."
                  args: ["..."]
        - raiseError:
            raise:
              error:
                type: "execution_failed"
                status: 500
                title: "Execution failed"
                detail: "${$error.message}"
```

---

## 4. ジョブキャンセル仕様

### 4.1 API

```
JobService.Delete(job_id) -> SuccessResponse
```

### 4.2 動作

| ジョブ状態 | Delete呼び出し時の動作 |
|----------|----------------------|
| PENDING | キューから削除。JobResultは作成されない |
| RUNNING | ジョブをキャンセルし、JobResultにCANCELLED状態で記録 |
| WAIT_RESULT | ジョブをキャンセルし、JobResultにCANCELLED状態で記録 |

### 4.3 クリーンアップ処理

ワークフロー内で`try-catch`を使用し、エラー時（キャンセル含む）のクリーンアップ処理を定義することを推奨。

```yaml
try:
  - mainTask:
      # メイン処理
catch:
  as: error
  do:
    - cleanup:
        # worktree削除等のクリーンアップ処理
```

---

## 5. エージェントワークフロー設計

### 5.1 ワークフロー入力

```yaml
input:
  schema:
    document:
      type: object
      properties:
        owner:
          type: string
          description: "リポジトリオーナー"
        repo:
          type: string
          description: "リポジトリ名"
        issue_number:
          type: integer
          description: "Issue番号"
        issue_title:
          type: string
          description: "Issueタイトル"
        base_branch:
          type: string
          description: "ベースブランチ"
          default: "main"
        clone_url:
          type: string
          description: "認証付きクローンURL（トークン埋め込み）"
        base_clone_path:
          type: string
          description: "ベースクローンディレクトリパス"
        worktree_path:
          type: string
          description: "worktree作成先パス（タイムスタンプ付き）"
        branch_name:
          type: string
          description: "作成するブランチ名"
        mcp_server:
          type: string
          description: "使用するMCPサーバ名 (github/gitea)"
        custom_prompt:
          type: string
          description: "カスタムプロンプト（オプション）"
      required:
        - owner
        - repo
        - issue_number
        - issue_title
        - clone_url
        - base_clone_path
        - worktree_path
        - branch_name
        - mcp_server
```

#### 5.1.1 ディレクトリ構造

ワークフロー入力のパス変数は以下のディレクトリ構造を前提とする：

```
worktree_base_path/                          # アプリ設定（デフォルト: ~/.local-code-agent/worktrees）
├── {repo_identifier}/                       # リポジトリ識別子（local_path or owner/repo-name）
│   ├── .git/                                # ベースクローン（自動作成）
│   ├── issue-123-1704067200/                # worktree（タイムスタンプ付き）
│   ├── issue-123-1704153600/                # 同一Issueの再実行
│   └── issue-456-1704240000/
```

**変数の役割**:

| 変数 | 設定箇所 | 値の例 | 役割 |
|------|----------|--------|------|
| `worktree_base_path` | AppSettings | `~/.local-code-agent/worktrees` | 全リポジトリ共通のベースパス |
| `repo_identifier` | 自動計算 | `owner/repo-name` or `custom-name` | ベースクローン・worktreeのサブディレクトリ |
| `base_clone_path` | 自動計算 | `{worktree_base_path}/{repo_identifier}` | gitクローン先 |
| `worktree_path` | 自動計算 | `{base_clone_path}/issue-{N}-{timestamp}` | 作業ディレクトリ |
| `branch_name` | 自動計算 | `issue-{N}` | 作成するブランチ名 |
| `clone_url` | 自動計算 | `https://x-access-token:{token}@github.com/...` | 認証付きクローンURL |

**リポジトリ識別子の決定**:
- Repository.local_pathが設定されている場合: その値を使用
- 未設定の場合: `{owner}/{repo_name}` を使用

**タイムスタンプ**:
- 同一Issueの再実行時にパス衝突を避けるため、UNIXタイムスタンプ（秒）を付与
- 例: `issue-123-1704067200`

#### 5.1.2 認証付きクローンURL

プライベートリポジトリのクローンに対応するため、トークンを埋め込んだURLを使用する。

**URL形式**:
- GitHub: `https://x-access-token:{token}@github.com/{owner}/{repo}.git`
- Gitea: `https://git:{token}@{host}/{owner}/{repo}.git`

**トークンの取得**:
Runner定義（`RunnerService.FindByName`で取得）の`data.definition`フィールドにMCPサーバー設定が含まれており、以下の優先順位でトークンを抽出する。

**抽出優先順位**:
1. `envs`フィールドから直接取得
2. Docker実行の場合、`args`配列の`-e`オプションで指定された環境変数名を取得し、それに対応する`envs`の値を使用

**設定パターン例**:

パターン1: envsに直接設定（推奨）
```toml
[[server]]
name = "github"
command = "docker"
args = ["run", "-i", "--rm", "-e", "GITHUB_PERSONAL_ACCESS_TOKEN", "ghcr.io/github/github-mcp-server"]
envs = { GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_xxxx" }
```

パターン2: -eオプションで値も指定（KEY=VALUE形式）
```toml
[[server]]
name = "github"
command = "docker"
args = ["run", "-i", "--rm", "-e", "GITHUB_PERSONAL_ACCESS_TOKEN=ghp_xxxx", "ghcr.io/github/github-mcp-server"]
```

**抽出ロジック**:
```
1. envs から直接 GITHUB_PERSONAL_ACCESS_TOKEN / GITEA_ACCESS_TOKEN を取得
2. 取得できない場合、args から "-e" の次の要素を確認
   - "KEY=VALUE" 形式の場合: VALUE を使用
   - "KEY" のみの場合: envs[KEY] を使用
```

**セキュリティ要件**:
- `clone_url`は認証情報を含むため、ログ出力に含めてはならない
- エラーメッセージにもURLを含めない
- Debug実装でマスキングを行う

### 5.2 ワークフローステップ

| ステップ | 使用Runner | 説明 |
|---------|-----------|------|
| 1. ベースクローン存在確認 | COMMAND | `.git`ディレクトリの存在確認 |
| 2. 必要に応じてクローン | COMMAND | `git clone` (clone_url使用) |
| 3. ベースクローン更新 | COMMAND | `git fetch origin` |
| 4. Worktree作成 | COMMAND | `git worktree add` (worktree_path, branch_name使用) |
| 5. Issue情報取得 | MCP_SERVER | Issue本文・ラベル取得 |
| 6. Issueコメント取得 | MCP_SERVER | 追加要件の取得 |
| 7. プロンプト生成 | (set) | Liquidテンプレート展開 |
| 8. エージェント実行 | COMMAND | `claude --print` |
| 9. 変更プッシュ | COMMAND | `git push` |
| 10. PR作成 | MCP_SERVER | `create_pull_request` |
| 11. クリーンアップ | COMMAND | `git worktree remove` |

> **注意**: ブランチ名・パスの決定はクライアント側（agent.rs）で行い、ワークフローには計算済みの値を渡す。

### 5.3 エージェントプロンプト安全性

**問題**: シェルコマンド内でプロンプト文字列を直接展開すると、特殊文字によるコマンドインジェクションのリスクがある。

**推奨対応**:
1. プロンプトを一時ファイルに書き出す
2. `claude --print < prompt_file` で標準入力から読み込む

**方法1: HEREDOC方式**（シンプルだが、EOFマーカーが本文に含まれると壊れる）
```yaml
# プロンプトをファイルに書き出し
- writePromptFile:
    run:
      runner:
        name: COMMAND
        arguments:
          command: "sh"
          args: "${[\"-c\", \"cat > \" + $worktree_path + \"/.agent_prompt.txt << 'AGENT_PROMPT_EOF'\n\" + $agent_prompt + \"\nAGENT_PROMPT_EOF\"]}"

# ファイルからプロンプトを読み込んで実行
- runAgent:
    run:
      runner:
        name: COMMAND
        arguments:
          command: "sh"
          args: "${[\"-c\", \"cd \" + $worktree_path + \" && claude --print < .agent_prompt.txt\"]}"
```

**方法2: base64エンコード方式**（より安全、特殊文字を含むプロンプトでも安全）
```yaml
# プロンプトをbase64エンコードしてファイルに書き出し
- writePromptFile:
    run:
      runner:
        name: COMMAND
        arguments:
          command: "sh"
          args: "${[\"-c\", \"echo \" + ($agent_prompt | encode_base64) + \" | base64 -d > \" + $worktree_path + \"/.agent_prompt.txt\"]}"

# ファイルからプロンプトを読み込んで実行
- runAgent:
    run:
      runner:
        name: COMMAND
        arguments:
          command: "sh"
          args: "${[\"-c\", \"cd \" + $worktree_path + \" && claude --print < .agent_prompt.txt\"]}"
```

> **注意**: base64方式を使用する場合、jaqの`encode_base64`フィルタを使用する（jq標準の`@base64`はサポートされない）。

または、専用のCLAUDE_CODE_RUNNERプラグインの実装を検討。

### 5.4 ワークフロー出力

```yaml
output:
  schema:
    document:
      type: object
      properties:
        status:
          type: string
          enum: ["success", "failed"]
          description: "ワークフロー実行結果のステータス"
        pr_number:
          type: integer
          description: "作成されたPR番号（成功時のみ）"
        pr_url:
          type: string
          description: "作成されたPRのURL（成功時のみ）"
        error:
          type: object
          description: "エラー情報（失敗時のみ）"
          properties:
            type:
              type: string
              description: "エラー種別"
            message:
              type: string
              description: "エラーメッセージ"
      required:
        - status
```

---

## 6. GitHub/Gitea MCP API対応表

プラットフォーム間の互換性を確保するため、両方で利用可能なAPIのみを使用する。

| 機能 | GitHub MCP | Gitea MCP |
|------|------------|-----------|
| Issue一覧 | `list_issues` | `list_repo_issues` |
| Issue詳細（読取） | `issue_read` (method="get") | `get_issue_by_index` |
| Issueコメント取得 | `issue_read` (method="get_comments") | `list_issue_comments` |
| Issueコメント追加 | `add_issue_comment` | `create_issue_comment` |
| Issue検索 | `search_issues` | - |
| PR一覧 | `list_pull_requests` | `list_repo_pull_requests` |
| PR詳細（読取） | `pull_request_read` | `get_pull_request_by_index` |
| PR作成 | `create_pull_request` | `create_pull_request` |
| PRマージ | `merge_pull_request` | `merge_pull_request` |
| ブランチ一覧 | `list_branches` | `list_branches` |
| ブランチ作成 | `create_branch` | `create_branch` |
| ファイル取得 | `get_file_contents` | `get_file_content` |
| ファイル作成/更新 | `create_or_update_file` | - |
| コミット一覧 | `list_commits` | `list_repo_commits` |
| ユーザー情報 | `get_me` | `get_my_user_info` |

> **重要**: GitHub MCP Server v1.0.0以降、ツール名が変更されています（例: `get_issue` → `issue_read`）。
> 実装時に各MCPサーバの`tools/list`で利用可能なツールを必ず確認してください。

> **Toolsetについて**: GitHub MCP Serverは機能をToolsetとして整理しています。
> 一部のツールは明示的な有効化が必要な場合があります（`enable_toolset`で有効化）。

**レスポンスフィールド名の差異に関する注意**:
- 両プラットフォームで返却されるフィールド名が異なる場合がある（例: `body` vs `content`）
- ワークフロー内でプラットフォームを判定し、フィールド名を適切にマッピングする必要がある場合がある
- 実装時は各MCPサーバーのレスポンス形式を確認し、必要に応じてjq式で変換を行うこと

**プラットフォーム両対応のjq変換例**:

```yaml
# Issue情報取得後のフィールドマッピング
- fetchIssue:
    run:
      runner:
        name: "${.mcp_server}"
        # GitHub MCP v1.0.0+: issue_read, Gitea: get_issue_by_index
        using: "${if .mcp_server == \"github\" then \"issue_read\" else \"get_issue_by_index\" end}"
        arguments:
          owner: "${.owner}"
          repo: "${.repo}"
          issue_number: "${.issue_number}"
    export:
      as:
        # GitHub: body, Gitea: body または content (実装依存)
        issue_body: "${.body // .content // \"\"}"
        # GitHub: labels[].name, Gitea: labels[].name (共通)
        issue_labels: "${.labels // []}"
        # GitHub: html_url, Gitea: html_url (共通)
        issue_url: "${.html_url // \"\"}"

# PR作成後のレスポンスマッピング
- createPR:
    run:
      runner:
        name: "${.mcp_server}"
        using: "create_pull_request"
        arguments:
          # ... 省略
    export:
      as:
        # GitHub: number, Gitea: number (共通)
        pr_number: "${.number}"
        # GitHub: html_url, Gitea: html_url (共通)
        pr_url: "${.html_url // \"\"}"
```

> **注意**: jq式では`//`演算子（alternative operator）を使用して、最初に存在するフィールドを取得できる。

**参考リンク**:
- GitHub MCP Server: <https://github.com/github/github-mcp-server>
- Gitea MCP Server: <https://gitea.com/gitea/gitea-mcp>

---

## 7. クライアント統合パターン

本セクションでは、クライアントアプリケーションがjobworkerp-rsと統合する際の一般的なパターンを示す。具体的な実装技術（REST/gRPC-Web/Tauri等）はクライアント技術仕様で定義する。

### 7.1 ジョブ投入・結果取得

```
1. クライアント → jobworkerp-rs: EnqueueForStream()
2. クライアント ← jobworkerp-rs: stream ResultOutputItem
   - data: 進捗データを受信・表示
   - final_collected: 最終結果を受信
   - end: ストリーム終了
3. クライアント: 結果をローカルDBに保存
```

### 7.2 ジョブキャンセル

```
1. クライアント → jobworkerp-rs: Delete(job_id)
2. ワークフロー内: catch句でクリーンアップ実行
3. クライアント ← jobworkerp-rs: ストリーム終了（end）
4. クライアント: ローカルDBのステータスを「Cancelled」に更新
```

### 7.3 ステータス監視

**アクティブジョブ**: `EnqueueForStream`のストリーミングで自動取得

**履歴ジョブ**: `JobResultService.FindListBy`で検索

### 7.4 必要なデータモデル

クライアント側で管理するデータモデル:

#### 共通モデル（両モード）

| モデル | 説明 | 主要フィールド |
|-------|------|---------------|
| AppSettings | アプリケーション設定 | worktree_base_path, default_base_branch, agent_timeout |
| AgentJob | エージェントジョブ | repository_id, issue_number, jobworkerp_job_id, status |

**AppSettings詳細**:

| フィールド | 型 | 説明 | デフォルト |
|-----------|---|------|-----------|
| worktree_base_path | String | worktree作成先ベースパス | `~/.local-code-agent/worktrees` |
| default_base_branch | String | デフォルトベースブランチ | `main` |
| agent_timeout_minutes | i32 | エージェント実行タイムアウト（分） | 30 |
| sync_interval_minutes | i32 | PR状態同期間隔（分） | 10 |

> **注記**: `worktree_base_path`はワークフロー入力の必須パラメータ。アプリケーション設定画面で管理し、エージェント実行時にワークフロー引数として渡す。

#### 静的設定モード

トークン管理は`mcp-settings.toml`で行うため、最小限のモデルで運用可能。

| モデル | 説明 | 主要フィールド |
|-------|------|---------------|
| Repository | リポジトリ情報 | mcp_server_name, platform, owner, repo_name, local_path (optional) |

**local_pathの役割**:
- オプションフィールド
- リポジトリ識別子のカスタマイズに使用
- 未設定時は `owner/repo_name` がリポジトリ識別子として使用される
- 特殊ケースで重複が発生した場合の回避用

#### 動的設定モード

複数アカウント・プラットフォームを管理する場合、追加のモデルが必要。

| モデル | 説明 | 主要フィールド |
|-------|------|---------------|
| PlatformConfig | プラットフォーム設定 | platform, base_url, token_id, mcp_runner_name |
| Repository | リポジトリ情報 | platform_config_id, owner, repo_name, local_path |
| TokenStore | 暗号化トークン | platform, encrypted_token |

**AgentJobStatus**:
- Pending, PreparingWorkspace, FetchingIssue, RunningAgent, CreatingPR
- PrCreated, Merged, Completed, Failed, Cancelled

---

## 8. 前提条件・制約

### 8.1 jobworkerp-rs起動要件

- jobworkerp-rsが起動していること
- 必要に応じて`AUTH_TOKEN`環境変数が設定されていること

**静的設定モードの場合**:
- `mcp-settings.toml`にGitHub/Gitea MCPサーバが設定されていること
- トークンが`mcp-settings.toml`内に設定されていること

**動的設定モードの場合**:
- `RunnerService.Create`でMCPサーバーを登録する仕組みがクライアント側に実装されていること
- トークンをクライアント側で安全に保存・管理する仕組みがあること

### 8.2 Claude Code認証

ワーカープロセスを実行するユーザーアカウントで事前にClaude Code認証を完了しておくこと。

**認証情報の保存場所**:
- macOS/Linux: `~/.claude/`
- Windows: `%USERPROFILE%\.claude\`

### 8.3 リポジトリクローン

- ベースクローンはワークフロー実行時に自動作成される
- 初回実行時に `worktree_base_path/{repo_identifier}` にクローンが作成される
- 認証付きURL（トークン埋め込み）でプライベートリポジトリもクローン可能
- git worktreeを作成可能な権限があること

### 8.4 ネットワーク要件

- GitHub/Gitea APIへのアクセス（MCPサーバ経由）
- Claude APIへのアクセス（Claude Code経由）

---

## 付録A: ワークフロー定義サンプル

完全なワークフロー定義の参考実装:

```yaml
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
        owner:
          type: string
        repo:
          type: string
        issue_number:
          type: integer
        issue_title:
          type: string
        base_branch:
          type: string
          default: "main"
        clone_url:
          type: string
          description: "認証付きクローンURL（トークン埋め込み）"
        base_clone_path:
          type: string
          description: "ベースクローンディレクトリパス"
        worktree_path:
          type: string
          description: "worktree作成先パス（タイムスタンプ付き）"
        branch_name:
          type: string
          description: "作成するブランチ名"
        mcp_server:
          type: string
        custom_prompt:
          type: string
          description: "カスタムプロンプト（オプション）"
      required:
        - owner
        - repo
        - issue_number
        - issue_title
        - clone_url
        - base_clone_path
        - worktree_path
        - branch_name
        - mcp_server

do:
  # 1. メイン処理（エラーハンドリング付き）
  - mainProcessWithErrorHandling:
      try:
        # 1.1 ベースクローンの存在確認
        - checkBaseClone:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "test"
                  args: "${[\"-d\", .base_clone_path + \"/.git\"]}"
            export:
              as:
                base_clone_exists: true
            catch:
              as: error
              do:
                - setCloneNeeded:
                    set:
                      base_clone_exists: false

        # 1.2 必要に応じてクローン
        - cloneIfNeeded:
            if: "${$base_clone_exists == false}"
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "git"
                  args: "${[\"clone\", .clone_url, .base_clone_path]}"

        # 1.3 ベースクローンを最新に更新
        - fetchLatest:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "git"
                  args: "${[\"-C\", .base_clone_path, \"fetch\", \"origin\"]}"

        # 1.4 Worktree作成
        - createWorktree:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "git"
                  args: "${[\"-C\", .base_clone_path, \"worktree\", \"add\", .worktree_path, \"-b\", .branch_name, \"origin/\" + .base_branch]}"

        # 1.5 Issue情報取得
        # Note: GitHub MCP v1.0.0+では issue_read を使用、method="get"でissue詳細取得
        - fetchIssue:
            run:
              runner:
                name: "${.mcp_server}"
                using: "issue_read"
                arguments:
                  owner: "${.owner}"
                  repo: "${.repo}"
                  issue_number: "${.issue_number}"
                  method: "get"
            export:
              as:
                issue_body: "${.body}"

        # 1.6 Issueコメント取得
        # Note: GitHub MCP v1.0.0+では issue_read の method="get_comments" でコメント取得
        - fetchIssueComments:
            run:
              runner:
                name: "${.mcp_server}"
                using: "issue_read"
                arguments:
                  owner: "${.owner}"
                  repo: "${.repo}"
                  issue_number: "${.issue_number}"
                  method: "get_comments"
            export:
              as:
                issue_comments: "${.}"

        # 1.7 プロンプト生成
        - generatePrompt:
            set:
              agent_prompt: |
                $${
                以下のIssueを解決するコードを実装してください。

                ## Issue #{{ issue_number }}: {{ issue_title }}

                {{ issue_body }}

                ## 追加コメント
                {% for comment in issue_comments %}
                {{ comment.body }}

                {% endfor %}

                {% if custom_prompt %}
                ## 追加指示
                {{ custom_prompt }}
                {% endif %}

                ## 指示
                - 必要なファイルを作成・修正してください
                - テストを実行して動作確認してください
                - コミットメッセージは適切に記述してください
                }

        # 1.8 プロンプトファイル作成
        - writePromptFile:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "sh"
                  args: "${[\"-c\", \"cat > \" + .worktree_path + \"/.agent_prompt.txt << 'AGENT_PROMPT_EOF'\n\" + $agent_prompt + \"\nAGENT_PROMPT_EOF\"]}"

        # 1.9 エージェント実行
        - runAgent:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "sh"
                  args: "${[\"-c\", \"cd \" + .worktree_path + \" && claude --print < .agent_prompt.txt\"]}"
            timeout:
              after:
                minutes: 10

        # 1.10 変更プッシュ
        - pushChanges:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "git"
                  args: "${[\"-C\", .worktree_path, \"push\", \"-u\", \"origin\", .branch_name]}"

        # 1.11 PR作成
        - createPR:
            run:
              runner:
                name: "${.mcp_server}"
                using: "create_pull_request"
                arguments:
                  owner: "${.owner}"
                  repo: "${.repo}"
                  title: "${\"Fix #\" + (.issue_number | tostring) + \": \" + .issue_title}"
                  body: |
                    $${
                    ## Summary
                    This PR addresses #{{ issue_number }}.

                    ## Changes
                    Automatically generated by Local Code Agent Service.
                    }
                  head: "${.branch_name}"
                  base: "${.base_branch}"
            export:
              as:
                pr_number: "${.number}"
                pr_url: "${.html_url}"

        # 1.12 クリーンアップ
        - cleanup:
            run:
              runner:
                name: COMMAND
                arguments:
                  command: "git"
                  args: "${[\"-C\", .base_clone_path, \"worktree\", \"remove\", .worktree_path]}"

      catch:
        as: error
        do:
          # エラー時のクリーンアップ
          - cleanupOnError:
              run:
                runner:
                  name: COMMAND
                  arguments:
                    command: "sh"
                    args: "${[\"-c\", \"git -C \" + .base_clone_path + \" worktree remove --force \" + .worktree_path + \" 2>/dev/null || true\"]}"
          - raiseError:
              raise:
                error:
                  type: "agent_execution_failed"
                  status: 500
                  title: "Agent execution failed"
                  detail: "${$error.message}"

output:
  schema:
    document:
      type: object
      properties:
        pr_number:
          type: integer
        pr_url:
          type: string
```

---

## 付録B: MCPサーバ設定サンプル

```toml
# mcp-settings.toml

# GitHub MCP Server
[[server]]
name = "github"
description = "GitHub MCP Server for repository operations"
transport = "stdio"
command = "docker"
args = ["run", "-i", "--rm", "-e", "GITHUB_PERSONAL_ACCESS_TOKEN", "ghcr.io/github/github-mcp-server"]
envs = { GITHUB_PERSONAL_ACCESS_TOKEN = "ghp_xxxxxxxxxxxx" }

# Gitea MCP Server (stdio)
[[server]]
name = "gitea"
description = "Gitea MCP Server for repository operations"
transport = "stdio"
command = "gitea-mcp-server"
envs = { GITEA_TOKEN = "xxxxxxxx", GITEA_URL = "https://gitea.example.com" }

# Gitea MCP Server (SSE - 代替)
# [[server]]
# name = "gitea-sse"
# description = "Gitea MCP Server (SSE)"
# transport = "sse"
# url = "http://gitea-mcp.example.com:8080/sse"
#
# [server.headers]
# Authorization = "Bearer xxxxxxxx"
```
