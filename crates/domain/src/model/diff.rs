//! Tree-to-tree diffs, ordinary and combined.
//!
//! Mirrors the per-path fields gitweb reads in `parse_difftree_raw_line`. An
//! ordinary (single-parent) diff has one from-side per path; a combined diff
//! compares a merge against *all* its parents at once (`git diff-tree -c`/
//! `--cc`), so each path carries one from-side per parent and a single to-side.

use crate::model::change::{ChangeKind, ChangeStatus};
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

/// The from-side of one parent in a combined diff entry.
///
/// gitweb's combined branch of `parse_difftree_raw_line` keeps, per parent, the
/// source mode and id and a one-letter status describing the path relative to
/// that parent. Grouping the triple here means the per-parent facts can never
/// fall out of step, the way Perl's three parallel arrays can.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedParent {
    status: ChangeStatus,
    from_mode: FileMode,
    from_oid: ObjectId,
}

impl CombinedParent {
    /// Assembles one parent's from-side.
    #[must_use]
    pub fn new(status: ChangeStatus, from_mode: FileMode, from_oid: ObjectId) -> Self {
        Self {
            status,
            from_mode,
            from_oid,
        }
    }

    /// The status of the path relative to this parent.
    #[must_use]
    pub fn status(&self) -> ChangeStatus {
        self.status
    }

    /// This parent's from-side mode (`000000` when the path is new to it).
    #[must_use]
    pub fn from_mode(&self) -> FileMode {
        self.from_mode
    }

    /// This parent's from-side object id (all-zero when the path is new to it).
    #[must_use]
    pub fn from_oid(&self) -> &ObjectId {
        &self.from_oid
    }
}

/// One changed path in a combined diff: a merge result compared against every
/// parent at once, the way gitweb renders `git diff-tree -c`/`--cc`.
///
/// Only paths that differ from *all* parents appear (git's combined-diff rule),
/// so every [`CombinedParent`] records a real change. The to-side is the single
/// merge result (mode, id, path); a null to-id marks a path deleted in the
/// merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedDiffEntry {
    parents: Vec<CombinedParent>,
    to_mode: FileMode,
    to_oid: ObjectId,
    to_path: String,
}

impl CombinedDiffEntry {
    /// Assembles one combined-diff entry from its per-parent and result sides.
    #[must_use]
    pub fn new(
        parents: Vec<CombinedParent>,
        to_mode: FileMode,
        to_oid: ObjectId,
        to_path: String,
    ) -> Self {
        Self {
            parents,
            to_mode,
            to_oid,
            to_path,
        }
    }

    /// The per-parent from-sides, one per parent of the merge.
    #[must_use]
    pub fn parents(&self) -> &[CombinedParent] {
        &self.parents
    }

    /// The number of parents this path was merged from (gitweb's `nparents`).
    #[must_use]
    pub fn nparents(&self) -> usize {
        self.parents.len()
    }

    /// The merge-result mode (`000000` when the path is deleted in the merge).
    #[must_use]
    pub fn to_mode(&self) -> FileMode {
        self.to_mode
    }

    /// The merge-result object id (all-zero when the path is deleted).
    #[must_use]
    pub fn to_oid(&self) -> &ObjectId {
        &self.to_oid
    }

    /// The path in the merge result.
    #[must_use]
    pub fn to_path(&self) -> &str {
        &self.to_path
    }

    /// Whether the path was deleted in the merge — gitweb's `is_deleted`: the
    /// result id is the null id.
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.to_oid.is_null()
    }

    /// Whether the path has a prior version in at least one parent — gitweb's
    /// `$has_history ||= ($status ne 'A')`: some parent's change is not an add.
    #[must_use]
    pub fn has_history(&self) -> bool {
        self.parents
            .iter()
            .any(|parent: &CombinedParent| parent.status.kind() != ChangeKind::Added)
    }

    /// Whether the path still exists against at least one parent — gitweb's
    /// `$not_deleted ||= ($status ne 'D')`: some parent's change is not a delete.
    #[must_use]
    pub fn not_deleted(&self) -> bool {
        self.parents
            .iter()
            .any(|parent: &CombinedParent| parent.status.kind() != ChangeKind::Deleted)
    }
}

/// A combined diff: the set of paths a merge changed against all its parents.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct CombinedDiff {
    entries: Vec<CombinedDiffEntry>,
}

impl CombinedDiff {
    /// Builds a combined diff from its changed paths.
    #[must_use]
    pub fn new(entries: Vec<CombinedDiffEntry>) -> Self {
        Self { entries }
    }

    /// The changed paths.
    #[must_use]
    pub fn entries(&self) -> &[CombinedDiffEntry] {
        &self.entries
    }
}
