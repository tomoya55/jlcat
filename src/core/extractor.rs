use serde_json::Value;
use std::collections::HashMap;

/// Represents an extracted child table from nested data
#[derive(Debug, Clone)]
pub struct ChildTable {
    /// Name of the nested field (e.g., "orders", "address")
    pub name: String,
    /// Column names for this child table
    pub columns: Vec<String>,
    /// Rows with (parent_row_index, values)
    pub rows: Vec<(usize, Vec<Value>)>,
}

impl ChildTable {
    pub fn new(name: String) -> Self {
        Self {
            name,
            columns: Vec::new(),
            rows: Vec::new(),
        }
    }

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }

    /// Get columns with _parent_row prepended
    pub fn columns_with_parent(&self) -> Vec<String> {
        let mut cols = vec!["_parent_row".to_string()];
        cols.extend(self.columns.clone());
        cols
    }

    /// Get rows as Vec<Vec<Value>> with parent row index as first column
    /// Pads short rows with nulls to match column count (for heterogeneous arrays)
    pub fn rows_with_parent(&self) -> Vec<Vec<Value>> {
        let col_count = self.columns.len();
        self.rows
            .iter()
            .map(|(parent_idx, values)| {
                let mut row = vec![Value::Number((*parent_idx as i64).into())];
                row.extend(values.clone());
                // Pad with nulls if row is shorter than expected
                while row.len() < col_count + 1 {
                    row.push(Value::Null);
                }
                row
            })
            .collect()
    }
}

/// Extracts nested objects and arrays from JSON rows into child tables
pub struct NestedExtractor;

impl NestedExtractor {
    /// Extract all nested structures from rows (recursively)
    /// Returns a map of field_path -> ChildTable
    /// Nested structures use dotted paths (e.g., "user.address" for address inside user)
    pub fn extract(rows: &[Value]) -> HashMap<String, ChildTable> {
        let mut children: HashMap<String, ChildTable> = HashMap::new();

        for (row_idx, row) in rows.iter().enumerate() {
            if let Value::Object(obj) = row {
                for (key, value) in obj {
                    match value {
                        Value::Object(nested_obj) => {
                            Self::extract_object_recursive(&mut children, key, row_idx, nested_obj);
                        }
                        Value::Array(arr) => {
                            Self::extract_array_recursive(&mut children, key, row_idx, arr);
                        }
                        _ => {}
                    }
                }
            }
        }

        children
    }

    /// Extract a nested object into a child table row (recursively)
    fn extract_object_recursive(
        children: &mut HashMap<String, ChildTable>,
        path: &str,
        row_idx: usize,
        obj: &serde_json::Map<String, Value>,
    ) {
        // Collect nested structures to process after releasing borrow
        let mut nested_to_process: Vec<(String, Value)> = Vec::new();

        {
            let child = children
                .entry(path.to_string())
                .or_insert_with(|| ChildTable::new(path.to_string()));

            // Collect all keys from this object and add any new columns
            for obj_key in obj.keys() {
                if !child.columns.contains(obj_key) {
                    child.columns.push(obj_key.clone());
                }
            }

            // Create row with values in column order (flatten nested for display)
            let values: Vec<Value> = child
                .columns
                .iter()
                .map(|col| {
                    obj.get(col)
                        .map(|v| Self::flatten_value(v))
                        .unwrap_or(Value::Null)
                })
                .collect();

            child.rows.push((row_idx, values));

            // Collect nested structures for later processing
            for (key, value) in obj {
                match value {
                    Value::Object(_) | Value::Array(_) => {
                        let nested_path = format!("{}.{}", path, key);
                        nested_to_process.push((nested_path, value.clone()));
                    }
                    _ => {}
                }
            }
        }

        // Now process nested structures (borrow released)
        for (nested_path, value) in nested_to_process {
            match &value {
                Value::Object(nested_obj) => {
                    Self::extract_object_recursive(children, &nested_path, row_idx, nested_obj);
                }
                Value::Array(arr) => {
                    Self::extract_array_recursive(children, &nested_path, row_idx, arr);
                }
                _ => {}
            }
        }
    }

    /// Extract array elements into child table rows (recursively)
    fn extract_array_recursive(
        children: &mut HashMap<String, ChildTable>,
        path: &str,
        row_idx: usize,
        arr: &[Value],
    ) {
        // Collect nested structures to process after releasing borrow
        let mut nested_to_process: Vec<(String, Value)> = Vec::new();

        {
            let child = children
                .entry(path.to_string())
                .or_insert_with(|| ChildTable::new(path.to_string()));

            for element in arr {
                match element {
                    Value::Object(obj) => {
                        // Add columns from this object
                        for obj_key in obj.keys() {
                            if !child.columns.contains(obj_key) {
                                child.columns.push(obj_key.clone());
                            }
                        }

                        // Create row with flattened values
                        let values: Vec<Value> = child
                            .columns
                            .iter()
                            .map(|col| {
                                obj.get(col)
                                    .map(|v| Self::flatten_value(v))
                                    .unwrap_or(Value::Null)
                            })
                            .collect();

                        child.rows.push((row_idx, values));

                        // Collect nested structures for later processing
                        for (key, value) in obj {
                            match value {
                                Value::Object(_) | Value::Array(_) => {
                                    let nested_path = format!("{}.{}", path, key);
                                    nested_to_process.push((nested_path, value.clone()));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {
                        // For primitive arrays, use a "value" column
                        if !child.columns.contains(&"value".to_string()) {
                            child.columns.push("value".to_string());
                        }

                        // Build row with nulls for all columns except "value"
                        let values: Vec<Value> = child
                            .columns
                            .iter()
                            .map(|col| {
                                if col == "value" {
                                    element.clone()
                                } else {
                                    Value::Null
                                }
                            })
                            .collect();

                        child.rows.push((row_idx, values));
                    }
                }
            }

            // Normalize all rows to have the same number of columns
            let col_count = child.columns.len();
            for (_, values) in &mut child.rows {
                while values.len() < col_count {
                    values.push(Value::Null);
                }
            }
        }

        // Now process nested structures (borrow released)
        for (nested_path, value) in nested_to_process {
            match &value {
                Value::Object(obj) => {
                    Self::extract_object_recursive(children, &nested_path, row_idx, obj);
                }
                Value::Array(arr) => {
                    Self::extract_array_recursive(children, &nested_path, row_idx, arr);
                }
                _ => {}
            }
        }
    }

    /// Flatten a value for display in parent table (replace nested with placeholder)
    fn flatten_value(value: &Value) -> Value {
        match value {
            Value::Object(_) => Value::String("{...}".to_string()),
            Value::Array(_) => Value::String("[...]".to_string()),
            _ => value.clone(),
        }
    }

    /// Get flattened parent row with nested values replaced by placeholder
    pub fn flatten_row(row: &Value) -> Value {
        if let Value::Object(obj) = row {
            let mut flat = serde_json::Map::new();
            for (key, value) in obj {
                match value {
                    Value::Object(_) => {
                        flat.insert(key.clone(), Value::String("{...}".to_string()));
                    }
                    Value::Array(_) => {
                        flat.insert(key.clone(), Value::String("[...]".to_string()));
                    }
                    _ => {
                        flat.insert(key.clone(), value.clone());
                    }
                }
            }
            Value::Object(flat)
        } else {
            row.clone()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_extract_nested_object() {
        let rows = vec![
            json!({"id": 1, "address": {"city": "Tokyo", "zip": "100"}}),
            json!({"id": 2, "address": {"city": "Osaka", "zip": "530"}}),
        ];

        let children = NestedExtractor::extract(&rows);

        assert!(children.contains_key("address"));
        let address = &children["address"];
        assert_eq!(address.rows.len(), 2);
        assert!(address.columns.contains(&"city".to_string()));
        assert!(address.columns.contains(&"zip".to_string()));
    }

    #[test]
    fn test_extract_array_of_objects() {
        let rows = vec![json!({
            "id": 1,
            "orders": [
                {"item": "Apple", "qty": 2},
                {"item": "Banana", "qty": 3}
            ]
        })];

        let children = NestedExtractor::extract(&rows);

        assert!(children.contains_key("orders"));
        let orders = &children["orders"];
        assert_eq!(orders.rows.len(), 2); // 2 array elements
        assert!(orders.columns.contains(&"item".to_string()));
        assert!(orders.columns.contains(&"qty".to_string()));

        // Both rows should reference parent row 0
        assert_eq!(orders.rows[0].0, 0);
        assert_eq!(orders.rows[1].0, 0);
    }

    #[test]
    fn test_extract_primitive_array() {
        let rows = vec![json!({"id": 1, "tags": ["a", "b", "c"]})];

        let children = NestedExtractor::extract(&rows);

        assert!(children.contains_key("tags"));
        let tags = &children["tags"];
        assert_eq!(tags.rows.len(), 3);
        assert_eq!(tags.columns, vec!["value"]);
    }

    #[test]
    fn test_extract_multiple_parents() {
        let rows = vec![
            json!({"id": 1, "items": [{"name": "A"}]}),
            json!({"id": 2, "items": [{"name": "B"}, {"name": "C"}]}),
        ];

        let children = NestedExtractor::extract(&rows);
        let items = &children["items"];

        assert_eq!(items.rows.len(), 3); // 1 from row 0, 2 from row 1
        assert_eq!(items.rows[0].0, 0); // First item from parent 0
        assert_eq!(items.rows[1].0, 1); // Second item from parent 1
        assert_eq!(items.rows[2].0, 1); // Third item from parent 1
    }

    #[test]
    fn test_flatten_row() {
        let row = json!({
            "id": 1,
            "name": "Alice",
            "address": {"city": "Tokyo"},
            "orders": [1, 2, 3]
        });

        let flat = NestedExtractor::flatten_row(&row);

        assert_eq!(flat["id"], json!(1));
        assert_eq!(flat["name"], json!("Alice"));
        assert_eq!(flat["address"], json!("{...}"));
        assert_eq!(flat["orders"], json!("[...]"));
    }

    #[test]
    fn test_no_nested_data() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];

        let children = NestedExtractor::extract(&rows);

        assert!(children.is_empty());
    }

    #[test]
    fn test_mixed_nested_types() {
        let rows = vec![
            json!({"id": 1, "meta": {"type": "A"}}),
            json!({"id": 2}), // No meta field
            json!({"id": 3, "meta": {"type": "B", "extra": true}}),
        ];

        let children = NestedExtractor::extract(&rows);
        let meta = &children["meta"];

        assert_eq!(meta.rows.len(), 2); // Only rows with meta
        assert!(meta.columns.contains(&"type".to_string()));
        assert!(meta.columns.contains(&"extra".to_string()));
    }

    #[test]
    fn test_columns_with_parent() {
        let rows = vec![json!({"id": 1, "address": {"city": "Tokyo"}})];
        let children = NestedExtractor::extract(&rows);
        let address = &children["address"];

        let cols = address.columns_with_parent();
        assert_eq!(cols[0], "_parent_row");
        assert!(cols.contains(&"city".to_string()));
    }

    #[test]
    fn test_rows_with_parent() {
        let rows = vec![
            json!({"id": 1, "items": [{"name": "A"}]}),
            json!({"id": 2, "items": [{"name": "B"}]}),
        ];
        let children = NestedExtractor::extract(&rows);
        let items = &children["items"];

        let rows_with_parent = items.rows_with_parent();
        assert_eq!(rows_with_parent[0][0], json!(0)); // parent row 0
        assert_eq!(rows_with_parent[1][0], json!(1)); // parent row 1
    }

    #[test]
    fn test_heterogeneous_array() {
        // Array mixing objects and primitives
        let rows = vec![json!({
            "id": 1,
            "items": [{"name": "A"}, "B", {"name": "C"}]
        })];

        let children = NestedExtractor::extract(&rows);
        let items = &children["items"];

        // Should have both "name" and "value" columns
        assert!(items.columns.contains(&"name".to_string()));
        assert!(items.columns.contains(&"value".to_string()));
        assert_eq!(items.rows.len(), 3);

        // Each row should have the same number of values as columns
        for (_, values) in &items.rows {
            assert_eq!(
                values.len(),
                items.columns.len(),
                "Row values should match column count"
            );
        }

        // Object rows should have null in "value" column
        // Primitive rows should have null in "name" column
        let name_idx = items.columns.iter().position(|c| c == "name").unwrap();
        let value_idx = items.columns.iter().position(|c| c == "value").unwrap();

        // First row: object {"name": "A"}
        assert_eq!(items.rows[0].1[name_idx], json!("A"));
        assert_eq!(items.rows[0].1[value_idx], Value::Null);

        // Second row: primitive "B"
        assert_eq!(items.rows[1].1[name_idx], Value::Null);
        assert_eq!(items.rows[1].1[value_idx], json!("B"));

        // Third row: object {"name": "C"}
        assert_eq!(items.rows[2].1[name_idx], json!("C"));
        assert_eq!(items.rows[2].1[value_idx], Value::Null);
    }

    #[test]
    fn test_recursive_nested_objects() {
        // Deeply nested: user -> address -> coordinates
        let rows = vec![json!({
            "id": 1,
            "user": {
                "name": "Alice",
                "address": {
                    "city": "Tokyo",
                    "coordinates": {
                        "lat": 35.6762,
                        "lng": 139.6503
                    }
                }
            }
        })];

        let children = NestedExtractor::extract(&rows);

        // Should have child tables for all levels
        assert!(children.contains_key("user"), "Should have 'user' table");
        assert!(
            children.contains_key("user.address"),
            "Should have 'user.address' table"
        );
        assert!(
            children.contains_key("user.address.coordinates"),
            "Should have 'user.address.coordinates' table"
        );

        // Check user table has address as placeholder
        let user = &children["user"];
        assert!(user.columns.contains(&"name".to_string()));
        assert!(user.columns.contains(&"address".to_string()));

        // Check deepest level has actual values
        let coords = &children["user.address.coordinates"];
        assert!(coords.columns.contains(&"lat".to_string()));
        assert!(coords.columns.contains(&"lng".to_string()));
        assert_eq!(coords.rows.len(), 1);
    }

    #[test]
    fn test_recursive_array_with_nested_objects() {
        // Array elements containing nested objects
        let rows = vec![json!({
            "id": 1,
            "orders": [
                {
                    "item": "Apple",
                    "shipping": {"method": "express", "cost": 10}
                },
                {
                    "item": "Banana",
                    "shipping": {"method": "standard", "cost": 5}
                }
            ]
        })];

        let children = NestedExtractor::extract(&rows);

        // Should have child tables for orders and orders.shipping
        assert!(children.contains_key("orders"), "Should have 'orders' table");
        assert!(
            children.contains_key("orders.shipping"),
            "Should have 'orders.shipping' table"
        );

        // Check orders table
        let orders = &children["orders"];
        assert_eq!(orders.rows.len(), 2);
        assert!(orders.columns.contains(&"item".to_string()));
        assert!(orders.columns.contains(&"shipping".to_string()));

        // Check nested shipping table
        let shipping = &children["orders.shipping"];
        assert_eq!(shipping.rows.len(), 2);
        assert!(shipping.columns.contains(&"method".to_string()));
        assert!(shipping.columns.contains(&"cost".to_string()));
    }

    #[test]
    fn test_recursive_nested_arrays() {
        // Nested arrays: matrix containing arrays
        let rows = vec![json!({
            "id": 1,
            "data": {
                "tags": ["a", "b", "c"]
            }
        })];

        let children = NestedExtractor::extract(&rows);

        assert!(children.contains_key("data"), "Should have 'data' table");
        assert!(
            children.contains_key("data.tags"),
            "Should have 'data.tags' table"
        );

        let tags = &children["data.tags"];
        assert_eq!(tags.rows.len(), 3);
        assert_eq!(tags.columns, vec!["value"]);
    }
}
