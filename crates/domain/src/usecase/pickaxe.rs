//! The pickaxe-search use case — gitweb's `git_search` dispatch to
//! `git_search_changes` (the `pickaxe` facet).
//!
//! gitweb's `git_search` gates on the `search` feature (403 when off) and then on
//! `pickaxe` (403 "Pickaxe search is disabled"), validates the pattern while
//! parsing the request (400 on a malformed regexp), resolves the base revision
//! (`$hash`, defaulting to `git_get_head_hash`, 404 "Unknown commit object" when it
//! names no commit), and `git_search_changes` runs `git log -S<text> --raw` from
//! there: every commit that changed the pattern's occurrence count in some file,
//! listed newest-first, and under each — without `--pickaxe-all` — only the files
//! whose count changed. Each commit shows its date, author, and subject (linking to
//! the commit), with its changed files linking to their blobs; gitweb skips a
//! deleted file (`next if is_deleted`), having no blob to link. Unlike message
//! search, pickaxe is unpaged (`git log -S` has no count limit).
//!
//! This Control orchestrates that over the [`Repository`] port. The counting,
//! rooting, and per-commit file set are the adapter's job; here we gate, validate,
//! resolve the base, and map each [`PickaxeMatch`] into a [`PickaxeRow`] — the date
//! cell through [`CommitDate`]'s two-week swap, the author chopped to gitweb's 15
//! characters, the subject through [`Commit::title_short`], and the changed files
//! reduced to the ones that still have a blob (gitweb's deletion skip). The clock
//! is injected (`now`) so the view-model stays time-free.

use crate::error::DomainError;
use crate::model::chop::{ChopMode, chop_str};
use crate::model::commit::Commit;
use crate::model::commit_date::CommitDate;
use crate::model::object_id::ObjectId;
use crate::model::pickaxe::PickaxeMatch;
use crate::model::pickaxe_pattern::PickaxePattern;
use crate::port::repository::Repository;

/// gitweb's `chop_and_escape_str($co{'author_name'}, 15, 5)` — the pickaxe row's
/// author bound (the same width as the message search).
const AUTHOR_LEN: usize = 15;
/// The author chop's word slack.
const AUTHOR_SLACK: usize = 5;

/// One changed file under a matching commit: its path (gitweb's `to_file`) and the
/// id of its post-change blob (gitweb's `to_id`), the blob the path links to.
/// Deletions never reach here — they have no blob to link, so the assembly drops
/// them (gitweb's `next if is_deleted`).
#[derive(Debug, Clone)]
pub struct PickaxeFile {
    path: String,
    blob: String,
}

impl PickaxeFile {
    /// The file's path, relative to the repository root.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The id (full hex) of the file's post-change blob, the `hash` of its link.
    #[must_use]
    pub fn blob(&self) -> &str {
        &self.blob
    }
}

/// One matching commit on the pickaxe page: its id and tree id (for the commit and
/// tree links), the date cell, the author (full and chopped for the row), the
/// subject (full and short), and the changed files that still have a blob to link.
#[derive(Debug, Clone)]
pub struct PickaxeRow {
    id: String,
    tree_id: String,
    date: CommitDate,
    author: String,
    author_short: String,
    title: String,
    title_short: String,
    files: Vec<PickaxeFile>,
}

impl PickaxeRow {
    /// The commit object id (full hex): the `hash` of the commit link and the
    /// `hash_base` of every changed file's blob link.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The commit's tree id (full hex): the `hash` of the tree link (gitweb's
    /// `href(action=>"tree", hash=>$co{'tree'}, hash_base=>$co{'id'})`).
    #[must_use]
    pub fn tree_id(&self) -> &str {
        &self.tree_id
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

    /// The short commit subject shown in the row (chopped to 50).
    #[must_use]
    pub fn title_short(&self) -> &str {
        &self.title_short
    }

    /// The changed files under this commit (deletions already dropped), in the
    /// diff's path order.
    #[must_use]
    pub fn files(&self) -> &[PickaxeFile] {
        &self.files
    }
}

/// The assembled pickaxe results: the base commit's subject for the page header,
/// the base id the search rooted at, and the matching rows newest-first. There is
/// no paging — gitweb lists every match — so there is no further-page flag.
#[derive(Debug, Clone)]
pub struct PickaxeView {
    title: String,
    base_id: String,
    rows: Vec<PickaxeRow>,
}

impl PickaxeView {
    /// The base commit's subject (the results-page header title).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The base commit id the search rooted at (full hex), the way gitweb's page
    /// navigation carries `$hash`.
    #[must_use]
    pub fn base_id(&self) -> &str {
        &self.base_id
    }

    /// The matching commit rows, newest first.
    #[must_use]
    pub fn rows(&self) -> &[PickaxeRow] {
        &self.rows
    }
}

/// Assembles the pickaxe results over the [`Repository`] port. `search_enabled` and
/// `pickaxe_enabled` are the two feature gates (checked in gitweb's order);
/// `base_rev` is gitweb's `$hash` (`None` for HEAD); `text`/`use_regexp` are the
/// pattern and the `re`-box flag; `now` is the request-time epoch the date cells
/// are measured against.
///
/// # Errors
///
/// [`DomainError::Forbidden`] when search is disabled ("Search is disabled") or
/// pickaxe is disabled ("Pickaxe search is disabled"); [`DomainError::Invalid`]
/// when the pattern is a malformed regular expression; [`DomainError::NotFound`]
/// ("Unknown commit object") when the base revision names no commit. Any other
/// repository failure propagates.
pub fn assemble_pickaxe(
    repo: &dyn Repository,
    search_enabled: bool,
    pickaxe_enabled: bool,
    base_rev: Option<&str>,
    text: &str,
    use_regexp: bool,
    now: i64,
) -> Result<PickaxeView, DomainError> {
    if !search_enabled {
        return Err(DomainError::Forbidden("Search is disabled".to_owned()));
    }
    if !pickaxe_enabled {
        return Err(DomainError::Forbidden(
            "Pickaxe search is disabled".to_owned(),
        ));
    }
    // The pattern validates here, so a malformed regexp is the 400 before any
    // repository access — gitweb's order.
    let pattern: PickaxePattern = PickaxePattern::new(text, use_regexp)?;

    let base: ObjectId = match base_rev {
        Some(rev) => repo.resolve(rev).map_err(unknown_commit)?,
        None => repo.head().map_err(unknown_commit)?.target().clone(),
    };
    let base_commit: Commit = repo.find_commit(&base).map_err(unknown_commit)?;

    let matches: Vec<PickaxeMatch> = repo.pickaxe(&base, &pattern)?;
    let rows: Vec<PickaxeRow> = matches
        .iter()
        .map(|pickaxe_match: &PickaxeMatch| row_of(pickaxe_match, now))
        .collect();
    Ok(PickaxeView {
        title: base_commit.title(),
        base_id: base.as_str().to_owned(),
        rows,
    })
}

/// Maps one match into a result row: the commit's links, date, author (chopped to
/// gitweb's 15 characters), subject, and the changed files that still have a blob.
/// A deleted file (`to_blob` is `None`) is dropped here — gitweb's `next if
/// is_deleted` — so the same `filter_map` both skips it and unwraps the blob of
/// every file that survives.
fn row_of(pickaxe_match: &PickaxeMatch, now: i64) -> PickaxeRow {
    let commit: &Commit = pickaxe_match.commit();
    let author: String = commit.author().name().to_owned();
    let author_short: String = chop_str(&author, AUTHOR_LEN, AUTHOR_SLACK, ChopMode::Right);
    let files: Vec<PickaxeFile> = pickaxe_match
        .changes()
        .iter()
        .filter_map(|change| {
            change.to_blob().map(|blob: &ObjectId| PickaxeFile {
                path: change.path().to_owned(),
                blob: blob.as_str().to_owned(),
            })
        })
        .collect();
    PickaxeRow {
        id: commit.id().as_str().to_owned(),
        tree_id: commit.tree().as_str().to_owned(),
        date: CommitDate::new(commit.committer().epoch(), now),
        author_short,
        author,
        title: commit.title(),
        title_short: commit.title_short(),
        files,
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
