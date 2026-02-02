# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.0] - 2026-02-02

### Added

- **Auto-flatten nested objects** - Nested JSON objects are automatically flattened into dot-notation columns (e.g., `user.profile.age`)
- **Interactive TUI mode** (`-i`) - Browse data with keyboard navigation, search, and filtering
- **Multiple table styles** - ASCII, rounded, markdown, and plain output formats
- **Column selection** (`-c`) - Pick specific columns with wildcard support (e.g., `user.*`)
- **Sorting** (`-s`) - Sort by any column, ascending or descending (prefix with `-`)
- **Flexible input** - Read from files or stdin, supports both JSON arrays and JSONL formats
- **Lenient mode** (`--lenient`) - Skip invalid JSON lines instead of erroring
- **Disable flattening** (`--no-flatten`) - Keep nested objects as JSON strings
- Cross-platform support: Linux, macOS (Intel & Apple Silicon), Windows

[0.1.0]: https://github.com/tomoya55/jlcat/releases/tag/0.1.0
