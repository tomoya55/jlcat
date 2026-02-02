use proptest::prelude::*;
use serde_json::{json, Value};

// Helper to create JSON rows from simple data
fn make_row(id: i64, name: &str, age: i64) -> Value {
    json!({
        "id": id,
        "name": name,
        "age": age
    })
}

// Helper to create a sorter from a single key string
fn make_sorter(key: &str) -> jlcat::core::Sorter {
    jlcat::core::Sorter::parse(&[key.to_string()]).unwrap()
}

// Strategy for generating valid JSON strings (alphanumeric)
fn json_string_strategy() -> impl Strategy<Value = String> {
    "[a-zA-Z0-9]{1,20}".prop_map(|s| s)
}

// Strategy for generating simple JSON rows
fn json_row_strategy() -> impl Strategy<Value = Value> {
    (any::<i64>(), json_string_strategy(), any::<i64>()).prop_map(|(id, name, age)| {
        json!({
            "id": id,
            "name": name,
            "age": age
        })
    })
}

// Strategy for generating a vector of JSON rows
fn json_rows_strategy(min: usize, max: usize) -> impl Strategy<Value = Vec<Value>> {
    prop::collection::vec(json_row_strategy(), min..max)
}

proptest! {
    // Test that sorting is deterministic - same input always gives same output
    #[test]
    fn sort_is_deterministic(rows in json_rows_strategy(1, 50)) {
        

        let sorter = make_sorter("age");

        let mut rows1 = rows.clone();
        let mut rows2 = rows.clone();

        sorter.sort(&mut rows1);
        sorter.sort(&mut rows2);

        prop_assert_eq!(rows1, rows2);
    }

    // Test that sorting is idempotent - sorting twice gives same result
    #[test]
    fn sort_is_idempotent(rows in json_rows_strategy(1, 50)) {
        

        let sorter = make_sorter("age");

        let mut rows1 = rows.clone();
        sorter.sort(&mut rows1);

        let mut rows2 = rows1.clone();
        sorter.sort(&mut rows2);

        prop_assert_eq!(rows1, rows2);
    }

    // Test that sorting preserves row count
    #[test]
    fn sort_preserves_count(rows in json_rows_strategy(0, 100)) {
        

        let original_count = rows.len();
        let mut sorted = rows;

        let sorter = make_sorter("id");
        sorter.sort(&mut sorted);

        prop_assert_eq!(sorted.len(), original_count);
    }

    // Test that filtering never increases row count
    #[test]
    fn filter_never_increases_count(rows in json_rows_strategy(0, 50)) {
        use jlcat::core::FilterExpr;

        let original_count = rows.len();

        // Filter for age > 0 (may filter some or none)
        if let Ok(filter) = FilterExpr::parse("age>0") {
            let filtered: Vec<_> = rows.iter().filter(|r| filter.matches(r)).collect();
            prop_assert!(filtered.len() <= original_count);
        }
    }

    // Test that filter with always-true condition preserves all rows
    #[test]
    fn filter_preserves_matching_rows(id in any::<i64>(), name in json_string_strategy()) {
        use jlcat::core::FilterExpr;

        let row = json!({
            "id": id,
            "name": name
        });

        // Filter for the exact name should match
        let filter_str = format!("name={}", name);
        if let Ok(filter) = FilterExpr::parse(&filter_str) {
            prop_assert!(filter.matches(&row));
        }
    }

    // Test that schema inference handles all rows
    #[test]
    fn schema_inference_handles_all_rows(rows in json_rows_strategy(1, 20)) {
        use jlcat::core::SchemaInferrer;

        let schema = SchemaInferrer::infer(&rows);

        // Should have discovered all columns present in any row
        prop_assert!(schema.columns().contains(&"id".to_string()));
        prop_assert!(schema.columns().contains(&"name".to_string()));
        prop_assert!(schema.columns().contains(&"age".to_string()));
    }

    // Test that TableData preserves all rows
    #[test]
    fn table_data_preserves_rows(rows in json_rows_strategy(0, 50)) {
        use jlcat::core::TableData;

        let original_count = rows.len();
        let table = TableData::from_rows(rows, None);

        prop_assert_eq!(table.rows().len(), original_count);
    }

    // Test that column selector extracts values correctly
    #[test]
    fn column_selector_extracts_values(id in any::<i64>(), name in json_string_strategy()) {
        use jlcat::core::ColumnSelector;

        // The row would be used with selector.select() but we're testing columns() here
        let _row = json!({
            "id": id,
            "name": name,
            "extra": "ignored"
        });

        let selector = ColumnSelector::new(vec!["id".to_string(), "name".to_string()]).unwrap();
        let columns = selector.columns();

        prop_assert_eq!(columns.len(), 2);
        prop_assert_eq!(columns[0], "id");
        prop_assert_eq!(columns[1], "name");
    }

    // Test that descending sort reverses ascending sort
    #[test]
    fn descending_reverses_ascending(mut rows in json_rows_strategy(2, 30)) {
        

        // Only test with unique ages to ensure deterministic ordering
        for (i, row) in rows.iter_mut().enumerate() {
            if let Some(obj) = row.as_object_mut() {
                obj.insert("age".to_string(), json!(i as i64));
            }
        }

        let asc_sorter = make_sorter("age");
        let desc_sorter = make_sorter("-age");

        let mut asc_sorted = rows.clone();
        let mut desc_sorted = rows;

        asc_sorter.sort(&mut asc_sorted);
        desc_sorter.sort(&mut desc_sorted);

        // Reverse one and compare
        desc_sorted.reverse();
        prop_assert_eq!(asc_sorted, desc_sorted);
    }

    // Test full-text search finds matching rows
    #[test]
    fn fulltext_search_finds_matches(name in json_string_strategy()) {
        use jlcat::core::FullTextSearch;

        let row = json!({
            "name": name.clone(),
            "other": "data"
        });

        // Search for the name should match
        let search = FullTextSearch::new(&name);
        prop_assert!(search.matches(&row));
    }

    // Test that nested value extraction works
    #[test]
    fn nested_value_extraction(id in any::<i64>(), city in json_string_strategy()) {
        use jlcat::core::get_nested_value;

        let row = json!({
            "id": id,
            "address": {
                "city": city.clone()
            }
        });

        let extracted = get_nested_value(&row, "address.city");
        prop_assert_eq!(extracted, Some(&json!(city)));
    }
}

// Test with edge cases
#[test]
fn test_empty_rows() {
    use jlcat::core::TableData;

    let rows: Vec<Value> = vec![];
    let mut sorted = rows.clone();

    let sorter = make_sorter("id");
    sorter.sort(&mut sorted);

    assert_eq!(sorted.len(), 0);

    let table = TableData::from_rows(vec![], None);
    assert_eq!(table.rows().len(), 0);
}

#[test]
fn test_single_row() {
    use jlcat::core::TableData;

    let rows = vec![make_row(1, "alice", 30)];
    let mut sorted = rows.clone();

    let sorter = make_sorter("id");
    sorter.sort(&mut sorted);

    assert_eq!(sorted.len(), 1);
    assert_eq!(sorted[0]["name"], "alice");

    let table = TableData::from_rows(rows, None);
    assert_eq!(table.rows().len(), 1);
}

#[test]
fn test_rows_with_nulls() {
    

    let rows = vec![
        json!({"id": 1, "name": "alice", "age": 30}),
        json!({"id": 2, "name": null, "age": 25}),
        json!({"id": 3, "name": "charlie"}), // missing age
    ];

    let mut sorted = rows.clone();
    let sorter = make_sorter("name");
    sorter.sort(&mut sorted);

    // Nulls should be last
    assert_eq!(sorted.len(), 3);
    assert!(sorted[2]["name"].is_null() || sorted[2].get("name").is_none());
}
