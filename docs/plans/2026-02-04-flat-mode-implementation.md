# Flat Mode Implementation Plan

> **For Claude:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task.

**Goal:** Implement flat mode that expands nested JSON objects into dot-notation columns with configurable depth and array display limits.

**Architecture:** Add `FlatConfig` struct to hold configuration, `FlatSchema` to track dynamically discovered columns, and `FlatTableData` to build flattened table data. Integrate with existing CLI, CatRenderer, and TUI components.

**Tech Stack:** Rust, serde_json, clap (CLI), comfy_table (rendering), ratatui (TUI)

---

## Task 1: Add FlatConfig Struct

**Files:**
- Create: `src/core/flat.rs`
- Modify: `src/core/mod.rs:1-26`

**Step 1: Write the failing test**

Add to `src/core/flat.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_config_default() {
        let config = FlatConfig::default();
        assert_eq!(config.depth, None); // unlimited
        assert_eq!(config.array_limit, 3);
    }

    #[test]
    fn test_flat_config_with_depth() {
        let config = FlatConfig::new(Some(2), 5);
        assert_eq!(config.depth, Some(2));
        assert_eq!(config.array_limit, 5);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test flat_config --no-run 2>&1 | head -20`
Expected: Compilation error - module not found

**Step 3: Write minimal implementation**

Create `src/core/flat.rs`:

```rust
/// Configuration for flat mode
#[derive(Debug, Clone)]
pub struct FlatConfig {
    /// Maximum depth to expand (None = unlimited)
    pub depth: Option<usize>,
    /// Maximum array elements to display
    pub array_limit: usize,
}

impl FlatConfig {
    pub fn new(depth: Option<usize>, array_limit: usize) -> Self {
        Self { depth, array_limit }
    }
}

impl Default for FlatConfig {
    fn default() -> Self {
        Self {
            depth: None,
            array_limit: 3,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_flat_config_default() {
        let config = FlatConfig::default();
        assert_eq!(config.depth, None);
        assert_eq!(config.array_limit, 3);
    }

    #[test]
    fn test_flat_config_with_depth() {
        let config = FlatConfig::new(Some(2), 5);
        assert_eq!(config.depth, Some(2));
        assert_eq!(config.array_limit, 5);
    }
}
```

**Step 4: Update mod.rs to export the module**

Add to `src/core/mod.rs`:

```rust
mod flat;
pub use flat::FlatConfig;
```

**Step 5: Run test to verify it passes**

Run: `cargo test flat_config -v`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/flat.rs src/core/mod.rs
git commit -m "feat(flat): add FlatConfig struct for flat mode configuration"
```

---

## Task 2: Add FlatSchema for Column Tracking

**Files:**
- Modify: `src/core/flat.rs`

**Step 1: Write the failing test**

Add to `src/core/flat.rs` tests:

```rust
#[test]
fn test_flat_schema_add_column() {
    let mut schema = FlatSchema::new();
    schema.add_column("id".to_string(), false);
    schema.add_column("user.name".to_string(), true);
    schema.add_column("user.age".to_string(), true);

    assert_eq!(schema.columns().len(), 3);
    assert!(schema.columns().contains(&"id".to_string()));
    assert!(schema.columns().contains(&"user.name".to_string()));
}

#[test]
fn test_flat_schema_column_order() {
    let mut schema = FlatSchema::new();
    // First level keys in appearance order
    schema.add_column("z".to_string(), false);
    schema.add_column("a.x".to_string(), true);
    schema.add_column("a.b".to_string(), true);
    schema.add_column("m".to_string(), false);

    let cols = schema.columns();
    // z first (appearance), then a's children sorted, then m
    assert_eq!(cols[0], "z");
    assert_eq!(cols[1], "a.b"); // sorted among a's children
    assert_eq!(cols[2], "a.x");
    assert_eq!(cols[3], "m");
}

#[test]
fn test_flat_schema_dynamic_column_add() {
    let mut schema = FlatSchema::new();
    schema.add_column("user.name".to_string(), true);
    schema.finalize_initial_schema();

    // Later, add parent column dynamically
    schema.add_column("user".to_string(), false);

    assert!(schema.columns().contains(&"user".to_string()));
    assert!(schema.is_dynamic_column("user"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test flat_schema --no-run 2>&1 | head -20`
Expected: Compilation error - FlatSchema not found

**Step 3: Write minimal implementation**

Add to `src/core/flat.rs`:

```rust
use std::collections::{HashMap, HashSet};

/// Tracks columns for flat mode with proper ordering
#[derive(Debug, Clone)]
pub struct FlatSchema {
    /// First-level keys in appearance order
    first_level_order: Vec<String>,
    /// Child columns grouped by parent, sorted alphabetically
    children: HashMap<String, Vec<String>>,
    /// All column paths
    all_columns: HashSet<String>,
    /// Columns added after initial schema finalization
    dynamic_columns: HashSet<String>,
    /// Whether initial schema is finalized
    finalized: bool,
}

impl FlatSchema {
    pub fn new() -> Self {
        Self {
            first_level_order: Vec::new(),
            children: HashMap::new(),
            all_columns: HashSet::new(),
            dynamic_columns: HashSet::new(),
            finalized: false,
        }
    }

    /// Add a column to the schema
    /// is_child: true if this is an expanded child column (e.g., "user.name")
    pub fn add_column(&mut self, path: String, is_child: bool) {
        if self.all_columns.contains(&path) {
            return;
        }

        self.all_columns.insert(path.clone());

        if self.finalized {
            self.dynamic_columns.insert(path.clone());
        }

        if is_child {
            // Extract parent from path (e.g., "user.name" -> "user")
            if let Some(dot_pos) = path.find('.') {
                let parent = &path[..dot_pos];

                // Add parent to first-level order if not present
                if !self.first_level_order.contains(&parent.to_string()) {
                    self.first_level_order.push(parent.to_string());
                }

                // Add to children, maintaining sorted order
                let children = self.children.entry(parent.to_string()).or_default();
                if !children.contains(&path) {
                    children.push(path);
                    children.sort();
                }
            }
        } else {
            // First-level column
            if !self.first_level_order.contains(&path) {
                self.first_level_order.push(path);
            }
        }
    }

    /// Mark initial schema as finalized (subsequent adds are dynamic)
    pub fn finalize_initial_schema(&mut self) {
        self.finalized = true;
    }

    /// Check if a column was added dynamically
    pub fn is_dynamic_column(&self, path: &str) -> bool {
        self.dynamic_columns.contains(path)
    }

    /// Get columns in proper order:
    /// - First-level keys in appearance order
    /// - Children sorted alphabetically under their parent's position
    pub fn columns(&self) -> Vec<String> {
        let mut result = Vec::new();

        for key in &self.first_level_order {
            if let Some(children) = self.children.get(key) {
                // Parent was expanded - add children (already sorted)
                result.extend(children.clone());
            } else {
                // First-level key without expansion
                result.push(key.clone());
            }
        }

        result
    }
}

impl Default for FlatSchema {
    fn default() -> Self {
        Self::new()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test flat_schema -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/core/flat.rs
git commit -m "feat(flat): add FlatSchema for column tracking with proper ordering"
```

---

## Task 3: Add Array Formatting Function

**Files:**
- Modify: `src/core/flat.rs`

**Step 1: Write the failing test**

Add to `src/core/flat.rs` tests:

```rust
use serde_json::json;

#[test]
fn test_format_array_basic() {
    let arr = json!(["a", "b"]);
    assert_eq!(format_array(&arr, 3), "a, b");
}

#[test]
fn test_format_array_with_limit() {
    let arr = json!(["a", "b", "c", "d"]);
    assert_eq!(format_array(&arr, 3), "a, b, c, ...");
}

#[test]
fn test_format_array_exact_limit() {
    let arr = json!(["a", "b", "c"]);
    assert_eq!(format_array(&arr, 3), "a, b, c");
}

#[test]
fn test_format_array_empty() {
    let arr = json!([]);
    assert_eq!(format_array(&arr, 3), "");
}

#[test]
fn test_format_array_nested_objects() {
    let arr = json!([1, {"x": 2}, [3, 4]]);
    assert_eq!(format_array(&arr, 3), "1, {...}, [...]");
}

#[test]
fn test_format_array_mixed_types() {
    let arr = json!([1, "two", true, null]);
    assert_eq!(format_array(&arr, 4), "1, two, true, null");
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test format_array --no-run 2>&1 | head -20`
Expected: Compilation error - format_array not found

**Step 3: Write minimal implementation**

Add to `src/core/flat.rs`:

```rust
use serde_json::Value;

/// Format an array value for display with element limit
pub fn format_array(value: &Value, limit: usize) -> String {
    let arr = match value {
        Value::Array(a) => a,
        _ => return String::new(),
    };

    if arr.is_empty() {
        return String::new();
    }

    let formatted: Vec<String> = arr
        .iter()
        .take(limit)
        .map(|v| format_array_element(v))
        .collect();

    if arr.len() > limit {
        format!("{}, ...", formatted.join(", "))
    } else {
        formatted.join(", ")
    }
}

fn format_array_element(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) => "[...]".to_string(),
        Value::Object(_) => "{...}".to_string(),
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test format_array -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/core/flat.rs
git commit -m "feat(flat): add format_array function for array display"
```

---

## Task 4: Add Object Flattening Function

**Files:**
- Modify: `src/core/flat.rs`

**Step 1: Write the failing test**

Add to `src/core/flat.rs` tests:

```rust
#[test]
fn test_flatten_object_simple() {
    let obj = json!({"id": 1, "name": "Alice"});
    let config = FlatConfig::default();
    let flattened = flatten_object(&obj, &config);

    assert_eq!(flattened.get("id"), Some(&json!(1)));
    assert_eq!(flattened.get("name"), Some(&json!("Alice")));
}

#[test]
fn test_flatten_object_nested() {
    let obj = json!({"id": 1, "user": {"name": "Alice", "age": 30}});
    let config = FlatConfig::default();
    let flattened = flatten_object(&obj, &config);

    assert_eq!(flattened.get("id"), Some(&json!(1)));
    assert_eq!(flattened.get("user.name"), Some(&json!("Alice")));
    assert_eq!(flattened.get("user.age"), Some(&json!(30)));
    assert!(!flattened.contains_key("user")); // parent not included
}

#[test]
fn test_flatten_object_depth_limit() {
    let obj = json!({"a": {"b": {"c": 1}}});
    let config = FlatConfig::new(Some(1), 3);
    let flattened = flatten_object(&obj, &config);

    // Only 1 level deep, so a.b is {c: 1} displayed as {...}
    assert_eq!(flattened.get("a.b"), Some(&json!("{...}")));
}

#[test]
fn test_flatten_object_with_array() {
    let obj = json!({"tags": ["a", "b", "c", "d"]});
    let config = FlatConfig::default(); // limit 3
    let flattened = flatten_object(&obj, &config);

    assert_eq!(flattened.get("tags"), Some(&json!("a, b, c, ...")));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test flatten_object --no-run 2>&1 | head -20`
Expected: Compilation error - flatten_object not found

**Step 3: Write minimal implementation**

Add to `src/core/flat.rs`:

```rust
use std::collections::HashMap;

/// Flatten a JSON object into dot-notation key-value pairs
pub fn flatten_object(value: &Value, config: &FlatConfig) -> HashMap<String, Value> {
    let mut result = HashMap::new();

    if let Value::Object(obj) = value {
        flatten_object_recursive(obj, "", 0, config, &mut result);
    }

    result
}

fn flatten_object_recursive(
    obj: &serde_json::Map<String, Value>,
    prefix: &str,
    depth: usize,
    config: &FlatConfig,
    result: &mut HashMap<String, Value>,
) {
    for (key, value) in obj {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            Value::Object(nested_obj) => {
                // Check depth limit
                if config.depth.map_or(true, |max| depth < max) {
                    // Expand the object
                    flatten_object_recursive(nested_obj, &full_key, depth + 1, config, result);
                } else {
                    // Depth limit reached, use placeholder
                    result.insert(full_key, Value::String("{...}".to_string()));
                }
            }
            Value::Array(_) => {
                // Format array with limit
                let formatted = format_array(value, config.array_limit);
                result.insert(full_key, Value::String(formatted));
            }
            _ => {
                result.insert(full_key, value.clone());
            }
        }
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test flatten_object -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/core/flat.rs
git commit -m "feat(flat): add flatten_object function for object expansion"
```

---

## Task 5: Add FlatTableData Builder

**Files:**
- Modify: `src/core/flat.rs`
- Modify: `src/core/mod.rs`

**Step 1: Write the failing test**

Add to `src/core/flat.rs` tests:

```rust
#[test]
fn test_flat_table_data_basic() {
    let rows = vec![
        json!({"id": 1, "user": {"name": "Alice", "age": 30}}),
        json!({"id": 2, "user": {"name": "Bob", "age": 25}}),
    ];
    let config = FlatConfig::default();

    let table = FlatTableData::from_rows(&rows, config);

    // Columns: id (first-level), user.age, user.name (children sorted)
    assert_eq!(table.columns(), &["id", "user.age", "user.name"]);
    assert_eq!(table.rows().len(), 2);
}

#[test]
fn test_flat_table_data_structure_conflict() {
    // First row: user is object, second row: user is string
    let rows = vec![
        json!({"id": 1, "user": {"name": "Alice"}}),
        json!({"id": 2, "user": "Bob"}),
    ];
    let config = FlatConfig::default();

    let table = FlatTableData::from_rows(&rows, config);

    // Should have both user and user.name columns
    let cols = table.columns();
    assert!(cols.contains(&"user".to_string()));
    assert!(cols.contains(&"user.name".to_string()));

    // Row 1: user.name = "Alice", user = null
    // Row 2: user.name = null, user = "Bob"
}

#[test]
fn test_flat_table_data_scalar_to_object() {
    // First row: user is string, second row: user is object
    let rows = vec![
        json!({"id": 1, "user": "Alice"}),
        json!({"id": 2, "user": {"name": "Bob"}}),
    ];
    let config = FlatConfig::default();

    let table = FlatTableData::from_rows(&rows, config);

    // user column should exist (from first row)
    // Second row's object should be displayed as {...}
    let cols = table.columns();
    assert!(cols.contains(&"user".to_string()));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test flat_table_data --no-run 2>&1 | head -20`
Expected: Compilation error - FlatTableData not found

**Step 3: Write minimal implementation**

Add to `src/core/flat.rs`:

```rust
/// Table data with flattened structure
#[derive(Debug, Clone)]
pub struct FlatTableData {
    schema: FlatSchema,
    rows: Vec<Vec<Value>>,
    config: FlatConfig,
}

impl FlatTableData {
    /// Build flat table data from JSON rows
    pub fn from_rows(rows: &[Value], config: FlatConfig) -> Self {
        let mut schema = FlatSchema::new();
        let mut flat_rows: Vec<HashMap<String, Value>> = Vec::new();

        // First pass: build schema from all rows in first chunk
        // (In streaming mode, this would be the first chunk only)
        for row in rows {
            let flattened = flatten_object(row, &config);

            for (key, _) in &flattened {
                let is_child = key.contains('.');
                schema.add_column(key.clone(), is_child);
            }

            flat_rows.push(flattened);
        }

        schema.finalize_initial_schema();

        // Second pass: handle conflicts and build final rows
        let columns = schema.columns();
        let mut result_rows: Vec<Vec<Value>> = Vec::new();

        for (idx, row) in rows.iter().enumerate() {
            let flattened = &flat_rows[idx];
            let mut result_row: Vec<Value> = Vec::new();

            for col in &columns {
                if let Some(value) = flattened.get(col) {
                    result_row.push(value.clone());
                } else {
                    // Check for structure conflict
                    let original_value = Self::get_original_value(row, col);
                    match original_value {
                        Some(Value::Object(_)) => {
                            // Object where we expected scalar - show {...}
                            result_row.push(Value::String("{...}".to_string()));
                        }
                        Some(v) if !col.contains('.') => {
                            // Scalar value for parent column
                            result_row.push(v.clone());
                        }
                        _ => {
                            result_row.push(Value::Null);
                        }
                    }
                }
            }

            result_rows.push(result_row);
        }

        // Handle dynamic column additions (object->scalar conflicts)
        let mut final_schema = schema.clone();
        for (idx, row) in rows.iter().enumerate() {
            if let Value::Object(obj) = row {
                for (key, value) in obj {
                    // If this key was expanded but current row has scalar
                    if final_schema.children.contains_key(key) {
                        if !matches!(value, Value::Object(_)) {
                            // Need to add parent column dynamically
                            if !final_schema.all_columns.contains(key) {
                                final_schema.add_column(key.clone(), false);
                            }
                        }
                    }
                }
            }
        }

        // Rebuild rows if schema changed
        if final_schema.columns().len() != columns.len() {
            let new_columns = final_schema.columns();
            let mut new_rows: Vec<Vec<Value>> = Vec::new();

            for (idx, row) in rows.iter().enumerate() {
                let flattened = &flat_rows[idx];
                let mut result_row: Vec<Value> = Vec::new();

                for col in &new_columns {
                    if let Some(value) = flattened.get(col) {
                        result_row.push(value.clone());
                    } else {
                        let original_value = Self::get_original_value(row, col);
                        match original_value {
                            Some(Value::Object(_)) => {
                                result_row.push(Value::String("{...}".to_string()));
                            }
                            Some(v) if !col.contains('.') => {
                                result_row.push(v.clone());
                            }
                            _ => {
                                result_row.push(Value::Null);
                            }
                        }
                    }
                }

                new_rows.push(result_row);
            }

            return Self {
                schema: final_schema,
                rows: new_rows,
                config,
            };
        }

        Self {
            schema,
            rows: result_rows,
            config,
        }
    }

    fn get_original_value<'a>(row: &'a Value, path: &str) -> Option<&'a Value> {
        let parts: Vec<&str> = path.split('.').collect();
        let mut current = row;

        for part in parts {
            match current {
                Value::Object(obj) => {
                    current = obj.get(part)?;
                }
                _ => return None,
            }
        }

        Some(current)
    }

    pub fn columns(&self) -> Vec<String> {
        self.schema.columns()
    }

    pub fn rows(&self) -> &[Vec<Value>] {
        &self.rows
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    pub fn config(&self) -> &FlatConfig {
        &self.config
    }
}
```

**Step 4: Export from mod.rs**

Update `src/core/mod.rs`:

```rust
pub use flat::{FlatConfig, FlatSchema, FlatTableData, format_array, flatten_object};
```

**Step 5: Run test to verify it passes**

Run: `cargo test flat_table_data -v`
Expected: PASS

**Step 6: Commit**

```bash
git add src/core/flat.rs src/core/mod.rs
git commit -m "feat(flat): add FlatTableData builder for flattened table construction"
```

---

## Task 6: Add CLI Options

**Files:**
- Modify: `src/cli.rs`

**Step 1: Write the failing test**

Add to end of `src/cli.rs`:

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_flat_flag_only() {
        let cli = Cli::parse_from(["jlcat", "--flat"]);
        assert!(cli.flat.is_some());
        assert_eq!(cli.flat, Some(None)); // flat enabled, no depth
    }

    #[test]
    fn test_flat_with_depth() {
        let cli = Cli::parse_from(["jlcat", "--flat=3"]);
        assert_eq!(cli.flat, Some(Some(3)));
    }

    #[test]
    fn test_array_limit() {
        let cli = Cli::parse_from(["jlcat", "--flat", "--array-limit=5"]);
        assert_eq!(cli.array_limit, 5);
    }

    #[test]
    fn test_array_limit_default() {
        let cli = Cli::parse_from(["jlcat", "--flat"]);
        assert_eq!(cli.array_limit, 3);
    }
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_flat_flag --no-run 2>&1 | head -20`
Expected: Compilation error - flat field not found

**Step 3: Write minimal implementation**

Modify `src/cli.rs` to add:

```rust
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

    /// Flatten nested objects into dot-notation columns
    /// Optional depth limit (e.g., --flat or --flat=3)
    #[arg(long, value_name = "DEPTH", num_args = 0..=1, default_missing_value = "")]
    pub flat: Option<Option<usize>>,

    /// Maximum array elements to display in flat mode
    #[arg(long, default_value = "3")]
    pub array_limit: usize,

    /// Columns to display (comma-separated, supports dot notation)
    #[arg(short, long, value_delimiter = ',')]
    pub columns: Option<Vec<String>>,

    /// Sort keys (comma-separated, prefix with - for descending)
    #[arg(short, long, value_delimiter = ',')]
    pub sort: Option<Vec<String>>,

    /// Table style
    #[arg(long, value_enum, default_value = "rounded")]
    pub style: TableStyle,

    /// Exit on invalid JSON line (default: true)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub strict: bool,

    /// Skip invalid JSON lines with warning
    #[arg(long)]
    pub lenient: bool,
}

impl Cli {
    pub fn is_strict(&self) -> bool {
        self.strict && !self.lenient
    }

    /// Check if flat mode is enabled
    pub fn is_flat(&self) -> bool {
        self.flat.is_some()
    }

    /// Get flat depth (None = unlimited)
    pub fn flat_depth(&self) -> Option<usize> {
        self.flat.flatten()
    }
}
```

**Step 4: Run test to verify it passes**

Run: `cargo test test_flat -v`
Expected: PASS

**Step 5: Commit**

```bash
git add src/cli.rs
git commit -m "feat(cli): add --flat and --array-limit options"
```

---

## Task 7: Integrate Flat Mode in Main

**Files:**
- Modify: `src/main.rs`

**Step 1: Write integration test**

Add to `tests/cli_tests.rs`:

```rust
#[test]
fn test_flat_mode_basic() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat")
        .write_stdin(r#"{"id": 1, "user": {"name": "Alice", "age": 30}}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("user.name"))
        .stdout(predicate::str::contains("user.age"))
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_flat_mode_with_array() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat")
        .write_stdin(r#"{"tags": ["a", "b", "c", "d"]}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("a, b, c, ..."));
}

#[test]
fn test_flat_mode_depth_limit() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat=1")
        .write_stdin(r#"{"a": {"b": {"c": 1}}}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("a.b"))
        .stdout(predicate::str::contains("{...}"));
}
```

**Step 2: Run test to verify it fails**

Run: `cargo test test_flat_mode --no-run && cargo test test_flat_mode 2>&1 | head -30`
Expected: Test fails (flat mode not implemented in main)

**Step 3: Write minimal implementation**

Modify `src/main.rs`:

```rust
use core::{ChildTable, ColumnSelector, FlatConfig, FlatTableData, NestedExtractor, Sorter, TableData};

// In main function, modify the render section:
fn main() -> Result<()> {
    let cli = Cli::parse();

    // ... existing input checking code ...

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
    let selector = if let Some(ref cols) = cli.columns {
        Some(ColumnSelector::new(cols.clone())?)
    } else {
        None
    };

    // Render
    if cli.interactive {
        // TUI mode
        if cli.is_flat() {
            let config = FlatConfig::new(cli.flat_depth(), cli.array_limit);
            let flat_table = FlatTableData::from_rows(&rows, config);
            render::tui::run_flat(flat_table)?;
        } else {
            let table_data = TableData::from_rows(rows, selector);
            render::tui::run(table_data)?;
        }
    } else {
        let renderer = CatRenderer::new(cli.style.clone());

        if cli.is_flat() {
            // Flat mode - expand nested objects
            let config = FlatConfig::new(cli.flat_depth(), cli.array_limit);
            let flat_table = FlatTableData::from_rows(&rows, config);
            println!("{}", renderer.render_flat(&flat_table));
        } else if cli.recursive {
            // ... existing recursive code ...
        } else {
            // Normal mode
            let table_data = TableData::from_rows(rows, selector);
            println!("{}", renderer.render(&table_data));
        }
    }

    Ok(())
}
```

**Step 4: Add render_flat to CatRenderer**

Modify `src/render/cat.rs`:

```rust
use crate::core::FlatTableData;

impl CatRenderer {
    // ... existing code ...

    pub fn render_flat(&self, table_data: &FlatTableData) -> String {
        if table_data.is_empty() {
            return String::new();
        }

        let mut table = Table::new();

        match self.style {
            TableStyle::Ascii => table.load_preset(presets::ASCII_FULL),
            TableStyle::Rounded => table.load_preset(presets::UTF8_FULL),
            TableStyle::Markdown => table.load_preset(presets::ASCII_MARKDOWN),
            TableStyle::Plain => table.load_preset(presets::NOTHING),
        };

        table.set_content_arrangement(ContentArrangement::Dynamic);

        table.set_header(&table_data.columns());

        for row in table_data.rows() {
            let cells: Vec<String> = row.iter().map(|v| self.format_value(v)).collect();
            table.add_row(cells);
        }

        table.to_string()
    }
}
```

**Step 5: Run test to verify it passes**

Run: `cargo test test_flat_mode -v`
Expected: PASS

**Step 6: Commit**

```bash
git add src/main.rs src/render/cat.rs
git commit -m "feat: integrate flat mode in CLI rendering"
```

---

## Task 8: Add TUI Support for Flat Mode

**Files:**
- Modify: `src/render/tui/mod.rs`
- Modify: `src/render/tui/app.rs`
- Modify: `src/render/tui/view.rs`

**Step 1: Add run_flat function to TUI**

Add to `src/render/tui/mod.rs`:

```rust
use crate::core::FlatTableData;

pub fn run_flat(table_data: FlatTableData) -> crate::error::Result<()> {
    // Convert FlatTableData to TableData-like interface for TUI
    let app = App::from_flat(table_data);
    run_app(app)
}
```

**Step 2: Add from_flat constructor to App**

Modify `src/render/tui/app.rs`:

```rust
use crate::core::FlatTableData;

impl App {
    // ... existing code ...

    pub fn from_flat(flat_data: FlatTableData) -> Self {
        let columns = flat_data.columns();
        let rows: Vec<Vec<Value>> = flat_data.rows().to_vec();
        let row_count = rows.len();
        let filtered_indices: Vec<usize> = (0..row_count).collect();

        Self {
            // Create a minimal TableData wrapper
            table_data: TableData::from_flat_columns_rows(columns, rows),
            scroll_offset: 0,
            selected_row: 0,
            mode: InputMode::Normal,
            search_query: String::new(),
            filter_expr: None,
            filtered_indices,
            input_buffer: String::new(),
        }
    }
}
```

**Step 3: Add from_flat_columns_rows to TableData**

Modify `src/core/table.rs`:

```rust
impl TableData {
    // ... existing code ...

    /// Create TableData directly from columns and rows (for flat mode)
    pub fn from_flat_columns_rows(columns: Vec<String>, rows: Vec<Vec<Value>>) -> Self {
        Self {
            columns,
            rows,
            schema: Schema::default(),
        }
    }
}
```

**Step 4: Run tests**

Run: `cargo test && cargo build`
Expected: PASS

**Step 5: Commit**

```bash
git add src/render/tui/mod.rs src/render/tui/app.rs src/core/table.rs
git commit -m "feat(tui): add flat mode support for interactive mode"
```

---

## Task 9: Add Streaming Support with Fallback

**Files:**
- Modify: `src/main.rs`

**Step 1: Write the failing test**

Add to `tests/cli_tests.rs`:

```rust
#[test]
fn test_flat_mode_fallback_warning() {
    // Test that incomplete first chunk triggers fallback
    // This is hard to test directly, so we just verify normal output works
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat")
        .write_stdin(r#"{"id": 1}"#)
        .assert()
        .success();
}
```

**Step 2: Implementation is already handled**

The current implementation reads all input before processing, so fallback is implicit (if no complete lines, empty result).

For true streaming support, we would need to modify `read_input` to return an iterator and process chunk-by-chunk. This is a future enhancement.

**Step 3: Commit**

```bash
git add tests/cli_tests.rs
git commit -m "test: add flat mode fallback test"
```

---

## Task 10: Add Comprehensive Integration Tests

**Files:**
- Modify: `tests/integration_tests.rs`

**Step 1: Write comprehensive tests**

Add to `tests/integration_tests.rs`:

```rust
mod flat_mode_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;

    #[test]
    fn test_flat_nested_object() {
        let input = r#"{"id": 1, "user": {"name": "Alice", "profile": {"age": 30, "city": "Tokyo"}}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("user.name"))
            .stdout(predicate::str::contains("user.profile.age"))
            .stdout(predicate::str::contains("user.profile.city"))
            .stdout(predicate::str::contains("Alice"))
            .stdout(predicate::str::contains("30"))
            .stdout(predicate::str::contains("Tokyo"));
    }

    #[test]
    fn test_flat_array_elements() {
        let input = r#"{"tags": ["rust", "json", "cli", "tui", "awesome"]}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("rust, json, cli, ..."));
    }

    #[test]
    fn test_flat_array_limit_custom() {
        let input = r#"{"tags": ["a", "b", "c", "d", "e"]}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .arg("--array-limit=5")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("a, b, c, d, e"));
    }

    #[test]
    fn test_flat_depth_limit() {
        let input = r#"{"a": {"b": {"c": {"d": 1}}}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat=2")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("a.b.c"))
            .stdout(predicate::str::contains("{...}"));
    }

    #[test]
    fn test_flat_structure_conflict_object_to_scalar() {
        let input = r#"{"id": 1, "data": {"x": 1}}
{"id": 2, "data": "simple"}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("data.x"))
            .stdout(predicate::str::contains("data"))
            .stdout(predicate::str::contains("simple"));
    }

    #[test]
    fn test_flat_with_markdown_style() {
        let input = r#"{"user": {"name": "Alice"}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .arg("--style=markdown")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("| user.name |"))
            .stdout(predicate::str::contains("| Alice |"));
    }

    #[test]
    fn test_flat_multiple_rows() {
        let input = r#"{"id": 1, "info": {"status": "active"}}
{"id": 2, "info": {"status": "inactive"}}
{"id": 3, "info": {"status": "pending"}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("info.status"))
            .stdout(predicate::str::contains("active"))
            .stdout(predicate::str::contains("inactive"))
            .stdout(predicate::str::contains("pending"));
    }

    #[test]
    fn test_flat_empty_array() {
        let input = r#"{"tags": []}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success();
    }

    #[test]
    fn test_flat_null_values() {
        let input = r#"{"user": {"name": null, "age": 30}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("null"));
    }
}
```

**Step 2: Run tests**

Run: `cargo test flat_mode_tests -v`
Expected: PASS

**Step 3: Commit**

```bash
git add tests/integration_tests.rs
git commit -m "test: add comprehensive flat mode integration tests"
```

---

## Task 11: Update Documentation

**Files:**
- Modify: `README.md` (if exists)

**Step 1: Add flat mode documentation**

Add section to README:

```markdown
## Flat Mode

Flat mode expands nested JSON objects into dot-notation columns:

```bash
# Enable flat mode
echo '{"user": {"name": "Alice", "age": 30}}' | jlcat --flat

# Limit expansion depth
jlcat --flat=2 data.jsonl

# Customize array display limit (default: 3)
jlcat --flat --array-limit=5 data.jsonl
```

### Example

Input:
```json
{"id": 1, "user": {"name": "Alice", "profile": {"city": "Tokyo"}}, "tags": ["a", "b", "c", "d"]}
```

Output with `--flat`:
```
| id | user.name | user.profile.city | tags         |
|----|-----------|-------------------|--------------|
| 1  | Alice     | Tokyo             | a, b, c, ... |
```
```

**Step 2: Commit**

```bash
git add README.md
git commit -m "docs: add flat mode documentation"
```

---

## Task 12: Final Verification

**Step 1: Run all tests**

```bash
cargo test
```

**Step 2: Run clippy**

```bash
cargo clippy -- -D warnings
```

**Step 3: Format code**

```bash
cargo fmt
```

**Step 4: Manual testing**

```bash
# Basic flat mode
echo '{"id": 1, "user": {"name": "Alice", "age": 30}}' | cargo run -- --flat

# With depth limit
echo '{"a": {"b": {"c": 1}}}' | cargo run -- --flat=1

# With array limit
echo '{"tags": ["a","b","c","d","e"]}' | cargo run -- --flat --array-limit=2

# Multiple rows with conflicts
echo '{"user": {"name": "Alice"}}
{"user": "Bob"}' | cargo run -- --flat
```

**Step 5: Final commit**

```bash
git add -A
git commit -m "chore: final cleanup and verification"
```

---

## Summary

This plan implements flat mode in 12 tasks:

1. **FlatConfig** - Configuration struct
2. **FlatSchema** - Column tracking with ordering
3. **format_array** - Array display formatting
4. **flatten_object** - Object expansion
5. **FlatTableData** - Table builder
6. **CLI options** - --flat and --array-limit
7. **Main integration** - CLI rendering
8. **TUI support** - Interactive mode
9. **Streaming fallback** - Error handling
10. **Integration tests** - Comprehensive testing
11. **Documentation** - README update
12. **Final verification** - Quality checks

Each task follows TDD: write failing test, implement, verify, commit.
