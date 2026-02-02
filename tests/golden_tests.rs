use assert_cmd::Command;
use predicates::prelude::*;
use std::path::PathBuf;

fn fixtures_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("fixtures")
        .join("golden")
}

/// Test basic JSONL rendering
#[test]
fn golden_simple_jsonl() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.arg(&input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify structure - header row with column names
    assert!(stdout.contains("age"), "Should contain 'age' column");
    assert!(stdout.contains("id"), "Should contain 'id' column");
    assert!(stdout.contains("name"), "Should contain 'name' column");

    // Verify data rows
    assert!(stdout.contains("Alice"), "Should contain 'Alice'");
    assert!(stdout.contains("Bob"), "Should contain 'Bob'");
    assert!(stdout.contains("Charlie"), "Should contain 'Charlie'");

    // Verify numeric values
    assert!(stdout.contains("30"), "Should contain age 30");
    assert!(stdout.contains("25"), "Should contain age 25");
    assert!(stdout.contains("35"), "Should contain age 35");

    // Verify table formatting (rounded style by default)
    assert!(
        stdout.contains("┌") || stdout.contains("+"),
        "Should have table border"
    );
}

/// Test JSON array input (via stdin, as array detection only works for stdin)
#[test]
fn golden_json_array() {
    let input = r#"[{"product": "Apple", "price": 1.50},{"product": "Banana", "price": 0.75}]"#;

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.write_stdin(input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Verify columns
    assert!(stdout.contains("product"), "Should contain 'product' column");
    assert!(stdout.contains("price"), "Should contain 'price' column");

    // Verify data
    assert!(stdout.contains("Apple"), "Should contain 'Apple'");
    assert!(stdout.contains("Banana"), "Should contain 'Banana'");
}

/// Test nested object display
#[test]
fn golden_nested_jsonl() {
    let input = fixtures_dir().join("nested.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.arg(&input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Nested objects should show as {...}
    assert!(stdout.contains("{...}"), "Nested objects should show as {{...}}");
    assert!(stdout.contains("id"), "Should contain 'id' column");
    assert!(stdout.contains("user"), "Should contain 'user' column");
}

/// Test sorting output
#[test]
fn golden_sorted_ascending() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.arg("-s=age").arg(&input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Find positions of names to verify sort order
    let bob_pos = stdout.find("Bob").expect("Should contain Bob");
    let alice_pos = stdout.find("Alice").expect("Should contain Alice");
    let charlie_pos = stdout.find("Charlie").expect("Should contain Charlie");

    // Age order: Bob(25) < Alice(30) < Charlie(35)
    assert!(
        bob_pos < alice_pos,
        "Bob (25) should appear before Alice (30)"
    );
    assert!(
        alice_pos < charlie_pos,
        "Alice (30) should appear before Charlie (35)"
    );
}

/// Test descending sort
#[test]
fn golden_sorted_descending() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.arg("-s=-age").arg(&input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    let bob_pos = stdout.find("Bob").expect("Should contain Bob");
    let alice_pos = stdout.find("Alice").expect("Should contain Alice");
    let charlie_pos = stdout.find("Charlie").expect("Should contain Charlie");

    // Descending age order: Charlie(35) > Alice(30) > Bob(25)
    assert!(
        charlie_pos < alice_pos,
        "Charlie (35) should appear before Alice (30)"
    );
    assert!(
        alice_pos < bob_pos,
        "Alice (30) should appear before Bob (25)"
    );
}

/// Test column selection
#[test]
fn golden_column_selection() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.arg("-c").arg("name,age").arg(&input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Selected columns should be present
    assert!(stdout.contains("name"), "Should contain 'name' column");
    assert!(stdout.contains("age"), "Should contain 'age' column");

    // Non-selected column should NOT be present as header
    // (it might appear in the border/formatting but not as a column header)
    let lines: Vec<&str> = stdout.lines().collect();
    let header_line = lines.iter().find(|l| l.contains("name") && l.contains("age"));
    if let Some(header) = header_line {
        // Check that 'id' is not between column separators
        let parts: Vec<&str> = header.split(['┆', '│', '|']).collect();
        let has_id_column = parts.iter().any(|p| p.trim() == "id");
        assert!(!has_id_column, "Should NOT contain 'id' column in output");
    }
}

/// Test ASCII style output
#[test]
fn golden_ascii_style() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd
        .arg("--style")
        .arg("ascii")
        .arg(&input)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // ASCII style should use + and - for borders
    assert!(
        stdout.contains("+") && stdout.contains("-"),
        "ASCII style should use + and - for borders"
    );

    // Should NOT contain Unicode box drawing characters
    assert!(
        !stdout.contains("┌") && !stdout.contains("┆"),
        "ASCII style should not use Unicode box drawing"
    );
}

/// Test markdown style output
#[test]
fn golden_markdown_style() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd
        .arg("--style")
        .arg("markdown")
        .arg(&input)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Markdown style should use | for columns
    assert!(stdout.contains("|"), "Markdown style should use | separators");

    // Should have the header separator line with dashes
    assert!(
        stdout.contains("---") || stdout.contains("| -"),
        "Markdown should have separator line"
    );
}

/// Test empty input handling
#[test]
fn golden_empty_input() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin("")
        .assert()
        .success()
        .stdout(predicate::str::is_empty());
}

/// Test recursive expansion with nested arrays
#[test]
fn golden_recursive_expansion() {
    // Create input with nested array
    let input = r#"{"id": 1, "items": [{"name": "a"}, {"name": "b"}]}"#;

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd.arg("-r").write_stdin(input).output().unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Should show parent table with placeholder
    assert!(
        stdout.contains("[...]"),
        "Should show [...] placeholder for array"
    );

    // Should show child table header
    assert!(stdout.contains("items"), "Should show 'items' child table");
}

/// Test plain style (no borders)
#[test]
fn golden_plain_style() {
    let input = fixtures_dir().join("simple.jsonl");

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let output = cmd
        .arg("--style")
        .arg("plain")
        .arg(&input)
        .output()
        .unwrap();

    let stdout = String::from_utf8_lossy(&output.stdout);

    // Plain style should have data but minimal formatting
    assert!(stdout.contains("Alice"), "Should contain data");

    // Should NOT have heavy borders
    assert!(
        !stdout.contains("┌") && !stdout.contains("+--"),
        "Plain style should have minimal borders"
    );
}
