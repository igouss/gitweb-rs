//! Parsing the `$projects_list` file (gitweb's file-mode `git_get_projects_list`).
//!
//! When gitweb is configured with a projects-list *file* rather than a directory
//! to scan, each line names one project and, optionally, its owner — both
//! URL-encoded and separated by whitespace, e.g.
//! `git%2Fgit.git Linus+Torvalds`. This is the pure line grammar; reading the
//! file and confirming each path is an exported repository is the adapter's job.

use crate::model::url::unescape;

/// One entry parsed from a projects-list file: the project's store-relative
/// path and, when the line carries a second field, its owner. Both are
/// URL-decoded.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListEntry {
    path: String,
    owner: Option<String>,
}

impl ProjectListEntry {
    /// The store-relative project path.
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The owner named on the line, or `None` when the line has only a path.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }
}

/// Parses one projects-list line into its path and optional owner, URL-decoding
/// both. A blank or whitespace-only line yields `None` (gitweb skips it).
#[must_use]
pub fn parse_project_line(line: &str) -> Option<ProjectListEntry> {
    // gitweb's `split ' '`: ignore leading whitespace, split on whitespace runs.
    // The first field is the path, the optional second is the owner.
    let mut fields: std::str::SplitWhitespace<'_> = line.split_whitespace();
    let path: String = unescape(fields.next()?);
    let owner: Option<String> = fields.next().map(unescape);
    Some(ProjectListEntry { path, owner })
}
