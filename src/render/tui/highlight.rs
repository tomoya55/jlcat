//! JSON syntax highlighting for the detail view

use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};
use serde_json::Value;

/// Color scheme for JSON syntax highlighting
struct JsonColors;

impl JsonColors {
    const KEY: Color = Color::Cyan;
    const STRING: Color = Color::Green;
    const NUMBER: Color = Color::Yellow;
    const BOOLEAN: Color = Color::Magenta;
    const NULL: Color = Color::DarkGray;
    const PUNCTUATION: Color = Color::White;
}

/// Highlight a JSON value and return styled lines
pub fn highlight_json(value: &Value) -> Vec<Line<'static>> {
    let pretty = serde_json::to_string_pretty(value).unwrap_or_else(|_| "{}".to_string());
    pretty.lines().map(highlight_line).collect()
}

/// Highlight a single line of pretty-printed JSON
fn highlight_line(line: &str) -> Line<'static> {
    let mut spans: Vec<Span<'static>> = Vec::new();
    let mut chars = line.chars().peekable();
    let mut current = String::new();

    while let Some(c) = chars.next() {
        match c {
            // Start of a string (could be key or value)
            '"' => {
                // Flush any pending whitespace
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }

                // Collect the string content
                let mut string_content = String::from('"');
                while let Some(sc) = chars.next() {
                    string_content.push(sc);
                    if sc == '"' {
                        break;
                    }
                    if sc == '\\' {
                        // Handle escape sequence
                        if let Some(escaped) = chars.next() {
                            string_content.push(escaped);
                        }
                    }
                }

                // Check if this is a key (followed by ':')
                let is_key = {
                    // Skip whitespace to check for colon
                    let remaining: String = chars.clone().collect();
                    remaining.trim_start().starts_with(':')
                };

                let color = if is_key {
                    JsonColors::KEY
                } else {
                    JsonColors::STRING
                };

                spans.push(Span::styled(string_content, Style::default().fg(color)));
            }

            // Punctuation
            ':' | ',' | '{' | '}' | '[' | ']' => {
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }
                spans.push(Span::styled(
                    c.to_string(),
                    Style::default().fg(JsonColors::PUNCTUATION),
                ));
            }

            // Potential keyword or number start
            c if c.is_ascii_alphanumeric() || c == '-' || c == '.' => {
                if !current.is_empty() {
                    spans.push(Span::raw(current.clone()));
                    current.clear();
                }

                let mut token = String::from(c);
                while let Some(&next) = chars.peek() {
                    if next.is_ascii_alphanumeric()
                        || next == '.'
                        || next == '-'
                        || next == '+'
                        || next == 'e'
                        || next == 'E'
                    {
                        token.push(chars.next().unwrap());
                    } else {
                        break;
                    }
                }

                let style = match token.as_str() {
                    "true" | "false" => Style::default().fg(JsonColors::BOOLEAN),
                    "null" => Style::default().fg(JsonColors::NULL),
                    _ if is_number(&token) => Style::default().fg(JsonColors::NUMBER),
                    _ => Style::default(),
                };

                spans.push(Span::styled(token, style));
            }

            // Whitespace
            _ => {
                current.push(c);
            }
        }
    }

    // Flush remaining
    if !current.is_empty() {
        spans.push(Span::raw(current));
    }

    Line::from(spans)
}

/// Check if a string is a valid JSON number
fn is_number(s: &str) -> bool {
    s.parse::<f64>().is_ok()
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_highlight_simple_object() {
        let value = json!({"name": "Alice"});
        let lines = highlight_json(&value);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_highlight_with_numbers() {
        let value = json!({"age": 30, "score": 3.14});
        let lines = highlight_json(&value);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_highlight_with_boolean_null() {
        let value = json!({"active": true, "deleted": false, "data": null});
        let lines = highlight_json(&value);
        assert!(!lines.is_empty());
    }

    #[test]
    fn test_highlight_nested() {
        let value = json!({
            "user": {
                "name": "Alice",
                "profile": {
                    "age": 30
                }
            }
        });
        let lines = highlight_json(&value);
        assert!(lines.len() > 3); // Should be multiple lines
    }

    #[test]
    fn test_is_number() {
        assert!(is_number("123"));
        assert!(is_number("-123"));
        assert!(is_number("3.14"));
        assert!(is_number("-3.14"));
        assert!(is_number("1e10"));
        assert!(is_number("1.5e-3"));
        assert!(!is_number("abc"));
        assert!(!is_number("true"));
    }
}
