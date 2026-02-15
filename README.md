# jlcat

> JSON/JSONL table viewer with auto-flattening and TUI support

[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust](https://img.shields.io/badge/rust-1.70%2B-orange.svg)](https://www.rust-lang.org/)

jlcat displays JSON and JSONL data as beautifully formatted tables. Nested objects are automatically flattened into dot-notation columns, making complex data easy to read at a glance.

## Features

- **Auto-flatten nested objects** - `{"user": {"name": "Alice"}}` becomes column `user.name`
- **TUI mode** - Interactive browsing with keyboard navigation
- **Multiple styles** - ASCII, rounded, markdown, plain
- **Column selection** - Pick specific columns with wildcard support (`user.*`)
- **Sorting** - Sort by any column, ascending or descending
- **Flexible input** - Files, stdin, JSON arrays, JSONL

## Installation

### From source (Cargo)

```bash
cargo install --path .
```

### Build from source

```bash
git clone https://github.com/tomoya55/jlcat.git
cd jlcat
cargo build --release
```

## Usage

```
jlcat [OPTIONS] [FILE]
```

### Arguments

| Argument | Description |
|----------|-------------|
| `[FILE]` | JSON or JSONL file path (reads from stdin if omitted) |

### Options

| Option | Description |
|--------|-------------|
| `-i, --interactive` | Launch in interactive TUI mode |
| `--skip N` | Skip the first N rows while reading input |
| `--limit N` | Limit the number of rows read from input (`--head` alias) |
| `--tail N` | Read only the last N rows (conflicts with `--skip`/`--limit`) |
| `-c, --columns <COLS>` | Columns to display (comma-separated, supports wildcards) |
| `-s, --sort <KEYS>` | Sort by columns (prefix with `-` for descending) |
| `-r, --recursive` | Expand nested structures as child tables |
| `--flat[=DEPTH]` | Flatten nested objects with optional depth limit |
| `--array-limit=N` | Max array elements to show in flat mode (default: 3) |
| `--no-flatten` | Disable auto-flattening of nested objects |
| `--style <STYLE>` | Table style: `ascii`, `rounded`, `markdown`, `plain` |
| `--lenient` | Skip invalid JSON lines instead of erroring |
| `-h, --help` | Show help |
| `-V, --version` | Show version |

## Examples

### Basic usage

```bash
# From file
jlcat data.jsonl

# From stdin
cat data.jsonl | jlcat
curl -s https://api.example.com/users | jlcat
```

### Paging (large files)

```bash
# Show only the first 100 rows (alias: --head)
jlcat --limit 100 data.jsonl
jlcat --head 100 data.jsonl

# Skip the first 10k rows, then show the next 200 rows
jlcat --skip 10000 --limit 200 data.jsonl

# Show only the last 500 rows (useful for logs)
jlcat --tail 500 data.jsonl
```

### Auto-flattening (default)

Input:
```json
{"id": 1, "user": {"name": "Alice", "profile": {"age": 30}}, "tags": ["dev", "rust"]}
{"id": 2, "user": {"name": "Bob", "profile": {"age": 25}}, "tags": ["ops"]}
```

Output:
```
╭────┬───────────┬──────────────────┬────────────────╮
│ id │ user.name │ user.profile.age │ tags           │
├────┼───────────┼──────────────────┼────────────────┤
│ 1  │ Alice     │ 30               │ ["dev","rust"] │
│ 2  │ Bob       │ 25               │ ["ops"]        │
╰────┴───────────┴──────────────────┴────────────────╯
```

### Flat mode with depth limit

The `--flat` option enables explicit flat mode with dot-notation columns. Optionally specify a depth limit:

```bash
# Enable flat mode
echo '{"user": {"name": "Alice", "age": 30}}' | jlcat --flat

# Limit expansion depth
jlcat --flat=2 data.jsonl

# Customize array display limit (default: 3)
jlcat --flat --array-limit=5 data.jsonl
```

Arrays are displayed as comma-separated values with ellipsis for truncation:

```
| tags         |
|--------------|
| a, b, c, ... |
```

### Disable flattening

```bash
jlcat --no-flatten data.jsonl
```

```
╭────┬─────────────────────────────────────────┬────────────────╮
│ id │ user                                    │ tags           │
├────┼─────────────────────────────────────────┼────────────────┤
│ 1  │ {"name":"Alice","profile":{"age":30}}   │ ["dev","rust"] │
│ 2  │ {"name":"Bob","profile":{"age":25}}     │ ["ops"]        │
╰────┴─────────────────────────────────────────┴────────────────╯
```

### Column selection with wildcards

```bash
# Select specific columns
jlcat -c id,user.name data.jsonl

# Wildcard: all user.* columns
jlcat -c "id,user.*" data.jsonl
```

### Sorting

```bash
# Sort by name ascending
jlcat -s user.name data.jsonl

# Sort by age descending
jlcat -s -user.profile.age data.jsonl

# Multiple sort keys
jlcat -s "user.name,-user.profile.age" data.jsonl
```

### Table styles

```bash
jlcat --style ascii data.jsonl
jlcat --style markdown data.jsonl
jlcat --style plain data.jsonl
```

### Interactive TUI mode

```bash
jlcat -i data.jsonl
```

**TUI keybindings:**
- `j/k` or `↑/↓` - Navigate rows
- `h/l` or `←/→` - Scroll columns
- `Enter` - Open detail view (shows full JSON)
- `/` - Search
- `f` - Filter
- `c` - Clear filters
- `q` - Quit

**Detail view keybindings:**
- `j/k` or `↑/↓` - Scroll
- `g` - Go to top
- `G` - Go to bottom
- `Esc` - Close detail view
- `q` - Quit

## Input formats

jlcat automatically detects the input format:

| Format | Detection | Example |
|--------|-----------|---------|
| JSONL | Starts with `{` | `{"id": 1}\n{"id": 2}` |
| JSON Array | Starts with `[` | `[{"id": 1}, {"id": 2}]` |

## License

MIT
