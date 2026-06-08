//! The `shortlog` use case: gitweb's `git_shortlog` / `git_shortlog_body`, minus
//! the HTML.
//!
//! gitweb's `git_log_generic` walks history from a base revision (HEAD by
//! default) a page at a time, asking `parse_commits` for one commit more than the
//! page holds so it can tell whether to offer a "next"/"..." link. This use case
//! does the same over the [`Repository`] port: it resolves the base revision,
//! reads `limit + 1` commits at the requested `skip`, keeps `limit` of them as
//! rows, and reports the surplus as [`ShortlogView::has_more`]. Each row carries
//! the date cell (the two-week age/date swap, [`CommitDate`]), the author name
//! chopped to gitweb's 10 characters for the row, and the commit subject.
//!
//! The clock is injected (`now`) so the date cells are computed here, once, and
//! the view-model stays free of any time dependency — the same discipline as the
//! heads use case. An unborn repository (no HEAD) has no history and yields an
//! empty view rather than an error.

use crate::error::DomainError;
use crate::model::chop::{ChopMode, chop_str};
use crate::model::commit::Commit;
use crate::model::commit_date::CommitDate;
use crate::model::object_id::ObjectId;
use crate::port::repository::{Page, Repository};

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
    let start: ObjectId = match resolve_start(repo, rev)? {
        Some(oid) => oid,
        None => {
            return Ok(ShortlogView {
                rows: Vec::new(),
                has_more: false,
            });
        }
    };

    // Ask for one more than the page holds, the way gitweb does, so the surplus
    // tells us a further page exists without a second round trip.
    let probe: Page = Page::new(page.skip, page.limit + 1);
    let commits: Vec<Commit> = repo.history(&start, None, probe)?;
    let has_more: bool = commits.len() > page.limit;
    let rows: Vec<ShortlogRow> = commits
        .into_iter()
        .take(page.limit)
        .map(|commit: Commit| row_of(&commit, now))
        .collect();
    Ok(ShortlogView { rows, has_more })
}

/// Resolves the base revision to the commit history walks from: an explicit
/// revision through the port's resolver, or HEAD's target. An unborn HEAD yields
/// `None` (gitweb's empty `parse_commits` on an undefined base); any other
/// failure propagates.
fn resolve_start(
    repo: &dyn Repository,
    rev: Option<&str>,
) -> Result<Option<ObjectId>, DomainError> {
    match rev {
        Some(revision) => repo.resolve(revision).map(Some),
        None => match repo.head() {
            Ok(reference) => Ok(Some(reference.target().clone())),
            Err(DomainError::NotFound(_)) => Ok(None),
            Err(other) => Err(other),
        },
    }
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
