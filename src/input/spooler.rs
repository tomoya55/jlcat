use std::fs::File;
use std::io::{self, BufReader, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;

pub struct SpooledInput {
    temp_file: NamedTempFile,
}

impl SpooledInput {
    pub fn from_reader<R: Read>(mut reader: R) -> io::Result<Self> {
        let mut temp_file = NamedTempFile::new()?;
        io::copy(&mut reader, &mut temp_file)?;
        temp_file.flush()?;
        Ok(Self { temp_file })
    }

    pub fn path(&self) -> &Path {
        self.temp_file.path()
    }

    pub fn into_reader(self) -> io::Result<BufReader<File>> {
        let file = File::open(self.temp_file.path())?;
        Ok(BufReader::new(file))
    }

    pub fn reader(&self) -> io::Result<BufReader<File>> {
        let file = File::open(self.temp_file.path())?;
        Ok(BufReader::new(file))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn test_spool_stdin_to_tempfile() {
        let input = b"{\"id\": 1}\n{\"id\": 2}\n{\"id\": 3}\n";
        let cursor = Cursor::new(input.to_vec());

        let spooled = SpooledInput::from_reader(cursor).unwrap();

        // Should be able to read multiple times
        let content1 = std::fs::read_to_string(spooled.path()).unwrap();
        let content2 = std::fs::read_to_string(spooled.path()).unwrap();

        assert_eq!(content1, content2);
        assert!(content1.contains("\"id\": 1"));
        assert!(content1.contains("\"id\": 3"));
    }

    #[test]
    fn test_spool_reader_method() {
        let input = b"line1\nline2\n";
        let cursor = Cursor::new(input.to_vec());

        let spooled = SpooledInput::from_reader(cursor).unwrap();

        use std::io::BufRead;
        let reader = spooled.reader().unwrap();
        let lines: Vec<String> = reader.lines().map(|l| l.unwrap()).collect();

        assert_eq!(lines.len(), 2);
        assert_eq!(lines[0], "line1");
    }
}
