//! The commit-search use case — gitweb's `git_search` dispatch to
//! `git_search_message` (the `commit` / `author` / `committer` facets).
//!
//! gitweb's `git_search` checks the `search` feature (403 when off), validates
//! the pattern while parsing the request (400 on a malformed regexp), resolves
//! the base revision (`$hash`, defaulting to `git_get_head_hash`, 404 "Unknown
//! commit object" when it names no commit), and then `git_search_message` walks
//! `parse_commits($hash, 101, 100*$page, …)` — one page plus one, so a further
//! page is detectable — listing each matching commit with its date, author,
//! subject, and the highlighted matching fragments of its message
//! (`git_search_grep_body`).
//!
//! This Control orchestrates that over the [`Repository`] port. The fixed-vs-
//! regexp matching and the rooting are the adapter's job (it applies the
//! [`SearchPattern`] from `base`); here we gate, validate, resolve the base,
//! fetch the windowed page, and map each commit into a [`SearchRow`] — the date
//! cell through [`CommitDate`]'s two-week swap, the author chopped to gitweb's 15
//! characters, and the per-line highlight through
//! [`highlight_line`](crate::model::search_snippet::highlight_line). The clock is
//! injected (`now`) so the view-model stays time-free.

use crate::error::DomainError;
use crate::model::chop::{ChopMode, chop_str};
use crate::model::commit::Commit;
use crate::model::commit_date::CommitDate;
use crate::model::object_id::ObjectId;
use crate::model::search_pattern::SearchPattern;
use crate::model::search_snippet::{MatchSnippet, highlight_line};
use crate::port::repository::{Page, Repository, SearchKind, SearchQuery};

/// gitweb's `format_author_html('td', \%co, 15, 5)` — the search row's author
/// bound (wider than the shortlog's 10).
const AUTHOR_LEN: usize = 15;
/// The author chop's word slack.
const AUTHOR_SLACK: usize = 5;

/// What to search for: the facet, the raw pattern text, and whether it is a
/// regular expression. The pattern is validated (and the matcher built) inside
/// [`assemble_search`], so a malformed regexp surfaces as the use case's 400.
#[derive(Debug, Clone)]
pub struct SearchCriteria {
    /// The search facet (`commit` / `author` / `committer`).
    pub kind: SearchKind,
    /// The pattern text as the user typed it.
    pub text: String,
    /// Whether the pattern is a regular expression (gitweb's `search_use_regexp`).
    pub use_regexp: bool,
}

/// One matching commit on the results page: its id, the date cell, the author
/// (full and chopped for the row), the subject (full and short), and the
/// highlighted message-line snippets.
#[derive(Debug, Clone)]
pub struct SearchRow {
    id: String,
    date: CommitDate,
    author: String,
    author_short: String,
    title: String,
    title_short: String,
    snippets: Vec<MatchSnippet>,
}

impl SearchRow {
    /// The commit object id (full hex).
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The date cell with the two-week age/date swap resolved.
    #[must_use]
    pub fn date(&self) -> &CommitDate {
        &self.date
    }

    /// The full author name (the row's `title` when the name is chopped).
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// The author name chopped to gitweb's 15 characters for the row.
    #[must_use]
    pub fn author_short(&self) -> &str {
        &self.author_short
    }

    /// The full commit subject (the link `title` when the subject is chopped).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The short commit subject shown in the row.
    #[must_use]
    pub fn title_short(&self) -> &str {
        &self.title_short
    }

    /// The highlighted message-line snippets for this commit, in message order;
    /// empty when no message line matches (e.g. an author/committer match the
    /// message text does not echo).
    #[must_use]
    pub fn snippets(&self) -> &[MatchSnippet] {
        &self.snippets
    }
}

/// The assembled search results: the base commit's subject for the page header,
/// the base id the search rooted at, the matching rows for this page, and whether
/// a further page exists.
#[derive(Debug, Clone)]
pub struct SearchView {
    title: String,
    base_id: String,
    rows: Vec<SearchRow>,
    has_more: bool,
}

impl SearchView {
    /// The base commit's subject (the results-page header title).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The base commit id the search rooted at (full hex), the way gitweb's page
    /// navigation links carry `$hash`.
    #[must_use]
    pub fn base_id(&self) -> &str {
        &self.base_id
    }

    /// The matching commit rows for this page, newest first.
    #[must_use]
    pub fn rows(&self) -> &[SearchRow] {
        &self.rows
    }

    /// Whether a further page of matches exists (gitweb's `$#commitlist >= 100`).
    #[must_use]
    pub fn has_more(&self) -> bool {
        self.has_more
    }
}

/// Assembles the search results over the [`Repository`] port. `enabled` is the
/// `search` feature gate; `base_rev` is gitweb's `$hash` (`None` for HEAD); `now`
/// is the request-time epoch the date cells are measured against; `page` is the
/// display window (its `limit` is the page size).
///
/// # Errors
///
/// [`DomainError::Forbidden`] when search is disabled; [`DomainError::Invalid`]
/// when the pattern is a malformed regular expression;
/// [`DomainError::NotFound`] ("Unknown commit object") when the base revision
/// names no commit. Any other repository failure propagates.
pub fn assemble_search(
    repo: &dyn Repository,
    enabled: bool,
    base_rev: Option<&str>,
    criteria: &SearchCriteria,
    now: i64,
    page: Page,
) -> Result<SearchView, DomainError> {
    if !enabled {
        return Err(DomainError::Forbidden("Search is disabled".to_owned()));
    }
    let pattern: SearchPattern = SearchPattern::new(&criteria.text, criteria.use_regexp)?;
    let base: ObjectId = match base_rev {
        Some(rev) => repo.resolve(rev).map_err(unknown_commit)?,
        None => repo.head().map_err(unknown_commit)?.target().clone(),
    };
    let base_commit: Commit = repo.find_commit(&base).map_err(unknown_commit)?;

    // gitweb fetches 101 at skip 100*page; the surplus over the page size reveals
    // a further page without a second round trip.
    let query: SearchQuery = SearchQuery {
        kind: criteria.kind,
        pattern: pattern.clone(),
    };
    let probe: Page = Page::new(page.skip, page.limit + 1);
    let mut commits: Vec<Commit> = repo.search(&base, &query, probe)?;
    let has_more: bool = commits.len() > page.limit;
    commits.truncate(page.limit);

    let rows: Vec<SearchRow> = commits
        .iter()
        .map(|commit: &Commit| row_of(commit, &pattern, now))
        .collect();
    Ok(SearchView {
        title: base_commit.title(),
        base_id: base.as_str().to_owned(),
        rows,
        has_more,
    })
}

/// Maps one matching commit into a result row: its id, the date cell, the author
/// (full and chopped to gitweb's 15 characters), the subject, and the highlighted
/// snippets of every message line that matches the pattern.
fn row_of(commit: &Commit, pattern: &SearchPattern, now: i64) -> SearchRow {
    let author: String = commit.author().name().to_owned();
    let author_short: String = chop_str(&author, AUTHOR_LEN, AUTHOR_SLACK, ChopMode::Right);
    let snippets: Vec<MatchSnippet> = commit
        .message()
        .lines()
        .filter_map(|line: &str| highlight_line(line, pattern))
        .collect();
    SearchRow {
        id: commit.id().as_str().to_owned(),
        date: CommitDate::new(commit.committer().epoch(), now),
        author_short,
        author,
        title: commit.title(),
        title_short: commit.title_short(),
        snippets,
    }
}

/// Reduces a base-resolution failure to gitweb's 404 "Unknown commit object" —
/// `parse_commit($hash)` returning empty for a missing ref or a non-commit object
/// — while letting a genuine backend error propagate unchanged.
fn unknown_commit(error: DomainError) -> DomainError {
    match error {
        DomainError::NotFound(_) | DomainError::Invalid(_) => {
            DomainError::NotFound("Unknown commit object".to_owned())
        }
        other => other,
    }
}
