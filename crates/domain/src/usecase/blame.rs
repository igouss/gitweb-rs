//! The `blame` use case: gitweb's `git_blame_common` over `git blame -p`, minus
//! the HTML and the streaming.
//!
//! gitweb attributes every line of a file, as of a base commit, to the commit
//! that last touched it. The use case resolves the base revision (HEAD by
//! default, gitweb's `$hash_base`), reads that commit so the page can show its
//! subject — a missing one is gitweb's `die_error(404, "Commit not found")` — and
//! runs the repository's [`Blame`] over the path (a missing path is the port's
//! `NotFound`, gitweb's `die_error(404, "Error looking up file")`).
//!
//! The per-line attribution is then folded into *groups*: maximal runs of
//! consecutive final lines sharing one commit. A group is gitweb's unit — the
//! `<td rowspan>` cell of the server-rendered table and one `git blame
//! --incremental` record (`<sha> <orig> <final> <numlines>` then the commit's
//! `author` / `author-time` / `author-tz` / `filename`). The introducing commit's
//! author and date are read once per distinct commit (gitweb's `%metainfo`
//! cache), then shared by every group of that commit.
//!
//! The same view serves all three actions: the server-rendered `blame` table and
//! the `blame_incremental` shell both render the lines, and `blame_data` streams
//! the groups. The view is framework-free; the web boundary builds the links and
//! the render layer the markup or the text.

use std::collections::BTreeMap;

use crate::error::DomainError;
use crate::model::blame::{Blame, BlameLine};
use crate::model::object_id::ObjectId;
use crate::model::timestamp::Timestamp;
use crate::port::repository::Repository;

/// gitweb's `substr($full_rev, 0, 8)` — the blame view abbreviates a commit id to
/// a fixed 8 hex characters, not the repository's variable `--abbrev`.
const SHORT_ID_LEN: usize = 8;

/// One line of the blamed file: the commit that introduced it, its line number in
/// that commit (the original line number) and in the final file, and its text.
/// Mirrors a single [`BlameLine`], carried through so the table and the
/// incremental shell can render the file's contents.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameRow {
    commit: String,
    orig_lineno: usize,
    final_lineno: usize,
    content: String,
}

impl BlameRow {
    /// The full id of the commit that last touched this line.
    #[must_use]
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// The line's number in the commit that introduced it (gitweb's
    /// `$orig_lineno`).
    #[must_use]
    pub fn orig_lineno(&self) -> usize {
        self.orig_lineno
    }

    /// The line's number in the final file (gitweb's `$lineno`), the `id="l<n>"`
    /// row the incremental client fills.
    #[must_use]
    pub fn final_lineno(&self) -> usize {
        self.final_lineno
    }

    /// The line's text.
    #[must_use]
    pub fn content(&self) -> &str {
        &self.content
    }
}

/// A maximal run of consecutive final lines sharing one commit — gitweb's blame
/// group (one rowspan, one `--incremental` record). Carries the commit (full and
/// 8-char short id), its author and the author's local civil date (gitweb's
/// `iso-tz`) plus the raw epoch and zone the incremental stream emits, the
/// original and final line numbers of the group's first line, and the line count.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameGroup {
    commit: String,
    short_commit: String,
    author: String,
    author_time: i64,
    author_tz: String,
    date: String,
    orig_lineno: usize,
    final_lineno: usize,
    numlines: usize,
}

impl BlameGroup {
    /// The full id of the group's commit.
    #[must_use]
    pub fn commit(&self) -> &str {
        &self.commit
    }

    /// The 8-char short id gitweb shows in the `sha1` cell.
    #[must_use]
    pub fn short_commit(&self) -> &str {
        &self.short_commit
    }

    /// The commit author's display name.
    #[must_use]
    pub fn author(&self) -> &str {
        &self.author
    }

    /// The author time as a Unix epoch — the `author-time` value of the
    /// incremental record.
    #[must_use]
    pub fn author_time(&self) -> i64 {
        self.author_time
    }

    /// The author's timezone token (e.g. `"+0900"`) — the `author-tz` value of the
    /// incremental record.
    #[must_use]
    pub fn author_tz(&self) -> &str {
        &self.author_tz
    }

    /// The author's local civil date (gitweb's `iso-tz`), the `sha1` cell tooltip.
    #[must_use]
    pub fn date(&self) -> &str {
        &self.date
    }

    /// The original line number of the group's first line (its `<sha> <orig> …`
    /// field, and the line-number link anchor).
    #[must_use]
    pub fn orig_lineno(&self) -> usize {
        self.orig_lineno
    }

    /// The final line number of the group's first line (its `<sha> … <final> …`
    /// field).
    #[must_use]
    pub fn final_lineno(&self) -> usize {
        self.final_lineno
    }

    /// The number of lines in the group (its `<sha> … <numlines>` field and the
    /// cell's `rowspan`).
    #[must_use]
    pub fn numlines(&self) -> usize {
        self.numlines
    }
}

/// The assembled blame: the tracked path, the base commit's subject (the page
/// header), the groups (the incremental records / rowspan cells), and the lines
/// (the file's contents in final order).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameView {
    file_name: String,
    commit_title: String,
    groups: Vec<BlameGroup>,
    rows: Vec<BlameRow>,
}

impl BlameView {
    /// The tracked file (gitweb's `$file_name`).
    #[must_use]
    pub fn file_name(&self) -> &str {
        &self.file_name
    }

    /// The base commit's subject, shown in the page header (gitweb's
    /// `git_print_header_div('commit', $co{'title'})`).
    #[must_use]
    pub fn commit_title(&self) -> &str {
        &self.commit_title
    }

    /// The blame groups, in final-file order — the incremental records and the
    /// rowspan cells.
    #[must_use]
    pub fn groups(&self) -> &[BlameGroup] {
        &self.groups
    }

    /// The blamed lines, in final-file order — the file's contents.
    #[must_use]
    pub fn rows(&self) -> &[BlameRow] {
        &self.rows
    }
}

/// Assembles the blame of `path` as of `base_rev` (HEAD when `None`, gitweb's
/// `$hash_base ||= git_get_head_hash`) over the [`Repository`] port.
///
/// # Errors
///
/// Propagates the port's failure when the base revision does not resolve or its
/// commit is unreadable (gitweb's "Couldn't find base commit" / "Commit not
/// found"), when the path cannot be blamed (gitweb's "Error looking up file"), or
/// when an introducing commit cannot be read for its author and date.
pub fn assemble_blame(
    repo: &dyn Repository,
    base_rev: Option<&str>,
    path: &str,
) -> Result<BlameView, DomainError> {
    let base: ObjectId = repo.resolve(base_rev.unwrap_or("HEAD"))?;
    let commit_title: String = repo.find_commit(&base)?.title();
    let blame: Blame = repo.blame(&base, path)?;
    let groups: Vec<BlameGroup> = assemble_groups(repo, blame.lines())?;
    let rows: Vec<BlameRow> = blame.lines().iter().map(row_of).collect();
    Ok(BlameView {
        file_name: path.to_owned(),
        commit_title,
        groups,
        rows,
    })
}

/// Folds the per-line attribution into groups: a new group begins at the first
/// line and whenever the commit differs from the previous line's. Each group's
/// commit metadata (author, date) is read once per distinct commit and cached, so
/// a commit owning several groups is read only once — gitweb's `%metainfo`.
fn assemble_groups(
    repo: &dyn Repository,
    lines: &[BlameLine],
) -> Result<Vec<BlameGroup>, DomainError> {
    let mut cache: BTreeMap<String, CommitMeta> = BTreeMap::new();
    let mut groups: Vec<BlameGroup> = Vec::new();
    for line in lines {
        let commit: String = line.commit().as_str().to_owned();
        match groups.last_mut() {
            Some(last) if last.commit == commit => last.numlines += 1,
            _ => groups.push(new_group(repo, &mut cache, line)?),
        }
    }
    Ok(groups)
}

/// Starts a group at `line`, resolving (and caching) its commit's author and date.
fn new_group(
    repo: &dyn Repository,
    cache: &mut BTreeMap<String, CommitMeta>,
    line: &BlameLine,
) -> Result<BlameGroup, DomainError> {
    let commit: &ObjectId = line.commit();
    if !cache.contains_key(commit.as_str()) {
        let resolved: CommitMeta = commit_meta(repo, commit)?;
        cache.insert(commit.as_str().to_owned(), resolved);
    }
    let meta: &CommitMeta = cache.get(commit.as_str()).expect("inserted above");
    Ok(BlameGroup {
        commit: commit.as_str().to_owned(),
        short_commit: short_id(commit.as_str()),
        author: meta.author.clone(),
        author_time: meta.author_time,
        author_tz: meta.author_tz.clone(),
        date: meta.date.clone(),
        orig_lineno: line.orig_lineno(),
        final_lineno: line.final_lineno(),
        numlines: 1,
    })
}

/// The cached per-commit attribution: the author name, the author time and zone,
/// and the formatted local civil date (gitweb's `%metainfo` for one commit).
struct CommitMeta {
    author: String,
    author_time: i64,
    author_tz: String,
    date: String,
}

/// Reads a commit's author identity and folds it into the metadata the groups
/// share: the name, the epoch and zone the incremental stream emits, and the
/// `iso-tz` local date gitweb's `parse_date(...)` produces.
fn commit_meta(repo: &dyn Repository, commit: &ObjectId) -> Result<CommitMeta, DomainError> {
    let author = repo.find_commit(commit)?;
    let signature = author.author();
    let timestamp: Timestamp = Timestamp::from_signature(signature);
    Ok(CommitMeta {
        author: signature.name().to_owned(),
        author_time: signature.epoch(),
        author_tz: signature.timezone().to_owned(),
        date: timestamp.iso_tz(),
    })
}

/// Maps one [`BlameLine`] to a [`BlameRow`] (the file's content line).
fn row_of(line: &BlameLine) -> BlameRow {
    BlameRow {
        commit: line.commit().as_str().to_owned(),
        orig_lineno: line.orig_lineno(),
        final_lineno: line.final_lineno(),
        content: line.content().to_owned(),
    }
}

/// gitweb's `substr($full_rev, 0, 8)` — the first 8 hex characters of the id.
fn short_id(commit: &str) -> String {
    commit.chars().take(SHORT_ID_LEN).collect()
}
