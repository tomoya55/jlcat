use super::app::{App, InputMode};
use ratatui::{
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, TableState},
    Frame,
};
use serde_json::Value;

/// Render the application UI
pub fn render(frame: &mut Frame, app: &mut App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(3),    // Table
            Constraint::Length(3), // Footer/Status
        ])
        .split(frame.area());

    render_table(frame, app, chunks[0]);
    render_footer(frame, app, chunks[1]);

    // Update scroll based on actual viewport height
    let table_height = chunks[0].height.saturating_sub(3) as usize; // subtract borders and header
    app.ensure_visible_with_height(table_height);
}

fn render_table(frame: &mut Frame, app: &App, area: Rect) {
    let header_cells: Vec<Cell> = app
        .columns()
        .iter()
        .map(|h| {
            Cell::from(h.clone()).style(
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            )
        })
        .collect();

    let header = Row::new(header_cells).height(1).bottom_margin(1);

    // Calculate column widths
    let col_count = app.columns().len();
    let constraints: Vec<Constraint> = (0..col_count)
        .map(|_| Constraint::Percentage((100 / col_count.max(1)) as u16))
        .collect();

    // Build visible rows
    let table_height = area.height.saturating_sub(3) as usize;
    let start = app.scroll_offset();
    let end = (start + table_height).min(app.visible_row_count());

    let rows: Vec<Row> = (start..end)
        .map(|visible_idx| {
            let row_data = app.get_visible_row(visible_idx);
            let cells: Vec<Cell> = match row_data {
                Some(values) => values.iter().map(|v| Cell::from(format_value(v))).collect(),
                None => vec![Cell::from(""); col_count],
            };

            let style = if visible_idx == app.selected_row() {
                Style::default()
                    .bg(Color::Blue)
                    .fg(Color::White)
                    .add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };

            Row::new(cells).style(style)
        })
        .collect();

    let title = format!(
        " jlcat - {} rows ({} shown) ",
        app.visible_row_count(),
        rows.len()
    );

    let table = Table::new(rows, constraints)
        .header(header)
        .block(Block::default().borders(Borders::ALL).title(title))
        .row_highlight_style(Style::default().add_modifier(Modifier::REVERSED));

    // We manually handle selection via styling, so use StatefulWidget with empty state
    let mut state = TableState::default();
    frame.render_stateful_widget(table, area, &mut state);
}

fn render_footer(frame: &mut Frame, app: &App, area: Rect) {
    let content = match app.mode {
        InputMode::Normal => {
            // Show selected row details and help
            let selected = app.get_selected_row();
            let details = match selected {
                Some(values) => {
                    let cols = app.columns();
                    cols.iter()
                        .zip(values.iter())
                        .map(|(c, v)| format!("{}={}", c, format_value_short(v)))
                        .collect::<Vec<_>>()
                        .join(" | ")
                }
                None => "No data".to_string(),
            };

            let status = if !app.search_query().is_empty() || !app.filter_text().is_empty() {
                let mut parts = vec![];
                if !app.search_query().is_empty() {
                    parts.push(format!("search: {}", app.search_query()));
                }
                if !app.filter_text().is_empty() {
                    parts.push(format!("filter: {}", app.filter_text()));
                }
                format!(" [{}]", parts.join(", "))
            } else {
                String::new()
            };

            vec![
                Line::from(details),
                Line::from(Span::styled(
                    format!("q:quit  /:search  f:filter  c:clear{}", status),
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        InputMode::Search => {
            vec![
                Line::from(vec![
                    Span::styled("Search: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&app.input_buffer),
                    Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
                ]),
                Line::from(Span::styled(
                    "Enter:confirm  Esc:cancel",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        InputMode::Filter => {
            vec![
                Line::from(vec![
                    Span::styled("Filter: ", Style::default().fg(Color::Yellow)),
                    Span::raw(&app.input_buffer),
                    Span::styled("_", Style::default().add_modifier(Modifier::SLOW_BLINK)),
                ]),
                Line::from(Span::styled(
                    "Enter:confirm  Esc:cancel  (e.g., age>30 name~alice)",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
        InputMode::Detail => {
            vec![
                Line::from(Span::styled(
                    "Detail View",
                    Style::default().fg(Color::Yellow),
                )),
                Line::from(Span::styled(
                    "Esc:close",
                    Style::default().fg(Color::DarkGray),
                )),
            ]
        }
    };

    let paragraph = Paragraph::new(content).block(Block::default().borders(Borders::ALL));

    frame.render_widget(paragraph, area);
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "null".to_string(),
        Value::Bool(b) => b.to_string(),
        Value::Number(n) => n.to_string(),
        Value::String(s) => s.clone(),
        Value::Array(_) => "[...]".to_string(),
        Value::Object(_) => "{...}".to_string(),
    }
}

fn format_value_short(value: &Value) -> String {
    let s = format_value(value);
    let char_count = s.chars().count();
    if char_count > 20 {
        let truncated: String = s.chars().take(17).collect();
        format!("{}...", truncated)
    } else {
        s
    }
}
