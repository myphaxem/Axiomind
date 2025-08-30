# ADR-0001: ハンド履歴フォーマットは JSONL

## Context
自己学習と評価で大量のハンド履歴を扱う。追記とストリーム処理とツール互換性が重要。

## Options
- JSON 配列 は追記と復旧が難しい
- JSONL は一行一レコードで追記と部分復旧に強い
- CSV はネスト表現が難しい

## Decision
JSONL を採用する。一行一ハンド。文字コードは UTF-8。改行は LF。パスは `data/hands/YYYYMMDD/*.jsonl`。圧縮は `*.jsonl.zst` を併用可能。

## Consequences
- 取り込みと再生が容易
- 末尾途中行の破損を検出しやすい
- スキーマ進化はバージョンフィールドで管理する
