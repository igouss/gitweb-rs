//! The pickaxe result shape: the commits `git log -S` lists and, under each, the
//! files whose occurrence count of the pattern changed.
//!
//! Mirrors gitweb's `git_search_changes` reading of `git log -S<text> --raw`: the
//! `-S` filters which *commits* appear, and (without `--pickaxe-all`) `--raw`
//! limits each commit's listed paths to the ones that *touch* the pattern — the
//! files whose count of the string changed. So a [`PickaxeMatch`] pairs a matching
//! [`Commit`] with those [`PickaxeChange`]s.
//!
//! This is the raw port shape, faithful to git's `--raw`: a deleted file whose
//! removal changed the count is still a change here (`to_blob` is `None`), because
//! git lists it. Skipping deletions when laying out the page — gitweb's
//! `next if is_deleted` — is the use case's call (it has no blob to link), kept
//! out of the port so that rule is testable in one place.

use crate::model::commit::Commit;
use crate::model::object_id::ObjectId;

/// One file whose occurrence count of the pickaxe pattern changed in a commit:
/// its path (gitweb's `to_file`) and the id of its post-change blob (gitweb's
/// `to_id`), or `None` when the change deleted the file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct PickaxeChange {
    path: String,
    to_blob: Option<ObjectId>,
}

impl PickaxeChange {
    /// A file that still exists after the change, at blob `to_blob`.
    #[must_use]
    pub fn changed(path: String, to_blob: ObjectId) -> Self {
        Self {
            path,
            to_blob: Some(to_blob),
        }
    }

    /// A file the change deleted: it has no post-change blob to link to.
    #[must_use]
    pub fn deleted(path: String) -> Self {
        Self {
            path,
            to_blob: None,
        }
    }

    /// The file's path, relative to the repository root (gitweb's `to_file`).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The id of the file's post-change blob (gitweb's `to_id`), or `None` when
    /// the change deleted the file.
    #[must_use]
    pub fn to_blob(&self) -> Option<&ObjectId> {
        self.to_blob.as_ref()
    }

    /// Whether the change deleted the file — gitweb's `is_deleted`, the rows it
    /// skips because there is no blob to link.
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.to_blob.is_none()
    }
}

/// One matching commit on the pickaxe page: the commit and the files whose
/// occurrence count of the pattern it changed (git's `-S` per-commit `--raw`).
#[derive(Debug, Clone)]
pub struct PickaxeMatch {
    commit: Commit,
    changes: Vec<PickaxeChange>,
}

impl PickaxeMatch {
    /// Pairs a matching commit with its count-changing files.
    #[must_use]
    pub fn new(commit: Commit, changes: Vec<PickaxeChange>) -> Self {
        Self { commit, changes }
    }

    /// The matching commit.
    #[must_use]
    pub fn commit(&self) -> &Commit {
        &self.commit
    }

    /// The files whose occurrence count of the pattern changed, in the diff's
    /// path order — including deletions, which the page layout skips.
    #[must_use]
    pub fn changes(&self) -> &[PickaxeChange] {
        &self.changes
    }
}
