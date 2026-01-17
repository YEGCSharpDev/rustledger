//! LSP request and notification handlers.
//!
//! Each handler processes a specific LSP request type against
//! an immutable world snapshot.

pub mod code_actions;
pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod folding;
pub mod formatting;
pub mod hover;
pub mod rename;
pub mod semantic_tokens;
pub mod symbols;
pub mod workspace_symbols;
