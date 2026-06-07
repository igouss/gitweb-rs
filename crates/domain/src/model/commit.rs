//! Commit objects.
//!
//! Mirrors the derived fields gitweb computes in `parse_commit_text`: whether a
//! commit is a merge (more than one parent) and its display `title` /
//! `title_short` (the first non-empty message line, chopped to 80 / 50
//! characters, defaulting to `(no commit message)`).
//!
//! The raw header parsing gitweb does on `git rev-list --header` output is the
//! git adapter's job; this entity holds the already-structured fields and only
//! the derivations that are pure rules over them.

use crate::model::chop::{ChopMode, chop_str};
use crate::model::object_id::ObjectId;
use crate::model::signature::Signature;

/// gitweb's `chop_str($title, 80, 5)` — the full title bound.
const TITLE_LEN: usize = 80;
/// gitweb's `chop_str($title, 50, 5)` — the short title bound.
const TITLE_SHORT_LEN: usize = 50;
/// gitweb's `chop_str` slack for titles (`add_len`).
const TITLE_SLACK: usize = 5;
/// gitweb's placeholder for a commit with no usable message line.
const NO_MESSAGE: &str = "(no commit message)";

/// A git commit: identity, ancestry, authorship, and message.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Commit {
    id: ObjectId,
    tree: ObjectId,
    parents: Vec<ObjectId>,
    author: Signature,
    committer: Signature,
    message: String,
}

impl Commit {
    /// Assembles a commit from its already-structured fields.
    #[must_use]
    pub fn new(
        id: ObjectId,
        tree: ObjectId,
        parents: Vec<ObjectId>,
        author: Signature,
        committer: Signature,
        message: String,
    ) -> Self {
        Self {
            id,
            tree,
            parents,
            author,
            committer,
            message,
        }
    }

    /// The commit's own object id.
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// The root tree this commit snapshots.
    #[must_use]
    pub fn tree(&self) -> &ObjectId {
        &self.tree
    }

    /// The parent commits, in order; empty for a root commit.
    #[must_use]
    pub fn parents(&self) -> &[ObjectId] {
        &self.parents
    }

    /// The authoring identity.
    #[must_use]
    pub fn author(&self) -> &Signature {
        &self.author
    }

    /// The committing identity.
    #[must_use]
    pub fn committer(&self) -> &Signature {
        &self.committer
    }

    /// The full commit message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }

    /// Whether this is a merge: gitweb treats more than one parent as a merge.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    /// The display title: the first non-empty message line, chopped to 80
    /// characters, or `(no commit message)`.
    #[must_use]
    pub fn title(&self) -> String {
        self.chopped_title(TITLE_LEN)
    }

    /// The short display title: as [`Commit::title`] but chopped to 50.
    #[must_use]
    pub fn title_short(&self) -> String {
        self.chopped_title(TITLE_SHORT_LEN)
    }

    /// The first non-empty line of the message, if any.
    ///
    /// gitweb skips empty lines before taking the title; a whitespace-only line
    /// is non-empty and so is kept verbatim, matching `parse_commit_text`.
    fn first_line(&self) -> Option<&str> {
        self.message.lines().find(|line: &&str| !line.is_empty())
    }

    /// The first message line chopped to `len`, or the no-message placeholder.
    fn chopped_title(&self, len: usize) -> String {
        match self.first_line() {
            Some(line) => chop_str(line, len, TITLE_SLACK, ChopMode::Right),
            None => NO_MESSAGE.to_owned(),
        }
    }
}
