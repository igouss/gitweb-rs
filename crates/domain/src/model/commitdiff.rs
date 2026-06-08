//! The commitdiff diff-base rule (gitweb's `git_commitdiff` base selection).
//!
//! gitweb diffs a commit against a base tree it derives from the commit's
//! parents and an optional explicit parent (its `$hash_parent_param`):
//!
//! ```text
//! $hash_parent_param = defined $hash_parent
//!     ? $hash_parent
//!     : @{parents} > 1 ? '--cc' : $parent || '--root';
//! ```
//!
//! Our diff viewer renders two-tree unified diffs, not git's combined (`--cc`)
//! output, so the merge case is reduced to the merge's *first* parent rather than
//! the combined diff. The selection is otherwise gitweb's: an explicit parent
//! wins; a single parent is the base; a root commit diffs against the empty tree.
//!
//! This is the pure half of the capability. Picking the base is one thing —
//! reading the patch over the [`Repository`](crate::port::repository::Repository)
//! port is another — so only the selection lives here, shared by the commitdiff
//! host page (which turns the base into the viewer's diff URL) and the clean-diff
//! use case (which feeds it to the patch port).

use crate::model::object_id::ObjectId;

/// The tree a commitdiff is taken against — gitweb's `$hash_parent_param`,
/// restricted to the two-tree forms the diff viewer renders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DiffBase {
    /// The empty tree: a root commit's diff (gitweb's `--root`).
    EmptyTree,
    /// A specific parent or commit id to diff against.
    Commit(ObjectId),
}

/// Picks the base tree a commit is diffed against: the `explicit` parent when one
/// is given, otherwise the commit's first parent, or the empty tree when it has
/// none (a root commit). A merge with no explicit parent diffs against its first
/// parent — the two-tree reduction of gitweb's combined `--cc` default.
#[must_use]
pub fn diff_base(parents: &[ObjectId], explicit: Option<&ObjectId>) -> DiffBase {
    match explicit.or_else(|| parents.first()) {
        Some(base) => DiffBase::Commit(base.clone()),
        None => DiffBase::EmptyTree,
    }
}
