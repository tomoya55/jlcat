use clap::{Parser, ValueEnum};
use std::path::PathBuf;

#[derive(Parser, Debug)]
#[command(name = "jlcat")]
#[command(about = "JSON/JSONL table viewer with TUI support")]
#[command(version)]
pub struct Cli {
    /// JSON or JSONL file path (reads from stdin if omitted)
    pub file: Option<PathBuf>,

    /// Launch in interactive TUI mode
    #[arg(short, long)]
    pub interactive: bool,

    /// Recursively expand nested structures as child tables
    #[arg(short, long)]
    pub recursive: bool,

    /// Columns to display (comma-separated, supports dot notation)
    #[arg(short, long, value_delimiter = ',')]
    pub columns: Option<Vec<String>>,

    /// Sort keys (comma-separated, prefix with - for descending)
    #[arg(short, long, value_delimiter = ',')]
    pub sort: Option<Vec<String>>,

    /// Table style
    #[arg(long, value_enum, default_value = "rounded")]
    pub style: TableStyle,

    /// Exit on invalid JSON line (default: true)
    #[arg(long, default_value = "true", action = clap::ArgAction::Set)]
    pub strict: bool,

    /// Skip invalid JSON lines with warning
    #[arg(long)]
    pub lenient: bool,

    /// Flatten nested objects into dot-notation columns
    /// Optional depth limit (e.g., --flat or --flat=3)
    #[arg(long = "flat", value_name = "DEPTH", num_args = 0..=1, default_missing_value = "")]
    flat_raw: Option<String>,

    /// Maximum array elements to display in flat mode
    #[arg(long, default_value = "3")]
    pub array_limit: usize,
}

#[derive(ValueEnum, Clone, Debug, Default)]
pub enum TableStyle {
    Ascii,
    #[default]
    Rounded,
    Markdown,
    Plain,
}

impl Cli {
    pub fn is_strict(&self) -> bool {
        // Honor both flags: strict mode requires --strict=true (default) AND no --lenient
        self.strict && !self.lenient
    }

    /// Get flat option: None if not provided, Some(None) if --flat, Some(Some(n)) if --flat=n
    pub fn flat(&self) -> Option<Option<usize>> {
        self.flat_raw
            .as_ref()
            .map(|s| if s.is_empty() { None } else { s.parse().ok() })
    }

    /// Check if flat mode is enabled
    pub fn is_flat(&self) -> bool {
        self.flat_raw.is_some()
    }

    /// Get flat depth (None = unlimited)
    pub fn flat_depth(&self) -> Option<usize> {
        self.flat().flatten()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    #[test]
    fn test_flat_flag_only() {
        let cli = Cli::parse_from(["jlcat", "--flat"]);
        assert!(cli.flat().is_some());
        assert_eq!(cli.flat(), Some(None)); // flat enabled, no depth
    }

    #[test]
    fn test_flat_with_depth() {
        let cli = Cli::parse_from(["jlcat", "--flat=3"]);
        assert_eq!(cli.flat(), Some(Some(3)));
    }

    #[test]
    fn test_array_limit() {
        let cli = Cli::parse_from(["jlcat", "--flat", "--array-limit=5"]);
        assert_eq!(cli.array_limit, 5);
    }

    #[test]
    fn test_array_limit_default() {
        let cli = Cli::parse_from(["jlcat", "--flat"]);
        assert_eq!(cli.array_limit, 3);
    }
}
