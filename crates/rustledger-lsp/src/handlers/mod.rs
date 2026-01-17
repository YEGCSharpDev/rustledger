//! LSP request and notification handlers.
//!
//! Each handler processes a specific LSP request type against
//! an immutable world snapshot.

pub mod completion;
pub mod definition;
pub mod diagnostics;
pub mod hover;

// TODO: Add more handlers as we implement features
// pub mod symbols;
