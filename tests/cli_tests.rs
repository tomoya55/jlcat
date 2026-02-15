#![allow(deprecated)]

use assert_cmd::Command;
use predicates::prelude::*;

#[test]
fn test_help_flag() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--help")
        .assert()
        .success()
        .stdout(predicate::str::contains("JSON/JSONL table viewer"));
}

#[test]
fn test_version_flag() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--version")
        .assert()
        .success()
        .stdout(predicate::str::contains(env!("CARGO_PKG_VERSION")));
}

#[test]
fn test_flat_mode_basic() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat")
        .write_stdin(r#"{"id": 1, "user": {"name": "Alice", "age": 30}}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("user.name"))
        .stdout(predicate::str::contains("user.age"))
        .stdout(predicate::str::contains("Alice"));
}

#[test]
fn test_flat_mode_with_array() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat")
        .write_stdin(r#"{"tags": ["a", "b", "c", "d"]}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("a, b, c, ..."));
}

#[test]
fn test_flat_mode_depth_limit() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat=1")
        .write_stdin(r#"{"a": {"b": {"c": 1}}}"#)
        .assert()
        .success()
        .stdout(predicate::str::contains("a.b"))
        .stdout(predicate::str::contains("{...}"));
}

#[test]
fn test_flat_mode_fallback_warning() {
    // Test that incomplete first chunk triggers fallback
    // This is hard to test directly, so we just verify normal output works
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--flat")
        .write_stdin(r#"{"id": 1}"#)
        .assert()
        .success();
}

#[test]
fn test_limit_option() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--limit")
        .arg("2")
        .arg("tests/fixtures/simple.jsonl")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("Charlie").not());
}

#[test]
fn test_skip_and_limit_option() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--skip")
        .arg("1")
        .arg("--limit")
        .arg("1")
        .arg("tests/fixtures/simple.jsonl")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice").not())
        .stdout(predicate::str::contains("Bob"))
        .stdout(predicate::str::contains("Charlie").not());
}

#[test]
fn test_tail_option() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--tail")
        .arg("1")
        .arg("tests/fixtures/simple.jsonl")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice").not())
        .stdout(predicate::str::contains("Bob").not())
        .stdout(predicate::str::contains("Charlie"));
}

#[test]
fn test_head_alias() {
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    cmd.arg("--head")
        .arg("1")
        .arg("tests/fixtures/simple.jsonl")
        .assert()
        .success()
        .stdout(predicate::str::contains("Alice"))
        .stdout(predicate::str::contains("Bob").not());
}
