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
        let rows = vec![json!({"id": 1, "address": {"city": "Tokyo"}})];

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
        // String + Null = String (nullable string, not Mixed)
        assert_eq!(schema.column_type("name"), Some(ColumnType::String));
        assert_eq!(schema.column_type("active"), Some(ColumnType::Bool));
    }

    #[test]
    fn test_mixed_types() {
        let rows = vec![
            json!({"value": 1}),
            json!({"value": "string"}),
        ];

        let schema = SchemaInferrer::infer(&rows);

        // Number + String = Mixed
        assert_eq!(schema.column_type("value"), Some(ColumnType::Mixed));
    }

    #[test]
    fn test_streaming_inference() {
        let mut schema = Schema::new();

        SchemaInferrer::infer_streaming(&json!({"id": 1, "name": "Alice"}), &mut schema);
        assert_eq!(schema.columns().len(), 2);

        SchemaInferrer::infer_streaming(&json!({"id": 2, "age": 30}), &mut schema);
        assert_eq!(schema.columns().len(), 3);
        assert!(schema.columns().contains(&"age".to_string()));
    }
}
