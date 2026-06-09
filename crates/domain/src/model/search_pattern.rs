//! The commit-search pattern matcher — gitweb's `search_use_regexp` toggle.
//!
//! gitweb's `commit` / `author` / `committer` search hands git either
//! `--fixed-strings` (the default: the pattern is a literal string) or
//! `--extended-regexp` (when the user checks the *re* box), always together with
//! `--regexp-ignore-case`. So a search pattern is, in both modes, a
//! case-insensitive matcher; the only difference is whether the pattern's
//! metacharacters are taken literally or as a POSIX extended regular expression.
//!
//! This value object captures that single rule. A fixed pattern is compiled as
//! its [`regex::escape`]d form so metacharacters match literally; a regexp
//! pattern is compiled as written. Both carry the case-insensitive flag, so one
//! engine serves both modes — and the same matcher both filters commits
//! (mirroring `git log --grep`) and locates the first match for the result-row
//! highlight ([`crate::model::search_snippet`]). An ill-formed regular expression
//! is rejected as [`DomainError::Invalid`], the way gitweb dies 400 "Invalid
//! search regexp" while validating the request parameters.

use regex::{Regex, RegexBuilder};

use crate::error::DomainError;

/// A validated, case-insensitive commit-search pattern.
#[derive(Debug, Clone)]
pub struct SearchPattern {
    /// The pattern as the user typed it — what gitweb echoes back into the form
    /// and what the pickaxe facet feeds to git verbatim.
    raw: String,
    /// The compiled matcher: the escaped literal (fixed mode) or the pattern
    /// itself (regexp mode), always case-insensitive.
    matcher: Regex,
}

impl SearchPattern {
    /// Builds a matcher for `pattern`. With `use_regexp` it is a POSIX extended
    /// regular expression; otherwise it is a literal string (metacharacters
    /// escaped). Both match without regard to case.
    ///
    /// # Errors
    ///
    /// [`DomainError::Invalid`] when `use_regexp` is set and `pattern` is not a
    /// well-formed regular expression — gitweb's 400 "Invalid search regexp".
    pub fn new(pattern: &str, use_regexp: bool) -> Result<Self, DomainError> {
        let source: String = if use_regexp {
            pattern.to_owned()
        } else {
            regex::escape(pattern)
        };
        let matcher: Regex = RegexBuilder::new(&source)
            .case_insensitive(true)
            .build()
            .map_err(|_| DomainError::Invalid(format!("search regexp '{pattern}'")))?;
        Ok(Self {
            raw: pattern.to_owned(),
            matcher,
        })
    }

    /// Whether `haystack` contains a match, the way `git log --grep` keeps a
    /// commit whose message (or identity line) matches.
    #[must_use]
    pub fn is_match(&self, haystack: &str) -> bool {
        self.matcher.is_match(haystack)
    }

    /// The byte range `[start, end)` of the first match in `line`, or `None` when
    /// the line does not match — gitweb's `m/^(.*?)(<pattern>)(.*)$/i` split that
    /// drives the result-row highlight. Match boundaries fall on UTF-8 character
    /// boundaries, so the caller can slice `line` at them directly.
    #[must_use]
    pub fn first_match(&self, line: &str) -> Option<(usize, usize)> {
        self.matcher
            .find(line)
            .map(|m: regex::Match<'_>| (m.start(), m.end()))
    }

    /// The original pattern text the user typed.
    #[must_use]
    pub fn raw(&self) -> &str {
        &self.raw
    }
}
