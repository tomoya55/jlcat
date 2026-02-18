mod cli;
mod core;
mod error;
mod input;
mod render;

use clap::Parser;
use cli::Cli;
use core::{
    ChildTable, ColumnSelector, FlatConfig, FlatTableData, NestedExtractor, Sorter, TableData,
};
use error::{JlcatError, Result};
use input::{sniff_format, InputFormat};
use render::CatRenderer;
use serde_json::Value;
use std::collections::VecDeque;
use std::io::{self, BufRead, BufReader, Read};

fn main() -> Result<()> {
    let cli = Cli::parse();

    // Check for stdin without input
    if cli.file.is_none() && atty::is(atty::Stream::Stdin) {
        eprintln!("Usage: jlcat [OPTIONS] [FILE]");
        eprintln!("Try 'jlcat --help' for more information.");
        std::process::exit(1);
    }

    // Read input
    let rows = read_input(&cli)?;

    if rows.is_empty() {
        return Ok(());
    }

    // Apply sorting if specified
    let mut rows = rows;
    if let Some(ref sort_keys) = cli.sort {
        let sorter = Sorter::parse(sort_keys)?;
        sorter.sort(&mut rows);
    }

    // Build column selector if specified
    let selector = if let Some(ref cols) = cli.columns {
        Some(ColumnSelector::new(cols.clone())?)
    } else {
        None
    };

    // Render
    if cli.interactive {
        // TUI mode
        if cli.is_flat() {
            let config = FlatConfig::new(cli.flat_depth(), cli.array_limit);
            let flat_table = FlatTableData::from_rows(&rows, config);
            render::tui::run_flat(flat_table, rows)?;
        } else {
            let table_data = TableData::from_rows(rows.clone(), selector);
            render::tui::run(table_data, rows)?;
        }
    } else {
        let renderer = CatRenderer::new(cli.style.clone());

        if cli.is_flat() {
            // Flat mode - expand nested objects
            let config = FlatConfig::new(cli.flat_depth(), cli.array_limit);
            let flat_table = FlatTableData::from_rows(&rows, config);
            println!("{}", renderer.render_flat(&flat_table));
        } else if cli.recursive {
            // Extract nested structures
            let children = NestedExtractor::extract(&rows);

            // For parent table:
            // - If column selector is provided, use original rows so nested paths resolve
            // - Otherwise, flatten rows to show placeholders for nested structures
            let parent_table = if selector.is_some() {
                // Column selection: use original rows so paths like "address.city" work
                TableData::from_rows(rows.clone(), selector)
            } else {
                // No column selection: flatten to show placeholders
                let flat_rows: Vec<Value> = rows.iter().map(NestedExtractor::flatten_row).collect();
                TableData::from_rows(flat_rows, None)
            };
            println!("{}", renderer.render(&parent_table));

            // Render child tables
            let mut child_names: Vec<_> = children.keys().collect();
            child_names.sort(); // Consistent ordering

            for name in child_names {
                let child = &children[name];
                if !child.is_empty() {
                    println!("\n## {}\n", name);
                    let child_table = child_table_to_table_data(child);
                    println!("{}", renderer.render(&child_table));
                }
            }
        } else {
            // Normal mode - render all data as single table
            let table_data = TableData::from_rows(rows, selector);
            println!("{}", renderer.render(&table_data));
        }
    }

    Ok(())
}

fn read_input(cli: &Cli) -> Result<Vec<Value>> {
    let skip = cli.skip.unwrap_or(0);
    let limit = cli.limit;
    let tail = cli.tail;

    if let Some(ref path) = cli.file {
        let file = std::fs::File::open(path)?;
        let reader = BufReader::new(file);

        // Peek to detect format (same as stdin)
        let mut peekable = PeekableReader::new(reader);
        let peek = peekable.peek(64)?;

        match sniff_format(&peek) {
            Some(InputFormat::JsonArray) => {
                read_json_array(&mut peekable, cli.is_strict(), skip, limit, tail)
            }
            Some(InputFormat::JsonLines) | None => {
                read_from_lines(peekable.lines(), cli.is_strict(), skip, limit, tail)
            }
        }
    } else {
        let stdin = io::stdin();
        let reader = BufReader::new(stdin.lock());

        // Peek to detect format
        let mut peekable = PeekableReader::new(reader);
        let peek = peekable.peek(64)?;

        match sniff_format(&peek) {
            Some(InputFormat::JsonArray) => {
                read_json_array(&mut peekable, cli.is_strict(), skip, limit, tail)
            }
            Some(InputFormat::JsonLines) | None => {
                read_from_lines(peekable.lines(), cli.is_strict(), skip, limit, tail)
            }
        }
    }
}

fn read_from_lines<I>(
    lines: I,
    strict: bool,
    skip: usize,
    limit: Option<usize>,
    tail: Option<usize>,
) -> Result<Vec<Value>>
where
    I: Iterator<Item = io::Result<String>>,
{
    if tail == Some(0) || limit == Some(0) {
        return Ok(Vec::new());
    }

    let mut rows = Vec::new();
    let mut tail_buf: Option<(usize, VecDeque<Value>)> =
        tail.map(|n| (n, VecDeque::with_capacity(n)));
    let mut skipped = 0usize;

    for (line_num, line) in lines.enumerate() {
        let line = line?;
        if line.trim().is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(&line) {
            Ok(value) => {
                if value.is_object() {
                    if let Some((count, buf)) = tail_buf.as_mut() {
                        if buf.len() == *count {
                            buf.pop_front();
                        }
                        buf.push_back(value);
                    } else {
                        if skipped < skip {
                            skipped += 1;
                            continue;
                        }
                        rows.push(value);
                        if let Some(max) = limit {
                            if rows.len() >= max {
                                break;
                            }
                        }
                    }
                } else if strict {
                    return Err(JlcatError::JsonParse {
                        line: line_num + 1,
                        message: "expected JSON object, got non-object value".to_string(),
                    });
                } else {
                    eprintln!(
                        "jlcat: warning: line {}: expected JSON object, skipping",
                        line_num + 1
                    );
                }
            }
            Err(e) => {
                if strict {
                    return Err(JlcatError::JsonParse {
                        line: line_num + 1,
                        message: e.to_string(),
                    });
                } else {
                    eprintln!(
                        "jlcat: warning: line {}: invalid JSON, skipping",
                        line_num + 1
                    );
                }
            }
        }
    }

    if let Some((_, buf)) = tail_buf {
        Ok(buf.into_iter().collect())
    } else {
        Ok(rows)
    }
}

fn read_json_array<R: Read>(
    reader: &mut PeekableReader<R>,
    strict: bool,
    skip: usize,
    limit: Option<usize>,
    tail: Option<usize>,
) -> Result<Vec<Value>> {
    if tail == Some(0) || limit == Some(0) {
        return Ok(Vec::new());
    }

    enum PagingMode {
        Window { skip: usize, limit: Option<usize> },
        Tail { count: usize },
    }

    struct ArrayVisitor {
        strict: bool,
        mode: PagingMode,
    }

    impl<'de> serde::de::Visitor<'de> for ArrayVisitor {
        type Value = Vec<Value>;

        fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
            formatter.write_str("a JSON array")
        }

        fn visit_seq<A>(self, mut seq: A) -> std::result::Result<Self::Value, A::Error>
        where
            A: serde::de::SeqAccess<'de>,
        {
            match self.mode {
                PagingMode::Tail { count } => {
                    let mut buf: VecDeque<Value> = VecDeque::with_capacity(count);
                    let mut idx = 0usize;

                    while let Some(value) = seq.next_element::<Value>()? {
                        idx += 1;
                        if value.is_object() {
                            if buf.len() == count {
                                buf.pop_front();
                            }
                            buf.push_back(value);
                        } else if self.strict {
                            return Err(serde::de::Error::custom(format!(
                                "array element {} is not an object",
                                idx
                            )));
                        }
                    }

                    Ok(buf.into_iter().collect())
                }
                PagingMode::Window { skip, limit } => {
                    let mut rows: Vec<Value> = Vec::new();
                    let mut skipped = 0usize;
                    let mut idx = 0usize;
                    let mut limit_reached = false;

                    while let Some(value) = seq.next_element::<Value>()? {
                        idx += 1;

                        if !value.is_object() {
                            if self.strict {
                                return Err(serde::de::Error::custom(format!(
                                    "array element {} is not an object",
                                    idx
                                )));
                            }
                            continue;
                        }

                        if skipped < skip {
                            skipped += 1;
                            continue;
                        }

                        if let Some(max) = limit {
                            if rows.len() >= max {
                                limit_reached = true;
                                break;
                            }
                        }

                        rows.push(value);

                        if let Some(max) = limit {
                            if rows.len() >= max {
                                limit_reached = true;
                                break;
                            }
                        }
                    }

                    if limit_reached {
                        while seq.next_element::<serde::de::IgnoredAny>()?.is_some() {}
                    }

                    Ok(rows)
                }
            }
        }
    }

    let mode = if let Some(n) = tail {
        PagingMode::Tail { count: n }
    } else {
        PagingMode::Window { skip, limit }
    };

    let mut de = serde_json::Deserializer::from_reader(reader);
    serde::de::Deserializer::deserialize_seq(&mut de, ArrayVisitor { strict, mode }).map_err(|e| {
        JlcatError::JsonParse {
            line: 1,
            message: e.to_string(),
        }
    })
}

/// Convert a ChildTable to TableData for rendering
fn child_table_to_table_data(child: &ChildTable) -> TableData {
    let columns = child.columns_with_parent();
    let rows = child.rows_with_parent();

    // Convert to JSON objects for TableData
    let json_rows: Vec<Value> = rows
        .into_iter()
        .map(|values| {
            let mut obj = serde_json::Map::new();
            for (col, val) in columns.iter().zip(values) {
                obj.insert(col.clone(), val);
            }
            Value::Object(obj)
        })
        .collect();

    TableData::from_rows(json_rows, None)
}

/// A reader that can peek ahead without consuming bytes
struct PeekableReader<R: Read> {
    inner: R,
    buffer: Vec<u8>,
    pos: usize,
}

impl<R: Read> PeekableReader<R> {
    fn new(inner: R) -> Self {
        Self {
            inner,
            buffer: Vec::new(),
            pos: 0,
        }
    }

    fn peek(&mut self, n: usize) -> io::Result<Vec<u8>> {
        if self.buffer.len() < n {
            let mut buf = vec![0u8; n - self.buffer.len()];
            let read = self.inner.read(&mut buf)?;
            self.buffer.extend_from_slice(&buf[..read]);
        }
        Ok(self.buffer.clone())
    }

    fn lines(self) -> impl Iterator<Item = io::Result<String>> {
        // Create a reader that first yields buffered content, then the rest
        let chained = io::Cursor::new(self.buffer).chain(self.inner);
        BufReader::new(chained).lines()
    }
}

impl<R: Read> Read for PeekableReader<R> {
    fn read(&mut self, buf: &mut [u8]) -> io::Result<usize> {
        // First drain the buffer
        if self.pos < self.buffer.len() {
            let remaining = &self.buffer[self.pos..];
            let to_copy = std::cmp::min(remaining.len(), buf.len());
            buf[..to_copy].copy_from_slice(&remaining[..to_copy]);
            self.pos += to_copy;
            if to_copy < buf.len() {
                // Need more from inner
                let additional = self.inner.read(&mut buf[to_copy..])?;
                Ok(to_copy + additional)
            } else {
                Ok(to_copy)
            }
        } else {
            self.inner.read(buf)
        }
    }
}
