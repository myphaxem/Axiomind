# RUNBOOK

ローカルとオフラインを前提とする手順。

## セットアップ
- Rust stable を導入 rustup を使用
- Python 3.12 を導入
- 仮想環境を作成
  - `python -m venv .venv`
  - `.\\.venv\\Scripts\\Activate.ps1`
  - `pip install -U pip ruff black`

## データ
- ハンド履歴 data/hands/YYYYMMDD/*.jsonl
- 集計 DB data/db.sqlite
- ログ data/logs/*.log

## トラブルシュート
- 乱数の再現 `--seed` を指定し同一バージョンで再実行
- JSONL の破損 末尾途中行を検出し以降を破棄
- SQLite のロック 単一プロセスで書き込み バッチ化を使用
