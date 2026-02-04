use crate::core::{FilterExpr, FlatTableData, FullTextSearch, TableData};
use serde_json::Value;

/// Application state for TUI mode
pub struct App {
    /// The table data to display
    table_data: TableData,
    /// Original JSON records (before flattening)
    source_records: Vec<Value>,
    /// Current scroll offset (first visible row)
    scroll_offset: usize,
    /// Currently selected row index (in filtered view)
    selected_row: usize,
    /// Current input mode
    pub mode: InputMode,
    /// Search query (full text)
    search_query: String,
    /// Filter expression
    filter_expr: Option<FilterExpr>,
    /// Indices of rows matching current filter/search
    filtered_indices: Vec<usize>,
    /// Input buffer for search/filter
    pub input_buffer: String,
    /// State for detail view modal (when in Detail mode)
    detail_state: Option<DetailViewState>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputMode {
    Normal,
    Search,
    Filter,
    Detail,
}

/// State for the detail view modal
#[derive(Debug, Clone)]
pub struct DetailViewState {
    /// Scroll offset (line number)
    pub scroll_offset: usize,
    /// Total lines in the rendered JSON
    pub total_lines: usize,
    /// Viewport height (updated by view)
    pub viewport_height: usize,
}

impl DetailViewState {
    pub fn new(total_lines: usize) -> Self {
        Self {
            scroll_offset: 0,
            total_lines,
            viewport_height: 20, // Default, will be updated by view
        }
    }

    pub fn set_viewport_height(&mut self, height: usize) {
        self.viewport_height = height;
    }

    pub fn scroll_up(&mut self, lines: usize) {
        self.scroll_offset = self.scroll_offset.saturating_sub(lines);
    }

    pub fn scroll_down(&mut self, lines: usize) {
        let max_offset = self.total_lines.saturating_sub(self.viewport_height);
        self.scroll_offset = (self.scroll_offset + lines).min(max_offset);
    }

    pub fn go_to_top(&mut self) {
        self.scroll_offset = 0;
    }

    pub fn go_to_bottom(&mut self) {
        self.scroll_offset = self.total_lines.saturating_sub(self.viewport_height);
    }
}

impl App {
    pub fn new(table_data: TableData, source_records: Vec<Value>) -> Self {
        let row_count = table_data.rows().len();
        let filtered_indices: Vec<usize> = (0..row_count).collect();

        Self {
            table_data,
            source_records,
            scroll_offset: 0,
            selected_row: 0,
            mode: InputMode::Normal,
            search_query: String::new(),
            filter_expr: None,
            filtered_indices,
            input_buffer: String::new(),
            detail_state: None,
        }
    }

    /// Create App from flat table data (for flat mode TUI)
    pub fn from_flat(flat_data: FlatTableData, source_records: Vec<Value>) -> Self {
        let columns = flat_data.columns();
        let rows: Vec<Vec<Value>> = flat_data.rows().to_vec();
        let row_count = rows.len();
        let filtered_indices: Vec<usize> = (0..row_count).collect();

        Self {
            table_data: TableData::from_flat_columns_rows(columns, rows),
            source_records,
            scroll_offset: 0,
            selected_row: 0,
            mode: InputMode::Normal,
            search_query: String::new(),
            filter_expr: None,
            filtered_indices,
            input_buffer: String::new(),
            detail_state: None,
        }
    }

    // Getters
    pub fn columns(&self) -> &[String] {
        self.table_data.columns()
    }

    pub fn visible_row_count(&self) -> usize {
        self.filtered_indices.len()
    }

    pub fn selected_row(&self) -> usize {
        self.selected_row
    }

    pub fn scroll_offset(&self) -> usize {
        self.scroll_offset
    }

    pub fn search_query(&self) -> &str {
        &self.search_query
    }

    pub fn filter_text(&self) -> String {
        self.filter_expr
            .as_ref()
            .map(|f| {
                f.conditions
                    .iter()
                    .map(|c| {
                        let quoted_value = Self::quote_if_needed(&c.value);
                        format!("{}{}{}", c.column, c.op.as_str(), quoted_value)
                    })
                    .collect::<Vec<_>>()
                    .join(" ")
            })
            .unwrap_or_default()
    }

    /// Quote a filter value if it contains spaces or special characters
    fn quote_if_needed(value: &str) -> String {
        // Need quotes if value contains spaces or filter operator characters
        let needs_quotes = value.contains(' ')
            || value.contains('=')
            || value.contains('!')
            || value.contains('>')
            || value.contains('<')
            || value.contains('~');

        if needs_quotes {
            // Use double quotes, escape any existing double quotes
            let escaped = value.replace('"', r#"\""#);
            format!("\"{}\"", escaped)
        } else {
            value.to_string()
        }
    }

    /// Get the row at the given visible index
    pub fn get_visible_row(&self, visible_idx: usize) -> Option<&[Value]> {
        self.filtered_indices
            .get(visible_idx)
            .and_then(|&actual_idx| self.table_data.rows().get(actual_idx))
            .map(|v| v.as_slice())
    }

    /// Get the currently selected row's values
    pub fn get_selected_row(&self) -> Option<&[Value]> {
        self.get_visible_row(self.selected_row)
    }

    /// Get the original JSON for the currently selected row
    pub fn get_selected_source(&self) -> Option<&Value> {
        let actual_idx = *self.filtered_indices.get(self.selected_row)?;
        self.source_records.get(actual_idx)
    }

    /// Get the detail view state (if in Detail mode)
    pub fn detail_state(&self) -> Option<&DetailViewState> {
        self.detail_state.as_ref()
    }

    /// Get mutable detail view state
    pub fn detail_state_mut(&mut self) -> Option<&mut DetailViewState> {
        self.detail_state.as_mut()
    }

    /// Enter detail view mode for the selected row
    pub fn enter_detail_mode(&mut self, total_lines: usize) {
        self.mode = InputMode::Detail;
        self.detail_state = Some(DetailViewState::new(total_lines));
    }

    /// Exit detail view mode
    pub fn exit_detail_mode(&mut self) {
        self.mode = InputMode::Normal;
        self.detail_state = None;
    }

    // Navigation
    pub fn move_up(&mut self) {
        if self.selected_row > 0 {
            self.selected_row -= 1;
            self.ensure_visible();
        }
    }

    pub fn move_down(&mut self) {
        if self.selected_row + 1 < self.visible_row_count() {
            self.selected_row += 1;
            self.ensure_visible();
        }
    }

    pub fn page_up(&mut self, page_size: usize) {
        self.selected_row = self.selected_row.saturating_sub(page_size);
        self.ensure_visible();
    }

    pub fn page_down(&mut self, page_size: usize) {
        let max_row = self.visible_row_count().saturating_sub(1);
        self.selected_row = (self.selected_row + page_size).min(max_row);
        self.ensure_visible();
    }

    pub fn go_to_top(&mut self) {
        self.selected_row = 0;
        self.scroll_offset = 0;
    }

    pub fn go_to_bottom(&mut self) {
        self.selected_row = self.visible_row_count().saturating_sub(1);
        self.ensure_visible();
    }

    /// Ensure the selected row is visible in the viewport
    fn ensure_visible(&mut self) {
        // This will be called with actual viewport height from view
        // For now, use a reasonable default
        let viewport_height = 20;
        self.ensure_visible_with_height(viewport_height);
    }

    pub fn ensure_visible_with_height(&mut self, viewport_height: usize) {
        if self.selected_row < self.scroll_offset {
            self.scroll_offset = self.selected_row;
        } else if self.selected_row >= self.scroll_offset + viewport_height {
            self.scroll_offset = self.selected_row - viewport_height + 1;
        }
    }

    // Mode switching
    pub fn enter_search_mode(&mut self) {
        self.mode = InputMode::Search;
        self.input_buffer = self.search_query.clone();
    }

    pub fn enter_filter_mode(&mut self) {
        self.mode = InputMode::Filter;
        self.input_buffer = self.filter_text();
    }

    pub fn cancel_input(&mut self) {
        self.mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    pub fn confirm_input(&mut self) {
        match self.mode {
            InputMode::Search => {
                self.search_query = self.input_buffer.clone();
                self.apply_filters();
            }
            InputMode::Filter => {
                if self.input_buffer.is_empty() {
                    self.filter_expr = None;
                } else if let Ok(expr) = FilterExpr::parse(&self.input_buffer) {
                    self.filter_expr = Some(expr);
                }
                self.apply_filters();
            }
            InputMode::Normal | InputMode::Detail => {}
        }
        self.mode = InputMode::Normal;
        self.input_buffer.clear();
    }

    pub fn input_char(&mut self, c: char) {
        self.input_buffer.push(c);
    }

    pub fn input_backspace(&mut self) {
        self.input_buffer.pop();
    }

    /// Clear search and filter
    pub fn clear_filters(&mut self) {
        self.search_query.clear();
        self.filter_expr = None;
        self.apply_filters();
    }

    /// Apply search and filter to update filtered_indices
    fn apply_filters(&mut self) {
        let rows = self.table_data.rows();
        let columns = self.table_data.columns();

        self.filtered_indices = (0..rows.len())
            .filter(|&idx| {
                let row = &rows[idx];

                // Build a JSON object for filtering
                let row_obj: Value = {
                    let mut obj = serde_json::Map::new();
                    for (i, col) in columns.iter().enumerate() {
                        if let Some(val) = row.get(i) {
                            obj.insert(col.clone(), val.clone());
                        }
                    }
                    Value::Object(obj)
                };

                // Check search query
                if !self.search_query.is_empty() {
                    let search = FullTextSearch::new(&self.search_query);
                    if !search.matches(&row_obj) {
                        return false;
                    }
                }

                // Check filter expression
                if let Some(ref expr) = self.filter_expr {
                    if !expr.matches(&row_obj) {
                        return false;
                    }
                }

                true
            })
            .collect();

        // Reset selection if it's now out of bounds
        if self.selected_row >= self.filtered_indices.len() {
            self.selected_row = self.filtered_indices.len().saturating_sub(1);
        }
        self.scroll_offset = 0;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_quote_if_needed_simple() {
        // Simple values don't need quotes
        assert_eq!(App::quote_if_needed("alice"), "alice");
        assert_eq!(App::quote_if_needed("123"), "123");
        assert_eq!(App::quote_if_needed("true"), "true");
    }

    #[test]
    fn test_quote_if_needed_with_spaces() {
        // Values with spaces need quotes
        assert_eq!(App::quote_if_needed("Alice Smith"), "\"Alice Smith\"");
        assert_eq!(App::quote_if_needed("hello world"), "\"hello world\"");
    }

    #[test]
    fn test_quote_if_needed_with_operators() {
        // Values containing operator characters need quotes
        assert_eq!(App::quote_if_needed("a=b"), "\"a=b\"");
        assert_eq!(App::quote_if_needed("x>y"), "\"x>y\"");
        assert_eq!(App::quote_if_needed("foo~bar"), "\"foo~bar\"");
    }

    #[test]
    fn test_quote_if_needed_with_existing_quotes() {
        // Existing quotes should be escaped
        assert_eq!(
            App::quote_if_needed("say \"hello\""),
            "\"say \\\"hello\\\"\""
        );
    }

    #[test]
    fn test_detail_view_state_scroll() {
        let mut state = DetailViewState::new(100);
        state.set_viewport_height(20);
        assert_eq!(state.scroll_offset, 0);

        state.scroll_down(10);
        assert_eq!(state.scroll_offset, 10);

        state.scroll_up(5);
        assert_eq!(state.scroll_offset, 5);

        state.go_to_top();
        assert_eq!(state.scroll_offset, 0);

        state.go_to_bottom();
        assert_eq!(state.scroll_offset, 80); // 100 - 20
    }

    #[test]
    fn test_detail_view_state_scroll_bounds() {
        let mut state = DetailViewState::new(50);
        state.set_viewport_height(20);

        // Scroll down beyond max
        state.scroll_down(100);
        assert_eq!(state.scroll_offset, 30); // 50 - 20

        // Scroll up beyond 0
        state.scroll_up(100);
        assert_eq!(state.scroll_offset, 0);
    }
}
