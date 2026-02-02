#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum InputFormat {
    JsonLines,
    JsonArray,
}

/// Detects the likely input format based on the first few non-whitespace bytes.
/// This is a lightweight "sniffing" operation and does not perform full validation.
/// Returns None if the input is empty or doesn't start with a valid JSON character.
pub fn sniff_format(peek: &[u8]) -> Option<InputFormat> {
    if let Some(first_char) = peek.iter().find(|c| !c.is_ascii_whitespace()) {
        match first_char {
            b'[' => Some(InputFormat::JsonArray),
            b'{' => Some(InputFormat::JsonLines), // Assume JSONL for any object start
            _ => None,
        }
    } else {
        None // Empty or whitespace-only input
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sniff_json_lines() {
        let input = b"  {\"id\": 1}\n";
        assert_eq!(sniff_format(input), Some(InputFormat::JsonLines));
    }

    #[test]
    fn test_sniff_json_array() {
        let input = b"   [{\"id\": 1}]";
        assert_eq!(sniff_format(input), Some(InputFormat::JsonArray));
    }

    #[test]
    fn test_sniff_empty_input() {
        let input = b"   ";
        assert_eq!(sniff_format(input), None);
    }

    #[test]
    fn test_sniff_invalid_input() {
        let input = b"not json";
        assert_eq!(sniff_format(input), None);
    }
}
