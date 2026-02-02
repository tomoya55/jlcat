use crate::core::RowCache;
use serde_json::Value;
use std::fs::File;
use std::io::{self, Read, Seek};

use super::indexed::IndexedReader;

/// A cached reader that combines IndexedReader with RowCache
/// for efficient random access with caching
pub struct CachedReader<R: Read + Seek> {
    indexed: IndexedReader<R>,
    cache: RowCache,
}

impl CachedReader<File> {
    /// Create a CachedReader from a file path
    pub fn from_path(path: &std::path::Path) -> io::Result<Self> {
        Self::from_path_with_cache_size(path, 1000)
    }

    /// Create a CachedReader from a file path with custom cache size
    pub fn from_path_with_cache_size(path: &std::path::Path, cache_size: usize) -> io::Result<Self> {
        let indexed = IndexedReader::from_path(path)?;
        Ok(Self {
            indexed,
            cache: RowCache::new(cache_size),
        })
    }
}

impl<R: Read + Seek> CachedReader<R> {
    /// Create a new CachedReader with default cache size
    pub fn new(reader: R) -> io::Result<Self> {
        Self::with_cache_size(reader, 1000)
    }

    /// Create a new CachedReader with custom cache size
    pub fn with_cache_size(reader: R, cache_size: usize) -> io::Result<Self> {
        let indexed = IndexedReader::new(reader)?;
        Ok(Self {
            indexed,
            cache: RowCache::new(cache_size),
        })
    }

    /// Get the total number of rows
    pub fn row_count(&self) -> usize {
        self.indexed.row_count()
    }

    /// Get a row by index, using cache if available
    pub fn get_row(&mut self, index: usize) -> io::Result<Option<Value>> {
        // Check cache first
        if let Some(value) = self.cache.get(index) {
            return Ok(Some(value.clone()));
        }

        // Not in cache, read from indexed reader
        if let Some(value) = self.indexed.get_row(index)? {
            self.cache.insert(index, value.clone());
            Ok(Some(value))
        } else {
            Ok(None)
        }
    }

    /// Get a range of rows, using cache where available
    pub fn get_rows(&mut self, start: usize, end: usize) -> io::Result<Vec<Value>> {
        let end = end.min(self.row_count());
        let mut rows = Vec::with_capacity(end.saturating_sub(start));

        for i in start..end {
            if let Some(row) = self.get_row(i)? {
                rows.push(row);
            }
        }

        Ok(rows)
    }

    /// Prefetch a range of rows into the cache (for viewport loading)
    pub fn prefetch(&mut self, start: usize, end: usize) -> io::Result<()> {
        let end = end.min(self.row_count());
        for i in start..end {
            if !self.cache.contains(i) {
                if let Some(value) = self.indexed.get_row(i)? {
                    self.cache.insert(i, value);
                }
            }
        }
        Ok(())
    }

    /// Get all rows (for small files or initial load)
    pub fn get_all_rows(&mut self) -> io::Result<Vec<Value>> {
        self.get_rows(0, self.row_count())
    }

    /// Clear the cache
    pub fn clear_cache(&mut self) {
        self.cache.clear();
    }

    /// Get cache statistics
    pub fn cache_size(&self) -> usize {
        self.cache.len()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    fn create_test_reader(content: &str) -> CachedReader<Cursor<Vec<u8>>> {
        let cursor = Cursor::new(content.as_bytes().to_vec());
        CachedReader::with_cache_size(cursor, 10).unwrap()
    }

    #[test]
    fn test_get_row() {
        let content = r#"{"id": 1, "name": "alice"}
{"id": 2, "name": "bob"}
{"id": 3, "name": "charlie"}
"#;
        let mut reader = create_test_reader(content);

        let row = reader.get_row(1).unwrap().unwrap();
        assert_eq!(row["name"], "bob");

        // Second access should use cache
        let row_cached = reader.get_row(1).unwrap().unwrap();
        assert_eq!(row_cached["name"], "bob");
    }

    #[test]
    fn test_cache_usage() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
"#;
        let mut reader = create_test_reader(content);

        assert_eq!(reader.cache_size(), 0);

        reader.get_row(0).unwrap();
        assert_eq!(reader.cache_size(), 1);

        reader.get_row(1).unwrap();
        assert_eq!(reader.cache_size(), 2);

        // Access cached row should not increase cache size
        reader.get_row(0).unwrap();
        assert_eq!(reader.cache_size(), 2);
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
        assert_eq!(rows[2]["id"], 4);
    }

    #[test]
    fn test_prefetch() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
{"id": 4}
{"id": 5}
"#;
        let mut reader = create_test_reader(content);

        assert_eq!(reader.cache_size(), 0);

        reader.prefetch(0, 3).unwrap();
        assert_eq!(reader.cache_size(), 3);

        // Rows should now be cached
        assert!(reader.cache.contains(0));
        assert!(reader.cache.contains(1));
        assert!(reader.cache.contains(2));
    }

    #[test]
    fn test_get_all_rows() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
"#;
        let mut reader = create_test_reader(content);

        let rows = reader.get_all_rows().unwrap();
        assert_eq!(rows.len(), 3);
        assert_eq!(reader.cache_size(), 3);
    }

    #[test]
    fn test_clear_cache() {
        let content = r#"{"id": 1}
{"id": 2}
"#;
        let mut reader = create_test_reader(content);

        reader.get_all_rows().unwrap();
        assert_eq!(reader.cache_size(), 2);

        reader.clear_cache();
        assert_eq!(reader.cache_size(), 0);
    }

    #[test]
    fn test_row_count() {
        let content = r#"{"id": 1}
{"id": 2}
{"id": 3}
"#;
        let reader = create_test_reader(content);
        assert_eq!(reader.row_count(), 3);
    }
}
