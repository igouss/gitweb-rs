//! The shared history walk behind the log-shaped actions — gitweb's
//! `git_log_generic`.
//!
//! gitweb factors `shortlog`, `log` and `history` through one driver,
//! `git_log_generic`, which resolves the base revision (HEAD by default), walks
//! `parse_commits($base, 101, 100*$page)` — one more than the page holds — and
//! hands the commit list plus a "is there a next page" flag to a body renderer.
//! This is that driver, minus the HTML: [`walk_commits`] resolves the base, asks
//! the [`Repository`] port for `limit + 1` commits at the requested `skip`, keeps
//! `limit` of them, and reports the surplus as [`CommitWindow::has_more`]. The
//! `shortlog` and `log` use cases each map the resulting commits into their own
//! row type; the clock and the per-row formatting stay with them.
//!
//! An unborn repository (no HEAD) has no history and yields an empty window
//! rather than an error, matching gitweb's empty `parse_commits` on an undefined
//! base.

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::object_id::ObjectId;
use crate::port::repository::{Page, Repository};

/// One page of history: the commits to show and whether a further page exists.
pub(crate) struct CommitWindow {
    /// The commits for this page, newest first, already truncated to the page
    /// size.
    pub(crate) commits: Vec<Commit>,
    /// Whether a further page exists (gitweb's `$#commitlist >= 100`).
    pub(crate) has_more: bool,
}

/// Walks one page of history from `rev` (or HEAD when `None`), viewed through the
/// [`Repository`] port and optionally limited to commits that touch `path`
/// (gitweb's per-path `history` action; `None` is the unfiltered log/shortlog
/// walk). `page`'s `limit` is the page size; the walk asks for one more so the
/// surplus reveals a further page without a second round trip.
///
/// # Errors
///
/// Propagates the repository's error if resolving an explicit revision or reading
/// history fails. An unborn repository whose HEAD names no commit is not an
/// error — it yields an empty window.
pub(crate) fn walk_commits(
    repo: &dyn Repository,
    rev: Option<&str>,
    path: Option<&str>,
    page: Page,
) -> Result<CommitWindow, DomainError> {
    let start: ObjectId = match resolve_start(repo, rev)? {
        Some(oid) => oid,
        None => {
            return Ok(CommitWindow {
                commits: Vec::new(),
                has_more: false,
            });
        }
    };
    let probe: Page = Page::new(page.skip, page.limit + 1);
    let mut commits: Vec<Commit> = repo.history(&start, path, probe)?;
    let has_more: bool = commits.len() > page.limit;
    commits.truncate(page.limit);
    Ok(CommitWindow { commits, has_more })
}

/// Resolves the base revision the walk starts from: an explicit revision through
/// the port's resolver, or HEAD's target. An unborn HEAD yields `None` (gitweb's
/// empty `parse_commits` on an undefined base); any other failure propagates.
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
