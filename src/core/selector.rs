use super::path::CompiledPath;
use crate::error::Result;
use serde_json::Value;

#[derive(Debug, Clone)]
pub struct ColumnSelector {
    columns: Vec<(String, CompiledPath)>, // (original_name, compiled_path)
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

    pub fn select_values(&self, row: &Value) -> Vec<Value> {
        self.columns
            .iter()
            .map(|(_, path)| path.get(row).cloned().unwrap_or(Value::Null))
            .collect()
    }
}

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

    #[test]
    fn test_columns_list() {
        let selector = ColumnSelector::new(vec!["id".into(), "name".into()]).unwrap();
        assert_eq!(selector.columns(), vec!["id", "name"]);
    }

    #[test]
    fn test_select_values() {
        let selector = ColumnSelector::new(vec!["id".into(), "name".into()]).unwrap();
        let row = json!({"id": 1, "name": "Alice"});

        let values = selector.select_values(&row);

        assert_eq!(values, vec![json!(1), json!("Alice")]);
    }
}
