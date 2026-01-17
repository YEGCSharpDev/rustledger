//! Go-to-definition handler.
//!
//! Provides navigation to symbol definitions:
//! - Account → Open directive
//! - Currency → Commodity directive

use lsp_types::{GotoDefinitionParams, GotoDefinitionResponse, Location, Position, Range, Uri};
use rustledger_core::Directive;
use rustledger_parser::ParseResult;

/// Handle a go-to-definition request.
pub fn handle_goto_definition(
    params: &GotoDefinitionParams,
    source: &str,
    parse_result: &ParseResult,
    uri: &Uri,
) -> Option<GotoDefinitionResponse> {
    let position = params.text_document_position_params.position;

    // Get the word at the cursor position
    let word = get_word_at_position(source, position)?;

    tracing::debug!("Go-to-definition for word: {:?}", word);

    // Check if it's an account name
    if word.contains(':') || is_account_type(&word) {
        if let Some(location) = find_account_definition(&word, parse_result, source, uri) {
            return Some(GotoDefinitionResponse::Scalar(location));
        }
    }

    // Check if it's a currency
    if is_currency_like(&word) {
        if let Some(location) = find_currency_definition(&word, parse_result, source, uri) {
            return Some(GotoDefinitionResponse::Scalar(location));
        }
    }

    None
}

/// Get the word at a given position in the source.
fn get_word_at_position(source: &str, position: Position) -> Option<String> {
    let line = source.lines().nth(position.line as usize)?;
    let col = position.character as usize;

    if col > line.len() {
        return None;
    }

    // Find word boundaries
    let chars: Vec<char> = line.chars().collect();

    // Find start of word
    let mut start = col;
    while start > 0 && is_word_char(chars.get(start - 1).copied()?) {
        start -= 1;
    }

    // Find end of word
    let mut end = col;
    while end < chars.len() && is_word_char(chars[end]) {
        end += 1;
    }

    if start == end {
        return None;
    }

    Some(chars[start..end].iter().collect())
}

/// Check if a character is part of a word (including account separators).
fn is_word_char(c: char) -> bool {
    c.is_alphanumeric() || c == ':' || c == '_' || c == '-'
}

/// Check if a string looks like an account type.
fn is_account_type(s: &str) -> bool {
    matches!(
        s,
        "Assets" | "Liabilities" | "Equity" | "Income" | "Expenses"
    )
}

/// Check if a string looks like a currency (all uppercase, 2-5 chars).
fn is_currency_like(s: &str) -> bool {
    s.len() >= 2
        && s.len() <= 5
        && s.chars()
            .all(|c| c.is_ascii_uppercase() || c.is_ascii_digit())
}

/// Find the definition of an account (the Open directive).
fn find_account_definition(
    account: &str,
    parse_result: &ParseResult,
    source: &str,
    uri: &Uri,
) -> Option<Location> {
    for spanned_directive in &parse_result.directives {
        if let Directive::Open(open) = &spanned_directive.value {
            let open_account = open.account.to_string();
            // Match exact account or account prefix
            if open_account == account || account.starts_with(&format!("{}:", open_account)) {
                let (start_line, start_col) =
                    byte_offset_to_position(source, spanned_directive.span.start);
                let (end_line, end_col) =
                    byte_offset_to_position(source, spanned_directive.span.end);

                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position::new(start_line, start_col),
                        end: Position::new(end_line, end_col),
                    },
                });
            }
        }
    }
    None
}

/// Find the definition of a currency (the Commodity directive).
fn find_currency_definition(
    currency: &str,
    parse_result: &ParseResult,
    source: &str,
    uri: &Uri,
) -> Option<Location> {
    for spanned_directive in &parse_result.directives {
        if let Directive::Commodity(comm) = &spanned_directive.value {
            if comm.currency.as_ref() == currency {
                let (start_line, start_col) =
                    byte_offset_to_position(source, spanned_directive.span.start);
                let (end_line, end_col) =
                    byte_offset_to_position(source, spanned_directive.span.end);

                return Some(Location {
                    uri: uri.clone(),
                    range: Range {
                        start: Position::new(start_line, start_col),
                        end: Position::new(end_line, end_col),
                    },
                });
            }
        }
    }
    None
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
    fn test_get_word_at_position() {
        let source = "2024-01-01 open Assets:Bank USD";

        // "open" at position 11
        let word = get_word_at_position(source, Position::new(0, 11));
        assert_eq!(word, Some("open".to_string()));

        // "Assets:Bank" at position 16
        let word = get_word_at_position(source, Position::new(0, 20));
        assert_eq!(word, Some("Assets:Bank".to_string()));

        // "USD" at position 28
        let word = get_word_at_position(source, Position::new(0, 28));
        assert_eq!(word, Some("USD".to_string()));
    }

    #[test]
    fn test_is_currency_like() {
        assert!(is_currency_like("USD"));
        assert!(is_currency_like("EUR"));
        assert!(is_currency_like("BTC"));
        assert!(!is_currency_like("usd")); // lowercase
        assert!(!is_currency_like("U")); // too short
        assert!(!is_currency_like("TOOLONG")); // too long
    }

    #[test]
    fn test_is_account_type() {
        assert!(is_account_type("Assets"));
        assert!(is_account_type("Liabilities"));
        assert!(!is_account_type("assets")); // lowercase
        assert!(!is_account_type("Bank")); // not a root type
    }
}
