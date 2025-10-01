//! # テストヘルパー概要
//!
//! - `assertions` モジュール: `PokerAssertions` トレイトと `asserter()` を提供し、
//!   CLI 出力や JSONL の妥当性を共通化します。
//! - `cli_runner` モジュール: `CliRunner` が `Cargo` 生成バイナリ/ライブラリを呼び出し、
//!   標準出力・標準エラー・終了コード・実行時間を取得します。
//! - `temp_files` モジュール: `TempFileManager` が競合しない一時パスを作成し、Drop 時に掃除します。
//!
//! ```rust
//! use crate::helpers::{asserter, cli_runner::CliRunner, temp_files::TempFileManager};
//!
//! let cli = CliRunner::new().expect("cli runner");
//! let tmp = TempFileManager::new().expect("temp dir");
//! let out = tmp.create_file("hands.jsonl", "{}").expect("write");
//! let res = cli.run(&["sim", "--hands", "1", "--output", out.to_string_lossy().as_ref()]);
//! assert_eq!(res.exit_code, 0);
//! asserter().assert_jsonl_format(&std::fs::read_to_string(out).unwrap());
//! ```
//!
//! 上記スニペットを雛形として、新しい統合テストでも同じユーティリティを再利用してください。
pub mod assertions;
pub mod cli_runner;
pub mod temp_files;
