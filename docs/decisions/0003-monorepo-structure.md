# ADR-0003: モノレポ構成

## Context
Rust のエンジンと CLI と HTML UI と Python の AI を同じリポジトリで開発する。データ契約は JSONL を共有する。

## Decision
`rust` `python` `docs` `data` `tmp` を同一リポジトリに置く。

## Consequences
- 変更の一貫性を確保できる
- 依存とバージョンの同期が容易
- 分離が必要になればディレクトリ単位で切り出せる
