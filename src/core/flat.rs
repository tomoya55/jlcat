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
    pub fn is_dynamic_column(&self, path: &str) -> bool {
        self.dynamic_columns.contains(path)
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
        .map(format_array_element)
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
}
