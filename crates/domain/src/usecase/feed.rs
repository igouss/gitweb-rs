//! The `feed` use case: gitweb's `git_feed`, minus the XML and the links.
//!
//! Reads the recent log of a branch (HEAD by default), optionally narrowed to
//! one file's history, and assembles the structured [`Feed`] the RSS/Atom
//! serializers render. It mirrors `git_feed`'s shape:
//!
//!   1. read up to [`FEED_READ_LIMIT`] commits from the base
//!      (`parse_commits($head, 150, 0, $file_name)`);
//!   2. apply [`feed_window`] — keep the newest 20 plus anything still within
//!      48 hours, against the injected `now`;
//!   3. for each kept commit, diff it against its first parent (the empty tree
//!      for a root commit, gitweb's `$co{'parent'} || "--root"`) with rename
//!      detection at gitweb's default `-M`, narrowing to the feed's file when one
//!      is named (gitweb's `diff-tree … -- $file_name`).
//!
//! The clock is injected so the window is computed here and the domain stays
//! clock-free. An unborn repository (no HEAD) yields an empty feed, not an error.

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::diff::{Diff, DiffEntry};
use crate::model::feed::{FEED_READ_LIMIT, Feed, FeedEntry, FeedFile, comment_lines, feed_window};
use crate::model::object_id::ObjectId;
use crate::model::signature::Signature;
use crate::model::timestamp::Timestamp;
use crate::port::repository::{Page, RenameDetection, Repository};
use crate::usecase::log_generic::resolve_start;

/// Assembles the feed over the [`Repository`] port. `rev` is the base revision
/// (`None` for HEAD); `path` narrows the feed to one file's history (`None` for
/// the whole branch); `now` is the request-time epoch the 48-hour window is
/// measured against.
///
/// # Errors
///
/// Propagates the repository's error if resolving the base, reading history, or
/// diffing a commit fails. An unborn repository whose HEAD names no commit is not
/// an error — it yields an empty feed.
pub fn assemble_feed(
    repo: &dyn Repository,
    rev: Option<&str>,
    path: Option<&str>,
    now: i64,
) -> Result<Feed, DomainError> {
    let start: ObjectId = match resolve_start(repo, rev)? {
        Some(oid) => oid,
        None => return Ok(Feed::default()),
    };
    let commits: Vec<Commit> = repo.history(&start, path, Page::new(0, FEED_READ_LIMIT))?;
    let epochs: Vec<i64> = commits
        .iter()
        .map(|commit: &Commit| commit.committer().epoch())
        .collect();
    let keep: usize = feed_window(&epochs, now);
    let latest: Option<Timestamp> = commits
        .first()
        .map(|commit: &Commit| Timestamp::from_signature(commit.committer()));

    let mut entries: Vec<FeedEntry> = Vec::with_capacity(keep);
    for commit in commits.iter().take(keep) {
        entries.push(entry_of(repo, commit, path)?);
    }
    Ok(Feed::new(entries, latest))
}

/// Builds one feed entry: the commit's metadata plus the files it changed against
/// its first parent (the empty tree for a root commit), narrowed to `path`.
fn entry_of(
    repo: &dyn Repository,
    commit: &Commit,
    path: Option<&str>,
) -> Result<FeedEntry, DomainError> {
    let parent: Option<&ObjectId> = commit.parents().first();
    let diff: Diff = repo.diff(parent, commit.id(), RenameDetection::RenamesOnly)?;
    let files: Vec<FeedFile> = diff
        .entries()
        .iter()
        .filter(|entry: &&DiffEntry| matches_path(entry, path))
        .map(file_of)
        .collect();

    let author: &Signature = commit.author();
    let committer: &Signature = commit.committer();
    Ok(FeedEntry::new(
        commit.id().as_str().to_owned(),
        parent.map(|oid: &ObjectId| oid.as_str().to_owned()),
        commit.title(),
        author.full_ident(),
        author.name().to_owned(),
        author.email().map(str::to_owned),
        committer.name().to_owned(),
        committer.email().map(str::to_owned),
        Timestamp::from_signature(committer),
        comment_lines(commit.message())
            .into_iter()
            .map(str::to_owned)
            .collect(),
        files,
    ))
}

/// Maps a diff entry to the feed's changed-path record.
fn file_of(entry: &DiffEntry) -> FeedFile {
    FeedFile::new(
        entry.from_oid().as_str().to_owned(),
        entry.to_oid().as_str().to_owned(),
        entry.from_path().to_owned(),
        entry.to_path().to_owned(),
    )
}

/// Whether a diff entry is in scope for the feed: every entry for a whole-branch
/// feed, or only those touching `path` (the file itself or anything under it,
/// gitweb's `diff-tree … -- $file_name` pathspec) for a file-history feed.
fn matches_path(entry: &DiffEntry, path: Option<&str>) -> bool {
    match path {
        None => true,
        Some(path) => under(entry.to_path(), path) || under(entry.from_path(), path),
    }
}

/// Whether `candidate` is `path` itself or lives beneath it (`path/...`).
fn under(candidate: &str, path: &str) -> bool {
    candidate == path || candidate.starts_with(&format!("{path}/"))
}
