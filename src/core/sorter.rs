use super::path::CompiledPath;
use super::value::SortableValue;
use crate::error::{JlcatError, Result};
use serde_json::Value;
use std::cmp::Ordering;

#[derive(Debug, Clone)]
pub struct SortKey {
    pub path: CompiledPath,
    pub descending: bool,
}

impl SortKey {
    pub fn parse(s: &str) -> Result<Self> {
        if s.is_empty() {
            return Err(JlcatError::InvalidSortKey("empty sort key".into()));
        }

        let (descending, column) = if let Some(col) = s.strip_prefix('-') {
            (true, col)
        } else {
            (false, s)
        };

        if column.is_empty() {
            return Err(JlcatError::InvalidSortKey("empty column name".into()));
        }

        let path = CompiledPath::compile(column)?;
        Ok(Self { path, descending })
    }
}

#[derive(Debug, Clone)]
pub struct Sorter {
    keys: Vec<SortKey>,
}

impl Sorter {
    pub fn new(keys: Vec<SortKey>) -> Self {
        Self { keys }
    }

    pub fn parse(key_strs: &[String]) -> Result<Self> {
        let keys: Result<Vec<_>> = key_strs.iter().map(|s| SortKey::parse(s)).collect();
        Ok(Self::new(keys?))
    }

    pub fn sort(&self, rows: &mut [Value]) {
        rows.sort_by(|a, b| self.compare(a, b));
    }

    pub fn sort_indices(&self, rows: &[Value]) -> Vec<usize> {
        let mut indices: Vec<usize> = (0..rows.len()).collect();
        indices.sort_by(|&i, &j| self.compare(&rows[i], &rows[j]));
        indices
    }

    fn compare(&self, a: &Value, b: &Value) -> Ordering {
        for key in &self.keys {
            let val_a = key.path.get(a);
            let val_b = key.path.get(b);

            // Handle nulls-last for both ascending and descending
            let a_is_null = val_a.map_or(true, |v| v.is_null());
            let b_is_null = val_b.map_or(true, |v| v.is_null());

            if a_is_null && !b_is_null {
                return Ordering::Greater; // null goes last
            }
            if !a_is_null && b_is_null {
                return Ordering::Less; // non-null goes first
            }
            if a_is_null && b_is_null {
                continue; // both null, check next key
            }

            let ord = match (val_a, val_b) {
                (Some(va), Some(vb)) => SortableValue::new(va).cmp(&SortableValue::new(vb)),
                _ => Ordering::Equal,
            };

            let ord = if key.descending { ord.reverse() } else { ord };

            if ord != Ordering::Equal {
                return ord;
            }
        }
        Ordering::Equal
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_sort_key() {
        let key = SortKey::parse("name").unwrap();
        assert_eq!(key.path.original, "name");
        assert!(!key.descending);

        let key = SortKey::parse("-age").unwrap();
        assert_eq!(key.path.original, "age");
        assert!(key.descending);
    }

    #[test]
    fn test_parse_sort_key_nested() {
        let key = SortKey::parse("address.city").unwrap();
        assert_eq!(key.path.original, "address.city");
        assert!(!key.descending);

        let key = SortKey::parse("-address.zip").unwrap();
        assert_eq!(key.path.original, "address.zip");
        assert!(key.descending);
    }

    #[test]
    fn test_sort_ascending() {
        let mut rows = vec![
            json!({"id": 3, "name": "Charlie"}),
            json!({"id": 1, "name": "Alice"}),
            json!({"id": 2, "name": "Bob"}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("name").unwrap()]);
        sorter.sort(&mut rows);

        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
        assert_eq!(rows[2]["name"], "Charlie");
    }

    #[test]
    fn test_sort_descending() {
        let mut rows = vec![
            json!({"id": 1, "age": 30}),
            json!({"id": 2, "age": 25}),
            json!({"id": 3, "age": 35}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("-age").unwrap()]);
        sorter.sort(&mut rows);

        assert_eq!(rows[0]["age"], 35);
        assert_eq!(rows[1]["age"], 30);
        assert_eq!(rows[2]["age"], 25);
    }

    #[test]
    fn test_sort_multiple_keys() {
        let mut rows = vec![
            json!({"dept": "A", "age": 30}),
            json!({"dept": "B", "age": 25}),
            json!({"dept": "A", "age": 25}),
        ];

        let sorter = Sorter::new(vec![
            SortKey::parse("dept").unwrap(),
            SortKey::parse("-age").unwrap(),
        ]);
        sorter.sort(&mut rows);

        // dept A first, then by age desc
        assert_eq!(rows[0], json!({"dept": "A", "age": 30}));
        assert_eq!(rows[1], json!({"dept": "A", "age": 25}));
        assert_eq!(rows[2], json!({"dept": "B", "age": 25}));
    }

    #[test]
    fn test_sort_nulls_last_ascending() {
        let mut rows = vec![
            json!({"id": 1, "name": null}),
            json!({"id": 2, "name": "Bob"}),
            json!({"id": 3, "name": "Alice"}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("name").unwrap()]);
        sorter.sort(&mut rows);

        assert_eq!(rows[0]["name"], "Alice");
        assert_eq!(rows[1]["name"], "Bob");
        assert_eq!(rows[2]["name"], Value::Null);
    }

    #[test]
    fn test_sort_nulls_last_descending() {
        let mut rows = vec![
            json!({"id": 1, "name": null}),
            json!({"id": 2, "name": "Bob"}),
            json!({"id": 3, "name": "Alice"}),
        ];

        let sorter = Sorter::new(vec![SortKey::parse("-name").unwrap()]);
        sorter.sort(&mut rows);

        // Descending: Bob, Alice, null (nulls still last)
        assert_eq!(rows[0]["name"], "Bob");
        assert_eq!(rows[1]["name"], "Alice");
        assert_eq!(rows[2]["name"], Value::Null);
    }

    #[test]
    fn test_sort_indices() {
        let rows = vec![json!({"id": 3}), json!({"id": 1}), json!({"id": 2})];

        let sorter = Sorter::new(vec![SortKey::parse("id").unwrap()]);
        let indices = sorter.sort_indices(&rows);

        assert_eq!(indices, vec![1, 2, 0]); // id=1 at index 1, id=2 at index 2, id=3 at index 0
    }

    #[test]
    fn test_sorter_parse() {
        let sorter = Sorter::parse(&["name".to_string(), "-age".to_string()]).unwrap();
        assert_eq!(sorter.keys.len(), 2);
        assert!(!sorter.keys[0].descending);
        assert!(sorter.keys[1].descending);
    }
}
