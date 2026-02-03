#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_cat_mode_file() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("tests/fixtures/simple.jsonl")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("Charlie"));
}

#[test]
fn test_cat_mode_stdin() {
    let input = r#"{"id": 1, "name": "Test"}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Test"));
}

#[test]
fn test_column_selection() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-c", "name", "tests/fixtures/simple.jsonl"])
        .assert()
        .success()
        .stdout(predicate::str::contains("name"))
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_sort_ascending() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-c", "name,age", "-s", "age", "tests/fixtures/simple.jsonl"])
        .assert()
        .success();
    // Bob (25) should come before Alice (30) should come before Charlie (35)
}

#[test]
fn test_sort_descending() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-c", "name,age", "-s=-age", "tests/fixtures/simple.jsonl"])
        .assert()
        .success();
    // Charlie (35) should come first
}

#[test]
fn test_json_array_input() {
    let input = r#"[{"id": 1, "name": "A"}, {"id": 2, "name": "B"}]"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("A"))
        .stdout(predicate::str::contains("B"));
}

#[test]
fn test_ascii_style() {
    let input = r#"{"id": 1}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["--style", "ascii"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("+"))
        .stdout(predicate::str::contains("-"));
}

#[test]
fn test_markdown_style() {
    let input = r#"{"id": 1}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["--style", "markdown"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("|"));
}

#[test]
fn test_lenient_mode() {
    let input = "invalid json\n{\"id\": 1}";
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["--lenient"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("1"))
        .stderr(predicate::str::contains("warning"));
}

#[test]
fn test_strict_mode_error() {
    let input = "invalid json\n{\"id\": 1}";
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin(input).assert().failure();
}

#[test]
fn test_nested_column_selection() {
    let input = r#"{"id": 1, "address": {"city": "Tokyo"}}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-c", "id,address.city"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Tokyo"));
}

#[test]
fn test_empty_input() {
    let input = "";
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin(input).assert().success().stdout("");
}

#[test]
fn test_file_not_found() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("nonexistent.jsonl").assert().failure();
}

#[test]
fn test_recursive_nested_object() {
    let input = r#"{"id": 1, "address": {"city": "Tokyo"}}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-r"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("{...}"))
        .stdout(predicate::str::contains("## address"))
        .stdout(predicate::str::contains("Tokyo"))
        .stdout(predicate::str::contains("_parent_row"));
}

#[test]
fn test_recursive_array() {
    let input = r#"{"id": 1, "items": [{"name": "A"}, {"name": "B"}]}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-r"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("[...]"))
        .stdout(predicate::str::contains("## items"))
        .stdout(predicate::str::contains("A"))
        .stdout(predicate::str::contains("B"));
}

#[test]
fn test_recursive_no_nested() {
    // Should work normally when no nested data
    let input = r#"{"id": 1, "name": "Alice"}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-r"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_recursive_with_nested_column_selection() {
    // Nested column selection should work in recursive mode
    let input = r#"{"id": 1, "address": {"city": "Tokyo", "zip": "100"}}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["-r", "-c", "id,address.city"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Tokyo"))
        .stdout(predicate::str::contains("1"));
}

#[test]
fn test_strict_mode_rejects_non_object_jsonl() {
    // Strict mode (default) should reject non-object JSON values
    let input = "1\n\"foo\"\n[1,2,3]";
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.write_stdin(input)
        .assert()
        .failure()
        .stderr(predicate::str::contains("expected JSON object"));
}

#[test]
fn test_lenient_mode_skips_non_object_jsonl() {
    // Lenient mode should skip non-object values with warning
    let input = r#"{"id": 1}
42
{"id": 2}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["--lenient"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::contains("expected JSON object, skipping"));
}

#[test]
fn test_lenient_mode_renders_valid_objects_only() {
    // Lenient mode should render only valid objects
    let input = r#"{"name": "Alice"}
"string_value"
{"name": "Bob"}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["--lenient"])
        .write_stdin(input)
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("string_value").not());
}

#[test]
fn test_strict_false_enables_lenient_mode() {
    // --strict=false should behave like --lenient
    let input = r#"{"name": "Alice"}
invalid json line
{"name": "Bob"}"#;
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.args(["--strict=false"])
        .write_stdin(input)
        .assert()
        .success()
        .stderr(predicate::str::contains("warning"))
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob"));
}

mod flat_mode_tests {
    use assert_cmd::Command;
    use predicates::prelude::*;

    #[test]
    fn test_flat_nested_object() {
        let input =
            r#"{"id": 1, "user": {"name": "Alice", "profile": {"age": 30, "city": "Tokyo"}}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("user.name"))
            .stdout(predicate::str::contains("user.profile.age"))
            .stdout(predicate::str::contains("user.profile.city"))
            .stdout(predicate::str::contains("Alice"))
            .stdout(predicate::str::contains("30"))
            .stdout(predicate::str::contains("Tokyo"));
    }

    #[test]
    fn test_flat_array_elements() {
        let input = r#"{"tags": ["rust", "json", "cli", "tui", "awesome"]}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("rust, json, cli, ..."));
    }

    #[test]
    fn test_flat_array_limit_custom() {
        let input = r#"{"tags": ["a", "b", "c", "d", "e"]}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .arg("--array-limit=5")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("a, b, c, d, e"));
    }

    #[test]
    fn test_flat_depth_limit() {
        let input = r#"{"a": {"b": {"c": {"d": 1}}}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat=2")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("a.b.c"))
            .stdout(predicate::str::contains("{...}"));
    }

    #[test]
    fn test_flat_structure_conflict_object_to_scalar() {
        let input = r#"{"id": 1, "data": {"x": 1}}
{"id": 2, "data": "simple"}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("data.x"))
            .stdout(predicate::str::contains("data"))
            .stdout(predicate::str::contains("simple"));
    }

    #[test]
    fn test_flat_with_markdown_style() {
        let input = r#"{"user": {"name": "Alice"}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .arg("--style=markdown")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("| user.name |"))
            .stdout(predicate::str::contains("Alice"));
    }

    #[test]
    fn test_flat_multiple_rows() {
        let input = r#"{"id": 1, "info": {"status": "active"}}
{"id": 2, "info": {"status": "inactive"}}
{"id": 3, "info": {"status": "pending"}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("info.status"))
            .stdout(predicate::str::contains("active"))
            .stdout(predicate::str::contains("inactive"))
            .stdout(predicate::str::contains("pending"));
    }

    #[test]
    fn test_flat_empty_array() {
        let input = r#"{"tags": []}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat").write_stdin(input).assert().success();
    }

    #[test]
    fn test_flat_null_values() {
        let input = r#"{"user": {"name": null, "age": 30}}"#;

        let mut cmd = Command::cargo_bin("jlcat").unwrap();
        cmd.arg("--flat")
            .write_stdin(input)
            .assert()
            .success()
            .stdout(predicate::str::contains("null"));
    }
}
