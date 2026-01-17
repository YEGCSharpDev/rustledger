//! Virtual File System for document management.
//!
//! The VFS maintains the in-memory state of all open documents,
//! handling incremental updates from the editor.

use ropey::Rope;
use std::collections::HashMap;
use std::path::PathBuf;

/// A document in the virtual file system.
#[derive(Debug)]
pub struct Document {
    /// The document content as a rope for efficient editing.
    pub content: Rope,
    /// The document version (incremented on each change).
    pub version: i32,
}

impl Document {
    /// Create a new document with the given content.
    pub fn new(content: String, version: i32) -> Self {
        Self {
            content: Rope::from_str(&content),
            version,
        }
    }

    /// Get the document content as a string.
    pub fn text(&self) -> String {
        self.content.to_string()
    }
}

/// Virtual file system for managing open documents.
#[derive(Debug, Default)]
pub struct Vfs {
    /// Open documents indexed by path.
    documents: HashMap<PathBuf, Document>,
}

impl Vfs {
    /// Create a new empty VFS.
    pub fn new() -> Self {
        Self::default()
    }

    /// Open a document in the VFS.
    pub fn open(&mut self, path: PathBuf, content: String, version: i32) {
        self.documents.insert(path, Document::new(content, version));
    }

    /// Close a document in the VFS.
    pub fn close(&mut self, path: &PathBuf) {
        self.documents.remove(path);
    }

    /// Get a document by path.
    pub fn get(&self, path: &PathBuf) -> Option<&Document> {
        self.documents.get(path)
    }

    /// Update a document's content.
    pub fn update(&mut self, path: &PathBuf, content: String, version: i32) {
        if let Some(doc) = self.documents.get_mut(path) {
            doc.content = Rope::from_str(&content);
            doc.version = version;
        }
    }

    /// Get all open document paths.
    pub fn paths(&self) -> impl Iterator<Item = &PathBuf> {
        self.documents.keys()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_vfs_open_close() {
        let mut vfs = Vfs::new();
        let path = PathBuf::from("/test.beancount");

        vfs.open(path.clone(), "2024-01-01 open Assets:Bank".to_string(), 1);
        assert!(vfs.get(&path).is_some());

        vfs.close(&path);
        assert!(vfs.get(&path).is_none());
    }

    #[test]
    fn test_document_text() {
        let doc = Document::new("hello world".to_string(), 1);
        assert_eq!(doc.text(), "hello world");
    }
}
