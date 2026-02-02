use crate::cli::TableStyle;
use crate::core::TableData;
use comfy_table::{presets, ContentArrangement, Table};
use serde_json::Value;

pub struct CatRenderer {
    style: TableStyle,
}

impl CatRenderer {
    pub fn new(style: TableStyle) -> Self {
        Self { style }
    }

    pub fn render(&self, table_data: &TableData) -> String {
        if table_data.is_empty() {
            return String::new();
        }

        let mut table = Table::new();

        // Apply style
        match self.style {
            TableStyle::Ascii => table.load_preset(presets::ASCII_FULL),
            TableStyle::Rounded => table.load_preset(presets::UTF8_FULL),
            TableStyle::Markdown => table.load_preset(presets::ASCII_MARKDOWN),
            TableStyle::Plain => table.load_preset(presets::NOTHING),
        };

        table.set_content_arrangement(ContentArrangement::Dynamic);

        // Add header
        table.set_header(table_data.columns());

        // Add rows
        for row in table_data.rows() {
            let cells: Vec<String> = row.iter().map(|v| self.format_value(v)).collect();
            table.add_row(cells);
        }

        table.to_string()
    }

    fn format_value(&self, value: &Value) -> String {
        match value {
            Value::Null => "null".to_string(),
            Value::Bool(b) => b.to_string(),
            Value::Number(n) => n.to_string(),
            Value::String(s) => s.clone(),
            Value::Array(_) => "[...]".to_string(),
            Value::Object(_) => "{...}".to_string(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_render_simple_table() {
        let rows = vec![
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.contains("id"));
        assert!(output.contains("name"));
        assert!(output.contains("Alice"));
        assert!(output.contains("Bob"));
    }

    #[test]
    fn test_render_with_null() {
        let rows = vec![json!({"id": 1, "name": null})];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.contains("null"));
    }

    #[test]
    fn test_render_with_nested() {
        let rows = vec![json!({"id": 1, "data": {"nested": true}})];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.contains("{...}"));
    }

    #[test]
    fn test_render_with_array() {
        let rows = vec![json!({"id": 1, "items": [1, 2, 3]})];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.contains("[...]"));
    }

    #[test]
    fn test_render_empty_table() {
        let rows: Vec<Value> = vec![];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Rounded);

        let output = renderer.render(&table_data);

        assert!(output.is_empty());
    }

    #[test]
    fn test_render_ascii_style() {
        let rows = vec![json!({"id": 1})];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Ascii);

        let output = renderer.render(&table_data);

        assert!(output.contains("+"));
        assert!(output.contains("-"));
    }

    #[test]
    fn test_render_markdown_style() {
        let rows = vec![json!({"id": 1})];
        let table_data = TableData::from_rows(rows, None);
        let renderer = CatRenderer::new(TableStyle::Markdown);

        let output = renderer.render(&table_data);

        assert!(output.contains("|"));
    }
}
