use std::fs::File;
use std::io::{self, BufRead, BufReader, Read};
use std::path::Path;

pub trait InputSource {
    type Lines: Iterator<Item = io::Result<String>>;
    fn lines(self) -> Self::Lines;
}

pub struct FileSource {
    reader: BufReader<File>,
}

impl FileSource {
    pub fn open<P: AsRef<Path>>(path: P) -> io::Result<Self> {
        let file = File::open(path)?;
        Ok(Self {
            reader: BufReader::new(file),
        })
    }
}

impl InputSource for FileSource {
    type Lines = io::Lines<BufReader<File>>;

    fn lines(self) -> Self::Lines {
        self.reader.lines()
    }
}

pub struct StdinSource<R: Read> {
    reader: BufReader<R>,
}

impl StdinSource<io::Stdin> {
    pub fn new() -> Self {
        Self {
            reader: BufReader::new(io::stdin()),
        }
    }
}

impl Default for StdinSource<io::Stdin> {
    fn default() -> Self {
        Self::new()
    }
}

impl<R: Read> StdinSource<R> {
    pub fn from_reader(reader: R) -> Self {
        Self {
            reader: BufReader::new(reader),
        }
    }
}

impl<R: Read> InputSource for StdinSource<R> {
    type Lines = io::Lines<BufReader<R>>;

    fn lines(self) -> Self::Lines {
        self.reader.lines()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_file_source_read_lines() {
        let source = FileSource::open("tests/fixtures/simple.jsonl").unwrap();
        let lines: Vec<String> = source.lines().map(|l| l.unwrap()).collect();
        assert_eq!(lines.len(), 3);
        assert!(lines[0].contains("Alice"));
    }

    #[test]
    fn test_stdin_source_read_lines() {
        let data = r#"{"id": 1}
{"id": 2}"#;
        let cursor = Cursor::new(data);
        let source = StdinSource::from_reader(cursor);
        let lines: Vec<String> = source.lines().map(|l| l.unwrap()).collect();
        assert_eq!(lines.len(), 2);
    }
}
