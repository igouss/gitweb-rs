//! The `commitdiff_plain` use case: a commit's diff as a `git am`-able patch.
//!
//! gitweb's `git_commitdiff -format plain` frames git's raw diff in a mailbox
//! header. This use case assembles everything that frame needs from the
//! [`Repository`] port — resolving the commit-ish (defaulting to `HEAD`),
//! refusing a non-commit with gitweb's `die_error(404, "Unknown commit object")`
//! — and renders the patch body with the repository's default `index`
//! abbreviation, so the result is the short-id form bare `git diff-tree -p`
//! writes. The `self_url` for the `X-Git-Tag`-adjacent `X-Git-Url` line is the
//! boundary's job, so the produced [`CommitdiffPlain`] carries everything but
//! that link.
//!
//! The patch base is the pure [`diff_base`] rule's: an explicit parent, else the
//! first parent, else the empty tree (a root commit). git's `diff-tree` prints
//! the commit hash ahead of the patch only for the single-commit-ish form it
//! sees for a root (`--root`) commit — never for the two-tree form of a commit
//! with a parent — so the commit-id line rides along exactly when the base is
//! the empty tree.
//!
//! The `X-Git-Tag` line is gitweb's `git_get_rev_name_tags` — a `git name-rev
//! --tags` lookup — supplied by the port's [`Repository::rev_name_tag`]: the
//! `tags/<name>` body when a tag's tip is the commit (`<name>^0` for an
//! annotated tag), else `None`, which omits the line.
//!
//! [`Repository`]: crate::port::repository::Repository

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::commitdiff::{DiffBase, diff_base};
use crate::model::commitdiff_plain::CommitdiffPlain;
use crate::model::feed::comment_lines;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::Patch;
use crate::model::timestamp::Timestamp;
use crate::port::repository::{RenameDetection, Repository};

/// gitweb's site-default rename detection (`@diff_opts = ('-M')`): renames only,
/// the same default the `commitdiff` view uses.
const DETECTION: RenameDetection = RenameDetection::RenamesOnly;

/// Resolves `revision` (defaulting to `HEAD`) to a commit and assembles its
/// `commitdiff_plain` body — the mailbox header plus the abbreviated-`index`
/// patch — against `explicit_parent` when one is given, otherwise the base the
/// [`diff_base`] rule picks.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] with gitweb's `Unknown commit object`
/// message when the revision is not a commit, and propagates the repository's
/// failures (an unresolvable revision, a bad explicit parent, a patch read, the
/// abbreviation length).
pub fn assemble_commitdiff_plain(
    repo: &dyn Repository,
    revision: Option<&str>,
    explicit_parent: Option<&str>,
) -> Result<CommitdiffPlain, DomainError> {
    let oid: ObjectId = repo.resolve(revision.unwrap_or("HEAD"))?;
    if repo.object_kind(&oid)? != ObjectKind::Commit {
        return Err(DomainError::NotFound("Unknown commit object".to_owned()));
    }
    let commit: Commit = repo.find_commit(&oid)?;
    let tag: Option<String> = repo.rev_name_tag(&oid)?;

    let explicit: Option<ObjectId> = match explicit_parent {
        Some(rev) => Some(repo.resolve(rev)?),
        None => None,
    };
    let base: DiffBase = diff_base(commit.parents(), explicit.as_ref());
    let from: Option<ObjectId> = match &base {
        DiffBase::EmptyTree => None,
        DiffBase::Commit(parent) => Some(parent.clone()),
    };

    let patch: Patch = repo.patch(from.as_ref(), commit.id(), DETECTION)?;
    let abbrev: usize = repo.abbrev_length()?;
    let patch_body: String = patch.render_abbreviated(abbrev);

    // `git diff-tree` prefixes the patch with the commit hash for the
    // single-commit-ish (`--root`) form, which is exactly the empty-tree base.
    let commit_id: Option<String> = match base {
        DiffBase::EmptyTree => Some(commit.id().as_str().to_owned()),
        DiffBase::Commit(_) => None,
    };

    let comment: Vec<String> = comment_lines(commit.message())
        .into_iter()
        .map(str::to_owned)
        .collect();

    Ok(CommitdiffPlain::new(
        commit.author().full_ident(),
        Timestamp::from_signature(commit.author()),
        commit.title(),
        tag,
        comment,
        commit_id,
        patch_body,
    ))
}
