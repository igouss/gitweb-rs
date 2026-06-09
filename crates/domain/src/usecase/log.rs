//! The `log` use case: gitweb's `git_log` / `git_log_body`, minus the HTML.
//!
//! The verbose counterpart of the shortlog. It walks the same windowed history
//! ([`walk_commits`], gitweb's `git_log_generic`) and maps each commit into a
//! block-shaped row: the commit's *relative* age for the header (gitweb's
//! `age_string` — always relative here, unlike the shortlog's two-week date
//! swap), the author and the author's absolute date for the authorship line, the
//! subject for the header, and the processed message body
//! ([`log_lines`](crate::model::message_body::log_lines)).
//!
//! The clock is injected (`now`) so the header age is computed here, once, and
//! the view-model stays free of any time dependency. An unborn repository (no
//! HEAD) yields an empty view rather than an error.

use crate::error::DomainError;
use crate::model::age::Age;
use crate::model::commit::Commit;
use crate::model::message_body::{LogLine, log_lines};
use crate::model::ref_marker::{MarkerView, RefMarker};
use crate::model::timestamp::Timestamp;
use crate::port::repository::{Page, Repository};
use crate::usecase::log_generic::{CommitWindow, walk_commits};
use crate::usecase::ref_markers::{RefMarkerIndex, assemble_ref_markers};

/// One commit as the verbose log shows it: its identity, the header age and
/// subject, the authorship, and the message body.
#[derive(Debug, Clone)]
pub struct LogRow {
    id: String,
    age: String,
    author: String,
    timestamp: Timestamp,
    title: String,
    comment: Vec<LogLine>,
    markers: Vec<RefMarker>,
}

impl LogRow {
    /// The commit's object id (full hex).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The relative age shown in the block header (gitweb's `age_string`,
    /// measured against the committer time).
    #[must_use]
    pub fn age(&self) -> &str {
        &self.age
    }

    /// The author's display name.
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// The author's absolute date, shown in the authorship line.
    #[must_use]
    pub fn timestamp(&self) -> &Timestamp {
        &self.timestamp
    }

    /// The commit subject shown in the block header.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The processed message body lines (gitweb's `git_print_log`).
    #[must_use]
    pub fn comment(&self) -> &[LogLine] {
        &self.comment
    }

    /// The ref badges decorating this commit (gitweb's `format_ref_marker`), in
    /// ref-name order — empty when no ref tip is this commit. A direct ref links to
    /// the log; an annotated tag links to its tag page.
    #[must_use]
    pub fn markers(&self) -> &[RefMarker] {
        &self.markers
    }
}

/// The assembled verbose log: the rows for this page and whether one more exists.
#[derive(Debug, Clone)]
pub struct LogView {
    rows: Vec<LogRow>,
    has_more: bool,
}

impl LogView {
    /// The commit rows for this page.
    #[must_use]
    pub fn rows(&self) -> &[LogRow] {
        &self.rows
    }

    /// Whether a further page exists (gitweb's "next" affordance).
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }
}

/// Assembles the verbose log over the [`Repository`] port. `rev` is the base
/// revision (`None` for HEAD); `now` is the request-time epoch the header ages
/// are measured against; `page`'s `limit` is the page size.
///
/// # Errors
///
/// Propagates the repository's error if resolving an explicit revision or reading
/// history fails. An unborn repository whose HEAD names no commit is not an error
/// — it yields an empty view.
pub fn assemble_log(
    repo: &dyn Repository,
    rev: Option<&str>,
    now: i64,
    page: Page,
) -> Result<LogView, DomainError> {
    let window: CommitWindow = walk_commits(repo, rev, None, page)?;
    // gitweb reads every ref once (git_get_references) before badging each row;
    // the verbose log is one of the views format_ref_marker links by name.
    let markers: RefMarkerIndex = assemble_ref_markers(repo, MarkerView::Log)?;
    let rows: Vec<LogRow> = window
        .commits
        .into_iter()
        .map(|commit: Commit| row_of(&commit, now, &markers))
        .collect();
    Ok(LogView {
        rows,
        has_more: window.has_more,
    })
}

/// Maps one commit into a verbose log row: its id, the relative header age (from
/// the committer time), the author and the author's absolute date, the subject,
/// the processed message body, and the ref badges whose tip is this commit.
fn row_of(commit: &Commit, now: i64, markers: &RefMarkerIndex) -> LogRow {
    let age: Age = Age::from_seconds(now - commit.committer().epoch());
    LogRow {
        id: commit.id().as_str().to_owned(),
        age: age.humanized(),
        author: commit.author().name().to_owned(),
        timestamp: Timestamp::from_signature(commit.author()),
        title: commit.title(),
        comment: log_lines(commit.message()),
        markers: markers.for_commit(commit.id()),
    }
}
