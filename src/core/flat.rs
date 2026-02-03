use serde_json::Value;
use std::collections::{HashMap, HashSet};

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
    /// First-level columns that should appear even if they have children
    /// (for handling structure conflicts where a key is sometimes scalar, sometimes object)
    first_level_columns: HashSet<String>,
}

impl FlatSchema {
    pub fn new() -> Self {
        Self {
            first_level_order: Vec::new(),
            children: HashMap::new(),
            all_columns: HashSet::new(),
            dynamic_columns: HashSet::new(),
            finalized: false,
            first_level_columns: HashSet::new(),
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
            self.first_level_columns.insert(path.clone());
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
    #[allow(dead_code)]
    pub fn is_dynamic_column(&self, path: &str) -> bool {
        self.dynamic_columns.contains(path)
    }

    /// Check if a key has children (was expanded as an object)
    pub fn has_children(&self, key: &str) -> bool {
        self.children.contains_key(key)
    }

    /// Check if a column path exists in the schema
    pub fn contains_column(&self, path: &str) -> bool {
        self.all_columns.contains(path)
    }

    /// Get columns in proper order:
    /// - First-level keys in appearance order
    /// - Children sorted alphabetically under their parent's position
    pub fn columns(&self) -> Vec<String> {
        let mut result = Vec::new();

        for key in &self.first_level_order {
            // Check if this key was added as a first-level column (not just a parent)
            let is_first_level_col = self.first_level_columns.contains(key);
            let has_children = self.children.contains_key(key);

            if is_first_level_col {
                // Add the first-level column itself
                result.push(key.clone());
            }

            if has_children {
                // Add children (already sorted)
                result.extend(self.children.get(key).unwrap().clone());
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

/// Table data with flattened structure
#[derive(Debug, Clone)]
pub struct FlatTableData {
    schema: FlatSchema,
    rows: Vec<Vec<Value>>,
    #[allow(dead_code)]
    config: FlatConfig,
}

impl FlatTableData {
    /// Build flat table data from JSON rows
    pub fn from_rows(rows: &[Value], config: FlatConfig) -> Self {
        let mut schema = FlatSchema::new();
        let mut flat_rows: Vec<HashMap<String, Value>> = Vec::new();

        // First pass: build schema from all rows in first chunk
        // We process original JSON to preserve key order
        for row in rows {
            let flattened = flatten_object(row, &config);

            // Add columns by traversing original JSON structure (preserves order)
            Self::add_columns_from_json(&mut schema, row, "", 0, &config);

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
        for row in rows.iter() {
            if let Value::Object(obj) = row {
                for (key, value) in obj {
                    // If this key was expanded but current row has scalar
                    if final_schema.has_children(key) && !matches!(value, Value::Object(_)) {
                        // Need to add parent column dynamically
                        if !final_schema.contains_column(key) {
                            final_schema.add_column(key.clone(), false);
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

    /// Recursively add columns from JSON structure while preserving key order
    fn add_columns_from_json(
        schema: &mut FlatSchema,
        value: &Value,
        prefix: &str,
        depth: usize,
        config: &FlatConfig,
    ) {
        if let Value::Object(obj) = value {
            for (key, val) in obj {
                let full_key = if prefix.is_empty() {
                    key.clone()
                } else {
                    format!("{}.{}", prefix, key)
                };

                match val {
                    Value::Object(_) => {
                        // Check depth limit
                        if config.depth.is_none_or(|max| depth < max) {
                            // Expand the object - recurse but don't add parent as column
                            Self::add_columns_from_json(schema, val, &full_key, depth + 1, config);
                        } else {
                            // Depth limit reached - add as leaf column
                            let is_child = full_key.contains('.');
                            schema.add_column(full_key, is_child);
                        }
                    }
                    _ => {
                        // Scalar or array - add as column
                        let is_child = full_key.contains('.');
                        schema.add_column(full_key, is_child);
                    }
                }
            }
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

    #[allow(dead_code)]
    pub fn config(&self) -> &FlatConfig {
        &self.config
    }
}

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
                if config.depth.is_none_or(|max| depth < max) {
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

/// Format an array value for display with element limit
pub fn format_array(value: &Value, limit: usize) -> String {
    let arr = match value {
        Value::Array(a) => a,
        _ => return String::new(),
    };

    if arr.is_empty() {
        return String::new();
    }

    let formatted: Vec<String> = arr.iter().take(limit).map(format_array_element).collect();

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

#[cfg(test)]
mod tests {
    use super::*;
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
}
