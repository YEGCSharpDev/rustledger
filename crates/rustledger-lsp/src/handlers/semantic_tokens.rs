//! Semantic tokens handler for enhanced syntax highlighting.
//!
//! Provides semantic token information for:
//! - Dates
//! - Accounts
//! - Currencies
//! - Numbers
//! - Strings (payees, narrations)
//! - Keywords (directive types)
//! - Comments

use lsp_types::{
    SemanticToken, SemanticTokenModifier, SemanticTokenType, SemanticTokens,
    SemanticTokensFullOptions, SemanticTokensLegend, SemanticTokensOptions, SemanticTokensParams,
    SemanticTokensResult, SemanticTokensServerCapabilities,
};
use rustledger_core::Directive;
use rustledger_parser::ParseResult;

/// Token types we support.
pub const TOKEN_TYPES: &[SemanticTokenType] = &[
    SemanticTokenType::KEYWORD,  // 0: directive keywords (open, close, etc.)
    SemanticTokenType::NUMBER,   // 1: amounts
    SemanticTokenType::STRING,   // 2: payees, narrations
    SemanticTokenType::VARIABLE, // 3: accounts
    SemanticTokenType::TYPE,     // 4: currencies
    SemanticTokenType::COMMENT,  // 5: comments
    SemanticTokenType::OPERATOR, // 6: flags (*, !)
    SemanticTokenType::MACRO,    // 7: dates
];

/// Token modifiers we support.
pub const TOKEN_MODIFIERS: &[SemanticTokenModifier] = &[
    SemanticTokenModifier::DEFINITION, // 0: where something is defined
    SemanticTokenModifier::DEPRECATED, // 1: closed accounts
    SemanticTokenModifier::READONLY,   // 2: balance assertions
];

/// Get the semantic tokens legend for capability registration.
pub fn get_legend() -> SemanticTokensLegend {
    SemanticTokensLegend {
        token_types: TOKEN_TYPES.to_vec(),
        token_modifiers: TOKEN_MODIFIERS.to_vec(),
    }
}

/// Get the semantic tokens server capabilities.
pub fn get_capabilities() -> SemanticTokensServerCapabilities {
    SemanticTokensServerCapabilities::SemanticTokensOptions(SemanticTokensOptions {
        legend: get_legend(),
        full: Some(SemanticTokensFullOptions::Bool(true)),
        range: None,
        work_done_progress_options: Default::default(),
    })
}

/// Token type indices.
mod token_type {
    pub const KEYWORD: u32 = 0;
    pub const NUMBER: u32 = 1;
    pub const STRING: u32 = 2;
    pub const VARIABLE: u32 = 3; // accounts
    pub const TYPE: u32 = 4; // currencies
    #[allow(dead_code)] // Reserved for future use when we parse comments
    pub const COMMENT: u32 = 5;
    pub const OPERATOR: u32 = 6; // flags
    pub const MACRO: u32 = 7; // dates
}

/// Token modifier bits.
mod token_modifier {
    pub const DEFINITION: u32 = 1 << 0;
    pub const DEPRECATED: u32 = 1 << 1;
    #[allow(dead_code)]
    pub const READONLY: u32 = 1 << 2;
}

/// Handle a semantic tokens request.
pub fn handle_semantic_tokens(
    _params: &SemanticTokensParams,
    source: &str,
    parse_result: &ParseResult,
) -> Option<SemanticTokensResult> {
    let mut tokens = Vec::new();
    let mut prev_line = 0u32;
    let mut prev_start = 0u32;

    // Collect all tokens from directives
    let mut raw_tokens: Vec<RawToken> = Vec::new();

    for spanned in &parse_result.directives {
        collect_directive_tokens(&spanned.value, spanned.span.start, source, &mut raw_tokens);
    }

    // Sort tokens by position
    raw_tokens.sort_by_key(|t| (t.line, t.start));

    // Convert to delta-encoded semantic tokens
    for raw in raw_tokens {
        let delta_line = raw.line - prev_line;
        let delta_start = if delta_line == 0 {
            raw.start - prev_start
        } else {
            raw.start
        };

        tokens.push(SemanticToken {
            delta_line,
            delta_start,
            length: raw.length,
            token_type: raw.token_type,
            token_modifiers_bitset: raw.modifiers,
        });

        prev_line = raw.line;
        prev_start = raw.start;
    }

    if tokens.is_empty() {
        None
    } else {
        Some(SemanticTokensResult::Tokens(SemanticTokens {
            result_id: None,
            data: tokens,
        }))
    }
}

/// A raw token before delta encoding.
struct RawToken {
    line: u32,
    start: u32,
    length: u32,
    token_type: u32,
    modifiers: u32,
}

/// Collect tokens from a directive.
fn collect_directive_tokens(
    directive: &Directive,
    start_offset: usize,
    source: &str,
    tokens: &mut Vec<RawToken>,
) {
    let (line, col) = byte_offset_to_position(source, start_offset);

    match directive {
        Directive::Transaction(txn) => {
            // Date token
            tokens.push(RawToken {
                line,
                start: col,
                length: 10, // YYYY-MM-DD
                token_type: token_type::MACRO,
                modifiers: 0,
            });

            // Flag token (after date + space)
            let flag_col = col + 11;
            tokens.push(RawToken {
                line,
                start: flag_col,
                length: 1,
                token_type: token_type::OPERATOR,
                modifiers: 0,
            });

            // Payee if present (estimate position)
            if let Some(ref payee) = txn.payee {
                let payee_len = payee.len() as u32 + 2; // include quotes
                tokens.push(RawToken {
                    line,
                    start: flag_col + 2,
                    length: payee_len,
                    token_type: token_type::STRING,
                    modifiers: 0,
                });
            }

            // Postings
            for (i, posting) in txn.postings.iter().enumerate() {
                let posting_line = line + 1 + i as u32;

                // Account
                let account_str = posting.account.to_string();
                tokens.push(RawToken {
                    line: posting_line,
                    start: 2, // indentation
                    length: account_str.len() as u32,
                    token_type: token_type::VARIABLE,
                    modifiers: 0,
                });

                // Amount if present
                if let Some(ref units) = posting.units {
                    if let Some(num) = units.number() {
                        let num_str = num.to_string();
                        let num_start = 2 + account_str.len() as u32 + 2;
                        tokens.push(RawToken {
                            line: posting_line,
                            start: num_start,
                            length: num_str.len() as u32,
                            token_type: token_type::NUMBER,
                            modifiers: 0,
                        });

                        // Currency
                        if let Some(curr) = units.currency() {
                            let curr_str = curr.to_string();
                            tokens.push(RawToken {
                                line: posting_line,
                                start: num_start + num_str.len() as u32 + 1,
                                length: curr_str.len() as u32,
                                token_type: token_type::TYPE,
                                modifiers: 0,
                            });
                        }
                    }
                }
            }
        }

        Directive::Open(open) => {
            // Date
            tokens.push(RawToken {
                line,
                start: col,
                length: 10,
                token_type: token_type::MACRO,
                modifiers: 0,
            });

            // "open" keyword
            tokens.push(RawToken {
                line,
                start: col + 11,
                length: 4,
                token_type: token_type::KEYWORD,
                modifiers: 0,
            });

            // Account (definition)
            let account_str = open.account.to_string();
            tokens.push(RawToken {
                line,
                start: col + 16,
                length: account_str.len() as u32,
                token_type: token_type::VARIABLE,
                modifiers: token_modifier::DEFINITION,
            });

            // Currencies
            let mut curr_start = col + 17 + account_str.len() as u32;
            for curr in &open.currencies {
                let curr_str = curr.to_string();
                tokens.push(RawToken {
                    line,
                    start: curr_start,
                    length: curr_str.len() as u32,
                    token_type: token_type::TYPE,
                    modifiers: 0,
                });
                curr_start += curr_str.len() as u32 + 1;
            }
        }

        Directive::Close(close) => {
            // Date
            tokens.push(RawToken {
                line,
                start: col,
                length: 10,
                token_type: token_type::MACRO,
                modifiers: 0,
            });

            // "close" keyword
            tokens.push(RawToken {
                line,
                start: col + 11,
                length: 5,
                token_type: token_type::KEYWORD,
                modifiers: 0,
            });

            // Account (deprecated)
            let account_str = close.account.to_string();
            tokens.push(RawToken {
                line,
                start: col + 17,
                length: account_str.len() as u32,
                token_type: token_type::VARIABLE,
                modifiers: token_modifier::DEPRECATED,
            });
        }

        Directive::Balance(bal) => {
            // Date
            tokens.push(RawToken {
                line,
                start: col,
                length: 10,
                token_type: token_type::MACRO,
                modifiers: 0,
            });

            // "balance" keyword
            tokens.push(RawToken {
                line,
                start: col + 11,
                length: 7,
                token_type: token_type::KEYWORD,
                modifiers: 0,
            });

            // Account
            let account_str = bal.account.to_string();
            tokens.push(RawToken {
                line,
                start: col + 19,
                length: account_str.len() as u32,
                token_type: token_type::VARIABLE,
                modifiers: 0,
            });

            // Amount
            let num_str = bal.amount.number.to_string();
            let num_start = col + 20 + account_str.len() as u32;
            tokens.push(RawToken {
                line,
                start: num_start,
                length: num_str.len() as u32,
                token_type: token_type::NUMBER,
                modifiers: 0,
            });

            // Currency
            let curr_str = bal.amount.currency.to_string();
            tokens.push(RawToken {
                line,
                start: num_start + num_str.len() as u32 + 1,
                length: curr_str.len() as u32,
                token_type: token_type::TYPE,
                modifiers: 0,
            });
        }

        Directive::Commodity(comm) => {
            // Date
            tokens.push(RawToken {
                line,
                start: col,
                length: 10,
                token_type: token_type::MACRO,
                modifiers: 0,
            });

            // "commodity" keyword
            tokens.push(RawToken {
                line,
                start: col + 11,
                length: 9,
                token_type: token_type::KEYWORD,
                modifiers: 0,
            });

            // Currency (definition)
            let curr_str = comm.currency.to_string();
            tokens.push(RawToken {
                line,
                start: col + 21,
                length: curr_str.len() as u32,
                token_type: token_type::TYPE,
                modifiers: token_modifier::DEFINITION,
            });
        }

        Directive::Price(price) => {
            // Date
            tokens.push(RawToken {
                line,
                start: col,
                length: 10,
                token_type: token_type::MACRO,
                modifiers: 0,
            });

            // "price" keyword
            tokens.push(RawToken {
                line,
                start: col + 11,
                length: 5,
                token_type: token_type::KEYWORD,
                modifiers: 0,
            });

            // Currency
            let curr_str = price.currency.to_string();
            tokens.push(RawToken {
                line,
                start: col + 17,
                length: curr_str.len() as u32,
                token_type: token_type::TYPE,
                modifiers: 0,
            });

            // Amount
            let num_str = price.amount.number.to_string();
            let num_start = col + 18 + curr_str.len() as u32;
            tokens.push(RawToken {
                line,
                start: num_start,
                length: num_str.len() as u32,
                token_type: token_type::NUMBER,
                modifiers: 0,
            });

            // Target currency
            let target_curr = price.amount.currency.to_string();
            tokens.push(RawToken {
                line,
                start: num_start + num_str.len() as u32 + 1,
                length: target_curr.len() as u32,
                token_type: token_type::TYPE,
                modifiers: 0,
            });
        }

        // For other directives, just highlight the date and keyword
        _ => {
            // Date
            tokens.push(RawToken {
                line,
                start: col,
                length: 10,
                token_type: token_type::MACRO,
                modifiers: 0,
            });
        }
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
    use rustledger_parser::parse;

    #[test]
    fn test_semantic_tokens_basic() {
        let source = "2024-01-01 open Assets:Bank USD\n";
        let result = parse(source);
        let params = SemanticTokensParams {
            text_document: lsp_types::TextDocumentIdentifier {
                uri: "file:///test.beancount".parse().unwrap(),
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        };

        let response = handle_semantic_tokens(&params, source, &result);
        assert!(response.is_some());

        if let Some(SemanticTokensResult::Tokens(tokens)) = response {
            // Should have tokens for: date, keyword, account, currency
            assert!(!tokens.data.is_empty());
        }
    }
}
