use crate::error::{JlcatError, Result};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum PathSegment {
    Key(String),
    Index(usize),
}

#[derive(Debug, Clone)]
pub struct CompiledPath {
    pub segments: Vec<PathSegment>,
    pub original: String,
}

impl CompiledPath {
    pub fn compile(path: &str) -> Result<Self> {
        let mut segments = Vec::new();
        let mut current = String::new();
        let mut chars = path.chars().peekable();

        while let Some(c) = chars.next() {
            match c {
                '.' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Key(current.clone()));
                        current.clear();
                    }
                }
                '[' => {
                    if !current.is_empty() {
                        segments.push(PathSegment::Key(current.clone()));
                        current.clear();
                    }
                    // Parse index
                    let mut idx_str = String::new();
                    let mut found_bracket = false;
                    while let Some(&next_c) = chars.peek() {
                        if next_c == ']' {
                            chars.next();
                            found_bracket = true;
                            break;
                        }
                        idx_str.push(chars.next().unwrap());
                    }
                    if !found_bracket {
                        return Err(JlcatError::InvalidColumnPath(format!(
                            "unterminated array index in '{}'",
                            path
                        )));
                    }
                    let idx: usize = idx_str.parse().map_err(|_| {
                        JlcatError::InvalidColumnPath(format!(
                            "invalid index '{}' in '{}'",
                            idx_str, path
                        ))
                    })?;
                    segments.push(PathSegment::Index(idx));
                }
                ']' => {
                    return Err(JlcatError::InvalidColumnPath(format!(
                        "unexpected ']' in '{}'",
                        path
                    )));
                }
                _ => {
                    current.push(c);
                }
            }
        }

        if !current.is_empty() {
            segments.push(PathSegment::Key(current));
        }

        if segments.is_empty() {
            return Err(JlcatError::InvalidColumnPath(format!(
                "empty path '{}'",
                path
            )));
        }

        Ok(Self {
            segments,
            original: path.to_string(),
        })
    }

    pub fn get<'a>(&self, value: &'a Value) -> Option<&'a Value> {
        // First try literal key (for flattened column names like "address.city")
        if self.segments.len() > 1 {
            if let Some(v) = value.get(&self.original) {
                return Some(v);
            }
        }

        // Fall back to nested path lookup
        let mut current = value;

        for segment in &self.segments {
            current = match segment {
                PathSegment::Key(key) => current.get(key)?,
                PathSegment::Index(idx) => current.get(idx)?,
            };
        }

        Some(current)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_compile_simple_path() {
        let path = CompiledPath::compile("name").unwrap();
        assert_eq!(path.segments, vec![PathSegment::Key("name".into())]);
    }

    #[test]
    fn test_compile_nested_path() {
        let path = CompiledPath::compile("address.city").unwrap();
        assert_eq!(
            path.segments,
            vec![
                PathSegment::Key("address".into()),
                PathSegment::Key("city".into()),
            ]
        );
    }

    #[test]
    fn test_compile_array_index() {
        let path = CompiledPath::compile("orders[0].item").unwrap();
        assert_eq!(
            path.segments,
            vec![
                PathSegment::Key("orders".into()),
                PathSegment::Index(0),
                PathSegment::Key("item".into()),
            ]
        );
    }

    #[test]
    fn test_get_value() {
        let path = CompiledPath::compile("address.city").unwrap();
        let row = json!({"address": {"city": "Tokyo"}});
        assert_eq!(path.get(&row), Some(&json!("Tokyo")));
    }

    #[test]
    fn test_get_array_value() {
        let path = CompiledPath::compile("items[1].name").unwrap();
        let row = json!({"items": [{"name": "A"}, {"name": "B"}]});
        assert_eq!(path.get(&row), Some(&json!("B")));
    }

    #[test]
    fn test_get_missing_value() {
        let path = CompiledPath::compile("missing.field").unwrap();
        let row = json!({"other": 1});
        assert_eq!(path.get(&row), None);
    }

    #[test]
    fn test_get_literal_dotted_key() {
        // When column selection flattens "address.city" into a literal key
        let path = CompiledPath::compile("address.city").unwrap();
        let row = json!({"address.city": "Tokyo"});
        assert_eq!(path.get(&row), Some(&json!("Tokyo")));
    }

    #[test]
    fn test_get_prefers_literal_over_nested() {
        // Literal key takes precedence when both exist
        let path = CompiledPath::compile("address.city").unwrap();
        let row = json!({
            "address.city": "Literal",
            "address": {"city": "Nested"}
        });
        assert_eq!(path.get(&row), Some(&json!("Literal")));
    }

    #[test]
    fn test_unterminated_array_index_rejected() {
        // Missing closing bracket should be rejected
        let result = CompiledPath::compile("items[0");
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(
            err.to_string().contains("unterminated"),
            "Error should mention unterminated: {}",
            err
        );
    }

    #[test]
    fn test_unterminated_nested_array_index_rejected() {
        // Missing closing bracket in nested path should be rejected
        let result = CompiledPath::compile("user.items[1.name");
        assert!(result.is_err());
    }
}
