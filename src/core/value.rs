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

/// Helper function to get a nested value using dot notation.
/// First tries literal key lookup (for flattened column names like "address.city"),
/// then falls back to nested path traversal.
pub fn get_nested_value<'a>(value: &'a Value, path: &str) -> Option<&'a Value> {
    // First try literal key (for flattened column names like "address.city")
    if path.contains('.') || path.contains('[') {
        if let Some(v) = value.get(path) {
            return Some(v);
        }
    }

    // Fall back to nested path lookup
    let mut current = value;

    for part in path.split('.') {
        if part.is_empty() {
            continue;
        }

        // Handle array index notation (supports multiple indices like matrix[1][0])
        if let Some(first_bracket) = part.find('[') {
            let field = &part[..first_bracket];
            if !field.is_empty() {
                current = current.get(field)?;
            }

            // Process all [index] pairs in this segment
            let mut remaining = &part[first_bracket..];
            while remaining.starts_with('[') {
                let idx_end = remaining.find(']')?;
                let idx: usize = remaining[1..idx_end].parse().ok()?;
                current = current.get(idx)?;
                remaining = &remaining[idx_end + 1..];
            }
        } else {
            current = current.get(part)?;
        }
    }

    Some(current)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_ordering_numbers() {
        let j1 = json!(1);
        let j2 = json!(2);
        let j3 = json!(10);
        let v1 = SortableValue::new(&j1);
        let v2 = SortableValue::new(&j2);
        let v3 = SortableValue::new(&j3);

        assert!(v1 < v2);
        assert!(v2 < v3);
    }

    #[test]
    fn test_ordering_strings() {
        let j1 = json!("alice");
        let j2 = json!("bob");
        let v1 = SortableValue::new(&j1);
        let v2 = SortableValue::new(&j2);

        assert!(v1 < v2);
    }

    #[test]
    fn test_ordering_null_last() {
        let j1 = json!(1);
        let j2 = json!(null);
        let j3 = json!("z");
        let v1 = SortableValue::new(&j1);
        let v2 = SortableValue::new(&j2);
        let v3 = SortableValue::new(&j3);

        assert!(v1 < v2);
        assert!(v3 < v2);
    }

    #[test]
    fn test_ordering_mixed_types() {
        // numbers < strings < bools < null
        let jnum = json!(100);
        let jstr = json!("a");
        let jbool = json!(true);
        let jnull = json!(null);
        let num = SortableValue::new(&jnum);
        let str_val = SortableValue::new(&jstr);
        let bool_val = SortableValue::new(&jbool);
        let null_val = SortableValue::new(&jnull);

        assert!(num < str_val);
        assert!(str_val < bool_val);
        assert!(bool_val < null_val);
    }

    #[test]
    fn test_get_nested_simple() {
        let row = json!({"name": "Alice"});
        assert_eq!(get_nested_value(&row, "name"), Some(&json!("Alice")));
    }

    #[test]
    fn test_get_nested_deep() {
        let row = json!({"address": {"city": "Tokyo"}});
        assert_eq!(get_nested_value(&row, "address.city"), Some(&json!("Tokyo")));
    }

    #[test]
    fn test_get_nested_array() {
        let row = json!({"items": [1, 2, 3]});
        assert_eq!(get_nested_value(&row, "items[1]"), Some(&json!(2)));
    }

    #[test]
    fn test_get_literal_dotted_key() {
        // When column selection flattens "address.city" into a literal key
        let row = json!({"address.city": "Tokyo"});
        assert_eq!(get_nested_value(&row, "address.city"), Some(&json!("Tokyo")));
    }

    #[test]
    fn test_get_prefers_literal_over_nested() {
        // Literal key takes precedence when both exist
        let row = json!({
            "address.city": "Literal",
            "address": {"city": "Nested"}
        });
        assert_eq!(get_nested_value(&row, "address.city"), Some(&json!("Literal")));
    }

    #[test]
    fn test_get_literal_array_key() {
        // Literal key with bracket notation
        let row = json!({"items[0]": "Literal"});
        assert_eq!(get_nested_value(&row, "items[0]"), Some(&json!("Literal")));
    }

    #[test]
    fn test_get_multi_dimensional_array() {
        // 2D matrix access: matrix[1][0]
        let row = json!({"matrix": [[1, 2], [3, 4], [5, 6]]});
        assert_eq!(get_nested_value(&row, "matrix[1][0]"), Some(&json!(3)));
        assert_eq!(get_nested_value(&row, "matrix[2][1]"), Some(&json!(6)));
    }

    #[test]
    fn test_get_triple_nested_array() {
        // 3D array access: cube[0][1][2]
        let row = json!({"cube": [[[1, 2, 3], [4, 5, 6]], [[7, 8, 9], [10, 11, 12]]]});
        assert_eq!(get_nested_value(&row, "cube[0][1][2]"), Some(&json!(6)));
        assert_eq!(get_nested_value(&row, "cube[1][0][0]"), Some(&json!(7)));
    }

    #[test]
    fn test_get_nested_path_with_multi_index() {
        // Combined dot notation and multi-dimensional array
        let row = json!({"data": {"matrix": [[1, 2], [3, 4]]}});
        assert_eq!(get_nested_value(&row, "data.matrix[1][0]"), Some(&json!(3)));
    }
}
