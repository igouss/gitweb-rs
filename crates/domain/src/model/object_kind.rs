//! Git object types.
//!
//! Every git object is one of four kinds — the strings `commit`, `tree`,
//! `blob`, `tag` that `git cat-file -t` reports and that gitweb matches in
//! `parse_tag` (`type (.+)`) and elsewhere. The [`Repository`] port surfaces
//! the kind of a resolved id so callers can dispatch without re-reading it.
//!
//! [`Repository`]: crate::port::repository::Repository

/// What a git object is.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ObjectKind {
    /// A commit object.
    Commit,
    /// A tree object.
    Tree,
    /// A blob object.
    Blob,
    /// An annotated tag object.
    Tag,
}

impl ObjectKind {
    /// Parses git's lowercase type name, returning `None` for anything that is
    /// not one of the four object kinds.
    #[must_use]
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "commit" => Some(Self::Commit),
            "tree" => Some(Self::Tree),
            "blob" => Some(Self::Blob),
            "tag" => Some(Self::Tag),
            _ => None,
        }
    }

    /// The lowercase type name git uses (`"commit"`, `"tree"`, …).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Commit => "commit",
            Self::Tree => "tree",
            Self::Blob => "blob",
            Self::Tag => "tag",
        }
    }
}
