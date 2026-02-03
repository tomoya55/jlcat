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
        .stdout(predicate::str::contains("0.1.0"));
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
