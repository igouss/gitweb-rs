//! The `commitdiff` clean-diff use case: a commit's diff as git's unified patch.
//!
//! gitweb's `git_commitdiff` colourises a commit's diff in the page itself. We
//! modernize that: the page is a host shell, and the diff is rendered on the
//! client by the vendored `@pierre/diffs` viewer, which fetches the diff as a
//! *clean* unified patch — the bytes `git diff-tree -p` emits, starting at
//! `diff --git`. That clean text must not be `commitdiff_plain`, whose `From:` /
//! `Subject:` mail header breaks the viewer's `parsePatchFiles`; this use case
//! produces the bare patch instead.
//!
//! The orchestration mirrors [`assemble_commit`](crate::usecase::commit): resolve
//! the commit-ish (defaulting to `HEAD`), refuse a non-commit with gitweb's
//! `die_error(404, "Unknown commit object")`, then read the patch over the
//! [`Repository`] port against the base the pure
//! [`diff_base`](crate::model::commitdiff::diff_base) rule picks.
//!
//! [`Repository`]: crate::port::repository::Repository

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::commitdiff::{DiffBase, diff_base};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::Patch;
use crate::port::repository::{RenameDetection, Repository};

/// gitweb's site-default rename detection (`@diff_opts = ('-M')`): renames only,
/// the same default the [`commit`](crate::usecase::commit) view uses.
const DETECTION: RenameDetection = RenameDetection::RenamesOnly;

/// Resolves `revision` (defaulting to `HEAD`) to a commit and renders its diff —
/// against `explicit_parent` when one is given, otherwise the base the
/// [`diff_base`] rule picks — as git's clean unified patch text.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] with gitweb's `Unknown commit object`
/// message when the revision is not a commit, and propagates the repository's
/// failures (an unresolvable revision, a bad explicit parent, a patch read).
pub fn assemble_commit_diff(
    repo: &dyn Repository,
    revision: Option<&str>,
    explicit_parent: Option<&str>,
) -> Result<String, DomainError> {
    let oid: ObjectId = repo.resolve(revision.unwrap_or("HEAD"))?;
    if repo.object_kind(&oid)? != ObjectKind::Commit {
        return Err(DomainError::NotFound("Unknown commit object".to_owned()));
    }
    let commit: Commit = repo.find_commit(&oid)?;
    let explicit: Option<ObjectId> = match explicit_parent {
        Some(rev) => Some(repo.resolve(rev)?),
        None => None,
    };
    let from: Option<ObjectId> = match diff_base(commit.parents(), explicit.as_ref()) {
        DiffBase::EmptyTree => None,
        DiffBase::Commit(base) => Some(base),
    };
    let patch: Patch = repo.patch(from.as_ref(), commit.id(), DETECTION)?;
    Ok(patch.render())
}
