//! Syndication feeds (RSS / Atom) — the data and pure rules of gitweb's
//! `git_feed`.
//!
//! gitweb serves one feed body from the recent log of a branch (HEAD by
//! default), optionally narrowed to one file's history. This module owns the
//! framework-free parts: the structured [`Feed`] the use case assembles, and the
//! three pure rules `git_feed` leans on —
//!
//!   * [`feed_title`]   — the feed's title and "feed type" wording, derived from
//!     whether a branch (`hash`) and/or a file were named;
//!   * [`feed_window`]  — which of the (up to 150) commits the feed actually
//!     shows: always the newest 20, plus any beyond that still within 48 hours;
//!   * [`comment_lines`] — a commit message as the feed's `<pre>` body: the body
//!     lines with a leading four-space indent stripped and trailing blank lines
//!     dropped, exactly as gitweb's `parse_commit_text` builds its `comment`.
//!
//! Serializing the [`Feed`] to RSS/Atom XML, and building the absolute links it
//! embeds, are the render and web layers' jobs; here we only model the content.

use crate::model::timestamp::Timestamp;

/// How many commits gitweb reads for a feed before windowing
/// (`parse_commits($head, 150, 0, $file_name)`).
pub const FEED_READ_LIMIT: usize = 150;

/// gitweb always shows at least this many of the read commits, regardless of age
/// (`$i >= 20` is the first index it may drop).
const FEED_ALWAYS_SHOWN: usize = 20;

/// Past the always-shown prefix, gitweb keeps a commit only while it is within
/// this window of "now" (`48*60*60` seconds).
const FEED_RECENT_WINDOW_SECONDS: i64 = 48 * 60 * 60;

/// Builds gitweb's feed title and trailing "feed type" word from the request:
/// `"$site_name - $project/$action"`, then the branch and/or file it is scoped
/// to, then the kind of feed it is (`log`, `branch log`, or `history`). This is
/// the raw (un-escaped) string; the render layer escapes it.
///
/// Mirrors `git_feed`'s title assembly exactly: a named `hash` makes it a branch
/// log (and, with a file too, a history); a file alone makes it a history.
#[must_use]
pub fn feed_title(
    site_name: &str,
    project: &str,
    action: &str,
    hash: Option<&str>,
    file_name: Option<&str>,
) -> String {
    let mut title: String = format!("{site_name} - {project}/{action}");
    let feed_type: &str = match (hash, file_name) {
        (Some(hash), Some(file)) => {
            title.push_str(&format!(" - '{hash}' :: {file}"));
            "history"
        }
        (Some(hash), None) => {
            title.push_str(&format!(" - '{hash}'"));
            "branch log"
        }
        (None, Some(file)) => {
            title.push_str(&format!(" - {file}"));
            "history"
        }
        (None, None) => "log",
    };
    format!("{title} {feed_type}")
}

/// How many of `committer_epochs` (newest first) the feed shows, applying
/// gitweb's window: always the first [`FEED_ALWAYS_SHOWN`], and any beyond that
/// whose age against `now` is within [`FEED_RECENT_WINDOW_SECONDS`]. The list is
/// reverse-chronological, so the first commit past the prefix that is too old
/// ends the feed (`last` in gitweb's loop), and everything after it is dropped.
#[must_use]
pub fn feed_window(committer_epochs: &[i64], now: i64) -> usize {
    for (index, &epoch) in committer_epochs.iter().enumerate() {
        if index >= FEED_ALWAYS_SHOWN && now - epoch > FEED_RECENT_WINDOW_SECONDS {
            return index;
        }
    }
    committer_epochs.len()
}

/// A commit message rendered as the feed's `<pre>` body lines: gitweb's
/// `parse_commit_text` `comment` array. Each body line has a leading four-space
/// indent stripped (`s/^    //`), and trailing blank lines are dropped (Perl's
/// `split '\n'` discards trailing empty fields). Leading and interior blank
/// lines are kept verbatim.
#[must_use]
pub fn comment_lines(message: &str) -> Vec<&str> {
    let mut lines: Vec<&str> = message
        .lines()
        .map(|line: &str| line.strip_prefix("    ").unwrap_or(line))
        .collect();
    while lines.last() == Some(&"") {
        lines.pop();
    }
    lines
}

/// One changed path in a feed entry: the raw diff facts gitweb reads from
/// `diff-tree` (`from_id`/`to_id`, `from_file`/`to_file`), from which the web
/// layer builds the entry's `blobdiff` / `blame` / `history` links.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedFile {
    from_oid: String,
    to_oid: String,
    from_path: String,
    to_path: String,
}

impl FeedFile {
    /// Assembles one changed-path record.
    #[must_use]
    pub fn new(from_oid: String, to_oid: String, from_path: String, to_path: String) -> Self {
        Self {
            from_oid,
            to_oid,
            from_path,
            to_path,
        }
    }

    /// The from-side object id (all-zero hex when the path is created).
    #[must_use]
    pub fn from_oid(&self) -> &str {
        &self.from_oid
    }

    /// The to-side object id (all-zero hex when the path is deleted).
    #[must_use]
    pub fn to_oid(&self) -> &str {
        &self.to_oid
    }

    /// The from-side path (differs from the to-side only on a rename/copy).
    #[must_use]
    pub fn from_path(&self) -> &str {
        &self.from_path
    }

    /// The to-side path — gitweb's `$file`, the one the entry lists.
    #[must_use]
    pub fn to_path(&self) -> &str {
        &self.to_path
    }
}

/// One feed entry: a commit and the files it changed (against its first parent).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FeedEntry {
    id: String,
    parent: Option<String>,
    title: String,
    author_full: String,
    author_name: String,
    author_email: Option<String>,
    committer_name: String,
    committer_email: Option<String>,
    timestamp: Timestamp,
    comment: Vec<String>,
    files: Vec<FeedFile>,
}

impl FeedEntry {
    /// Assembles one feed entry from its already-resolved parts.
    #[must_use]
    #[allow(clippy::too_many_arguments)]
    pub fn new(
        id: String,
        parent: Option<String>,
        title: String,
        author_full: String,
        author_name: String,
        author_email: Option<String>,
        committer_name: String,
        committer_email: Option<String>,
        timestamp: Timestamp,
        comment: Vec<String>,
        files: Vec<FeedFile>,
    ) -> Self {
        Self {
            id,
            parent,
            title,
            author_full,
            author_name,
            author_email,
            committer_name,
            committer_email,
            timestamp,
            comment,
            files,
        }
    }

    /// The commit's object id (full hex) — the entry's `commitdiff` target.
    #[must_use]
    pub fn id(&self) -> &str {
        &self.id
    }

    /// The first parent's id, or `None` for a root commit (gitweb's
    /// `$co{'parent'}`, used as the `blobdiff` `hash_parent_base`).
    #[must_use]
    pub fn parent(&self) -> Option<&str> {
        self.parent.as_deref()
    }

    /// The commit subject, chopped to 80 (gitweb's `$co{'title'}`).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The whole author ident (`name <email>`) — RSS's `<author>`.
    #[must_use]
    pub fn author_full(&self) -> &str {
        &self.author_full
    }

    /// The author's display name — Atom's `<author><name>`.
    #[must_use]
    pub fn author_name(&self) -> &str {
        &self.author_name
    }

    /// The author's email, if any — Atom's `<author><email>`.
    #[must_use]
    pub fn author_email(&self) -> Option<&str> {
        self.author_email.as_deref()
    }

    /// The committer's display name — Atom's `<contributor><name>`.
    #[must_use]
    pub fn committer_name(&self) -> &str {
        &self.committer_name
    }

    /// The committer's email, if any — Atom's `<contributor><email>`.
    #[must_use]
    pub fn committer_email(&self) -> Option<&str> {
        self.committer_email.as_deref()
    }

    /// The committer timestamp — the entry's `pubDate` (RSS) / `updated` (Atom).
    #[must_use]
    pub fn timestamp(&self) -> &Timestamp {
        &self.timestamp
    }

    /// The message body lines for the entry's `<pre>` block.
    #[must_use]
    pub fn comment(&self) -> &[String] {
        &self.comment
    }

    /// The files this commit changed, in diff order.
    #[must_use]
    pub fn files(&self) -> &[FeedFile] {
        &self.files
    }
}

/// An assembled feed: its entries (newest first) and the newest commit's
/// timestamp, which drives the channel/feed date and the `Last-Modified` header.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Feed {
    entries: Vec<FeedEntry>,
    latest: Option<Timestamp>,
}

impl Feed {
    /// Builds a feed from its entries and the newest commit's timestamp
    /// (`None` for an unborn/empty repository, which has no commits).
    #[must_use]
    pub fn new(entries: Vec<FeedEntry>, latest: Option<Timestamp>) -> Self {
        Self { entries, latest }
    }

    /// The feed entries, newest first.
    #[must_use]
    pub fn entries(&self) -> &[FeedEntry] {
        &self.entries
    }

    /// The newest commit's timestamp, or `None` when the feed has no commits —
    /// gitweb's `%latest_date`, the source of the channel date and the
    /// `Last-Modified` header.
    #[must_use]
    pub fn latest(&self) -> Option<&Timestamp> {
        self.latest.as_ref()
    }
}
