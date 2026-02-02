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
}
