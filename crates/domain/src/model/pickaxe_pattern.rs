//! The pickaxe matcher — gitweb's `git_search_changes` `-S` / `--pickaxe-regex`.
//!
//! gitweb's pickaxe search runs `git log -S<text>` (plus `--pickaxe-regex` when
//! `search_use_regexp` is set) and lists the commits where the *number of
//! occurrences* of the pattern in some file changed against the parent. So a
//! pickaxe pattern is not a presence test like [`GrepPattern`] — it is an
//! occurrence *counter*, and git's `-S` lists a path only when that count differs.
//!
//! It is deliberately a *different* type from both
//! [`SearchPattern`](crate::model::search_pattern::SearchPattern) and
//! [`GrepPattern`](crate::model::grep_pattern::GrepPattern):
//!
//! - the commit/author/committer search ([`SearchPattern`]) is always
//!   case-insensitive (git's `-i` on every facet);
//! - git-grep's fixed mode is case-sensitive but its regexp mode is
//!   case-*insensitive* (`-E -i`);
//! - pickaxe is case-**sensitive in both modes** — git's `-S` / `--pickaxe-regex`
//!   carry no `-i` at all, the asymmetry gitweb's help text spells out ("This is
//!   case sensitive").
//!
//! Both modes operate over raw bytes. The fixed mode counts non-overlapping
//! literal occurrences (advancing past each match, as git's substring counter
//! does). The regexp mode is built byte-oriented and ASCII-only (`unicode(false)`,
//! case-sensitive), mirroring git's `regexec` in the C locale, and counts its
//! non-overlapping matches. A malformed regular expression is rejected as
//! [`DomainError::Invalid`], gitweb's 400 "Invalid search regexp".

use regex::bytes::{Regex, RegexBuilder};

use crate::error::DomainError;

/// A validated pickaxe matcher in one of gitweb's two modes.
#[derive(Debug, Clone)]
pub struct PickaxePattern {
    matcher: Matcher,
}

/// The two pickaxe modes, both case-sensitive.
#[derive(Debug, Clone)]
enum Matcher {
    /// `git log -S`: a literal byte substring, counted non-overlapping.
    Fixed(Vec<u8>),
    /// `git log -S --pickaxe-regex`: a case-sensitive POSIX ERE, matches counted.
    Regexp(Regex),
}

impl PickaxePattern {
    /// Builds a matcher for `pattern`. With `use_regexp` it is a case-sensitive
    /// POSIX extended regular expression (`--pickaxe-regex`); otherwise it is a
    /// case-sensitive literal byte string (`-S`).
    ///
    /// # Errors
    ///
    /// [`DomainError::Invalid`] when `use_regexp` is set and `pattern` is not a
    /// well-formed regular expression — gitweb's 400 "Invalid search regexp".
    pub fn new(pattern: &str, use_regexp: bool) -> Result<Self, DomainError> {
        let matcher: Matcher = if use_regexp {
            let regex: Regex = RegexBuilder::new(pattern)
                .case_insensitive(false)
                .unicode(false)
                .build()
                .map_err(|_| DomainError::Invalid(format!("search regexp '{pattern}'")))?;
            Matcher::Regexp(regex)
        } else {
            Matcher::Fixed(pattern.as_bytes().to_vec())
        };
        Ok(Self { matcher })
    }

    /// The number of non-overlapping occurrences of the pattern in `content`'s
    /// bytes — git's pickaxe occurrence count, case-sensitive in both modes. An
    /// empty fixed pattern counts zero (gitweb requires at least two characters
    /// anyway), keeping the window size non-zero.
    #[must_use]
    pub fn count(&self, content: &[u8]) -> usize {
        match &self.matcher {
            Matcher::Fixed(needle) => count_substring(content, needle),
            Matcher::Regexp(regex) => regex.find_iter(content).count(),
        }
    }

    /// Whether the pattern's occurrence count differs between `before` and
    /// `after` — git's `-S` rule for listing a path (and, summed over a commit's
    /// changed files, for listing the commit). A file the change leaves with the
    /// same count is not a pickaxe hit, even if its bytes otherwise changed.
    #[must_use]
    pub fn changed(&self, before: &[u8], after: &[u8]) -> bool {
        self.count(before) != self.count(after)
    }
}

/// Counts non-overlapping occurrences of `needle` in `haystack` — git's literal
/// `-S` counter, which advances past each match (so `"aa"` occurs twice in
/// `"aaaa"`, not three times). An empty needle counts zero.
fn count_substring(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count: usize = 0;
    let mut index: usize = 0;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            count += 1;
            index += needle.len();
        } else {
            index += 1;
        }
    }
    count
}
