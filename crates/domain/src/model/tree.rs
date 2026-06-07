//! Tree objects: an ordered list of entries.
//!
//! Mirrors the fields gitweb reads in `parse_ls_tree_line` — mode, name, and
//! object id per entry. The mode already carries the entry's kind and
//! permission string (see [`FileMode`]); ordering is whatever git returns.

use crate::model::file_mode::FileMode;
use crate::model::object_id::ObjectId;

/// One entry in a tree: a named child object with its mode.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TreeEntry {
    mode: FileMode,
    name: String,
    oid: ObjectId,
}

impl TreeEntry {
    /// Builds a tree entry.
    #[must_use]
    pub fn new(mode: FileMode, name: String, oid: ObjectId) -> Self {
        Self { mode, name, oid }
    }

    /// The entry's file mode (carries kind and permissions).
    #[must_use]
    pub fn mode(&self) -> FileMode {
        self.mode
    }

    /// The entry's leaf name within its tree.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The id of the object this entry points at.
    #[must_use]
    pub fn oid(&self) -> &ObjectId {
        &self.oid
    }
}

/// A git tree: the children of one directory.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Tree {
    entries: Vec<TreeEntry>,
}

impl Tree {
    /// Builds a tree from its entries.
    #[must_use]
    pub fn new(entries: Vec<TreeEntry>) -> Self {
        Self { entries }
    }

    /// The entries, in the order git returned them.
    #[must_use]
    pub fn entries(&self) -> &[TreeEntry] {
        &self.entries
    }
}
