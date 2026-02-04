# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to [Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.3] - 2026-02-04

### Added

- **Detail view modal** - Press Enter in TUI mode to view selected row's full JSON with syntax highlighting
- **JSON syntax highlighting** - Keys (Cyan), strings (Green), numbers (Yellow), booleans (Magenta), null (DarkGray)
- **Detail view navigation** - j/k, arrows, PageUp/Down for scrolling; g/G for top/bottom; Esc to close
- Detail view keybindings documented in README

### Fixed

- Dynamic viewport height calculation for detail view scrolling

## [0.1.2] - 2026-02-04

### Added

- Release command for streamlined version management

## [0.1.1] - 2026-02-04

### Added

- **Flat mode** (`--flat`) - Alternative column generation that expands nested objects and arrays into separate columns with indexed suffixes (e.g., `items.0.name`, `items.1.name`)
- **Array limit** (`--array-limit N`) - Control maximum array elements to expand in flat mode (default: 10)
- Flat mode support in both CLI and interactive TUI modes

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

[0.1.3]: https://github.com/tomoya55/jlcat/releases/tag/v0.1.3
[0.1.2]: https://github.com/tomoya55/jlcat/releases/tag/v0.1.2
[0.1.1]: https://github.com/tomoya55/jlcat/releases/tag/v0.1.1
[0.1.0]: https://github.com/tomoya55/jlcat/releases/tag/v0.1.0
