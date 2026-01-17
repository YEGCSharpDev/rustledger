//! Diagnostics handler for publishing parse errors.

use lsp_types::{Diagnostic, DiagnosticSeverity, Position, Range};
use rustledger_parser::{ParseError, ParseResult};

/// Convert parse errors to LSP diagnostics.
pub fn parse_errors_to_diagnostics(result: &ParseResult, source: &str) -> Vec<Diagnostic> {
    result
        .errors
        .iter()
        .map(|e| parse_error_to_diagnostic(e, source))
        .collect()
}

/// Convert a single parse error to an LSP diagnostic.
pub fn parse_error_to_diagnostic(error: &ParseError, source: &str) -> Diagnostic {
    let (start_line, start_col) = byte_offset_to_position(source, error.span.start);
    let (end_line, end_col) = byte_offset_to_position(source, error.span.end);

    Diagnostic {
        range: Range {
            start: Position::new(start_line, start_col),
            end: Position::new(end_line, end_col),
        },
        severity: Some(DiagnosticSeverity::ERROR),
        code: Some(lsp_types::NumberOrString::String(format!(
            "P{:04}",
            error.kind_code()
        ))),
        source: Some("rustledger".to_string()),
        message: error.message(),
        related_information: None,
        tags: None,
        code_description: None,
        data: None,
    }
}

/// Convert a byte offset to a line/column position (0-based for LSP).
fn byte_offset_to_position(source: &str, offset: usize) -> (u32, u32) {
    let mut line = 0u32;
    let mut col = 0u32;

    for (i, ch) in source.char_indices() {
        if i >= offset {
            break;
        }
        if ch == '\n' {
            line += 1;
            col = 0;
        } else {
            col += 1;
        }
    }

    (line, col)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_byte_offset_to_position() {
        let source = "line1\nline2\nline3";

        assert_eq!(byte_offset_to_position(source, 0), (0, 0));
        assert_eq!(byte_offset_to_position(source, 5), (0, 5));
        assert_eq!(byte_offset_to_position(source, 6), (1, 0));
        assert_eq!(byte_offset_to_position(source, 12), (2, 0));
    }
}
