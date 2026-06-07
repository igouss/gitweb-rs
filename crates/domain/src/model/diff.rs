//! Tree-to-tree diffs.
//!
//! Mirrors the per-path fields gitweb reads in `parse_difftree_raw_line` for an
//! ordinary (non-combined) diff: the from/to modes and object ids, the change
//! status, and the from/to paths. Combined diffs against multiple parents are a
//! later capability and are not modelled here.

use crate::model::change::ChangeStatus;
use crate::model::file_mode::FileMode;
use crate::model::object_id::ObjectId;

/// One changed path in a tree-to-tree diff.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DiffEntry {
    status: ChangeStatus,
    from_mode: FileMode,
    to_mode: FileMode,
    from_oid: ObjectId,
    to_oid: ObjectId,
    from_path: String,
    to_path: String,
}

impl DiffEntry {
    /// Assembles one diff entry.
    #[must_use]
    pub fn new(
        status: ChangeStatus,
        from_mode: FileMode,
        to_mode: FileMode,
        from_oid: ObjectId,
        to_oid: ObjectId,
        from_path: String,
        to_path: String,
    ) -> Self {
        Self {
            status,
            from_mode,
            to_mode,
            from_oid,
            to_oid,
            from_path,
            to_path,
        }
    }

    /// What changed for this path.
    #[must_use]
    pub fn status(&self) -> ChangeStatus {
        self.status
    }

    /// The mode on the from side (`000000` when the path is being created).
    #[must_use]
    pub fn from_mode(&self) -> FileMode {
        self.from_mode
    }

    /// The mode on the to side (`000000` when the path is being deleted).
    #[must_use]
    pub fn to_mode(&self) -> FileMode {
        self.to_mode
    }

    /// The from-side object id (all-zero when the path is being created).
    #[must_use]
    pub fn from_oid(&self) -> &ObjectId {
        &self.from_oid
    }

    /// The to-side object id (all-zero when the path is being deleted).
    #[must_use]
    pub fn to_oid(&self) -> &ObjectId {
        &self.to_oid
    }

    /// The from-side path (differs from the to-side path only on rename/copy).
    #[must_use]
    pub fn from_path(&self) -> &str {
        &self.from_path
    }

    /// The to-side path.
    #[must_use]
    pub fn to_path(&self) -> &str {
        &self.to_path
    }
}

/// A diff between two trees: the set of changed paths.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Diff {
    entries: Vec<DiffEntry>,
}

impl Diff {
    /// Builds a diff from its changed paths.
    #[must_use]
    pub fn new(entries: Vec<DiffEntry>) -> Self {
        Self { entries }
    }

    /// The changed paths.
    #[must_use]
    pub fn entries(&self) -> &[DiffEntry] {
        &self.entries
    }
}
