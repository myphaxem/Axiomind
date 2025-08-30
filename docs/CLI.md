# CLI

バイナリ名 `axm`

## 共通オプション
- `--seed <u64>` 乱数シード 既定なし
- `--ai-version <id>` AI のモデルバージョン 既定 latest
- `--adaptive <on|off>` AI のリアルタイム適応 既定 on

## コマンド
- `play` 対戦を実行 `--vs ai|human --hands <N> --level <L>`
- `replay` ハンド履歴を再生 `--input <path> --speed <n>`
- `sim` 大量対戦シミュレーション `--hands <N> --ai <name>`
- `eval` ポリシー評価 `--ai-a <name> --ai-b <name> --hands <N>`
- `stats` JSONL から集計 `--input <file|dir>`
- `verify` ルールと保存則の検証
- `serve` ローカル UI サーバを起動 `--open --port <n>`
- `deal` 1 ハンドだけ配って表示
- `bench` 役判定や状態遷移のベンチマーク
- `rng` 乱数の検証
- `cfg` 既定設定の表示と上書き
- `doctor` 環境診断
- `export` 形式変換や抽出
- `dataset` データセット作成と分割
- `train` 学習を起動

