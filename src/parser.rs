//! JSONL line-by-line parser for Claude Code transcript files.
//!
//! Reads a `.jsonl` file and produces a `Vec<TranscriptEntry>`.
//! Blank lines and BOMs are tolerated. Malformed lines are collected
//! as [`ParseWarning`] rather than aborting the entire file.

use std::fs::File;
use std::io::{BufRead, BufReader};
use std::path::Path;

use crate::model::entry::TranscriptEntry;

/// A recoverable parse warning for a single line.
#[derive(Debug, Clone)]
pub struct ParseWarning {
    /// 1-based line number in the file.
    pub line: usize,
    /// Human-readable description of the problem.
    pub message: String,
}

/// Result of parsing a JSONL file.
#[derive(Debug)]
pub struct ParseResult {
    /// Successfully parsed entries.
    pub entries: Vec<TranscriptEntry>,
    /// Warnings collected for non-fatal issues (malformed lines, etc.).
    pub warnings: Vec<ParseWarning>,
}

/// Parse a JSONL file at the given path.
///
/// # Errors
///
/// Returns `std::io::Error` only if the file cannot be opened or read.
/// Malformed lines are surfaced as [`ParseWarning`] entries and do NOT
/// cause the entire parse to fail.
pub fn parse_file(path: &Path) -> Result<ParseResult, std::io::Error> {
    let file = File::open(path)?;
    let reader = BufReader::new(file);
    parse_reader(reader)
}

/// Parse JSONL from any `BufRead` implementor.
///
/// This is the core parse routine and is tested directly so we can
/// feed inline fixtures without touching the filesystem.
pub fn parse_reader<R: BufRead>(reader: R) -> Result<ParseResult, std::io::Error> {
    let mut entries = Vec::new();
    let mut warnings = Vec::new();

    for (line_num, line_result) in reader.lines().enumerate() {
        let line = line_result?;
        let line_no = line_num + 1; // 1-based

        // Remove UTF-8 BOM on first line.
        let line = if line_num == 0 {
            line.strip_prefix('\u{FEFF}').unwrap_or(&line).to_string()
        } else {
            line
        };

        // Skip blank lines.
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }

        // Parse the line as a TranscriptEntry.
        match serde_json::from_str::<TranscriptEntry>(trimmed) {
            Ok(entry) => entries.push(entry),
            Err(e) => {
                warnings.push(ParseWarning {
                    line: line_no,
                    message: format!("failed to parse line {}: {}", line_no, e),
                });
            }
        }
    }

    Ok(ParseResult { entries, warnings })
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Cursor;

    #[test]
    fn parse_empty_input() {
        let input = "";
        let result = parse_reader(Cursor::new(input)).unwrap();
        assert!(result.entries.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn parse_blank_lines_only() {
        let input = "\n\n  \n";
        let result = parse_reader(Cursor::new(input)).unwrap();
        assert!(result.entries.is_empty());
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn parse_single_user_entry() {
        let input = serde_json::json!({
            "type": "user",
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-1",
            "message": {
                "role": "user",
                "content": [{"type": "text", "text": "Hello"}]
            }
        })
        .to_string();

        let result = parse_reader(Cursor::new(input)).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert!(result.warnings.is_empty());
        assert_eq!(result.entries[0].entry_type(), "user");
    }

    #[test]
    fn parse_multiple_entries() {
        let user = serde_json::json!({
            "type": "user",
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-1",
            "message": {"role": "user", "content": [{"type": "text", "text": "hi"}]}
        })
        .to_string();

        let assistant = serde_json::json!({
            "type": "assistant",
            "uuid": "660e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:05Z",
            "sessionId": "session-1",
            "message": {
                "role": "assistant",
                "content": [{"type": "text", "text": "Hello!"}]
            }
        })
        .to_string();

        let file = format!("{}\n{}\n", user, assistant);
        let result = parse_reader(Cursor::new(&file)).unwrap();

        assert_eq!(result.entries.len(), 2);
        assert!(result.warnings.is_empty());
        assert_eq!(result.entries[0].entry_type(), "user");
        assert_eq!(result.entries[1].entry_type(), "assistant");
    }

    #[test]
    fn malformed_line_collects_warning_not_abort() {
        let good = serde_json::json!({
            "type": "user",
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-1",
            "message": {"role": "user", "content": [{"type": "text", "text": "hi"}]}
        })
        .to_string();

        let bad = "this is not valid json";

        let file = format!("{}\n{}\n{}\n", good, bad, good);
        let result = parse_reader(Cursor::new(&file)).unwrap();

        // Both good entries should parse.
        assert_eq!(result.entries.len(), 2);
        // One warning for the bad line.
        assert_eq!(result.warnings.len(), 1);
        assert_eq!(result.warnings[0].line, 2);
        assert!(
            result.warnings[0].message.contains("failed to parse line"),
            "warning message should mention the line: {}",
            result.warnings[0].message
        );
    }

    #[test]
    fn bom_at_start_is_stripped() {
        let json = serde_json::json!({
            "type": "user",
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-1",
            "message": {"role": "user", "content": [{"type": "text", "text": "hi"}]}
        })
        .to_string();

        let with_bom = format!("\u{FEFF}{}", json);
        let result = parse_reader(Cursor::new(&with_bom)).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert!(result.warnings.is_empty());
    }

    #[test]
    fn unknown_entry_type_parses_as_unknown_variant() {
        let input = serde_json::json!({
            "type": "future-type",
            "uuid": "550e8400-e29b-41d4-a716-446655440000",
            "timestamp": "2025-06-15T10:30:00Z",
            "sessionId": "session-1",
            "extraField": 42
        })
        .to_string();

        let result = parse_reader(Cursor::new(input)).unwrap();
        assert_eq!(result.entries.len(), 1);
        assert_eq!(result.entries[0].entry_type(), "future-type");
    }

    #[test]
    fn parse_header_line() {
        // Some JSONL files start with a header line like {"type":"sessionHeader",...}
        let input = serde_json::json!({
            "type": "system",
            "uuid": "00000000-0000-0000-0000-000000000001",
            "timestamp": "2025-06-15T10:29:00Z",
            "sessionId": "session-1",
            "system": {"version": "1.0.0"}
        })
        .to_string();

        let result = parse_reader(Cursor::new(input)).unwrap();
        assert_eq!(result.entries.len(), 1);
    }
}
