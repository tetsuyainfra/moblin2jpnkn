# Commit message instructions

- Write commit messages in Japanese.
- Use the Conventional Commits format:
  ```
  <type>(<scope>): <summary>

  <body>
  ```
- Choose a type that matches the change from the allowed types listed below.
- Add a scope when it makes the affected area clearer, such as `role_skelton` or `tests`.
- Keep the summary concise, describe the user-visible or functional change, and do not end it with a period.
- Use the staged changes as the source of truth. Do not claim behavior, tests, or files changed unless the diff supports it.
- Add a body(<body>) only when context is needed to explain why the change was made or an important implementation detail.
- For incompatible changes, include `!` after the type or scope and explain the impact in the body with `BREAKING CHANGE:`.
- Do not include issue numbers, release versions, or other details unless they are present in the provided context.

## Allowed Types
  - fix: バグ修正
  - hotfix: 緊急修正
  - feat: 新機能の追加
  - docs: ドキュメントの変更
  - update: 既存の機能に問題がないが、改善や調整を行った場合
  - change: 既存の機能に影響のある変更
  - refactor: リファクタリング
  - clean: 不要なファイルやコードの削除
  - perf: パフォーマンス改善
  - delete: コードやファイルの削除
  - rename: 名前の変更
  - move: ファイルやコードの移動
  - upgrade: 依存関係やツールのアップグレード
  - revert: 変更の取り消し
  - sytle: コードスタイルの修正
  - test: テストの追加や修正
  - build: ビルド関連の変更
  - ci: CI設定の変更
  - chore: その他の雑務

## Examples

```text
feat(role_skelton): テスト用のローカル実行設定を追加
fix(tests): 親ディレクトリのロールを明示して読み込む
docs: パッケージ更新手順を追記