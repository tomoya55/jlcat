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
            sel.columns().iter().map(|s| s.to_string()).collect()
        } else {
            schema.columns().to_vec()
        };

        let table_rows: Vec<Vec<Value>> = rows
            .iter()
            .map(|row| {
                columns
                    .iter()
                    .map(|col| get_nested_value(row, col).cloned().unwrap_or(Value::Null))
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

    pub fn is_empty(&self) -> bool {
        self.rows.is_empty()
    }
}

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
        let rows = vec![json!({"id": 1, "name": "Alice", "age": 30})];
        let selector = ColumnSelector::new(vec!["name".into(), "id".into()]).unwrap();

        let table = TableData::from_rows(rows, Some(selector));

        assert_eq!(table.columns(), &["name", "id"]);
    }

    #[test]
    fn test_table_data_get_cell() {
        let rows = vec![json!({"id": 1, "name": "Alice"})];

        let table = TableData::from_rows(rows, None);

        assert_eq!(table.get_cell(0, 0), Some(&json!(1)));
        assert_eq!(table.get_cell(0, 1), Some(&json!("Alice")));
    }

    #[test]
    fn test_table_data_get_row() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];

        let table = TableData::from_rows(rows, None);

        let row = table.get_row(1).unwrap();
        assert_eq!(row.len(), 2);
        assert_eq!(row[0], json!(2));
        assert_eq!(row[1], json!("Bob"));
    }

    #[test]
    fn test_table_data_nested_columns() {
        let rows = vec![json!({"id": 1, "address": {"city": "Tokyo"}})];
        let selector = ColumnSelector::new(vec!["id".into(), "address.city".into()]).unwrap();

        let table = TableData::from_rows(rows, Some(selector));

        assert_eq!(table.columns(), &["id", "address.city"]);
        assert_eq!(table.get_cell(0, 1), Some(&json!("Tokyo")));
    }

    #[test]
    fn test_table_data_missing_values() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2}), // missing name
        ];

        let table = TableData::from_rows(rows, None);

        assert_eq!(table.get_cell(0, 1), Some(&json!("Alice")));
        assert_eq!(table.get_cell(1, 1), Some(&Value::Null));
    }

    #[test]
    fn test_table_data_empty() {
        let rows: Vec<Value> = vec![];
        let table = TableData::from_rows(rows, None);

        assert!(table.is_empty());
        assert_eq!(table.row_count(), 0);
    }
}
