//! Content-grep hits: the file/line matches `git grep` lists at one revision.
//!
//! Mirrors gitweb's `git_search_files` (`git grep -n -z -F <pattern> <tree>`):
//! a literal, case-sensitive content search over the regular files of one tree.
//! A text file yields one [`GrepMatch::Line`] per line that contains the
//! pattern, carrying the line's 1-based number and its raw text; a binary file
//! whose bytes contain the pattern yields a single [`GrepMatch::Binary`] naming
//! the file with no line text, exactly as `git grep` prints
//! `Binary file <path> matches`.
//!
//! Highlighting the matched substring and expanding tabs are *view* concerns, so
//! a line carries its text verbatim (only decoded to UTF-8 via the fallback
//! encoding) and lets the boundary decide presentation. The per-file matching is
//! the business rule and lives here; the tree walk and blob reads are the
//! adapter's, which delegates each file to [`file_matches`] and caps the whole
//! result at [`GREP_MATCH_LIMIT`].

use crate::model::binary::is_binary;
use crate::model::encoding::{FallbackEncoding, to_utf8};

/// gitweb's cap on the number of grep matches it lists before printing its
/// "Too many matches, listing trimmed" notice (`git_search_files`).
pub const GREP_MATCH_LIMIT: usize = 1000;

/// One content-search hit, in the two shapes `git grep -n` prints.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum GrepMatch {
    /// A matching line in a text file: the file path, the 1-based line number,
    /// and the line's text (without its terminator, decoded to UTF-8).
    Line {
        /// The matched file's path, relative to the searched tree.
        path: String,
        /// The 1-based number of the matching line.
        line_no: usize,
        /// The matching line's text, verbatim apart from UTF-8 decoding.
        text: String,
    },
    /// A binary file whose bytes contain the pattern: reported by path with no
    /// line text, as `git grep` does.
    Binary {
        /// The matched file's path, relative to the searched tree.
        path: String,
    },
}

impl GrepMatch {
    /// A matching line in a text file.
    #[must_use]
    pub fn line(path: String, line_no: usize, text: String) -> Self {
        Self::Line {
            path,
            line_no,
            text,
        }
    }

    /// A binary file whose bytes contain the pattern.
    #[must_use]
    pub fn binary(path: String) -> Self {
        Self::Binary { path }
    }

    /// The path of the matched file, common to both kinds.
    #[must_use]
    pub fn path(&self) -> &str {
        match self {
            Self::Line { path, .. } | Self::Binary { path } => path,
        }
    }

    /// The 1-based line number of a line hit; `None` for a binary-file hit.
    #[must_use]
    pub fn line_no(&self) -> Option<usize> {
        match self {
            Self::Line { line_no, .. } => Some(*line_no),
            Self::Binary { .. } => None,
        }
    }

    /// The matching line's text; `None` for a binary-file hit.
    #[must_use]
    pub fn text(&self) -> Option<&str> {
        match self {
            Self::Line { text, .. } => Some(text),
            Self::Binary { .. } => None,
        }
    }

    /// Whether this hit is a binary file rather than a text line.
    #[must_use]
    pub fn is_binary(&self) -> bool {
        matches!(self, Self::Binary { .. })
    }
}

/// The matches for a content search over one tree, capped at
/// [`GREP_MATCH_LIMIT`].
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct GrepResults {
    matches: Vec<GrepMatch>,
    trimmed: bool,
}

impl GrepResults {
    /// Builds a result set from its (already-capped) matches and whether the
    /// cap trimmed any further matches.
    #[must_use]
    pub fn new(matches: Vec<GrepMatch>, trimmed: bool) -> Self {
        Self { matches, trimmed }
    }

    /// The matches, grouped by file in tree order, each file's lines in order.
    #[must_use]
    pub fn matches(&self) -> &[GrepMatch] {
        &self.matches
    }

    /// Whether the [`GREP_MATCH_LIMIT`] cap dropped further matches, so the view
    /// should show gitweb's "listing trimmed" notice.
    #[must_use]
    pub fn trimmed(&self) -> bool {
        self.trimmed
    }
}

/// The grep matches for a single file's bytes, given a literal `pattern` — the
/// pure per-file rule gitweb's `git grep -n` applies to each tracked file.
///
/// An empty pattern matches nothing (there is nothing to find). A binary file
/// (a NUL in its first bytes, [`is_binary`]) yields a single
/// [`GrepMatch::Binary`] iff its bytes contain the pattern, with no line text.
/// A text file yields one [`GrepMatch::Line`] per line that contains the
/// pattern, numbered from 1; the match is a case-sensitive byte substring (git's
/// `-F` without `-i`), so a line is listed at most once however many times the
/// pattern occurs in it.
#[must_use]
pub fn file_matches(path: &str, content: &[u8], pattern: &str) -> Vec<GrepMatch> {
    let needle: &[u8] = pattern.as_bytes();
    if needle.is_empty() {
        return Vec::new();
    }
    if is_binary(content) {
        return if contains(content, needle) {
            vec![GrepMatch::binary(path.to_owned())]
        } else {
            Vec::new()
        };
    }
    // `split_inclusive` yields one item per `\n`-terminated line plus a final
    // unterminated line, and nothing for empty content — exactly git's line
    // model. The terminator is stripped before matching and storing the text.
    content
        .split_inclusive(|&byte: &u8| byte == b'\n')
        .map(strip_newline)
        .enumerate()
        .filter(|(_, line): &(usize, &[u8])| contains(line, needle))
        .map(|(index, line): (usize, &[u8])| {
            GrepMatch::line(
                path.to_owned(),
                index + 1,
                to_utf8(line, FallbackEncoding::default()),
            )
        })
        .collect()
}

/// Drops a single trailing `\n` from a line, leaving any `\r` (git does not
/// strip carriage returns) and the rest of the bytes intact.
fn strip_newline(line: &[u8]) -> &[u8] {
    line.strip_suffix(b"\n").unwrap_or(line)
}

/// Whether `needle` occurs in `haystack` as a literal byte substring — git's
/// `-F` (fixed strings) without `-i`, so the test is case-sensitive. `needle` is
/// always non-empty here, so the window size is valid.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window: &[u8]| window == needle)
}
