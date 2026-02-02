use super::path::CompiledPath;
use crate::error::{JlcatError, Result};
use serde_json::Value;

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum FilterOp {
    Eq,          // =
    Ne,          // !=
    Gt,          // >
    Gte,         // >=
    Lt,          // <
    Lte,         // <=
    Contains,    // ~
    NotContains, // !~
}

#[derive(Debug, Clone)]
pub struct FilterCondition {
    pub column: String,
    pub path: CompiledPath,
    pub op: FilterOp,
    pub value: String,
}

impl FilterCondition {
    fn matches(&self, row: &Value) -> bool {
        let row_value = self.path.get(row);

        match &self.op {
            FilterOp::Eq => self.matches_eq(row_value),
            FilterOp::Ne => !self.matches_eq(row_value),
            FilterOp::Gt => self.matches_cmp(row_value, |ord| ord == std::cmp::Ordering::Greater),
            FilterOp::Gte => self.matches_cmp(row_value, |ord| ord != std::cmp::Ordering::Less),
            FilterOp::Lt => self.matches_cmp(row_value, |ord| ord == std::cmp::Ordering::Less),
            FilterOp::Lte => self.matches_cmp(row_value, |ord| ord != std::cmp::Ordering::Greater),
            FilterOp::Contains => self.matches_contains(row_value),
            FilterOp::NotContains => !self.matches_contains(row_value),
        }
    }

    fn matches_eq(&self, row_value: Option<&Value>) -> bool {
        match row_value {
            Some(Value::String(s)) => s == &self.value,
            Some(Value::Number(n)) => n.to_string() == self.value,
            Some(Value::Bool(b)) => b.to_string() == self.value,
            Some(Value::Null) => self.value == "null",
            _ => false,
        }
    }

    fn matches_cmp<F>(&self, row_value: Option<&Value>, predicate: F) -> bool
    where
        F: Fn(std::cmp::Ordering) -> bool,
    {
        let filter_num: f64 = match self.value.parse() {
            Ok(n) => n,
            Err(_) => return false,
        };

        match row_value {
            Some(Value::Number(n)) => {
                if let Some(row_num) = n.as_f64() {
                    predicate(
                        row_num
                            .partial_cmp(&filter_num)
                            .unwrap_or(std::cmp::Ordering::Equal),
                    )
                } else {
                    false
                }
            }
            _ => false,
        }
    }

    fn matches_contains(&self, row_value: Option<&Value>) -> bool {
        let search_lower = self.value.to_lowercase();
        match row_value {
            Some(Value::String(s)) => s.to_lowercase().contains(&search_lower),
            Some(v) => v.to_string().to_lowercase().contains(&search_lower),
            None => false,
        }
    }
}

#[derive(Debug, Clone)]
pub struct FilterExpr {
    pub conditions: Vec<FilterCondition>,
}

impl FilterExpr {
    pub fn parse(input: &str) -> Result<Self> {
        let mut conditions = Vec::new();
        let mut chars = input.chars().peekable();

        while chars.peek().is_some() {
            // Skip whitespace
            while chars.peek() == Some(&' ') {
                chars.next();
            }

            if chars.peek().is_none() {
                break;
            }

            // Parse column name
            let mut column = String::new();
            while let Some(&c) = chars.peek() {
                if c == '=' || c == '!' || c == '>' || c == '<' || c == '~' {
                    break;
                }
                if c == ' ' {
                    break;
                }
                column.push(chars.next().unwrap());
            }

            if column.is_empty() {
                return Err(JlcatError::InvalidFilter("empty column name".into()));
            }

            // Parse operator
            let op = match chars.peek() {
                Some('=') => {
                    chars.next();
                    FilterOp::Eq
                }
                Some('!') => {
                    chars.next();
                    match chars.peek() {
                        Some('=') => {
                            chars.next();
                            FilterOp::Ne
                        }
                        Some('~') => {
                            chars.next();
                            FilterOp::NotContains
                        }
                        _ => {
                            return Err(JlcatError::InvalidFilter(
                                "expected = or ~ after !".into(),
                            ))
                        }
                    }
                }
                Some('>') => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        FilterOp::Gte
                    } else {
                        FilterOp::Gt
                    }
                }
                Some('<') => {
                    chars.next();
                    if chars.peek() == Some(&'=') {
                        chars.next();
                        FilterOp::Lte
                    } else {
                        FilterOp::Lt
                    }
                }
                Some('~') => {
                    chars.next();
                    FilterOp::Contains
                }
                _ => return Err(JlcatError::InvalidFilter("missing operator".into())),
            };

            // Parse value
            let value = if chars.peek() == Some(&'"') || chars.peek() == Some(&'\'') {
                let quote = chars.next().unwrap();
                let mut val = String::new();
                while let Some(c) = chars.next() {
                    if c == quote {
                        break;
                    }
                    val.push(c);
                }
                val
            } else {
                let mut val = String::new();
                while let Some(&c) = chars.peek() {
                    if c == ' ' {
                        break;
                    }
                    val.push(chars.next().unwrap());
                }
                val
            };

            let path = CompiledPath::compile(&column)?;
            conditions.push(FilterCondition {
                column,
                path,
                op,
                value,
            });
        }

        Ok(Self { conditions })
    }

    pub fn matches(&self, row: &Value) -> bool {
        self.conditions.iter().all(|c| c.matches(row))
    }
}

#[derive(Debug, Clone)]
pub struct FullTextSearch {
    query: String,
}

impl FullTextSearch {
    pub fn new(query: &str) -> Self {
        Self {
            query: query.to_lowercase(),
        }
    }

    pub fn matches(&self, row: &Value) -> bool {
        self.search_value(row)
    }

    fn search_value(&self, value: &Value) -> bool {
        match value {
            Value::String(s) => s.to_lowercase().contains(&self.query),
            Value::Number(n) => n.to_string().contains(&self.query),
            Value::Bool(b) => b.to_string().contains(&self.query),
            Value::Array(arr) => arr.iter().any(|v| self.search_value(v)),
            Value::Object(obj) => obj.values().any(|v| self.search_value(v)),
            Value::Null => false,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_equals() {
        let expr = FilterExpr::parse("status=active").unwrap();
        assert_eq!(expr.conditions.len(), 1);
        assert_eq!(expr.conditions[0].column, "status");
        assert_eq!(expr.conditions[0].op, FilterOp::Eq);
        assert_eq!(expr.conditions[0].value, "active");
    }

    #[test]
    fn test_parse_quoted_value() {
        let expr = FilterExpr::parse(r#"name="John Doe""#).unwrap();
        assert_eq!(expr.conditions[0].value, "John Doe");
    }

    #[test]
    fn test_parse_single_quoted_value() {
        let expr = FilterExpr::parse("name='value,with,commas'").unwrap();
        assert_eq!(expr.conditions[0].value, "value,with,commas");
    }

    #[test]
    fn test_parse_multiple_conditions() {
        let expr = FilterExpr::parse("status=active age>30").unwrap();
        assert_eq!(expr.conditions.len(), 2);
        assert_eq!(expr.conditions[0].column, "status");
        assert_eq!(expr.conditions[1].column, "age");
    }

    #[test]
    fn test_parse_operators() {
        assert_eq!(
            FilterExpr::parse("age>30").unwrap().conditions[0].op,
            FilterOp::Gt
        );
        assert_eq!(
            FilterExpr::parse("age>=30").unwrap().conditions[0].op,
            FilterOp::Gte
        );
        assert_eq!(
            FilterExpr::parse("age<30").unwrap().conditions[0].op,
            FilterOp::Lt
        );
        assert_eq!(
            FilterExpr::parse("age<=30").unwrap().conditions[0].op,
            FilterOp::Lte
        );
        assert_eq!(
            FilterExpr::parse("name~alice").unwrap().conditions[0].op,
            FilterOp::Contains
        );
        assert_eq!(
            FilterExpr::parse("name!~bob").unwrap().conditions[0].op,
            FilterOp::NotContains
        );
        assert_eq!(
            FilterExpr::parse("status!=inactive").unwrap().conditions[0].op,
            FilterOp::Ne
        );
    }

    #[test]
    fn test_filter_matches() {
        let expr = FilterExpr::parse("status=active age>25").unwrap();

        assert!(expr.matches(&json!({"status": "active", "age": 30})));
        assert!(!expr.matches(&json!({"status": "active", "age": 20})));
        assert!(!expr.matches(&json!({"status": "inactive", "age": 30})));
    }

    #[test]
    fn test_filter_contains() {
        let expr = FilterExpr::parse("name~alice").unwrap();

        assert!(expr.matches(&json!({"name": "alice smith"})));
        assert!(expr.matches(&json!({"name": "Alice"}))); // case insensitive
        assert!(!expr.matches(&json!({"name": "bob"})));
    }

    #[test]
    fn test_filter_not_contains() {
        let expr = FilterExpr::parse("name!~bob").unwrap();

        assert!(expr.matches(&json!({"name": "alice"})));
        assert!(!expr.matches(&json!({"name": "bob"})));
        assert!(!expr.matches(&json!({"name": "Bobby"})));
    }

    #[test]
    fn test_filter_nested() {
        let expr = FilterExpr::parse("address.city=Tokyo").unwrap();
        assert!(expr.matches(&json!({"address": {"city": "Tokyo"}})));
        assert!(!expr.matches(&json!({"address": {"city": "Osaka"}})));
    }

    #[test]
    fn test_fulltext_search() {
        let search = FullTextSearch::new("alice");

        assert!(search.matches(&json!({"name": "Alice", "role": "admin"})));
        assert!(search.matches(&json!({"desc": "User alice@example.com"})));
        assert!(!search.matches(&json!({"name": "Bob"})));
    }

    #[test]
    fn test_fulltext_search_nested() {
        let search = FullTextSearch::new("tokyo");

        assert!(search.matches(&json!({"address": {"city": "Tokyo"}})));
        assert!(search.matches(&json!({"items": ["Tokyo", "Osaka"]})));
        assert!(!search.matches(&json!({"city": "Osaka"})));
    }
}
