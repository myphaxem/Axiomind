# ADR-0004: Git 運用

## Decision
- main は常時クリーン テストとビルドが通る状態
- 作業は feature ブランチで行い PR で main に統合
- コミットメッセージは Conventional Commits
- タグは SemVer 形式 例 `v0.1.0`

## Consequences
- 変更履歴の可読性が上がる 自動生成の変更履歴に対応しやすい
- リリース時の差分が明確
