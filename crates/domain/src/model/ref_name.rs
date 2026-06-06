//! Shortening fully-qualified ref names for display.
//!
//! Mirrors gitweb stripping `refs/heads/`, `refs/tags/` and `refs/remotes/`
//! from ref names (see `git_get_heads_list` / `git_get_tags_list`). A ref in
//! any other namespace keeps its leaf name but is annotated with the namespace
//! — gitweb's `name (dir)` form — so it remains unambiguous.

use std::borrow::Cow;

/// A fully-qualified git ref name (e.g. `refs/heads/main`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefName {
    full: String,
}

/// Ref namespaces shown without annotation (their dir is implied).
const CLEAN_NAMESPACES: [&str; 3] = ["heads", "tags", "remotes"];

impl RefName {
    /// Wraps a fully-qualified ref name.
    #[must_use]
    pub fn new(full: impl Into<String>) -> Self {
        Self { full: full.into() }
    }

    /// The full, unmodified ref name.
    #[must_use]
    pub fn full(&self) -> &str {
        &self.full
    }

    /// The display short name.
    ///
    /// `refs/heads/X`, `refs/tags/X` and `refs/remotes/X` become `X`; any
    /// other `refs/<dir>/X` becomes `X (<dir>)`; anything not matching
    /// `refs/<dir>/...` is returned unchanged.
    #[must_use]
    pub fn short(&self) -> Cow<'_, str> {
        let after_refs: &str = match self.full.strip_prefix("refs/") {
            Some(rest) => rest,
            None => return Cow::Borrowed(&self.full),
        };
        let (namespace, leaf): (&str, &str) = match after_refs.split_once('/') {
            Some(parts) => parts,
            None => return Cow::Borrowed(&self.full),
        };
        if CLEAN_NAMESPACES.contains(&namespace) {
            Cow::Borrowed(leaf)
        } else {
            Cow::Owned(format!("{leaf} ({namespace})"))
        }
    }
}
