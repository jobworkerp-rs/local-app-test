# Local Code Agent

GitHub/Gitea リポジトリのIssue・PRを管理し、コーディングエージェントを使用して自動的にプランニング・実装・PR作成を行うローカルサービス。

## 概要

jobworkerp-rsをバックエンドとして利用し、ワークフローベースでエージェント処理を実行するデスクトップアプリケーション。

## 前提条件

- Node.js 20.19以上 または 22.12以上
- pnpm 9.0以上
- Rust stable 1.75以上
- jobworkerp-rsが起動していること
- Claude Code認証が完了していること

## セットアップ

```bash
# 依存関係のインストール
pnpm install

# 開発サーバーの起動
pnpm tauri dev
```

## ドキュメント

- [PRD](docs/local-code-agent-service-prd.md) - サービス要件定義
- [技術統合仕様](docs/local-code-agent-jobworkerp-integration.md) - jobworkerp-rs統合の技術詳細
- [フロントエンド技術仕様](docs/local-code-agent-frontend-tech-spec.md) - クライアント実装仕様
- [実装計画](docs/local-code-agent-implementation-plan.md) - 実装フェーズと計画

## ライセンス

MIT
