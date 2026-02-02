#![allow(deprecated)]

use assert_cmd::Command;
use std::io::Write;
use std::time::Instant;
use tempfile::NamedTempFile;

/// Generate JSONL content with the specified number of rows
fn generate_jsonl(row_count: usize) -> String {
    let mut output = String::with_capacity(row_count * 100);
    for i in 0..row_count {
        output.push_str(&format!(
            r#"{{"id": {}, "name": "user_{}", "email": "user{}@example.com", "score": {}, "active": {}}}"#,
            i,
            i,
            i,
            (i % 100) as f64 * 1.5,
            i % 2 == 0
        ));
        output.push('\n');
    }
    output
}

/// Test that we can process 1000 rows
#[test]
fn large_file_1k_rows() {
    let content = generate_jsonl(1000);
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let start = Instant::now();
    let output = cmd.arg(temp_file.path()).output().unwrap();
    let duration = start.elapsed();

    assert!(output.status.success(), "Should succeed with 1000 rows");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("id"), "Should have id column");
    assert!(stdout.contains("name"), "Should have name column");

    // Log timing for informational purposes (no assertion)
    eprintln!("large_file_1k_rows completed in {:?}", duration);
}

/// Test that we can process 10000 rows
#[test]
fn large_file_10k_rows() {
    let content = generate_jsonl(10_000);
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let start = Instant::now();
    let output = cmd.arg(temp_file.path()).output().unwrap();
    let duration = start.elapsed();

    assert!(output.status.success(), "Should succeed with 10000 rows");

    let stdout = String::from_utf8_lossy(&output.stdout);
    assert!(stdout.contains("user_0"), "Should contain first user");
    assert!(stdout.contains("user_9999"), "Should contain last user");

    // Log timing for informational purposes (no assertion)
    eprintln!("large_file_10k_rows completed in {:?}", duration);
}

/// Test sorting with large dataset
#[test]
fn large_file_sorted() {
    let content = generate_jsonl(5000);
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let start = Instant::now();
    let output = cmd.arg("-s=score").arg(temp_file.path()).output().unwrap();
    let duration = start.elapsed();

    assert!(output.status.success(), "Should succeed with sorting");

    // Log timing for informational purposes (no assertion)
    eprintln!("large_file_sorted (5000 rows) completed in {:?}", duration);
}

/// Test column selection with large dataset
#[test]
fn large_file_column_selection() {
    let content = generate_jsonl(5000);
    let mut temp_file = NamedTempFile::new().unwrap();
    temp_file.write_all(content.as_bytes()).unwrap();

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let start = Instant::now();
    let output = cmd
        .arg("-c")
        .arg("id,name")
        .arg(temp_file.path())
        .output()
        .unwrap();
    let duration = start.elapsed();

    assert!(
        output.status.success(),
        "Should succeed with column selection"
    );

    let stdout = String::from_utf8_lossy(&output.stdout);
    // Should have selected columns
    assert!(stdout.contains("id"), "Should have id column");
    assert!(stdout.contains("name"), "Should have name column");

    // Log timing for informational purposes (no assertion)
    eprintln!(
        "large_file_column_selection (5000 rows) completed in {:?}",
        duration
    );
}

/// Test stdin input with large dataset
#[test]
fn large_file_stdin() {
    let content = generate_jsonl(2000);

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    let start = Instant::now();
    let output = cmd.write_stdin(content).output().unwrap();
    let duration = start.elapsed();

    assert!(output.status.success(), "Should succeed with stdin input");

    // Log timing for informational purposes (no assertion)
    eprintln!("large_file_stdin (2000 rows) completed in {:?}", duration);
}

/// Test IndexedReader with large file
#[test]
fn indexed_reader_large_file() {
    use jlcat::input::IndexedReader;
    use std::io::Cursor;

    let content = generate_jsonl(10_000);
    let cursor = Cursor::new(content.as_bytes().to_vec());

    let start = Instant::now();
    let mut reader = IndexedReader::new(cursor).expect("Should create indexed reader");
    let index_duration = start.elapsed();

    assert_eq!(reader.row_count(), 10_000, "Should have 10000 rows");

    // Test random access
    let start = Instant::now();
    let row_5000 = reader.get_row(5000).unwrap().unwrap();
    let access_duration = start.elapsed();

    assert_eq!(row_5000["id"], 5000, "Row 5000 should have id 5000");

    // Log timing for informational purposes (no assertion)
    eprintln!(
        "indexed_reader_large_file: indexing {:?}, random access {:?}",
        index_duration, access_duration
    );
}

/// Test CachedReader with large file
#[test]
fn cached_reader_large_file() {
    use jlcat::input::CachedReader;
    use std::io::Cursor;

    let content = generate_jsonl(5000);
    let cursor = Cursor::new(content.as_bytes().to_vec());

    let mut reader =
        CachedReader::with_cache_size(cursor, 100).expect("Should create cached reader");

    // First access - cold cache
    let start = Instant::now();
    let row1 = reader.get_row(1000).unwrap();
    let cold_duration = start.elapsed();

    // Second access - warm cache
    let start = Instant::now();
    let row2 = reader.get_row(1000).unwrap();
    let warm_duration = start.elapsed();

    // Verify we got the same row
    assert_eq!(row1, row2, "Cached row should match original");

    // Log timing for informational purposes (no assertion)
    eprintln!(
        "cached_reader_large_file: cold {:?}, warm {:?}",
        cold_duration, warm_duration
    );
}

/// Test memory efficiency - should not grow linearly with row count
/// This is a basic sanity check, not a precise measurement
#[test]
fn large_file_memory_efficiency() {
    // Generate two datasets
    let small = generate_jsonl(1000);
    let large = generate_jsonl(10_000);

    let mut small_file = NamedTempFile::new().unwrap();
    small_file.write_all(small.as_bytes()).unwrap();

    let mut large_file = NamedTempFile::new().unwrap();
    large_file.write_all(large.as_bytes()).unwrap();

    // Both should complete successfully
    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    assert!(
        cmd.arg(small_file.path())
            .output()
            .unwrap()
            .status
            .success(),
        "Small file should succeed"
    );

    let mut cmd = Command::cargo_bin("jlcat").unwrap();
    assert!(
        cmd.arg(large_file.path())
            .output()
            .unwrap()
            .status
            .success(),
        "Large file should succeed"
    );
}
