# jlcat Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** JSON/JSONLをテーブル表示するCLIツール。catモード（静的出力）とTUIモード（インタラクティブ）を提供。

**Architecture:** 2つの実行パスを持つ設計。(1) Streaming Pipeline: catモードで`-s`/`-r`なしの場合、ワンパス・定数メモリで処理。(2) Indexed Pipeline: TUI/ソート/再帰展開が必要な場合、インデックス構築・遅延パース・キャッシュで処理。stdinからの入力でランダムアクセスが必要な場合は自動的に一時ファイルにスプール。

**Tech Stack:** Rust, clap (CLI), serde_json (JSON), ratatui (TUI), comfy-table (cat output), memmap2 (large files), tempfile (stdin spooling)

---

## 仕様決定事項（レビュー指摘対応）

### 1. 実行パスの分離

| 条件 | 実行パス | メモリモデル |
|------|----------|--------------|
| catモード + `-s`なし + `-r`なし | Streaming | ワンパス、定数メモリ |
| catモード + `-s`あり または `-r`あり | Indexed | 全行インデックス化 |
| TUIモード（`-i`） | Indexed | 全行インデックス化 |

### 2. 入力形式サポート

**Peek-based検出（先頭バイトのみ）:**
- 最初の非空白文字が `[` → JSON配列としてストリームパース
- 最初の非空白文字が `{` → JSONLとして処理（単一オブジェクトもJSONL 1行として正しく処理される）

| 形式 | 検出方法 | 処理 |
|------|----------|------|
| JSON配列 | 先頭が `[` | ストリームパースで各要素を行として扱う |
| JSONL / 単一オブジェクト | 先頭が `{` | 行単位でパース（単一オブジェクトは1行JSONLとして扱う） |

> **設計判断**: Pretty-printされた単一JSONオブジェクトを誤検出するリスクを避けるため、`{`で始まる入力は全てJSONLとして扱う。2行目以降でパースエラーになった場合は`--lenient`でスキップするか、`--strict`でエラー終了。

**stdin処理:**
- Streamingパス: そのままストリーム処理
- Indexedパス: 自動的に一時ファイル（`/tmp/jlcat-XXXXXX`）にスプール

### 3. スキーマ推論

- **方式**: インクリメンタルユニオン（全行スキャンで新カラム発見時に追加）
- **カラム順序**: first-seen順（`-c`指定時はその順序を優先）
- **型追跡**: 各カラムごとに `null/number/bool/string/object/array` の出現カウント
- **Streamingパスでの制約**: 最初の行でスキーマ確定、後続行の新カラムは無視

> **注意（ヘルプにも記載）**: Streamingモード（`-s`/`-r`/`-i`なし）では、最初の行で検出されたカラムのみ表示されます。後続行に新しいカラムがあっても無視されます。全カラムを表示するには`-i`（TUIモード）を使用するか、`-c`でカラムを明示的に指定してください。

### 4. ソート仕様

```
-s <KEY>[,<KEY>...]
  KEY = [-]<column_path>
  - プレフィックスで降順、なしで昇順
  例: -s name,-age,address.city
```

| 項目 | 仕様 |
|------|------|
| 安定性 | 安定ソート（同値の相対順序を保持） |
| null順序 | nulls last（昇順でも降順でも末尾） |
| 欠損フィールド | nullとして扱う |
| 型混在 | 数値 < 文字列 < bool < null（型で先にグループ化） |
| 文字列比較 | バイト単位（ロケール非依存、高速） |

### 5. フィルター構文

```
# 全文検索（TUI: / キー）
/alice              # 全カラムから "alice" を含む行（大文字小文字区別なし）

# カラムフィルター（TUI: f キー）
status=active                    # 完全一致
status="value with spaces"       # スペース含む値はクォート
status='value,with,commas'       # カンマ含む値
age>30                           # 数値比較（>, <, >=, <=）
name~alice                       # 部分一致（contains）
name!~bob                        # 部分不一致
status!=inactive                 # 不一致

# 複数条件（AND）
status=active age>30             # スペース区切りでAND
```

| 演算子 | 意味 |
|--------|------|
| `=` | 完全一致 |
| `!=` | 不一致 |
| `>`, `<`, `>=`, `<=` | 数値比較 |
| `~` | 部分一致（contains） |
| `!~` | 部分不一致 |

### 6. ネスト展開（-r）仕様

**ドット記法:**
- `address.city` → ネストオブジェクトのフィールド
- `orders[0].item` → 配列の特定インデックス
- `orders[].item` → 配列を展開して複数行に（親行が複製される）

**catモード出力順序:**
1. 親テーブルを全行出力
2. 子テーブルを種類ごとに出力（外部キー付き）
3. 孫テーブル以降も同様

**TUIモードナビゲーション:**
- スタックベース（親→子→孫と潜り、Backspaceで戻る）
- 最大深度制限なし（JSONは非循環なので安全）

### 7. エラーハンドリング

```
--strict    # 不正なJSON行でエラー終了（デフォルト）
--lenient   # 不正なJSON行をスキップして警告表示
```

- 警告形式: `jlcat: warning: line 42: invalid JSON, skipping`
- 行番号は1-indexed

### 8. TUIキャンセル

- `Esc`: 現在の操作をキャンセル（検索中→キャンセル、フィルター入力→クリア）
- バックグラウンドスキャン中は進捗表示 `[Scanning... 45%] Press Esc to cancel`

### 9. ターミナル復旧

- `panic::set_hook` でパニック時もターミナル状態を復旧
- `Drop` トレイトでRAII的にクリーンアップ

### 10. カラム幅・truncation

- 最大幅: ターミナル幅に応じて自動計算
- 長い値: `...` で省略（末尾3文字を `...` に置換）
- TUIでは選択行の値をフルテキストでフッターに表示

---

## CLIインターフェース（確定版）

```
jlcat [OPTIONS] [FILE]

引数:
  [FILE]  JSONまたはJSONLファイル（省略時はstdinから読み込み）

オプション:
  -i, --interactive     TUIモードで起動（デフォルトはcatモード）
  -r, --recursive       ネスト構造を再帰的に子テーブルとして展開
  -c, --columns <COLS>  表示カラムを指定（カンマ区切り、ドット記法対応）
                        例: -c id,name,address.city
  -s, --sort <KEYS>     ソートキー指定（カンマ区切り、-で降順）
                        例: -s name,-age
  --style <STYLE>       テーブルスタイル [ascii|rounded|markdown|plain]
                        デフォルト: rounded
  --strict              不正なJSON行でエラー終了（デフォルト）
  --lenient             不正なJSON行をスキップ
  -h, --help            ヘルプ表示
  -V, --version         バージョン表示
```

---

## ディレクトリ構成

```
jlcat/
├── Cargo.toml
├── src/
│   ├── main.rs              # エントリポイント
│   ├── cli.rs               # clap定義
│   ├── error.rs             # エラー型定義
│   ├── input/
│   │   ├── mod.rs
│   │   ├── source.rs        # InputSource trait + File/Stdin実装
│   │   ├── detector.rs      # JSON/JSONL形式検出
│   │   ├── parser.rs        # StreamingParser / IndexedParser
│   │   └── spooler.rs       # stdin → tempfile スプール
│   ├── core/
│   │   ├── mod.rs
│   │   ├── schema.rs        # Schema, ColumnType, SchemaInferrer
│   │   ├── path.rs          # CompiledPath（事前パースされたアクセスパス）
│   │   ├── value.rs         # SortableValue（ソート用Ord実装）
│   │   ├── selector.rs      # ColumnSelector（ドット記法パース）
│   │   ├── extractor.rs     # NestedExtractor（-r処理）
│   │   ├── sorter.rs        # Sorter（安定ソート、nulls last）
│   │   ├── filter.rs        # FilterExpr, FilterParser
│   │   └── table.rs         # TableData, Row
│   ├── pipeline/
│   │   ├── mod.rs
│   │   ├── streaming.rs     # StreamingPipeline（ワンパス）
│   │   └── indexed.rs       # IndexedPipeline（インデックス構築）
│   └── render/
│       ├── mod.rs
│       ├── cat.rs           # CatRenderer (comfy-table)
│       └── tui/
│           ├── mod.rs
│           ├── app.rs       # App状態管理
│           ├── view.rs      # 描画ロジック
│           ├── scroll.rs    # VirtualScroll
│           ├── search.rs    # SearchEngine
│           └── input.rs     # InputHandler
└── tests/
    ├── cli_tests.rs         # CLIインテグレーション
    ├── parser_tests.rs      # パーサーユニット
    ├── filter_tests.rs      # フィルター構文
    ├── sort_tests.rs        # ソートロジック
    └── fixtures/
        ├── simple.jsonl
        ├── nested.jsonl
        ├── large.jsonl       # 10000行
        └── malformed.jsonl   # エラーケース
```

---

## 依存クレート

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
ratatui = "0.29"
crossterm = "0.28"
comfy-table = "7"
memmap2 = "0.9"
tempfile = "3"
thiserror = "2"
unicode-width = "0.2"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
proptest = "1"
```

---

## 実装タスク

### Phase 1: プロジェクト基盤

#### Task 1.1: プロジェクト初期化

**Files:**
- Create: `Cargo.toml`
- Create: `src/main.rs`
- Create: `src/error.rs`

**Step 1: Cargo.toml作成**

```toml
[package]
name = "jlcat"
version = "0.1.0"
edition = "2021"
description = "JSON/JSONL table viewer with TUI support"
license = "MIT"

[dependencies]
clap = { version = "4", features = ["derive"] }
serde = { version = "1", features = ["derive"] }
serde_json = "1"
thiserror = "2"

[dev-dependencies]
assert_cmd = "2"
predicates = "3"
```

**Step 2: エラー型定義**

```rust
// src/error.rs
use thiserror::Error;

#[derive(Error, Debug)]
pub enum JlcatError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON parse error at line {line}: {message}")]
    JsonParse { line: usize, message: String },

    #[error("Invalid column path: {0}")]
    InvalidColumnPath(String),

    #[error("Invalid filter expression: {0}")]
    InvalidFilter(String),

    #[error("Invalid sort key: {0}")]
    InvalidSortKey(String),
}

pub type Result<T> = std::result::Result<T, JlcatError>;
```

**Step 3: main.rs スケルトン**

```rust
// src/main.rs
mod error;

use error::Result;

fn main() -> Result<()> {
    println!("jlcat v0.1.0");
    Ok(())
}
```

**Step 4: ビルド確認**

Run: `cargo build`
Expected: Compiles successfully

**Step 5: コミット**

```bash
git add Cargo.toml src/
git commit -m "feat: initialize project with error types"
```

---

#### Task 1.2: CLI引数パース

**Files:**
- Create: `src/cli.rs`
- Modify: `src/main.rs`
- Create: `tests/cli_tests.rs`

**Step 1: 失敗するテスト作成**

```rust
// tests/cli_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON/JSONL table viewer"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains("0.1.0"));
}
```

**Step 2: テスト実行（失敗確認）**

Run: `cargo test test_help_flag`
Expected: FAIL (--help not implemented)

**Step 3: CLI定義実装**

```rust
// src/cli.rs
use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jlcat")]
#[command(about = "JSON/JSONL table viewer with TUI support")]
#[command(version)]
pub struct Cli {
    /// JSON or JSONL file path (reads from stdin if omitted)
    pub file: Option<PathBuf>,

    /// Launch in interactive TUI mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Recursively expand nested structures as child tables
    #[arg(short, long)]
    pub recursive: bool,

    /// Columns to display (comma-separated, supports dot notation)
    #[arg(short, long, value_delimiter = ',')]
    pub columns: Option<Vec<String>>,

    /// Sort keys (comma-separated, prefix with - for descending)
    #[arg(short, long, value_delimiter = ',')]
    pub sort: Option<Vec<String>>,

    /// Table style
    #[arg(long, value_enum, default_value = "rounded")]
    pub style: TableStyle,

    /// Exit on invalid JSON line (default)
    #[arg(long, default_value = "true")]
    pub strict: bool,

    /// Skip invalid JSON lines with warning
    #[arg(long)]
    pub lenient: bool,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum TableStyle {
    Ascii,
    #[default]
    Rounded,
    Markdown,
    Plain,
}

impl Cli {
    pub fn is_strict(&self) -> bool {
        !self.lenient
    }
}
```

**Step 4: main.rsでCLI使用**

```rust
// src/main.rs
mod cli;
mod error;

use clap::Parser;
use cli::Cli;
use error::Result;

fn main() -> Result<()> {
    let cli = Cli::parse();

    if cli.file.is_none() && atty::is(atty::Stream::Stdin) {
        eprintln!("Usage: jlcat [OPTIONS] [FILE]");
        eprintln!("Try 'jlcat --help' for more information.");
        std::process::exit(1);
    }

    println!("{:?}", cli);
    Ok(())
}
```

**Step 5: Cargo.tomlにatty追加**

```toml
[dependencies]
# ... existing deps ...
atty = "0.2"
```

**Step 6: テスト実行（成功確認）**

Run: `cargo test test_help_flag test_version_flag`
Expected: PASS

**Step 7: コミット**

```bash
git add src/cli.rs src/main.rs tests/cli_tests.rs Cargo.toml
git commit -m "feat: add CLI argument parsing with clap"
```

---

### Phase 2: 入力レイヤー

#### Task 2.1: 入力ソース抽象化

**Files:**
- Create: `src/input/mod.rs`
- Create: `src/input/source.rs`
- Create: `tests/fixtures/simple.jsonl`

**Step 1: テスト用フィクスチャ作成**

```jsonl
{"id": 1, "name": "Alice", "age": 30}
{"id": 2, "name": "Bob", "age": 25}
{"id": 3, "name": "Charlie", "age": 35}
```

Save to: `tests/fixtures/simple.jsonl`

**Step 2: 失敗するテスト作成**

```rust
// src/input/source.rs の末尾に追加（後でテストモジュールとして）
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_file_source_read_lines() {
        let source = FileSource::open("tests/fixtures/simple.jsonl").unwrap();
        let lines: Vec<String> = source.lines().map(|l| l.unwrap()).collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Alice"));
    }

    #[test]
    fn test_stdin_source_read_lines() {
        let data = r#"{"id": 1}
{"id": 2}"#;
        let cursor = Cursor::new(data);
        let source = StdinSource::from_reader(cursor);
        let lines: Vec<String> = source.lines().map(|l| l.unwrap()).collect();
        assert_eq!(lines.len(), 2);
    }
}
```

**Step 3: InputSource実装**

```rust
// src/input/source.rs
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

pub trait InputSource {
    type Lines: Iterator<Item = io::Result<String>>;
    fn lines(self) -> Self::Lines;
}

pub struct FileSource {
    reader: BufReader<File>,
}

impl FileSource {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }
}

impl InputSource for FileSource {
    type Lines = io::Lines<BufReader<File>>;

    fn lines(self) -> Self::Lines {
        self.reader.lines()
    }
}

pub struct StdinSource<R: Read> {
    reader: BufReader<R>,
}

impl StdinSource<io::Stdin> {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
        }
    }
}

impl<R: Read> StdinSource<R> {
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }
}

impl<R: Read> InputSource for StdinSource<R> {
    type Lines = io::Lines<BufReader<R>>;

    fn lines(self) -> Self::Lines {
        self.reader.lines()
    }
}
```

**Step 4: mod.rs作成**

```rust
// src/input/mod.rs
mod source;

pub use source::{FileSource, InputSource, StdinSource};
```

**Step 5: テスト実行**

Run: `cargo test test_file_source test_stdin_source`
Expected: PASS

**Step 6: コミット**

```bash
git add src/input/ tests/fixtures/simple.jsonl
git commit -m "feat: add InputSource trait with File and Stdin implementations"
```

---

#### Task 2.2: Input Format Sniffing

> **設計変更**: 複雑でエラーを起こしやすい`detect_format`を廃止。代わりに、入力ストリームの最初の数バイトを「嗅ぐ（sniffing）」だけの軽量な戦略に切り替える。`{`で始まる入力はすべてJSONLとして扱い、パース戦略を単純化する。

**Files:**
- Create: `src/input/detector.rs`
- Modify: `src/input/mod.rs`

**Step 1: 失敗するテスト作成**

```rust
// src/input/detector.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniff_json_lines() {
        let input = b"  {\"id\": 1}\n";
        assert_eq!(sniff_format(input), Some(InputFormat::JsonLines));
    }

    #[test]
    fn test_sniff_json_array() {
        let input = b"   [{\"id\": 1}]";
        assert_eq!(sniff_format(input), Some(InputFormat::JsonArray));
    }

    #[test]
    fn test_sniff_empty_input() {
        let input = b"   ";
        assert_eq!(sniff_format(input), None);
    }

    #[test]
    fn test_sniff_invalid_input() {
        let input = b"not json";
        assert_eq!(sniff_format(input), None);
    }
}
```

**Step 2: 形式Sniffing実装**

```rust
// src/input/detector.rs
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    JsonLines,
    JsonArray,
}

/// Detects the likely input format based on the first few non-whitespace bytes.
/// This is a lightweight "sniffing" operation and does not perform full validation.
/// Returns None if the input is empty or doesn't start with a valid JSON character.
pub fn sniff_format(peek: &[u8]) -> Option<InputFormat> {
    if let Some(first_char) = peek.iter().find(|c| !c.is_ascii_whitespace()) {
        match first_char {
            b'[' => Some(InputFormat::JsonArray),
            b'{' => Some(InputFormat::JsonLines), // Assume JSONL for any object start
            _ => None,
        }
    } else {
        None // Empty or whitespace-only input
    }
}
```

**Step 3: mod.rs更新**

```rust
// src/input/mod.rs
mod detector;
mod source;

pub use detector::{sniff_format, InputFormat};
pub use source::{FileSource, InputSource, StdinSource};
```

**Step 4: テスト実行**

Run: `cargo test detector::tests`
Expected: PASS

**Step 5: コミット**

```bash
git add src/input/detector.rs src/input/mod.rs
git commit -m "refactor: simplify input detection to lightweight sniffing"
```

---

#### Task 2.3: stdinスプーラー

**Files:**
- Create: `src/input/spooler.rs`
- Modify: `src/input/mod.rs`
- Modify: `Cargo.toml`

**Step 1: Cargo.tomlにtempfile追加**

```toml
[dependencies]
# ... existing ...
tempfile = "3"
```

**Step 2: 失敗するテスト作成**

```rust
// src/input/spooler.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_spool_stdin_to_tempfile() {
        let input = b"{\"id\": 1}\n{\"id\": 2}\n{\"id\": 3}\n";
        let cursor = Cursor::new(input.to_vec());

        let spooled = SpooledInput::from_reader(cursor).unwrap();

        // Should be able to read multiple times
        let content1 = std::fs::read_to_string(spooled.path()).unwrap();
        let content2 = std::fs::read_to_string(spooled.path()).unwrap();

        assert_eq!(content1, content2);
        assert!(content1.contains("\"id\": 1"));
        assert!(content1.contains("\"id\": 3"));
    }
}
```

**Step 3: スプーラー実装**

```rust
// src/input/spooler.rs
use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::{Path, PathBuf};
use tempfile::NamedTempFile;

pub struct SpooledInput {
    temp_file: NamedTempFile,
}

impl SpooledInput {
    pub fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut temp_file = NamedTempFile::new()?;
        io::copy(&mut reader, &mut temp_file)?;
        temp_file.flush()?;
        Ok(Self { temp_file })
    }

    pub fn path(&self) -> &Path {
        self.temp_file.path()
    }

    pub fn into_reader(self) -> io::Result<BufReader<File>> {
        let file = File::open(self.temp_file.path())?;
        Ok(BufReader::new(file))
    }

    pub fn reader(&self) -> io::Result<BufReader<File>> {
        let file = File::open(self.temp_file.path())?;
        Ok(BufReader::new(file))
    }
}
```

**Step 4: mod.rs更新**

```rust
// src/input/mod.rs
mod detector;
mod source;
mod spooler;

pub use detector::{detect_format, detect_format_from_peek, InputFormat};
pub use source::{FileSource, InputSource, StdinSource};
pub use spooler::SpooledInput;
```

**Step 5: テスト実行**

Run: `cargo test spooler::tests`
Expected: PASS

**Step 6: コミット**

```bash
git add src/input/spooler.rs src/input/mod.rs Cargo.toml
git commit -m "feat: add stdin spooler for indexed pipeline"
```

---

### Phase 3: コアエンジン

#### Task 3.1: スキーマ推論

**Files:**
- Create: `src/core/mod.rs`
- Create: `src/core/schema.rs`

**Step 1: 失敗するテスト作成**

```rust
// src/core/schema.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_infer_schema_simple() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob", "age": 30}),
        ];

        let schema = SchemaInferrer::infer(&rows);

        assert_eq!(schema.columns().len(), 3);
        assert_eq!(schema.columns()[0], "id");
        assert_eq!(schema.columns()[1], "name");
        assert_eq!(schema.columns()[2], "age");
    }

    #[test]
    fn test_infer_schema_nested() {
        let rows = vec![
            json!({"id": 1, "address": {"city": "Tokyo"}}),
        ];

        let schema = SchemaInferrer::infer(&rows);

        assert_eq!(schema.columns().len(), 2);
        assert!(schema.has_nested("address"));
    }

    #[test]
    fn test_column_types() {
        let rows = vec![
            json!({"id": 1, "name": "Alice", "active": true}),
            json!({"id": 2, "name": null, "active": false}),
        ];

        let schema = SchemaInferrer::infer(&rows);

        assert_eq!(schema.column_type("id"), Some(ColumnType::Number));
        assert_eq!(schema.column_type("name"), Some(ColumnType::Mixed));
        assert_eq!(schema.column_type("active"), Some(ColumnType::Bool));
    }
}
```

**Step 2: スキーマ実装**

```rust
// src/core/schema.rs
use serde_json::Value;
use std::collections::{HashMap, HashSet};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ColumnType {
    Null,
    Bool,
    Number,
    String,
    Array,
    Object,
    Mixed,
}

impl ColumnType {
    fn from_value(value: &Value) -> Self {
        match value {
            Value::Null => ColumnType::Null,
            Value::Bool(_) => ColumnType::Bool,
            Value::Number(_) => ColumnType::Number,
            Value::String(_) => ColumnType::String,
            Value::Array(_) => ColumnType::Array,
            Value::Object(_) => ColumnType::Object,
        }
    }

    fn merge(self, other: Self) -> Self {
        if self == other {
            self
        } else if self == ColumnType::Null {
            other
        } else if other == ColumnType::Null {
            self
        } else {
            ColumnType::Mixed
        }
    }
}

#[derive(Debug, Clone)]
pub struct Schema {
    columns: Vec<String>,
    types: HashMap<String, ColumnType>,
    nested: HashSet<String>,
}

impl Schema {
    pub fn new() -> Self {
        Self {
            columns: Vec::new(),
            types: HashMap::new(),
            nested: HashSet::new(),
        }
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn column_type(&self, name: &str) -> Option<ColumnType> {
        self.types.get(name).copied()
    }

    pub fn has_nested(&self, name: &str) -> bool {
        self.nested.contains(name)
    }

    fn add_column(&mut self, name: String, col_type: ColumnType) {
        if let Some(existing) = self.types.get_mut(&name) {
            *existing = existing.merge(col_type);
        } else {
            self.columns.push(name.clone());
            self.types.insert(name.clone(), col_type);
        }

        if col_type == ColumnType::Object || col_type == ColumnType::Array {
            self.nested.insert(name);
        }
    }
}

impl Default for Schema {
    fn default() -> Self {
        Self::new()
    }
}

pub struct SchemaInferrer;

impl SchemaInferrer {
    pub fn infer(rows: &[Value]) -> Schema {
        let mut schema = Schema::new();

        for row in rows {
            if let Value::Object(obj) = row {
                for (key, value) in obj {
                    let col_type = ColumnType::from_value(value);
                    schema.add_column(key.clone(), col_type);
                }
            }
        }

        schema
    }

    pub fn infer_streaming(row: &Value, schema: &mut Schema) {
        if let Value::Object(obj) = row {
            for (key, value) in obj {
                let col_type = ColumnType::from_value(value);
                schema.add_column(key.clone(), col_type);
            }
        }
    }
}
```

**Step 3: mod.rs作成**

```rust
// src/core/mod.rs
mod schema;

pub use schema::{ColumnType, Schema, SchemaInferrer};
```

**Step 4: main.rsにmod追加**

```rust
// src/main.rs 冒頭に追加
mod core;
```

**Step 5: テスト実行**

Run: `cargo test schema::tests`
Expected: PASS

**Step 6: コミット**

```bash
git add src/core/ src/main.rs
git commit -m "feat: add schema inference with type tracking"
```

---

#### Task 3.2: CompiledPath + SortableValue（パフォーマンス最適化）

> **設計判断**: ドット記法パス（例: `address.city`）を毎行で文字列分割するのは非効率。構築時に事前パースして`Vec<PathSegment>`に変換し、ループ内では分割処理をスキップ。

**Files:**
- Create: `src/core/path.rs`
- Create: `src/core/value.rs`
- Modify: `src/core/mod.rs`

**Step 1: 失敗するテスト作成（CompiledPath）**

```rust
// src/core/path.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_compile_simple_path() {
        let path = CompiledPath::compile("name").unwrap();
        assert_eq!(path.segments, vec![PathSegment::Key("name".into())]);
    }

    #[test]
    fn test_compile_nested_path() {
        let path = CompiledPath::compile("address.city").unwrap();
        assert_eq!(path.segments, vec![
            PathSegment::Key("address".into()),
            PathSegment::Key("city".into()),
        ]);
    }

    #[test]
    fn test_compile_array_index() {
        let path = CompiledPath::compile("orders[0].item").unwrap();
        assert_eq!(path.segments, vec![
            PathSegment::Key("orders".into()),
            PathSegment::Index(0),
            PathSegment::Key("item".into()),
        ]);
    }

    #[test]
    fn test_get_value() {
        let path = CompiledPath::compile("address.city").unwrap();
        let row = json!({"address": {"city": "Tokyo"}});
        assert_eq!(path.get(&row), Some(&json!("Tokyo")));
    }

    #[test]
    fn test_get_array_value() {
        let path = CompiledPath::compile("items[1].name").unwrap();
        let row = json!({"items": [{"name": "A"}, {"name": "B"}]});
        assert_eq!(path.get(&row), Some(&json!("B")));
    }
}
```

**Step 2: CompiledPath実装**

```rust
// src/core/path.rs
use crate::error::{JlcatError, Result};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct CompiledPath {
    pub segments: Vec<PathSegment>,
    pub original: String,
}

impl CompiledPath {
    pub fn compile(path: &str) -> Result<Self> {
        let mut segments = Vec::new();

        for part in path.split('.') {
            if part.is_empty() {
                continue;
            }

            // Handle array index notation: field[0] or [0]
            if let Some(idx_start) = part.find('[') {
                let field = &part[..idx_start];
                if !field.is_empty() {
                    segments.push(PathSegment::Key(field.to_string()));
                }

                let idx_end = part.find(']').ok_or_else(|| {
                    JlcatError::InvalidColumnPath(format!("unclosed bracket in '{}'", path))
                })?;

                let idx: usize = part[idx_start + 1..idx_end].parse().map_err(|_| {
                    JlcatError::InvalidColumnPath(format!("invalid index in '{}'", path))
                })?;

                segments.push(PathSegment::Index(idx));

                // Handle trailing parts after ]
                let rest = &part[idx_end + 1..];
                if !rest.is_empty() && rest.starts_with('.') {
                    // Will be handled by next iteration
                }
            } else {
                segments.push(PathSegment::Key(part.to_string()));
            }
        }

        Ok(Self {
            segments,
            original: path.to_string(),
        })
    }

    pub fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        let mut current = value;

        for segment in &self.segments {
            current = match segment {
                PathSegment::Key(key) => current.get(key)?,
                PathSegment::Index(idx) => current.get(idx)?,
            };
        }

        Some(current)
    }
}
```

**Step 3: 失敗するテスト作成（SortableValue）**

```rust
// src/core/value.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ordering_numbers() {
        let v1 = SortableValue::new(&json!(1));
        let v2 = SortableValue::new(&json!(2));
        let v3 = SortableValue::new(&json!(10));

        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_ordering_strings() {
        let v1 = SortableValue::new(&json!("alice"));
        let v2 = SortableValue::new(&json!("bob"));

        assert!(v1 < v2);
    }

    #[test]
    fn test_ordering_null_last() {
        let v1 = SortableValue::new(&json!(1));
        let v2 = SortableValue::new(&json!(null));
        let v3 = SortableValue::new(&json!("z"));

        assert!(v1 < v2);
        assert!(v3 < v2);
    }

    #[test]
    fn test_ordering_mixed_types() {
        // numbers < strings < bools < null
        let num = SortableValue::new(&json!(100));
        let str = SortableValue::new(&json!("a"));
        let bool_val = SortableValue::new(&json!(true));
        let null_val = SortableValue::new(&json!(null));

        assert!(num < str);
        assert!(str < bool_val);
        assert!(bool_val < null_val);
    }
}
```

**Step 4: SortableValue実装**

```rust
// src/core/value.rs
use serde_json::Value;
use std::cmp::Ordering;

/// Wrapper for JSON values that implements Ord for sorting.
/// Ordering: numbers < strings < bools < null
/// Nulls are always last (both ascending and descending).
#[derive(Debug, Clone)]
pub struct SortableValue<'a> {
    value: &'a Value,
}

impl<'a> SortableValue<'a> {
    pub fn new(value: &'a Value) -> Self {
        Self { value }
    }

    fn type_order(&self) -> u8 {
        match self.value {
            Value::Number(_) => 0,
            Value::String(_) => 1,
            Value::Bool(_) => 2,
            Value::Array(_) => 3,
            Value::Object(_) => 4,
            Value::Null => 5, // Always last
        }
    }
}

impl<'a> PartialEq for SortableValue<'a> {
    fn eq(&self, other: &Self) -> bool {
        self.cmp(other) == Ordering::Equal
    }
}

impl<'a> Eq for SortableValue<'a> {}

impl<'a> PartialOrd for SortableValue<'a> {
    fn partial_cmp(&self, other: &Self) -> Option<Ordering> {
        Some(self.cmp(other))
    }
}

impl<'a> Ord for SortableValue<'a> {
    fn cmp(&self, other: &Self) -> Ordering {
        let type_ord = self.type_order().cmp(&other.type_order());
        if type_ord != Ordering::Equal {
            return type_ord;
        }

        match (self.value, other.value) {
            (Value::Number(a), Value::Number(b)) => {
                let a_f = a.as_f64().unwrap_or(f64::NAN);
                let b_f = b.as_f64().unwrap_or(f64::NAN);
                a_f.partial_cmp(&b_f).unwrap_or(Ordering::Equal)
            }
            (Value::String(a), Value::String(b)) => a.cmp(b),
            (Value::Bool(a), Value::Bool(b)) => a.cmp(b),
            _ => Ordering::Equal,
        }
    }
}
```

**Step 5: mod.rs更新**

```rust
// src/core/mod.rs
mod path;
mod schema;
mod value;

pub use path::{CompiledPath, PathSegment};
pub use schema::{ColumnType, Schema, SchemaInferrer};
pub use value::SortableValue;
```

**Step 6: テスト実行**

Run: `cargo test path::tests value::tests`
Expected: PASS

**Step 7: コミット**

```bash
git add src/core/path.rs src/core/value.rs src/core/mod.rs
git commit -m "feat: add CompiledPath and SortableValue for optimized access"
```

---

#### Task 3.3: カラムセレクター

> **パフォーマンス**: CompiledPathを使用してパスを事前コンパイル。selectループ内での文字列分割を回避。

**Files:**
- Create: `src/core/selector.rs`
- Modify: `src/core/mod.rs`

**Step 1: 失敗するテスト作成**

```rust
// src/core/selector.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_select_simple_columns() {
        let selector = ColumnSelector::new(vec!["id".into(), "name".into()]).unwrap();
        let row = json!({"id": 1, "name": "Alice", "age": 30});

        let selected = selector.select(&row);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], ("id".to_string(), json!(1)));
        assert_eq!(selected[1], ("name".to_string(), json!("Alice")));
    }

    #[test]
    fn test_select_nested_columns() {
        let selector = ColumnSelector::new(vec!["id".into(), "address.city".into()]).unwrap();
        let row = json!({"id": 1, "address": {"city": "Tokyo", "zip": "100"}});

        let selected = selector.select(&row);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[0], ("id".to_string(), json!(1)));
        assert_eq!(selected[1], ("address.city".to_string(), json!("Tokyo")));
    }

    #[test]
    fn test_select_missing_column() {
        let selector = ColumnSelector::new(vec!["id".into(), "missing".into()]).unwrap();
        let row = json!({"id": 1});

        let selected = selector.select(&row);

        assert_eq!(selected.len(), 2);
        assert_eq!(selected[1], ("missing".to_string(), Value::Null));
    }
}
```

**Step 2: ColumnSelector実装（CompiledPath使用）**

```rust
// src/core/selector.rs
use super::path::CompiledPath;
use crate::error::Result;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ColumnSelector {
    columns: Vec<(String, CompiledPath)>,  // (original_name, compiled_path)
}

impl ColumnSelector {
    pub fn new(columns: Vec<String>) -> Result<Self> {
        let compiled: Result<Vec<_>> = columns
            .into_iter()
            .map(|col| {
                let path = CompiledPath::compile(&col)?;
                Ok((col, path))
            })
            .collect();

        Ok(Self { columns: compiled? })
    }

    pub fn columns(&self) -> Vec<&str> {
        self.columns.iter().map(|(name, _)| name.as_str()).collect()
    }

    pub fn select(&self, row: &Value) -> Vec<(String, Value)> {
        self.columns
            .iter()
            .map(|(name, path)| {
                let value = path.get(row).cloned().unwrap_or(Value::Null);
                (name.clone(), value)
            })
            .collect()
    }
}
```

**Step 3: mod.rs更新**

```rust
// src/core/mod.rs
mod schema;
mod selector;
mod value;

pub use schema::{ColumnType, Schema, SchemaInferrer};
pub use selector::ColumnSelector;
pub use value::{get_nested_value, SortableValue};
```

**Step 4: テスト実行**

Run: `cargo test selector::tests`
Expected: PASS

**Step 5: コミット**

```bash
git add src/core/selector.rs src/core/mod.rs
git commit -m "feat: add ColumnSelector with dot notation support"
```

---

#### Task 3.4: ソーター

> **パフォーマンス**: CompiledPathを使用してソートキーを事前コンパイル。compareループ内での文字列分割を回避。

**Files:**
- Create: `src/core/sorter.rs`
- Modify: `src/core/mod.rs`

**Step 1: 失敗するテスト作成**

```rust
// src/core/sorter.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_sort_key() {
        let key = SortKey::parse("name").unwrap();
        assert_eq!(key.path.original, "name");
        assert!(!key.descending);

        let key = SortKey::parse("-age").unwrap();
        assert_eq!(key.path.original, "age");
        assert!(key.descending);
    }

    #[test]
    fn test_sort_ascending() {
        let mut rows = vec![
            json!({"id": 3, "name": "Charlie"}),
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("name").unwrap()]);
        sorter.sort(&mut rows);

        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
        assert_eq!(rows[2]["name"], "Charlie");
    }

    #[test]
    fn test_sort_descending() {
        let mut rows = vec![
            json!({"id": 1, "age": 30}),
            json!({"id": 2, "age": 25}),
            json!({"id": 3, "age": 35}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("-age").unwrap()]);
        sorter.sort(&mut rows);

        assert_eq!(rows[0]["age"], 35);
        assert_eq!(rows[1]["age"], 30);
        assert_eq!(rows[2]["age"], 25);
    }

    #[test]
    fn test_sort_multiple_keys() {
        let mut rows = vec![
            json!({"dept": "A", "age": 30}),
            json!({"dept": "B", "age": 25}),
            json!({"dept": "A", "age": 25}),
        ];

        let sorter = Sorter::new(vec![
            SortKey::parse("dept").unwrap(),
            SortKey::parse("-age").unwrap(),
        ]);
        sorter.sort(&mut rows);

        // dept A first, then by age desc
        assert_eq!(rows[0], json!({"dept": "A", "age": 30}));
        assert_eq!(rows[1], json!({"dept": "A", "age": 25}));
        assert_eq!(rows[2], json!({"dept": "B", "age": 25}));
    }

    #[test]
    fn test_sort_nulls_last() {
        let mut rows = vec![
            json!({"id": 1, "name": null}),
            json!({"id": 2, "name": "Bob"}),
            json!({"id": 3, "name": "Alice"}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("name").unwrap()]);
        sorter.sort(&mut rows);

        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
        assert_eq!(rows[2]["name"], Value::Null);
    }
}
```

**Step 2: Sorter実装（CompiledPath使用）**

```rust
// src/core/sorter.rs
use super::path::CompiledPath;
use super::value::SortableValue;
use crate::error::{JlcatError, Result};
use serde_json::Value;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct SortKey {
    pub path: CompiledPath,  // 事前コンパイル済みパス
    pub descending: bool,
}

impl SortKey {
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(JlcatError::InvalidSortKey("empty sort key".into()));
        }

        let (descending, column) = if let Some(col) = s.strip_prefix('-') {
            (true, col)
        } else {
            (false, s)
        };

        if column.is_empty() {
            return Err(JlcatError::InvalidSortKey("empty column name".into()));
        }

        let path = CompiledPath::compile(column)?;
        Ok(Self { path, descending })
    }
}

#[derive(Debug, Clone)]
pub struct Sorter {
    keys: Vec<SortKey>,
}

impl Sorter {
    pub fn new(keys: Vec<SortKey>) -> Self {
        Self { keys }
    }

    pub fn parse(key_strs: &[String]) -> Result<Self> {
        let keys: Result<Vec<_>> = key_strs.iter().map(|s| SortKey::parse(s)).collect();
        Ok(Self::new(keys?))
    }

    pub fn sort(&self, rows: &mut [Value]) {
        rows.sort_by(|a, b| self.compare(a, b));
    }

    pub fn sort_indices(&self, rows: &[Value]) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..rows.len()).collect();
        indices.sort_by(|&i, &j| self.compare(&rows[i], &rows[j]));
        indices
    }

    fn compare(&self, a: &Value, b: &Value) -> Ordering {
        for key in &self.keys {
            // CompiledPath.get() を使用（文字列分割なし）
            let val_a = key.path.get(a);
            let val_b = key.path.get(b);

            let ord = match (val_a, val_b) {
                (Some(va), Some(vb)) => {
                    SortableValue::new(va).cmp(&SortableValue::new(vb))
                }
                (Some(_), None) => Ordering::Less,
                (None, Some(_)) => Ordering::Greater,
                (None, None) => Ordering::Equal,
            };

            let ord = if key.descending { ord.reverse() } else { ord };

            // For nulls-last in descending order, we need special handling
            if key.descending {
                let a_is_null = val_a.map_or(true, |v| v.is_null());
                let b_is_null = val_b.map_or(true, |v| v.is_null());
                if a_is_null && !b_is_null {
                    return Ordering::Greater; // null goes last
                }
                if !a_is_null && b_is_null {
                    return Ordering::Less; // non-null goes first
                }
            }

            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    }
}
```

**Step 3: mod.rs更新**

```rust
// src/core/mod.rs
mod schema;
mod selector;
mod sorter;
mod value;

pub use schema::{ColumnType, Schema, SchemaInferrer};
pub use selector::ColumnSelector;
pub use sorter::{SortKey, Sorter};
pub use value::{get_nested_value, SortableValue};
```

**Step 4: テスト実行**

Run: `cargo test sorter::tests`
Expected: PASS

**Step 5: コミット**

```bash
git add src/core/sorter.rs src/core/mod.rs
git commit -m "feat: add Sorter with multi-key and nulls-last support"
```

---

#### Task 3.5: フィルターパーサー

> **将来の拡張**: 現在の手書きパーサーはv1の構文には十分。OR演算子や括弧によるグルーピングなど複雑な構文を追加する場合は、`nom`や`chumsky`などのパーサーコンビネータへの移行を検討。

**Files:**
- Create: `src/core/filter.rs`
- Modify: `src/core/mod.rs`

**Step 1: 失敗するテスト作成**

```rust
// src/core/filter.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_equals() {
        let expr = FilterExpr::parse("status=active").unwrap();
        assert_eq!(expr.conditions.len(), 1);
        assert_eq!(expr.conditions[0].column, "status");
        assert_eq!(expr.conditions[0].op, FilterOp::Eq);
        assert_eq!(expr.conditions[0].value, "active");
    }

    #[test]
    fn test_parse_quoted_value() {
        let expr = FilterExpr::parse(r#"name="John Doe""#).unwrap();
        assert_eq!(expr.conditions[0].value, "John Doe");
    }

    #[test]
    fn test_parse_multiple_conditions() {
        let expr = FilterExpr::parse("status=active age>30").unwrap();
        assert_eq!(expr.conditions.len(), 2);
    }

    #[test]
    fn test_parse_operators() {
        assert_eq!(FilterExpr::parse("age>30").unwrap().conditions[0].op, FilterOp::Gt);
        assert_eq!(FilterExpr::parse("age>=30").unwrap().conditions[0].op, FilterOp::Gte);
        assert_eq!(FilterExpr::parse("age<30").unwrap().conditions[0].op, FilterOp::Lt);
        assert_eq!(FilterExpr::parse("age<=30").unwrap().conditions[0].op, FilterOp::Lte);
        assert_eq!(FilterExpr::parse("name~alice").unwrap().conditions[0].op, FilterOp::Contains);
        assert_eq!(FilterExpr::parse("name!~bob").unwrap().conditions[0].op, FilterOp::NotContains);
        assert_eq!(FilterExpr::parse("status!=inactive").unwrap().conditions[0].op, FilterOp::Ne);
    }

    #[test]
    fn test_filter_matches() {
        let expr = FilterExpr::parse("status=active age>25").unwrap();

        assert!(expr.matches(&json!({"status": "active", "age": 30})));
        assert!(!expr.matches(&json!({"status": "active", "age": 20})));
        assert!(!expr.matches(&json!({"status": "inactive", "age": 30})));
    }

    #[test]
    fn test_filter_contains() {
        let expr = FilterExpr::parse("name~alice").unwrap();

        assert!(expr.matches(&json!({"name": "alice smith"})));
        assert!(expr.matches(&json!({"name": "Alice"})));  // case insensitive
        assert!(!expr.matches(&json!({"name": "bob"})));
    }

    #[test]
    fn test_fulltext_search() {
        let search = FullTextSearch::new("alice");

        assert!(search.matches(&json!({"name": "Alice", "role": "admin"})));
        assert!(search.matches(&json!({"desc": "User alice@example.com"})));
        assert!(!search.matches(&json!({"name": "Bob"})));
    }
}
```

**Step 2: Filter実装（CompiledPath使用）**

```rust
// src/core/filter.rs
use super::path::CompiledPath;
use crate::error::{JlcatError, Result};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOp {
    Eq,         // =
    Ne,         // !=
    Gt,         // >
    Gte,        // >=
    Lt,         // <
    Lte,        // <=
    Contains,   // ~
    NotContains, // !~
}

#[derive(Debug, Clone)]
pub struct FilterCondition {
    pub path: CompiledPath,  // 事前コンパイル済みパス
    pub op: FilterOp,
    pub value: String,
}

impl FilterCondition {
    fn matches(&self, row: &Value) -> bool {
        let row_value = self.path.get(row);  // CompiledPath.get() を使用

        match &self.op {
            FilterOp::Eq => self.matches_eq(row_value),
            FilterOp::Ne => !self.matches_eq(row_value),
            FilterOp::Gt => self.matches_cmp(row_value, |ord| ord == std::cmp::Ordering::Greater),
            FilterOp::Gte => self.matches_cmp(row_value, |ord| ord != std::cmp::Ordering::Less),
            FilterOp::Lt => self.matches_cmp(row_value, |ord| ord == std::cmp::Ordering::Less),
            FilterOp::Lte => self.matches_cmp(row_value, |ord| ord != std::cmp::Ordering::Greater),
            FilterOp::Contains => self.matches_contains(row_value, false),
            FilterOp::NotContains => !self.matches_contains(row_value, false),
        }
    }

    fn matches_eq(&self, row_value: Option<&Value>) -> bool {
        match row_value {
            Some(Value::String(s)) => s == &self.value,
            Some(Value::Number(n)) => n.to_string() == self.value,
            Some(Value::Bool(b)) => b.to_string() == self.value,
            Some(Value::Null) => self.value == "null",
            _ => false,
        }
    }

    fn matches_cmp<F>(&self, row_value: Option<&Value>, predicate: F) -> bool
    where
        F: Fn(std::cmp::Ordering) -> bool
    {
        let filter_num: f64 = match self.value.parse() {
            Ok(n) => n,
            Err(_) => return false,
        };

        match row_value {
            Some(Value::Number(n)) => {
                if let Some(row_num) = n.as_f64() {
                    predicate(row_num.partial_cmp(&filter_num).unwrap_or(std::cmp::Ordering::Equal))
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn matches_contains(&self, row_value: Option<&Value>, _case_sensitive: bool) -> bool {
        let search_lower = self.value.to_lowercase();
        match row_value {
            Some(Value::String(s)) => s.to_lowercase().contains(&search_lower),
            Some(v) => v.to_string().to_lowercase().contains(&search_lower),
            None => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilterExpr {
    pub conditions: Vec<FilterCondition>,
}

impl FilterExpr {
    pub fn parse(input: &str) -> Result<Self> {
        let mut conditions = Vec::new();
        let mut chars = input.chars().peekable();

        while chars.peek().is_some() {
            // Skip whitespace
            while chars.peek() == Some(&' ') {
                chars.next();
            }

            if chars.peek().is_none() {
                break;
            }

            // Parse column name
            let mut column = String::new();
            while let Some(&c) = chars.peek() {
                if c == '=' || c == '!' || c == '>' || c == '<' || c == '~' {
                    break;
                }
                column.push(chars.next().unwrap());
            }

            if column.is_empty() {
                return Err(JlcatError::InvalidFilter("empty column name".into()));
            }

            // Parse operator
            let op = match chars.peek() {
                Some('=') => {
                    chars.next();
                    FilterOp::Eq
                }
                Some('!') => {
                    chars.next();
                    match chars.peek() {
                        Some('=') => { chars.next(); FilterOp::Ne }
                        Some('~') => { chars.next(); FilterOp::NotContains }
                        _ => return Err(JlcatError::InvalidFilter("expected = or ~ after !".into())),
                    }
                }
                Some('>') => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        FilterOp::Gte
                    } else {
                        FilterOp::Gt
                    }
                }
                Some('<') => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        FilterOp::Lte
                    } else {
                        FilterOp::Lt
                    }
                }
                Some('~') => {
                    chars.next();
                    FilterOp::Contains
                }
                _ => return Err(JlcatError::InvalidFilter("missing operator".into())),
            };

            // Parse value
            let value = if chars.peek() == Some(&'"') || chars.peek() == Some(&'\'') {
                let quote = chars.next().unwrap();
                let mut val = String::new();
                while let Some(c) = chars.next() {
                    if c == quote {
                        break;
                    }
                    val.push(c);
                }
                val
            } else {
                let mut val = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' {
                        break;
                    }
                    val.push(chars.next().unwrap());
                }
                val
            };

            let path = CompiledPath::compile(&column)?;
            conditions.push(FilterCondition { path, op, value });
        }

        Ok(Self { conditions })
    }

    pub fn matches(&self, row: &Value) -> bool {
        self.conditions.iter().all(|c| c.matches(row))
    }
}

#[derive(Debug, Clone)]
pub struct FullTextSearch {
    query: String,
}

impl FullTextSearch {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_lowercase(),
        }
    }

    pub fn matches(&self, row: &Value) -> bool {
        self.search_value(row)
    }

    fn search_value(&self, value: &Value) -> bool {
        match value {
            Value::String(s) => s.to_lowercase().contains(&self.query),
            Value::Number(n) => n.to_string().contains(&self.query),
            Value::Bool(b) => b.to_string().contains(&self.query),
            Value::Array(arr) => arr.iter().any(|v| self.search_value(v)),
            Value::Object(obj) => obj.values().any(|v| self.search_value(v)),
            Value::Null => false,
        }
    }
}
```

**Step 3: mod.rs更新**

```rust
// src/core/mod.rs
mod filter;
mod schema;
mod selector;
mod sorter;
mod value;

pub use filter::{FilterCondition, FilterExpr, FilterOp, FullTextSearch};
pub use schema::{ColumnType, Schema, SchemaInferrer};
pub use selector::ColumnSelector;
pub use sorter::{SortKey, Sorter};
pub use value::{get_nested_value, SortableValue};
```

**Step 4: テスト実行**

Run: `cargo test filter::tests`
Expected: PASS

**Step 5: コミット**

```bash
git add src/core/filter.rs src/core/mod.rs
git commit -m "feat: add FilterExpr with operators and quoting support"
```

---

#### Task 3.6: TableData構造体

**Files:**
- Create: `src/core/table.rs`
- Modify: `src/core/mod.rs`

**Step 1: 失敗するテスト作成**

```rust
// src/core/table.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_table_data_from_rows() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];

        let table = TableData::from_rows(rows, None);

        assert_eq!(table.row_count(), 2);
        assert_eq!(table.columns(), &["id", "name"]);
    }

    #[test]
    fn test_table_data_with_column_selector() {
        let rows = vec![
            json!({"id": 1, "name": "Alice", "age": 30}),
        ];
        let selector = ColumnSelector::new(vec!["name".into(), "id".into()]);

        let table = TableData::from_rows(rows, Some(selector));

        assert_eq!(table.columns(), &["name", "id"]);
    }

    #[test]
    fn test_table_data_get_cell() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
        ];

        let table = TableData::from_rows(rows, None);

        assert_eq!(table.get_cell(0, 0), Some(&json!(1)));
        assert_eq!(table.get_cell(0, 1), Some(&json!("Alice")));
    }
}
```

**Step 2: TableData実装**

```rust
// src/core/table.rs
use super::schema::{Schema, SchemaInferrer};
use super::selector::ColumnSelector;
use super::value::get_nested_value;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct TableData {
    columns: Vec<String>,
    rows: Vec<Vec<Value>>,
    schema: Schema,
}

impl TableData {
    pub fn from_rows(rows: Vec<Value>, selector: Option<ColumnSelector>) -> Self {
        let schema = SchemaInferrer::infer(&rows);

        let columns: Vec<String> = if let Some(ref sel) = selector {
            sel.columns().to_vec()
        } else {
            schema.columns().to_vec()
        };

        let table_rows: Vec<Vec<Value>> = rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|col| {
                        get_nested_value(row, col)
                            .cloned()
                            .unwrap_or(Value::Null)
                    })
                    .collect()
            })
            .collect();

        Self {
            columns,
            rows: table_rows,
            schema,
        }
    }

    pub fn columns(&self) -> &[String] {
        &self.columns
    }

    pub fn rows(&self) -> &[Vec<Value>] {
        &self.rows
    }

    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    pub fn column_count(&self) -> usize {
        self.columns.len()
    }

    pub fn get_cell(&self, row: usize, col: usize) -> Option<&Value> {
        self.rows.get(row).and_then(|r| r.get(col))
    }

    pub fn get_row(&self, index: usize) -> Option<&[Value]> {
        self.rows.get(index).map(|r| r.as_slice())
    }

    pub fn schema(&self) -> &Schema {
        &self.schema
    }
}
```

**Step 3: mod.rs更新**

```rust
// src/core/mod.rs
mod filter;
mod schema;
mod selector;
mod sorter;
mod table;
mod value;

pub use filter::{FilterCondition, FilterExpr, FilterOp, FullTextSearch};
pub use schema::{ColumnType, Schema, SchemaInferrer};
pub use selector::ColumnSelector;
pub use sorter::{SortKey, Sorter};
pub use table::TableData;
pub use value::{get_nested_value, SortableValue};
```

**Step 4: テスト実行**

Run: `cargo test table::tests`
Expected: PASS

**Step 5: コミット**

```bash
git add src/core/table.rs src/core/mod.rs
git commit -m "feat: add TableData structure for rendering"
```

---

### Phase 4: catモードレンダラー

#### Task 4.1: CatRenderer基本実装

**Files:**
- Create: `src/render/mod.rs`
- Create: `src/render/cat.rs`
- Modify: `Cargo.toml`

**Step 1: Cargo.tomlにcomfy-table追加**

```toml
[dependencies]
# ... existing ...
comfy-table = "7"
```

**Step 2: 失敗するテスト作成**

```rust
// src/render/cat.rs の末尾
#[cfg(test)]
mod tests {
    use super::*;
    use crate::core::TableData;
    use serde_json::json;

    #[test]
    fn test_render_simple_table() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.contains("id"));
        assert!(output.contains("name"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn test_render_with_null() {
        let rows = vec![
            json!({"id": 1, "name": null}),
        ];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.contains("null") || output.contains(""));
    }
}
```

**Step 3: CatRenderer実装**

```rust
// src/render/cat.rs
use crate::cli::TableStyle;
use crate::core::TableData;
use comfy_table::{presets, Table, ContentArrangement};
use serde_json::Value;

pub struct CatRenderer {
    style: TableStyle,
}

impl CatRenderer {
    pub fn new(style: TableStyle) -> Self {
        Self { style }
    }

    pub fn render(&self, table_data: &TableData) -> String {
        let mut table = Table::new();

        // Apply style
        match self.style {
            TableStyle::Ascii => table.load_preset(presets::ASCII_FULL),
            TableStyle::Rounded => table.load_preset(presets::UTF8_FULL),
            TableStyle::Markdown => table.load_preset(presets::ASCII_MARKDOWN),
            TableStyle::Plain => table.load_preset(presets::NOTHING),
        };

        table.set_content_arrangement(ContentArrangement::Dynamic);

        // Add header
        table.set_header(table_data.columns());

        // Add rows
        for row in table_data.rows() {
            let cells: Vec<String> = row.iter().map(|v| self.format_value(v)).collect();
            table.add_row(cells);
        }

        table.to_string()
    }

    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(_) => "[...]".to_string(),
            Value::Object(_) => "{...}".to_string(),
        }
    }
}
```

**Step 4: mod.rs作成**

```rust
// src/render/mod.rs
mod cat;

pub use cat::CatRenderer;
```

**Step 5: main.rsにmod追加**

```rust
// src/main.rs
mod cli;
mod core;
mod error;
mod input;
mod render;
```

**Step 6: テスト実行**

Run: `cargo test cat::tests`
Expected: PASS

**Step 7: コミット**

```bash
git add src/render/ src/main.rs Cargo.toml
git commit -m "feat: add CatRenderer with comfy-table"
```

---

#### Task 4.2: catモード統合

**Files:**
- Modify: `src/main.rs`
- Create: `tests/integration_tests.rs`

**Step 1: 失敗するテスト作成**

```rust
// tests/integration_tests.rs
use assert_cmd::Command;
use predicates::prelude::*;
use std::fs;

#[test]
fn test_cat_mode_file() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("tests/fixtures/simple.jsonl")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob"));
}

#[test]
fn test_cat_mode_stdin() {
    let input = r#"{"id": 1, "name": "Test"}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Test"));
}

#[test]
fn test_column_selection() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-c", "name", "tests/fixtures/simple.jsonl"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name"))
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::is_match("id").unwrap().not());
}
```

**Step 2: main.rs統合実装**

```rust
// src/main.rs
mod cli;
mod core;
mod error;
mod input;
mod render;

use clap::Parser;
use cli::Cli;
use core::{ColumnSelector, SchemaInferrer, Sorter, TableData};
use error::Result;
use input::{FileSource, InputSource, StdinSource};
use render::CatRenderer;
use serde_json::Value;
use std::io::{self, BufRead};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check for stdin without input
    if cli.file.is_none() && atty::is(atty::Stream::Stdin) {
        eprintln!("Usage: jlcat [OPTIONS] [FILE]");
        eprintln!("Try 'jlcat --help' for more information.");
        std::process::exit(1);
    }

    // Read input
    let rows = read_input(&cli)?;

    if rows.is_empty() {
        return Ok(());
    }

    // Apply sorting if specified
    let mut rows = rows;
    if let Some(ref sort_keys) = cli.sort {
        let sorter = Sorter::parse(sort_keys)?;
        sorter.sort(&mut rows);
    }

    // Build column selector if specified
    let selector = cli.columns.as_ref().map(|cols| ColumnSelector::new(cols.clone()));

    // Create table data
    let table_data = TableData::from_rows(rows, selector);

    // Render
    if cli.interactive {
        // TUI mode - to be implemented
        eprintln!("TUI mode not yet implemented");
        std::process::exit(1);
    } else {
        let renderer = CatRenderer::new(cli.style.clone());
        println!("{}", renderer.render(&table_data));
    }

    Ok(())
}

fn read_input(cli: &Cli) -> Result<Vec<Value>> {
    let mut rows = Vec::new();
    let strict = cli.is_strict();

    if let Some(ref path) = cli.file {
        let source = FileSource::open(path)?;
        for (line_num, line) in source.lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(&line) {
                Ok(value) => rows.push(value),
                Err(e) => {
                    if strict {
                        return Err(error::JlcatError::JsonParse {
                            line: line_num + 1,
                            message: e.to_string(),
                        });
                    } else {
                        eprintln!("jlcat: warning: line {}: invalid JSON, skipping", line_num + 1);
                    }
                }
            }
        }
    } else {
        let stdin = io::stdin();
        for (line_num, line) in stdin.lock().lines().enumerate() {
            let line = line?;
            if line.trim().is_empty() {
                continue;
            }
            match serde_json::from_str(&line) {
                Ok(value) => rows.push(value),
                Err(e) => {
                    if strict {
                        return Err(error::JlcatError::JsonParse {
                            line: line_num + 1,
                            message: e.to_string(),
                        });
                    } else {
                        eprintln!("jlcat: warning: line {}: invalid JSON, skipping", line_num + 1);
                    }
                }
            }
        }
    }

    Ok(rows)
}
```

**Step 3: テスト実行**

Run: `cargo test integration_tests`
Expected: PASS

**Step 4: 手動確認**

Run: `echo '{"id":1,"name":"Alice"}' | cargo run`
Expected: Table with id and name columns

**Step 5: コミット**

```bash
git add src/main.rs tests/integration_tests.rs
git commit -m "feat: integrate cat mode with file and stdin support"
```

---

### Phase 5以降（残りのタスク概要）

以下のタスクは詳細を省略し、概要のみ記載：

#### Phase 5: ネスト展開（-r）
- Task 5.1: NestedExtractor実装
- Task 5.2: 子テーブル生成
- Task 5.3: catモードでの-r出力

#### Phase 6: TUIモード
- Task 6.1: ratatui基盤セットアップ
- Task 6.2: App状態管理
- Task 6.3: テーブルビュー描画
- Task 6.4: 仮想スクロール実装
- Task 6.5: キーボードハンドリング
- Task 6.6: 検索/フィルター入力UI
- Task 6.7: パニックフック・ターミナル復旧

#### Phase 7: Indexedパイプライン
- Task 7.1: 行オフセットインデックス
- Task 7.2: LRUキャッシュ
- Task 7.3: バックグラウンドスキャン

#### Phase 8: テスト強化
- Task 8.1: プロパティテスト（proptest）
- Task 8.2: ゴールデンテスト
- Task 8.3: 大規模ファイルテスト

---

## テスト戦略

| カテゴリ | ツール | 対象 |
|----------|--------|------|
| ユニットテスト | cargo test | 各モジュール |
| インテグレーション | assert_cmd | CLI全体 |
| プロパティテスト | proptest | スキーマ推論、フィルター |
| ゴールデンテスト | insta | テーブルレンダリング |
| ファズテスト | cargo-fuzz | JSONパース |

---

## パフォーマンス目標（参考値）

| 指標 | 目標 | 備考 |
|------|------|------|
| 10万行初期表示 | < 500ms | Indexed pipeline |
| フィルター反映 | < 200ms | キャッシュヒット時 |
| スクロール | 60fps | 仮想スクロール |

※ハードウェア: M1 Mac, SSD, 16GB RAM を想定。実測値により調整。

---

## 参考

- [tv](https://github.com/uzimaru0000/tv) - JSON to table view CLI (Rust)
- [ratatui](https://ratatui.rs/) - TUI library
- [comfy-table](https://github.com/Nukesor/comfy-table) - Table rendering
