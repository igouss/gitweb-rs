//! The `patch` use case: a single commit as a `git format-patch` mail.
//!
//! gitweb's `git_patch` is `git_commitdiff(-format => 'patch', -single => 1)`:
//! it streams `git format-patch --stdout -1` for one commit. This use case
//! assembles that mail from the [`Repository`] port — gating on the `patches`
//! feature (gitweb's `die_error(403, "Patch view not allowed") unless
//! $patch_max`), resolving the commit-ish (defaulting to `HEAD`), refusing a
//! non-commit with the 404 "Unknown commit object" — and frames it with the
//! [`FormatPatch`] rule: the mailbox header, the diffstat git's `--stat
//! --summary` produces, and the unified diff with abbreviated `index` ids.
//!
//! The diff base is the commit's first parent (the empty tree for a root commit,
//! a `--root` create diff). A merge's first-parent reduction is the same
//! placeholder `commitdiff_plain` uses today; the combined `--cc` diff a merge
//! `patch` should really show needs a combined-patch text primitive the port
//! does not yet expose, and is unified across the diff endpoints separately.
//!
//! The single-line ASCII subject of an ordinary commit needs no RFC-2047
//! encoding; the volatile git version on the signature is the boundary's, passed
//! through to [`FormatPatch`].
//!
//! A binary file is embedded as git's base85 `GIT binary patch` body, byte-for-byte
//! with `git format-patch --binary` (which `format-patch` implies):
//! [`Patch::render_format_patch`] picks whichever is shorter of the deflated blob
//! (`literal`) or a deflated git binary delta against the pre-image (`delta`) and
//! writes a full `index`, via [`crate::model::binary_patch`]. The `Bin <old> -> <new>
//! bytes` diffstat row is byte-exact ([`binary_change`] reads the blob sizes over
//! the port). Only `patch` / `patches` embed the blob this way; the plain diff
//! endpoints stream bare `git diff-tree -p`, whose binary body is the `Binary files
//! … differ` notice.
//!
//! [`Repository`]: crate::port::repository::Repository
//! [`Patch::render_format_patch`]: crate::model::patch::Patch::render_format_patch

use crate::error::DomainError;
use crate::model::change::ChangeKind;
use crate::model::commit::Commit;
use crate::model::diffstat::{Diffstat, DiffstatEntry, StatChange};
use crate::model::format_patch::{FormatPatch, PatchEntry};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::{FilePatch, Patch};
use crate::model::timestamp::Timestamp;
use crate::port::repository::{Page, RenameDetection, Repository};

/// gitweb's site-default rename detection (`@diff_opts = ('-M')`), the same git
/// format-patch runs.
const DETECTION: RenameDetection = RenameDetection::RenamesOnly;

/// Resolves `revision` (defaulting to `HEAD`) to a commit and assembles its
/// `patch` mail. `patch_max` is the `patches` feature's limit (gitweb's
/// `$patch_max`); zero forbids the view. `version` is the git version stamped on
/// the mail's signature.
///
/// # Errors
///
/// Returns [`DomainError::Forbidden`] (gitweb's 403 "Patch view not allowed")
/// when the `patches` feature is off, [`DomainError::NotFound`] (gitweb's 404
/// "Unknown commit object") when the revision is not a commit, and propagates the
/// repository's own failures.
pub fn assemble_patch(
    repo: &dyn Repository,
    revision: Option<&str>,
    patch_max: usize,
    version: &str,
) -> Result<FormatPatch, DomainError> {
    if patch_max == 0 {
        return Err(DomainError::Forbidden("Patch view not allowed".to_owned()));
    }

    let oid: ObjectId = repo.resolve(revision.unwrap_or("HEAD"))?;
    if repo.object_kind(&oid)? != ObjectKind::Commit {
        return Err(DomainError::NotFound("Unknown commit object".to_owned()));
    }
    let commit: Commit = repo.find_commit(&oid)?;

    // The single form is unnumbered (`[PATCH]`), one mail.
    let entry: PatchEntry = build_entry(repo, &oid, &commit, None)?;
    Ok(FormatPatch::new(vec![entry], version))
}

/// Resolves `revision` (defaulting to `HEAD`) to a commit and assembles the
/// `patches` mailbox stream: the most recent `patch_max` commits reachable from
/// it, oldest-first, each `Subject` numbered `[PATCH i/N]` where `N` is the count
/// emitted — gitweb's `git_patches`, `git format-patch -n -<patch_max> --root
/// <hash>`. `version` is the git version stamped on every mail's signature.
///
/// Each commit's mail diffs it against its own first parent (the empty tree for
/// the root, a `--root` create), so a window that stops short of the root still
/// diffs its oldest commit against the parent just outside it — the same patch
/// `git format-patch` writes. A merge in the range reduces to its first parent;
/// the combined `--cc` rendering is unified separately (see the merge-fidelity
/// bead), and the parity corpus is kept linear.
///
/// # Errors
///
/// Returns [`DomainError::Forbidden`] (gitweb's 403 "Patch view not allowed")
/// when the `patches` feature is off, [`DomainError::NotFound`] (gitweb's 404
/// "Unknown commit object") when the revision is not a commit, and propagates the
/// repository's own failures.
pub fn assemble_patches(
    repo: &dyn Repository,
    revision: Option<&str>,
    patch_max: usize,
    version: &str,
) -> Result<FormatPatch, DomainError> {
    if patch_max == 0 {
        return Err(DomainError::Forbidden("Patch view not allowed".to_owned()));
    }

    let oid: ObjectId = repo.resolve(revision.unwrap_or("HEAD"))?;
    if repo.object_kind(&oid)? != ObjectKind::Commit {
        return Err(DomainError::NotFound("Unknown commit object".to_owned()));
    }

    // The most recent `patch_max` commits reachable from the tip, newest-first
    // (git rev-list order). `git format-patch` caps the range the same way (`-N`).
    let commits: Vec<Commit> = repo.history(&oid, None, Page::new(0, patch_max))?;
    let total: usize = commits.len();

    // format-patch emits oldest-first, numbered `[PATCH i/N]`; reverse the
    // newest-first walk and count off the index against the emitted total.
    let mut entries: Vec<PatchEntry> = Vec::with_capacity(total);
    for (offset, commit) in commits.iter().rev().enumerate() {
        let number: Option<(usize, usize)> = Some((offset + 1, total));
        let entry: PatchEntry = build_entry(repo, commit.id(), commit, number)?;
        entries.push(entry);
    }
    Ok(FormatPatch::new(entries, version))
}

/// Builds one commit's `PatchEntry`: the mailbox header fields, the diffstat over
/// its diff against the first parent (the empty tree for a root commit), and the
/// abbreviated-`index` diff body. `number` is `Some((i, n))` for a numbered
/// stream, `None` for the single form.
pub(crate) fn build_entry(
    repo: &dyn Repository,
    oid: &ObjectId,
    commit: &Commit,
    number: Option<(usize, usize)>,
) -> Result<PatchEntry, DomainError> {
    let from: Option<ObjectId> = commit.parents().first().cloned();
    let patch: Patch = repo.patch(from.as_ref(), oid, DETECTION)?;
    let abbrev: usize = repo.abbrev_length()?;
    let diff_body: String = patch.render_format_patch(abbrev);
    let diffstat: Diffstat = build_diffstat(repo, &patch)?;

    let (subject, body): (String, Vec<String>) = split_message(commit.message());
    Ok(PatchEntry::new(
        oid.as_str().to_owned(),
        commit.author().full_ident(),
        Timestamp::from_signature(commit.author()),
        subject,
        number,
        body,
        diffstat,
        diff_body,
    ))
}

/// Builds the diffstat over a patch: each file's added/deleted line counts for a
/// text change, its old/new blob sizes for a binary one (read through the port,
/// with the absent side of a create or delete counted as zero bytes).
fn build_diffstat(repo: &dyn Repository, patch: &Patch) -> Result<Diffstat, DomainError> {
    let mut entries: Vec<DiffstatEntry> = Vec::with_capacity(patch.files().len());
    for file in patch.files() {
        let change: StatChange = match file.line_counts() {
            Some((added, deleted)) => StatChange::Text { added, deleted },
            None => binary_change(repo, file)?,
        };
        entries.push(DiffstatEntry::new(
            file.status(),
            file.from_mode(),
            file.to_mode(),
            file.from_path(),
            file.to_path(),
            change,
        ));
    }
    Ok(Diffstat::new(entries))
}

/// The byte-size magnitude of a binary file, git's `Bin <old> -> <new> bytes`:
/// the side that does not exist (a create's pre-image, a delete's post-image)
/// is zero, the other read through the port.
fn binary_change(repo: &dyn Repository, file: &FilePatch) -> Result<StatChange, DomainError> {
    let old_size: u64 = match file.status().kind() {
        ChangeKind::Added => 0,
        _ => repo.object_size(file.from_oid())?,
    };
    let new_size: u64 = match file.status().kind() {
        ChangeKind::Deleted => 0,
        _ => repo.object_size(file.to_oid())?,
    };
    Ok(StatChange::Binary { old_size, new_size })
}

/// Splits a commit message into git's format-patch subject and body: the first
/// non-empty line is the subject; the body is what follows, with the blank-line
/// separator and any trailing blank lines dropped, internal blanks kept. The
/// single-line-subject assumption an ordinary commit satisfies; git folds a
/// multi-line first paragraph into the subject, a deeper edge left for later.
fn split_message(message: &str) -> (String, Vec<String>) {
    let mut lines = message.lines().skip_while(|line: &&str| line.is_empty());
    let subject: String = lines.next().unwrap_or_default().to_owned();
    let mut body: Vec<String> = lines.map(str::to_owned).collect();
    while body.first().map(String::as_str) == Some("") {
        body.remove(0);
    }
    while body.last().map(String::as_str) == Some("") {
        body.pop();
    }
    (subject, body)
}
