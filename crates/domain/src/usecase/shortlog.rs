//! The `shortlog` use case: gitweb's `git_shortlog` / `git_shortlog_body`, minus
//! the HTML.
//!
//! gitweb's `git_log_generic` walks history from a base revision (HEAD by
//! default) a page at a time, asking `parse_commits` for one commit more than the
//! page holds so it can tell whether to offer a "next"/"..." link. That walk is
//! shared with the verbose `log` action, so it lives in
//! [`walk_commits`](crate::usecase::log_generic::walk_commits); this use case
//! drives it and maps each commit into a shortlog row carrying the date cell (the
//! two-week age/date swap, [`CommitDate`]), the author name chopped to gitweb's
//! 10 characters for the row, and the commit subject. The surplus the walk
//! reports becomes [`ShortlogView::has_more`].
//!
//! The clock is injected (`now`) so the date cells are computed here, once, and
//! the view-model stays free of any time dependency — the same discipline as the
//! heads use case. An unborn repository (no HEAD) has no history and yields an
//! empty view rather than an error.

use crate::error::DomainError;
use crate::model::chop::{ChopMode, chop_str};
use crate::model::commit::Commit;
use crate::model::commit_date::CommitDate;
use crate::port::repository::{Page, Repository};
use crate::usecase::log_generic::{CommitWindow, walk_commits};

/// gitweb's `chop_str($author_name, 10)` — the row author bound.
const AUTHOR_LEN: usize = 10;
/// gitweb's default `chop_str` slack (`$add_len ||= 10`).
const AUTHOR_SLACK: usize = 10;

/// One commit as it appears on the shortlog: identity, date cell, author, subject.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortlogRow {
    id: String,
    date: CommitDate,
    author: String,
    author_short: String,
    title: String,
    title_short: String,
}

impl ShortlogRow {
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

    /// The author name chopped for the row (gitweb's `chop_str(.., 10)`).
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
}

/// The assembled shortlog: the rows for this page and whether one more exists.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ShortlogView {
    rows: Vec<ShortlogRow>,
    has_more: bool,
}

impl ShortlogView {
    /// The commit rows for this page.
    #[must_use]
    pub fn rows(&self) -> &[ShortlogRow] {
        &self.rows
    }

    /// Whether a further page exists (gitweb's "..." / "next" affordance).
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }
}

/// Assembles the shortlog over the [`Repository`] port. `rev` is the base
/// revision (`None` for HEAD); `now` is the request-time epoch the date cells are
/// measured against; `page` is the display window (its `limit` is the page size).
///
/// # Errors
///
/// Propagates the repository's error if resolving an explicit revision or
/// reading history fails. An unborn repository whose HEAD names no commit is not
/// an error — it yields an empty view.
pub fn assemble_shortlog(
    repo: &dyn Repository,
    rev: Option<&str>,
    now: i64,
    page: Page,
) -> Result<ShortlogView, DomainError> {
    let window: CommitWindow = walk_commits(repo, rev, page)?;
    let rows: Vec<ShortlogRow> = window
        .commits
        .into_iter()
        .map(|commit: Commit| row_of(&commit, now))
        .collect();
    Ok(ShortlogView {
        rows,
        has_more: window.has_more,
    })
}

/// Turns one commit into a shortlog row: its id, the date cell, the author name
/// (full and chopped to gitweb's 10 characters), and the subject.
fn row_of(commit: &Commit, now: i64) -> ShortlogRow {
    let author: String = commit.author().name().to_owned();
    let author_short: String = chop_str(&author, AUTHOR_LEN, AUTHOR_SLACK, ChopMode::Right);
    ShortlogRow {
        id: commit.id().as_str().to_owned(),
        date: CommitDate::new(commit.committer().epoch(), now),
        author_short,
        author,
        title: commit.title(),
        title_short: commit.title_short(),
    }
}
