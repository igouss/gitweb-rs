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
//! Two base-selection rules live here, because gitweb uses the same
//! `$hash_parent_param` for two different things:
//!
//! - [`diff_base`] is the *viewer's* base. Our diff viewer renders two-tree
//!   unified diffs, not git's combined (`--cc`) output, so the merge case is
//!   reduced to the merge's *first* parent rather than the combined diff. The
//!   selection is otherwise gitweb's: an explicit parent wins; a single parent is
//!   the base; a root commit diffs against the empty tree.
//! - [`changed_files_base`] is the *table's* base ([`ChangedFilesBase`]): the
//!   shape git_difftree_body is fed (`$use_parents`). It keeps the combined form a
//!   merge shows by default — so a merge's table is combined, but an explicit
//!   parent still wins and makes it an ordinary two-tree diff against that parent.
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

/// How a commit's changed-files table (gitweb's `git_difftree_body`) is taken —
/// gitweb's `$hash_parent_param` / `$use_parents` choice. Unlike [`DiffBase`],
/// which the two-tree diff viewer renders, this keeps the combined form a merge
/// shows by default, so the merge and explicit-parent cases differ.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangedFilesBase {
    /// The combined diff against every parent of a merge with no explicit parent
    /// (gitweb's `--cc`).
    Combined,
    /// An ordinary two-tree diff against a single base: the explicit parent or the
    /// first parent (`Some`), or the empty tree for a root commit (`None`,
    /// gitweb's `--root`).
    Ordinary {
        /// The base tree, or `None` for the empty tree of a root commit.
        base: Option<ObjectId>,
    },
}

/// Picks the shape and base of a commit's changed-files table: an explicit parent
/// always wins and makes it an ordinary two-tree diff against that parent;
/// otherwise a merge is the combined diff against every parent, a single-parent
/// commit is the two-tree diff against its parent, and a root commit is the
/// two-tree diff against the empty tree. gitweb's `git_commitdiff`
/// `$hash_parent_param` (`$hash_parent` when given, else `--cc` for a merge or
/// `$parent || '--root'`) together with `$use_parents`.
#[must_use]
pub fn changed_files_base(parents: &[ObjectId], explicit: Option<&ObjectId>) -> ChangedFilesBase {
    match explicit {
        Some(base) => ChangedFilesBase::Ordinary {
            base: Some(base.clone()),
        },
        None if parents.len() > 1 => ChangedFilesBase::Combined,
        None => ChangedFilesBase::Ordinary {
            base: parents.first().cloned(),
        },
    }
}
