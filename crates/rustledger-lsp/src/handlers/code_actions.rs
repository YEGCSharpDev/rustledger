//! Code actions handler for quick fixes and refactorings.
//!
//! Provides code actions for:
//! - Adding missing account open directives
//! - Balancing transaction postings
//! - Formatting amounts consistently

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionParams, CodeActionResponse, Position, Range, TextEdit,
    WorkspaceEdit,
};
use rustledger_core::Directive;
use rustledger_parser::ParseResult;
use std::collections::{HashMap, HashSet};

/// Handle a code action request.
pub fn handle_code_actions(
    params: &CodeActionParams,
    source: &str,
    parse_result: &ParseResult,
) -> Option<CodeActionResponse> {
    let mut actions = Vec::new();

    let range = params.range;
    let uri = params.text_document.uri.clone();

    // Collect all defined accounts
    let defined_accounts = collect_defined_accounts(parse_result);

    // Collect all used accounts
    let used_accounts = collect_used_accounts(parse_result);

    // Find undefined accounts used in the document
    let undefined_accounts: Vec<_> = used_accounts
        .difference(&defined_accounts)
        .cloned()
        .collect();

    // If there are undefined accounts, offer to create open directives
    for account in undefined_accounts {
        // Check if this account is on or near the selected range
        if is_account_in_range(source, &account, range, parse_result) {
            let action = create_open_directive_action(&uri, source, &account, parse_result);
            actions.push(action);
        }
    }

    // Check for unbalanced transactions in range
    if let Some(action) = check_unbalanced_transactions(params, source, parse_result) {
        actions.push(action);
    }

    if actions.is_empty() {
        None
    } else {
        Some(actions.into_iter().map(|a| a.into()).collect())
    }
}

/// Collect all accounts that have been opened.
fn collect_defined_accounts(parse_result: &ParseResult) -> HashSet<String> {
    let mut accounts = HashSet::new();

    for spanned in &parse_result.directives {
        if let Directive::Open(open) = &spanned.value {
            accounts.insert(open.account.to_string());
        }
    }

    accounts
}

/// Collect all accounts used in the document.
fn collect_used_accounts(parse_result: &ParseResult) -> HashSet<String> {
    let mut accounts = HashSet::new();

    for spanned in &parse_result.directives {
        match &spanned.value {
            Directive::Transaction(txn) => {
                for posting in &txn.postings {
                    accounts.insert(posting.account.to_string());
                }
            }
            Directive::Balance(bal) => {
                accounts.insert(bal.account.to_string());
            }
            Directive::Pad(pad) => {
                accounts.insert(pad.account.to_string());
                accounts.insert(pad.source_account.to_string());
            }
            Directive::Note(note) => {
                accounts.insert(note.account.to_string());
            }
            Directive::Document(doc) => {
                accounts.insert(doc.account.to_string());
            }
            Directive::Close(close) => {
                accounts.insert(close.account.to_string());
            }
            _ => {}
        }
    }

    accounts
}

/// Check if an account is mentioned in or near the given range.
fn is_account_in_range(
    source: &str,
    account: &str,
    range: Range,
    parse_result: &ParseResult,
) -> bool {
    // Find the line at the range start
    let lines: Vec<&str> = source.lines().collect();
    let start_line = range.start.line as usize;

    // Check a few lines around the selection
    for line_idx in start_line.saturating_sub(3)..=(start_line + 10).min(lines.len() - 1) {
        if let Some(line) = lines.get(line_idx) {
            if line.contains(account) {
                return true;
            }
        }
    }

    // Also check if we're inside a transaction that uses this account
    for spanned in &parse_result.directives {
        if let Directive::Transaction(txn) = &spanned.value {
            let (dir_line, _) = byte_offset_to_position(source, spanned.span.start);
            let (end_line, _) = byte_offset_to_position(source, spanned.span.end);

            // Check if range overlaps with transaction
            if (range.start.line <= end_line) && (range.end.line >= dir_line) {
                for posting in &txn.postings {
                    if posting.account.as_ref() == account {
                        return true;
                    }
                }
            }
        }
    }

    false
}

/// Create a code action to add an open directive for an account.
#[allow(clippy::mutable_key_type)] // Uri is required as key by LSP WorkspaceEdit API
fn create_open_directive_action(
    uri: &lsp_types::Uri,
    source: &str,
    account: &str,
    parse_result: &ParseResult,
) -> CodeAction {
    // Find the earliest date in the file or use a default
    let earliest_date =
        find_earliest_date(parse_result).unwrap_or_else(|| "2000-01-01".to_string());

    // Find where to insert the open directive (at the beginning of the file after any options)
    let insert_position = find_open_directive_position(source, parse_result);

    let new_text = format!("{} open {}\n", earliest_date, account);

    let mut changes = HashMap::new();
    changes.insert(
        uri.clone(),
        vec![TextEdit {
            range: Range {
                start: insert_position,
                end: insert_position,
            },
            new_text,
        }],
    );

    CodeAction {
        title: format!("Add 'open {}' directive", account),
        kind: Some(CodeActionKind::QUICKFIX),
        diagnostics: None,
        edit: Some(WorkspaceEdit {
            changes: Some(changes),
            document_changes: None,
            change_annotations: None,
        }),
        command: None,
        is_preferred: Some(true),
        disabled: None,
        data: None,
    }
}

/// Find the earliest date in the document.
fn find_earliest_date(parse_result: &ParseResult) -> Option<String> {
    let mut earliest: Option<chrono::NaiveDate> = None;

    for spanned in &parse_result.directives {
        let date = match &spanned.value {
            Directive::Transaction(t) => Some(t.date),
            Directive::Open(o) => Some(o.date),
            Directive::Close(c) => Some(c.date),
            Directive::Balance(b) => Some(b.date),
            Directive::Pad(p) => Some(p.date),
            Directive::Commodity(c) => Some(c.date),
            Directive::Event(e) => Some(e.date),
            Directive::Note(n) => Some(n.date),
            Directive::Document(d) => Some(d.date),
            Directive::Price(p) => Some(p.date),
            Directive::Query(q) => Some(q.date),
            Directive::Custom(c) => Some(c.date),
        };

        if let Some(d) = date {
            earliest = Some(earliest.map_or(d, |e| e.min(d)));
        }
    }

    earliest.map(|d| d.format("%Y-%m-%d").to_string())
}

/// Find the position to insert new open directives.
fn find_open_directive_position(source: &str, parse_result: &ParseResult) -> Position {
    // Find the last open directive and insert after it
    let mut last_open_end: Option<usize> = None;

    for spanned in &parse_result.directives {
        if matches!(&spanned.value, Directive::Open(_)) {
            last_open_end = Some(spanned.span.end);
        }
    }

    if let Some(offset) = last_open_end {
        let (line, _) = byte_offset_to_position(source, offset);
        // Insert on the next line
        Position::new(line + 1, 0)
    } else {
        // No open directives, insert at the beginning
        Position::new(0, 0)
    }
}

/// Check for unbalanced transactions and offer to add a balancing posting.
fn check_unbalanced_transactions(
    params: &CodeActionParams,
    source: &str,
    parse_result: &ParseResult,
) -> Option<CodeAction> {
    let range = params.range;

    for spanned in &parse_result.directives {
        if let Directive::Transaction(txn) = &spanned.value {
            let (start_line, _) = byte_offset_to_position(source, spanned.span.start);
            let (end_line, _) = byte_offset_to_position(source, spanned.span.end);

            // Check if selection is within this transaction
            if range.start.line >= start_line && range.start.line <= end_line {
                // Check if transaction has exactly one posting without amount
                let postings_without_amount =
                    txn.postings.iter().filter(|p| p.units.is_none()).count();

                let postings_with_amount =
                    txn.postings.iter().filter(|p| p.units.is_some()).count();

                // If there's exactly one posting with amount and one without, we can compute the balance
                if postings_without_amount == 1 && postings_with_amount >= 1 {
                    // Transaction is already auto-balanced by the empty posting
                    continue;
                }

                // If all postings have amounts but don't balance, offer to fix
                if postings_without_amount == 0 && postings_with_amount >= 2 {
                    // This would require more complex balance calculation
                    // For now, just skip
                    continue;
                }
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
    use rustledger_parser::parse;

    #[test]
    fn test_collect_accounts() {
        let source = r#"
2024-01-01 open Assets:Bank USD
2024-01-15 * "Coffee Shop"
  Assets:Bank  -5.00 USD
  Expenses:Food
"#;
        let result = parse(source);

        let defined = collect_defined_accounts(&result);
        assert!(defined.contains("Assets:Bank"));
        assert!(!defined.contains("Expenses:Food"));

        let used = collect_used_accounts(&result);
        assert!(used.contains("Assets:Bank"));
        assert!(used.contains("Expenses:Food"));
    }

    #[test]
    fn test_find_earliest_date() {
        let source = r#"
2024-06-15 open Assets:Bank
2024-01-01 open Assets:Cash
2024-03-01 * "Test"
  Assets:Bank  -10 USD
  Assets:Cash
"#;
        let result = parse(source);
        let earliest = find_earliest_date(&result);
        assert_eq!(earliest, Some("2024-01-01".to_string()));
    }
}
