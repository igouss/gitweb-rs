//! The content-grep matcher — gitweb's `git_search_files` `search_use_regexp`
//! toggle.
//!
//! gitweb greps a tree with `git grep -n -z` plus either `-F` (the default: the
//! pattern is a literal string, matched **case-sensitively**) or `-E -i` (when
//! the user checks the *re* box: a POSIX extended regular expression, matched
//! **case-insensitively**). This value object captures exactly that one rule,
//! over raw bytes so the same matcher serves a text line and a whole binary file.
//!
//! It is deliberately a *different* type from
//! [`SearchPattern`](crate::model::search_pattern::SearchPattern), which the
//! commit/author/committer search uses: that matcher is always case-insensitive
//! (git's `-i` on every facet), whereas git-grep's fixed mode is
//! case-sensitive — the asymmetry gitweb's own help text warns about ("if regexp
//! mode is turned off, the matches are case-sensitive"). The result-row
//! *highlight*, by contrast, is always case-insensitive and so reuses
//! `SearchPattern`; matching and highlighting are two separate rules, as they are
//! in gitweb (`-F`/`-E -i` for the grep, `m/…/i` for the span).
//!
//! The regexp matcher is built byte-oriented and ASCII-only (`unicode(false)`),
//! mirroring git-grep in the C locale: `.` matches any byte but a newline, `-i`
//! folds ASCII case, and arbitrary (non-UTF-8) bytes match literally. A malformed
//! regular expression is rejected as [`DomainError::Invalid`], the way gitweb dies
//! 400 "Invalid search regexp" while validating the request.

use regex::bytes::{Regex, RegexBuilder};

use crate::error::DomainError;

/// A validated content-grep matcher in one of gitweb's two modes.
#[derive(Debug, Clone)]
pub struct GrepPattern {
    matcher: Matcher,
}

/// The two grep modes, each with its own case rule.
#[derive(Debug, Clone)]
enum Matcher {
    /// `git grep -F`: a literal byte substring, matched case-sensitively.
    Fixed(Vec<u8>),
    /// `git grep -E -i`: a POSIX ERE, matched case-insensitively.
    Regexp(Regex),
}

impl GrepPattern {
    /// Builds a matcher for `pattern`. With `use_regexp` it is a case-insensitive
    /// POSIX extended regular expression (`git grep -E -i`); otherwise it is a
    /// case-sensitive literal string (`git grep -F`).
    ///
    /// # Errors
    ///
    /// [`DomainError::Invalid`] when `use_regexp` is set and `pattern` is not a
    /// well-formed regular expression — gitweb's 400 "Invalid search regexp".
    pub fn new(pattern: &str, use_regexp: bool) -> Result<Self, DomainError> {
        let matcher: Matcher = if use_regexp {
            let regex: Regex = RegexBuilder::new(pattern)
                .case_insensitive(true)
                .unicode(false)
                .build()
                .map_err(|_| DomainError::Invalid(format!("search regexp '{pattern}'")))?;
            Matcher::Regexp(regex)
        } else {
            Matcher::Fixed(pattern.as_bytes().to_vec())
        };
        Ok(Self { matcher })
    }

    /// Whether `haystack`'s bytes contain a match — the test git-grep applies to a
    /// text line (to list it) or to a binary file's bytes (to report it as a
    /// "Binary file … matches"). An empty fixed pattern matches nothing.
    #[must_use]
    pub fn matches(&self, haystack: &[u8]) -> bool {
        match &self.matcher {
            Matcher::Fixed(needle) => contains(haystack, needle),
            Matcher::Regexp(regex) => regex.is_match(haystack),
        }
    }
}

/// Whether `needle` occurs in `haystack` as a literal byte substring — git's `-F`
/// (fixed strings) without `-i`, so the test is case-sensitive. An empty needle
/// matches nothing (gitweb requires at least two characters anyway), which also
/// keeps the window size non-zero.
fn contains(haystack: &[u8], needle: &[u8]) -> bool {
    if needle.is_empty() {
        return false;
    }
    haystack
        .windows(needle.len())
        .any(|window: &[u8]| window == needle)
}
