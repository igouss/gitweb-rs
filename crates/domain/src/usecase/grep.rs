//! The grep-search use case — gitweb's `git_search` dispatch to
//! `git_search_files` (the `grep` facet).
//!
//! gitweb's `git_search` gates on the `search` feature (403 when off) and then on
//! `grep` (403 "Grep search is disabled"), validates the pattern while parsing the
//! request (400 on a malformed regexp), resolves the base revision (`$hash`,
//! defaulting to `git_get_head_hash`, 404 "Unknown commit object" when it names no
//! commit), and `git_search_files` greps that revision's tree (`git grep -n -z`,
//! `-F` or `-E -i`) listing — grouped per file in tree order — each matching line
//! with its matched span highlighted (untabified, the whole regexp matched
//! case-insensitively for display), each file linking to its blob and each line to
//! `blob#lN`; a binary match shows "Binary file"; the listing is capped, with a
//! "listing trimmed" notice when it overflows.
//!
//! This Control orchestrates that over the [`Repository`] port. The two matchers
//! are deliberately separate: a [`GrepPattern`] does the matching (fixed mode is
//! case-sensitive), while the always-case-insensitive [`SearchPattern`] drives the
//! display highlight — exactly gitweb's `-F`/`-E -i` versus `m/…/i` split. The
//! matching, rooting, and 1000-match cap are the adapter's job; here we gate,
//! validate, resolve the base, group the matches per file, and untabify and
//! highlight each line.

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::grep::{GrepMatch, GrepResults};
use crate::model::grep_pattern::GrepPattern;
use crate::model::object_id::ObjectId;
use crate::model::search_pattern::SearchPattern;
use crate::model::search_snippet::{MatchSnippet, highlight_full};
use crate::model::untabify::untabify;
use crate::port::repository::Repository;

/// One matched line's display text: either split around the highlighted span
/// (gitweb's `m/^(.*)(…)(.*)$/i` matched) or shown whole when the highlight does
/// not match (gitweb's else branch). Both are already untabified.
#[derive(Debug, Clone)]
pub enum GrepLine {
    /// The line split into lead / matched span / trail.
    Highlighted(MatchSnippet),
    /// The whole untabified line, with no span marked.
    Plain(String),
}

/// One row under a file: a matching line (with its 1-based number) or the binary
/// marker git-grep prints for a binary file (`Binary file <path> matches`).
#[derive(Debug, Clone)]
pub enum GrepRow {
    /// A binary file whose bytes matched: shown as "Binary file", no line text.
    Binary,
    /// A matching text line: its 1-based number and its display text.
    Line {
        /// The 1-based line number, linking to `blob#lN`.
        line_no: usize,
        /// The untabified, optionally-highlighted line text.
        line: GrepLine,
    },
}

/// The matches for one file: its path (relative to the searched tree, linking to
/// its blob) and the rows under it in line order.
#[derive(Debug, Clone)]
pub struct GrepFileView {
    path: String,
    rows: Vec<GrepRow>,
}

impl GrepFileView {
    /// The file path, relative to the searched tree.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The matching rows under this file, in line order.
    #[must_use]
    pub fn rows(&self) -> &[GrepRow] {
        &self.rows
    }
}

/// The assembled grep results: the base commit's subject (the page header), the
/// base id the search rooted at (gitweb's `$co{'id'}`, the `hash_base` of every
/// blob link), the matching files in tree order, and whether the cap trimmed the
/// listing.
#[derive(Debug, Clone)]
pub struct GrepView {
    title: String,
    base_id: String,
    files: Vec<GrepFileView>,
    trimmed: bool,
}

impl GrepView {
    /// The base commit's subject, shown as the results header.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The base commit id the search rooted at (full hex), the `hash_base` of
    /// every blob link.
    #[must_use]
    pub fn base_id(&self) -> &str {
        &self.base_id
    }

    /// The matching files, grouped in tree order.
    #[must_use]
    pub fn files(&self) -> &[GrepFileView] {
        &self.files
    }

    /// Whether the match cap dropped further matches, so the view shows gitweb's
    /// "Too many matches, listing trimmed" notice.
    #[must_use]
    pub fn trimmed(&self) -> bool {
        self.trimmed
    }
}

/// Assembles the grep results over the [`Repository`] port. `search_enabled` and
/// `grep_enabled` are the two feature gates (checked in gitweb's order);
/// `base_rev` is gitweb's `$hash` (`None` for HEAD); `text`/`use_regexp` are the
/// pattern and the `re`-box flag.
///
/// # Errors
///
/// [`DomainError::Forbidden`] when search is disabled ("Search is disabled") or
/// grep is disabled ("Grep search is disabled"); [`DomainError::Invalid`] when the
/// pattern is a malformed regular expression; [`DomainError::NotFound`] ("Unknown
/// commit object") when the base revision names no commit. Any other repository
/// failure propagates.
pub fn assemble_grep(
    repo: &dyn Repository,
    search_enabled: bool,
    grep_enabled: bool,
    base_rev: Option<&str>,
    text: &str,
    use_regexp: bool,
) -> Result<GrepView, DomainError> {
    if !search_enabled {
        return Err(DomainError::Forbidden("Search is disabled".to_owned()));
    }
    if !grep_enabled {
        return Err(DomainError::Forbidden("Grep search is disabled".to_owned()));
    }
    // Two matchers, as in gitweb: the grep itself (`-F`/`-E -i`) and the
    // always-case-insensitive display highlight (`m/…/i`). Both validate the
    // regexp, so a malformed pattern is the 400 before any repository access.
    let matcher: GrepPattern = GrepPattern::new(text, use_regexp)?;
    let highlighter: SearchPattern = SearchPattern::new(text, use_regexp)?;

    let base: ObjectId = match base_rev {
        Some(rev) => repo.resolve(rev).map_err(unknown_commit)?,
        None => repo.head().map_err(unknown_commit)?.target().clone(),
    };
    let base_commit: Commit = repo.find_commit(&base).map_err(unknown_commit)?;

    let results: GrepResults = repo.grep(&base, &matcher)?;
    Ok(GrepView {
        title: base_commit.title(),
        base_id: base.as_str().to_owned(),
        files: group_files(results.matches(), &highlighter),
        trimmed: results.trimmed(),
    })
}

/// Groups the flat, file-ordered match list into per-file views, untabifying and
/// highlighting each line. Matches arrive grouped by file in tree order, so a run
/// of matches sharing a path becomes one [`GrepFileView`].
fn group_files(matches: &[GrepMatch], highlighter: &SearchPattern) -> Vec<GrepFileView> {
    let mut files: Vec<GrepFileView> = Vec::new();
    for grep_match in matches {
        let row: GrepRow = row_of(grep_match, highlighter);
        match files.last_mut() {
            Some(file) if file.path == grep_match.path() => file.rows.push(row),
            _ => files.push(GrepFileView {
                path: grep_match.path().to_owned(),
                rows: vec![row],
            }),
        }
    }
    files
}

/// Maps one match to its row: a binary file's marker, or a line untabified and
/// split around its highlighted span (whole when the highlight does not match,
/// gitweb's else branch).
fn row_of(grep_match: &GrepMatch, highlighter: &SearchPattern) -> GrepRow {
    match (grep_match.line_no(), grep_match.text()) {
        (Some(line_no), Some(text)) => {
            let expanded: String = untabify(text);
            let line: GrepLine = match highlight_full(&expanded, highlighter) {
                Some(snippet) => GrepLine::Highlighted(snippet),
                None => GrepLine::Plain(expanded),
            };
            GrepRow::Line { line_no, line }
        }
        _ => GrepRow::Binary,
    }
}

/// Reduces a base-resolution failure to gitweb's 404 "Unknown commit object" —
/// `parse_commit($hash)` returning empty for a missing ref or a non-commit
/// object — while letting a genuine backend error propagate unchanged.
fn unknown_commit(error: DomainError) -> DomainError {
    match error {
        DomainError::NotFound(_) | DomainError::Invalid(_) => {
            DomainError::NotFound("Unknown commit object".to_owned())
        }
        other => other,
    }
}
