use serde_json::Value;
use std::fs::File;
use std::io::{self, BufRead, BufReader, Read, Seek, SeekFrom};

/// An indexed reader that stores byte offsets for each row,
/// enabling random access without re-parsing the entire file.
pub struct IndexedReader<R: Read + Seek> {
    reader: BufReader<R>,
    /// Byte offsets where each row starts
    offsets: Vec<u64>,
    /// Total number of rows
    row_count: usize,
}

impl IndexedReader<File> {
    /// Create an IndexedReader from a file path
    pub fn from_path(path: &std::path::Path) -> io::Result<Self> {
        let file = File::open(path)?;
        Self::new(file)
    }
}

impl<R: Read + Seek> IndexedReader<R> {
    /// Create a new IndexedReader by scanning the input to build an offset index
    pub fn new(reader: R) -> io::Result<Self> {
        let mut buf_reader = BufReader::new(reader);
        let offsets = Self::build_index(&mut buf_reader)?;
        let row_count = offsets.len();

        Ok(Self {
            reader: buf_reader,
            offsets,
            row_count,
        })
    }

    /// Build the offset index by scanning all lines
    fn build_index<T: BufRead + Seek>(reader: &mut T) -> io::Result<Vec<u64>> {
        let mut offsets = Vec::new();
        let mut line = String::new();

        // Start from beginning
        reader.seek(SeekFrom::Start(0))?;

        loop {
            let offset = reader.stream_position()?;
            line.clear();
            let bytes_read = reader.read_line(&mut line)?;

            if bytes_read == 0 {
                break;
            }

            // Only record offset if line has content (not just whitespace)
            if !line.trim().is_empty() {
                offsets.push(offset);
            }
        }

        // Reset to beginning
        reader.seek(SeekFrom::Start(0))?;

        Ok(offsets)
    }

    /// Get the total number of rows
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Get offsets (for testing)
    pub fn offsets(&self) -> &[u64] {
        &self.offsets
    }

    /// Read and parse a specific row by index
    pub fn get_row(&mut self, index: usize) -> io::Result<Option<Value>> {
        if index >= self.row_count {
            return Ok(None);
        }

        let offset = self.offsets[index];
        self.reader.seek(SeekFrom::Start(offset))?;

        let mut line = String::new();
        self.reader.read_line(&mut line)?;

        match serde_json::from_str(&line) {
            Ok(value) => Ok(Some(value)),
            Err(e) => Err(io::Error::new(
                io::ErrorKind::InvalidData,
                format!("JSON parse error at row {}: {}", index, e),
            )),
        }
    }

    /// Read a range of rows
    pub fn get_rows(&mut self, start: usize, end: usize) -> io::Result<Vec<Value>> {
        let end = end.min(self.row_count);
        let mut rows = Vec::with_capacity(end.saturating_sub(start));

        for i in start..end {
            if let Some(row) = self.get_row(i)? {
                rows.push(row);
            }
        }

        Ok(rows)
    }

    /// Iterate over all rows (for initial load or full scan)
    pub fn iter(&mut self) -> IndexedRowIterator<'_, R> {
        IndexedRowIterator {
            reader: self,
            current: 0,
        }
    }
}

/// Iterator over rows in an IndexedReader
pub struct IndexedRowIterator<'a, R: Read + Seek> {
    reader: &'a mut IndexedReader<R>,
    current: usize,
}

impl<'a, R: Read + Seek> Iterator for IndexedRowIterator<'a, R> {
    type Item = io::Result<Value>;

    fn next(&mut self) -> Option<Self::Item> {
        if self.current >= self.reader.row_count {
            return None;
        }

        let result = self.reader.get_row(self.current);
        self.current += 1;

        match result {
            Ok(Some(value)) => Some(Ok(value)),
            Ok(None) => None,
            Err(e) => Some(Err(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_test_reader(content: &str) -> IndexedReader<Cursor<Vec<u8>>> {
        let cursor = Cursor::new(content.as_bytes().to_vec());
        IndexedReader::new(cursor).unwrap()
    }

    #[test]
    fn test_build_index() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
"#;
        let reader = create_test_reader(content);
        assert_eq!(reader.row_count(), 3);
        assert_eq!(reader.offsets().len(), 3);
    }

    #[test]
    fn test_build_index_with_empty_lines() {
        let content = r#"{"id": 1}

{"id": 2}

{"id": 3}
"#;
        let reader = create_test_reader(content);
        // Should skip empty lines
        assert_eq!(reader.row_count(), 3);
    }

    #[test]
    fn test_get_row() {
        let content = r#"{"id": 1, "name": "alice"}
{"id": 2, "name": "bob"}
{"id": 3, "name": "charlie"}
"#;
        let mut reader = create_test_reader(content);

        // Access rows in any order
        let row2 = reader.get_row(1).unwrap().unwrap();
        assert_eq!(row2["name"], "bob");

        let row0 = reader.get_row(0).unwrap().unwrap();
        assert_eq!(row0["name"], "alice");

        let row2_again = reader.get_row(1).unwrap().unwrap();
        assert_eq!(row2_again["name"], "bob");
    }

    #[test]
    fn test_get_row_out_of_bounds() {
        let content = r#"{"id": 1}
"#;
        let mut reader = create_test_reader(content);
        assert!(reader.get_row(10).unwrap().is_none());
    }

    #[test]
    fn test_get_rows_range() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
{"id": 4}
{"id": 5}
"#;
        let mut reader = create_test_reader(content);

        let rows = reader.get_rows(1, 4).unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["id"], 2);
        assert_eq!(rows[1]["id"], 3);
        assert_eq!(rows[2]["id"], 4);
    }

    #[test]
    fn test_iterator() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
"#;
        let mut reader = create_test_reader(content);

        let rows: Vec<Value> = reader.iter().map(|r| r.unwrap()).collect();
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0]["id"], 1);
        assert_eq!(rows[2]["id"], 3);
    }

    #[test]
    fn test_empty_input() {
        let reader = create_test_reader("");
        assert_eq!(reader.row_count(), 0);
    }

    #[test]
    fn test_single_row() {
        let content = r#"{"single": true}"#;
        let mut reader = create_test_reader(content);
        assert_eq!(reader.row_count(), 1);

        let row = reader.get_row(0).unwrap().unwrap();
        assert_eq!(row["single"], true);
    }
}
