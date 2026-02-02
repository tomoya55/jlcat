# jlcat 設計書

JSON/JSONLをテーブル表示するCLIツール。静的なcatモードに加え、フィルター/検索可能なTUIモードを提供。

## CLIインターフェース

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
                        例: -s name,-age  (nameで昇順、ageで降順)
  --style <STYLE>       テーブルスタイル [ascii|rounded|markdown|plain]
  -h, --help            ヘルプ表示
  -V, --version         バージョン表示
```

### 使用例

```bash
# 基本的なテーブル表示
$ cat users.jsonl | jlcat

# カラム指定（ドット記法でネストもフラット化）
$ jlcat -c id,name,address.city users.jsonl

# ソート（ageで降順、同ageならnameで昇順）
$ jlcat -s -age,name users.jsonl

# ネスト構造を子テーブルとして展開
$ jlcat -r users.jsonl

# TUIモードで起動
$ jlcat -i users.jsonl
```

## アーキテクチャ

```
┌─────────────────────────────────────────────────────────────┐
│                         jlcat                               │
├─────────────────────────────────────────────────────────────┤
│  CLI Parser (clap)                                          │
│    └─> Options { mode, columns, sort, recursive, style }    │
├─────────────────────────────────────────────────────────────┤
│  Input Layer                                                │
│    ├─ StdinReader   ─┐                                      │
│    └─ FileReader    ─┴─> StreamingJsonParser                │
│                           └─> Vec<JsonRow> (チャンク単位)    │
├─────────────────────────────────────────────────────────────┤
│  Core Engine                                                │
│    ├─ SchemaInferrer    ... カラム構造を推論                 │
│    ├─ ColumnSelector    ... -c 指定でフィルタ/フラット化     │
│    ├─ NestedExtractor   ... -r でネスト構造を子テーブル化    │
│    ├─ Sorter            ... -s 指定でソート                  │
│    └─ TableData         ... 表示用データ構造                 │
├─────────────────────────────────────────────────────────────┤
│  Output Layer                                               │
│    ├─ CatRenderer       ... 静的テーブル出力 (comfy-table)   │
│    └─ TuiRenderer       ... インタラクティブ (ratatui)       │
│          ├─ VirtualScroll                                   │
│          ├─ SearchEngine (全文 + カラム指定)                 │
│          └─ KeyHandler                                      │
└─────────────────────────────────────────────────────────────┘
```

**設計方針:**
- 入力は常にストリーミング処理（メモリ効率）
- Core Engineはモードに依存しない（テスタビリティ）
- TuiRendererは仮想スクロールで大量行に対応

## TUIモード

### 画面構成

```
┌─ jlcat ──────────────────────────────────────────────────────┐
│ Filter: status=active                              [2/1000]  │
├──────────────────────────────────────────────────────────────┤
│  id │ name      │ status   │ created_at          │ address  │
├─────┼───────────┼──────────┼─────────────────────┼──────────┤
│  3  │ Alice     │ active   │ 2024-01-15 10:30:00 │ ...      │
│  7  │ Bob       │ active   │ 2024-01-16 14:22:00 │ ...      │
│     │           │          │                     │          │
├──────────────────────────────────────────────────────────────┤
│ [/] search  [f] filter  [s] sort  [c] columns  [q] quit     │
└──────────────────────────────────────────────────────────────┘
```

### キーバインド

| キー | 動作 |
|------|------|
| `j/k` または `↑/↓` | 行移動 |
| `g/G` | 先頭/末尾へ |
| `/` | 全文検索モード |
| `f` | カラムフィルター入力（`column=value` 形式） |
| `s` | ソートカラム選択 |
| `c` | 表示カラム選択（トグル） |
| `Enter` | ネスト構造を展開（`-r` 時） |
| `Backspace` | 親テーブルに戻る |
| `Esc` | フィルター解除 / モード終了 |
| `q` | 終了 |

### フィルター構文

- **全文検索**: `/alice` → 全カラムから "alice" を含む行
- **カラム指定**: `f` → `status=active` → statusが "active" の行
- **複数条件**: `status=active,role=admin`（AND条件）

## ネスト構造の表示（-r オプション）

### 入力例

```json
{"id": 1, "name": "Alice", "orders": [{"item": "Book", "price": 1500}, {"item": "Pen", "price": 200}]}
{"id": 2, "name": "Bob", "orders": [{"item": "Desk", "price": 25000}]}
```

### 出力

```
── Main ──────────────────────────────
│ id │ name  │ orders │
├────┼───────┼────────┤
│ 1  │ Alice │ [...]  │
│ 2  │ Bob   │ [...]  │

── orders (→id) ──────────────────────
│ id │ item  │ price  │
├────┼───────┼────────┤
│ 1  │ Book  │ 1500   │
│ 1  │ Pen   │ 200    │
│ 2  │ Desk  │ 25000  │
```

### 仕様

- 子テーブルのタイトルに `(→id)` で外部キー元を表示
- 配列は展開して複数行に（1親 → N子の関係）
- オブジェクトのネストも同様に子テーブル化
- 再帰的にどこまでも展開（`orders[].details.supplier` なども対応）

### TUIモードでの動作

- `Enter` で選択行のネスト構造にフォーカス移動
- `Backspace` で親テーブルに戻る
- 子テーブルでもフィルター/検索が有効

## パフォーマンス戦略

### 大量データ（数万行）を高速処理するための設計

**読み込み:**
1. 初回スキャン（高速）
   - 最初の100行でスキーマ推論
   - 全行をインデックス化（行オフセットのみ保持）
   - ファイル全体をメモリに載せない

2. オンデマンド読み込み
   - 表示に必要な行だけパース（仮想スクロール）
   - LRUキャッシュで最近表示した行を保持

**検索/フィルター:**
- 初回フィルター: バックグラウンドで全行スキャン
- 結果をインデックス化してキャッシュ
- インクリメンタル検索: デバウンス(100ms)で負荷軽減
- Ctrl+C でキャンセル可能

**ソート:**
- 対象カラムの値のみ抽出 → 外部ソートアルゴリズム
- インデックス配列のみ並び替え（データ本体は移動しない）

### 目標パフォーマンス

- 10万行JSONL: 初期表示 < 500ms
- フィルター反映: < 200ms（キャッシュヒット時）
- スクロール: 60fps維持

## 依存クレート

```toml
[dependencies]
clap = { version = "4", features = ["derive"] }    # CLI引数パース
serde = { version = "1", features = ["derive"] }   # シリアライズ
serde_json = "1"                                   # JSONパース
ratatui = "0.29"                                   # TUI
crossterm = "0.28"                                 # ターミナル制御
comfy-table = "7"                                  # catモードのテーブル出力
memmap2 = "0.9"                                    # 大規模ファイルのメモリマップ
```

## ディレクトリ構成

```
jlcat/
├── Cargo.toml
├── src/
│   ├── main.rs              # エントリポイント
│   ├── cli.rs               # clap定義
│   ├── input/
│   │   ├── mod.rs
│   │   ├── reader.rs        # stdin/file読み込み
│   │   └── parser.rs        # JSONLストリーミングパース
│   ├── core/
│   │   ├── mod.rs
│   │   ├── schema.rs        # スキーマ推論
│   │   ├── selector.rs      # カラム選択/フラット化
│   │   ├── extractor.rs     # ネスト構造抽出
│   │   ├── sorter.rs        # ソート処理
│   │   └── table.rs         # TableData構造体
│   ├── render/
│   │   ├── mod.rs
│   │   ├── cat.rs           # 静的テーブル出力
│   │   └── tui/
│   │       ├── mod.rs
│   │       ├── app.rs       # TUIアプリ状態管理
│   │       ├── view.rs      # 描画ロジック
│   │       ├── scroll.rs    # 仮想スクロール
│   │       └── search.rs    # 検索/フィルター
│   └── error.rs             # エラー型定義
└── tests/
    ├── cli_tests.rs         # CLIインテグレーション
    └── fixtures/            # テスト用JSONLファイル
```

## MVP機能（優先度順）

1. catモードでの基本テーブル表示
2. JSONL対応
3. `-c` カラム選択
4. `-s` ソート
5. `-r` ネスト展開
6. TUIモード（仮想スクロール）
7. 検索/フィルター機能

## 参考

- [tv](https://github.com/uzimaru0000/tv) - JSON to table view CLI (Rust)
