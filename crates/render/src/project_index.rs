//! gitweb's `git_project_index` body — the machine-readable `index.aux`.
//!
//! Format-stable (verified by golden conformance, not just behaviourally), so
//! this reproduces gitweb's output to the byte: one line per project,
//! `path owner`, each field CGI-quoted with [`esc_index_field`] (the slash kept,
//! a space turned into `+`), each line newline-ended. The web boundary maps the
//! domain index into this view; here we quote and join.

use crate::escape::esc_index_field;

/// One project line: the raw path and owner the boundary supplies (this layer
/// applies gitweb's CGI quoting).
#[derive(Debug, Clone)]
pub struct ProjectIndexEntry {
    /// The store-relative project path (raw).
    pub path: String,
    /// The resolved owner, or `""` when none (raw).
    pub owner: String,
}

/// The whole machine-readable index: the project entries, in discovery order.
#[derive(Debug, Clone)]
pub struct ProjectIndexView {
    /// The projects, in discovery order.
    pub entries: Vec<ProjectIndexEntry>,
}

/// Serializes the project index (gitweb's `git_project_index` body): one
/// CGI-quoted `path owner` line per project, each ended by a newline.
#[must_use]
pub fn project_index(view: &ProjectIndexView) -> String {
    view.entries.iter().map(entry_line).collect()
}

/// One `path owner\n` line, both fields CGI-quoted.
fn entry_line(entry: &ProjectIndexEntry) -> String {
    format!(
        "{} {}\n",
        esc_index_field(&entry.path),
        esc_index_field(&entry.owner)
    )
}
