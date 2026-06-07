//! Blame: line-by-line attribution of a file to commits.
//!
//! Mirrors the data gitweb renders in `git_blame_common` / `git_blame_data`:
//! each line of the final file carries the commit that introduced it and the
//! line numbers in the original and final versions.

use crate::model::object_id::ObjectId;

/// One blamed line of a file.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlameLine {
    commit: ObjectId,
    orig_lineno: usize,
    final_lineno: usize,
    content: String,
}

impl BlameLine {
    /// Builds a blamed line.
    #[must_use]
    pub fn new(commit: ObjectId, orig_lineno: usize, final_lineno: usize, content: String) -> Self {
        Self {
            commit,
            orig_lineno,
            final_lineno,
            content,
        }
    }

    /// The commit that last touched this line.
    #[must_use]
    pub fn commit(&self) -> &ObjectId {
        &self.commit
    }

    /// The line number in the commit that introduced it.
    #[must_use]
    pub fn orig_lineno(&self) -> usize {
        self.orig_lineno
    }

    /// The line number in the final file.
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

/// The blame for a whole file: one entry per final line, in order.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Blame {
    lines: Vec<BlameLine>,
}

impl Blame {
    /// Builds a blame from its lines.
    #[must_use]
    pub fn new(lines: Vec<BlameLine>) -> Self {
        Self { lines }
    }

    /// The blamed lines, in final-file order.
    #[must_use]
    pub fn lines(&self) -> &[BlameLine] {
        &self.lines
    }
}
