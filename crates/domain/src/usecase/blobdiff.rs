//! The `blobdiff` use case: resolving the single file an html diff page shows.
//!
//! gitweb's `git_blobdiff` (html) renders the diff of *one* file between two
//! base commits. Before it can, it resolves *which* file from the raw diff-tree:
//! directly by the file's new-side path (`f=`), or — the legacy by-hash URI — by
//! the blob's new-side id, grepping the tree's `to_id` column. It then reads the
//! base commit's subject for the page header — or, when the base is not a commit
//! (a raw tree in a hand-crafted URI), falls back to the degenerate
//! `"<new-id> vs <old-id>"` title gitweb's `else` branch prints.
//!
//! We modernize the page itself: the diff body is rendered on the client by the
//! `@pierre/diffs` viewer, which fetches the clean single-file patch from the
//! diff-text endpoint ([`commitdiff`](crate::usecase::commitdiff)'s
//! `assemble_commit_diff` scoped to the path). So this use case is only the
//! *resolution* — it produces the [`BlobdiffView`] the host page needs (the
//! commit subject, the resolved file name, and a rename's old path), leaving the
//! diff bytes to that separate fetch.
//!
//! The resolution rule lives on [`Patch`](crate::model::patch::Patch)
//! ([`select_by_to_path`](crate::model::patch::Patch::select_by_to_path) /
//! [`select_by_to_oid`](crate::model::patch::Patch::select_by_to_oid)); this use
//! case orchestrates it over the [`Repository`] port and maps its outcome to
//! gitweb's status codes — more than one match is 400 "Ambiguous blob diff
//! specification", no match is 404 "Blob diff not found", and neither selector
//! is 400 "Missing one of the blob diff parameters".
//!
//! [`Repository`]: crate::port::repository::Repository

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::{FilePatch, FileSelection, Patch};
use crate::port::repository::{RenameDetection, Repository};

/// gitweb's site-default rename detection (`@diff_opts = ('-M')`): the same
/// default the `commit` / `commitdiff` / `blobdiff_plain` views use, so a renamed
/// path is paired old→new and the view can report its old path.
const DETECTION: RenameDetection = RenameDetection::RenamesOnly;

/// What the html `blobdiff` host page needs from the resolution: the base
/// commit's subject for the header, the resolved file's new-side path for the
/// page path and the viewer's fetch, and — for a rename or copy — its old-side
/// path (gitweb's `file_parent`), so the `raw` affordance and the fetch carry it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobdiffView {
    title: String,
    file_name: String,
    file_parent: Option<String>,
}

impl BlobdiffView {
    /// Assembles the view from its resolved parts.
    #[must_use]
    pub fn new(
        title: impl Into<String>,
        file_name: impl Into<String>,
        file_parent: Option<String>,
    ) -> Self {
        Self {
            title: title.into(),
            file_name: file_name.into(),
            file_parent,
        }
    }

    /// The base commit's subject (gitweb's `$co{'title'}`), shown in the header.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The resolved new-side path (gitweb's `to_file`): the page path and the
    /// `f=` the viewer's clean-diff fetch carries.
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The old-side path (gitweb's `file_parent`), present only when it differs
    /// from [`file_name`](Self::file_name) — a rename or copy.
    #[must_use]
    pub fn file_parent(&self) -> Option<&str> {
        self.file_parent.as_deref()
    }
}

/// Resolves the one file an html `blobdiff` shows between `hash_parent_base` and
/// `hash_base`: select it by its new-side path (`file_name`) or, failing that, by
/// a blob's new-side id (`hash`), then take the header title — the base commit's
/// subject, or the degenerate `"<new-id> vs <old-id>"` when the base is not a
/// commit (see [`blobdiff_title`]).
///
/// # Errors
///
/// - [`DomainError::Invalid`] `Missing one of the blob diff parameters` when
///   neither `file_name` nor `hash` is given.
/// - [`DomainError::Invalid`] `Ambiguous blob diff specification` when more than
///   one file matches (only reachable by id, when two files share new-side
///   content).
/// - [`DomainError::NotFound`] `Blob diff not found` when no file matches.
/// - Propagates the repository's failures (an unresolvable base, a patch read).
pub fn assemble_blobdiff(
    repo: &dyn Repository,
    hash_base: &str,
    hash_parent_base: &str,
    file_name: Option<&str>,
    hash: Option<&str>,
) -> Result<BlobdiffView, DomainError> {
    let to: ObjectId = repo.resolve(hash_base)?;
    let from: ObjectId = repo.resolve(hash_parent_base)?;
    let patch: Patch = repo.patch(Some(&from), &to, DETECTION)?;

    let selection: FileSelection<'_> = match (file_name, hash) {
        (Some(name), _) => patch.select_by_to_path(name),
        (None, Some(id)) => patch.select_by_to_oid(id),
        (None, None) => {
            return Err(DomainError::Invalid(
                "Missing one of the blob diff parameters".to_owned(),
            ));
        }
    };
    let file: &FilePatch = match selection {
        FileSelection::One(file) => file,
        FileSelection::Missing => {
            return Err(DomainError::NotFound("Blob diff not found".to_owned()));
        }
        FileSelection::Ambiguous => {
            return Err(DomainError::Invalid(
                "Ambiguous blob diff specification".to_owned(),
            ));
        }
    };

    // gitweb fills `$file_parent ||= $diffinfo{'from_file'}` and shows the old
    // path only when it differs — a rename or copy.
    let file_parent: Option<String> =
        (file.from_path() != file.to_path()).then(|| file.from_path().to_owned());

    let title: String = blobdiff_title(repo, &to, file)?;
    Ok(BlobdiffView::new(
        title,
        file.to_path().to_owned(),
        file_parent,
    ))
}

/// The header title for a `blobdiff`: the base commit's subject when the base is
/// a commit (gitweb's `parse_commit($hash_base)` succeeding), else the degenerate
/// `"<new-id> vs <old-id>"` gitweb's `git_blobdiff` `else` branch prints — the
/// resolved file's new-side then old-side blob ids (`$hash` and `$hash_parent`).
///
/// Reachable only for a base that is tree-ish but not a commit — a raw tree id in
/// a hand-crafted URI; every changed-files / history / feed link carries a commit
/// base. Gating on the kind keeps a genuine repository failure propagating rather
/// than collapsing into a degenerate title. (An annotated tag, which gitweb's
/// `parse_commit` would peel to its commit, is out of scope: no link emits one
/// here, and a bare tag id does not peel.)
fn blobdiff_title(
    repo: &dyn Repository,
    base: &ObjectId,
    file: &FilePatch,
) -> Result<String, DomainError> {
    match repo.object_kind(base)? {
        ObjectKind::Commit => {
            let commit: Commit = repo.find_commit(base)?;
            Ok(commit.title())
        }
        _ => Ok(format!(
            "{} vs {}",
            file.to_oid().as_str(),
            file.from_oid().as_str()
        )),
    }
}
