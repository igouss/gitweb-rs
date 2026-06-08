//! The `history` use case: gitweb's `git_history` / `git_history_body`, minus the
//! HTML.
//!
//! gitweb's per-path log. It walks the same windowed history the shortlog and
//! log do ([`walk_commits`], gitweb's `git_log_generic`), but limited to the
//! commits that touch one file or directory, and decorates each row with the
//! links that only make sense for a tracked path: the blob/tree at that commit,
//! the raw file, and — for a file whose content here differs from the newest
//! version in this view — a "diff to current".
//!
//! Two facts are shared by the whole view, not per row, so they are resolved
//! once. The file's *type* (gitweb's `$ftype`: a blob for a file, a tree for a
//! directory) is read from the first commit in the window that still has the
//! path — gitweb assumes it is constant across the history. The *current* blob
//! (gitweb's `$file_hash` / `$blob_current`) is that same first-present id; every
//! row's "diff to current" compares its own blob against it. A path that no
//! commit in the window still carries has no resolvable type, which is gitweb's
//! `die_error(500, "Unknown type of object")`.
//!
//! The clock is injected (`now`) so the date cells (the two-week age/date swap,
//! [`CommitDate`]) are computed here, once, and the view stays free of any time
//! dependency.

use crate::error::DomainError;
use crate::model::chop::{ChopMode, chop_str};
use crate::model::commit::Commit;
use crate::model::commit_date::CommitDate;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::port::repository::{Page, Repository};
use crate::usecase::log_generic::{CommitWindow, walk_commits};

/// gitweb's `format_author_html('td', \%co, 15, 3)` — the history row author
/// bound (15 characters, 3 of word slack), tighter than the shortlog's 10.
const AUTHOR_LEN: usize = 15;
/// gitweb's `$add_len` for the history author chop.
const AUTHOR_SLACK: usize = 3;

/// One commit as the per-path history shows it: identity, date cell, author, the
/// subject, and — only when this commit's version of the file differs from the
/// view's current one — the blob to offer a "diff to current" from.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryRow {
    id: String,
    date: CommitDate,
    author: String,
    author_short: String,
    title: String,
    title_short: String,
    diff_to_current: Option<String>,
}

impl HistoryRow {
    /// The commit's object id (full hex).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The date cell with the two-week age/date swap resolved.
    #[must_use]
    pub fn date(&self) -> &CommitDate {
        &self.date
    }

    /// The full author name.
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// The author name chopped for the row (gitweb's `chop_str(.., 15, 3)`).
    #[must_use]
    pub fn author_short(&self) -> &str {
        &self.author_short
    }

    /// The full commit subject.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The short commit subject.
    #[must_use]
    pub fn title_short(&self) -> &str {
        &self.title_short
    }

    /// The id of the file's blob *at this commit*, present only when a "diff to
    /// current" is offered: the path is a file, it exists here, and its content
    /// differs from the view's current blob (gitweb's `if (defined $blob_current
    /// && defined $blob_parent && $blob_current ne $blob_parent)`). The link's
    /// other side is [`HistoryView::file_hash`].
    #[must_use]
    pub fn diff_to_current(&self) -> Option<&str> {
        self.diff_to_current.as_deref()
    }
}

/// The assembled per-path history: the tracked path, the type and current blob
/// shared by the whole view, the rows for this page, and whether one more exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HistoryView {
    file_name: String,
    file_type: ObjectKind,
    file_hash: String,
    rows: Vec<HistoryRow>,
    has_more: bool,
}

impl HistoryView {
    /// The tracked path (gitweb's `$file_name`).
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The path's object type, constant across the history (gitweb's `$ftype`):
    /// [`ObjectKind::Blob`] for a file, [`ObjectKind::Tree`] for a directory.
    #[must_use]
    pub fn file_type(&self) -> ObjectKind {
        self.file_type
    }

    /// The id of the path's *current* object — the newest version in this view
    /// (gitweb's `$file_hash` / `$blob_current`). The "diff to current" links
    /// target this; for a directory it is the current subtree id.
    #[must_use]
    pub fn file_hash(&self) -> &str {
        &self.file_hash
    }

    /// The commit rows for this page.
    #[must_use]
    pub fn rows(&self) -> &[HistoryRow] {
        &self.rows
    }

    /// Whether a further page exists (gitweb's "next" affordance).
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }
}

/// Assembles the per-path history over the [`Repository`] port. `base_rev` is the
/// base revision the walk starts from (`None` for HEAD, gitweb's `$hash_base`);
/// `path` is the tracked file or directory; `now` is the request-time epoch the
/// date cells are measured against; `page`'s `limit` is the page size.
///
/// # Errors
///
/// Propagates the repository's error if resolving the revision or reading history
/// fails. Returns [`DomainError::Backend`] — gitweb's `die_error(500, "Unknown
/// type of object")` — when no commit in the window still carries `path`, so its
/// type cannot be resolved (an unborn repository or a never-existent path).
pub fn assemble_history(
    repo: &dyn Repository,
    base_rev: Option<&str>,
    path: &str,
    now: i64,
    page: Page,
) -> Result<HistoryView, DomainError> {
    let window: CommitWindow = walk_commits(repo, base_rev, Some(path), page)?;
    let file_hash: ObjectId = current_blob(repo, &window.commits, path)?
        .ok_or_else(|| DomainError::Backend("Unknown type of object".to_owned()))?;
    let file_type: ObjectKind = repo.object_kind(&file_hash)?;
    let rows: Vec<HistoryRow> = window
        .commits
        .iter()
        .map(|commit: &Commit| row_of(repo, commit, path, &file_hash, file_type, now))
        .collect::<Result<Vec<HistoryRow>, DomainError>>()?;
    Ok(HistoryView {
        file_name: path.to_owned(),
        file_type,
        file_hash: file_hash.as_str().to_owned(),
        rows,
        has_more: window.has_more,
    })
}

/// The path's *current* object id: the id it has at the first commit in the
/// window that still carries it, newest first (gitweb's loop until
/// `git_get_hash_by_path` is defined). `None` when no commit in the window has
/// the path — every one of them deleted it, or there were none at all.
fn current_blob(
    repo: &dyn Repository,
    commits: &[Commit],
    path: &str,
) -> Result<Option<ObjectId>, DomainError> {
    for commit in commits {
        if let Some(oid) = repo.path_id(commit.id(), path)? {
            return Ok(Some(oid));
        }
    }
    Ok(None)
}

/// Maps one commit into a history row: its id, the date cell, the author (full
/// and chopped to gitweb's 15/3 bound), the subject, and the "diff to current"
/// blob when this is a file whose content here differs from the current one.
fn row_of(
    repo: &dyn Repository,
    commit: &Commit,
    path: &str,
    file_hash: &ObjectId,
    file_type: ObjectKind,
    now: i64,
) -> Result<HistoryRow, DomainError> {
    let author: String = commit.author().name().to_owned();
    let author_short: String = chop_str(&author, AUTHOR_LEN, AUTHOR_SLACK, ChopMode::Right);
    let diff_to_current: Option<String> =
        diff_to_current(repo, commit, path, file_hash, file_type)?;
    Ok(HistoryRow {
        id: commit.id().as_str().to_owned(),
        date: CommitDate::new(commit.committer().epoch(), now),
        author_short,
        author,
        title: commit.title(),
        title_short: commit.title_short(),
        diff_to_current,
    })
}

/// The blob this commit shows for the file, when a "diff to current" applies:
/// only for a file (a tree has no blobdiff), only when the path exists here, and
/// only when that blob differs from the view's current one (gitweb's three-part
/// guard). Otherwise `None`.
fn diff_to_current(
    repo: &dyn Repository,
    commit: &Commit,
    path: &str,
    file_hash: &ObjectId,
    file_type: ObjectKind,
) -> Result<Option<String>, DomainError> {
    if file_type != ObjectKind::Blob {
        return Ok(None);
    }
    match repo.path_id(commit.id(), path)? {
        Some(parent) if &parent != file_hash => Ok(Some(parent.as_str().to_owned())),
        _ => Ok(None),
    }
}
