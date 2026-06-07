//! Repository export visibility: which repositories the server may serve.
//!
//! Mirrors gitweb's `check_head_link` / `check_export_ok` plus the
//! `$strict_export` gate (`is_valid_project`). The filesystem probing — does
//! `HEAD` exist, is the export marker present — belongs to an adapter; this
//! module owns only the pure decision. Given the configured gates and the facts
//! an adapter gathered, is the repository visible? Keeping the decision here
//! makes it exhaustively testable without touching a disk.

/// The server's configured export gates (gitweb config knobs).
///
/// Each field maps to a gitweb variable; a `false` gate is simply not enforced.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct ExportPolicy {
    /// `$export_ok`: an export-marker file must be present in the repository.
    pub require_marker: bool,
    /// `$export_auth_hook`: a custom authorization hook is configured.
    pub has_auth_hook: bool,
    /// `$strict_export`: only repositories in the projects list are served.
    pub strict: bool,
}

/// What an adapter observed about a candidate repository directory.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
pub struct RepoFacts {
    /// `check_head_link`: `HEAD` is a real file, or a symlink into `refs/heads/`.
    pub head_linked: bool,
    /// The export-marker file is present in the repository directory.
    pub marker_present: bool,
    /// The configured auth hook authorized this repository.
    pub auth_hook_allows: bool,
    /// The repository appears in the configured projects list.
    pub in_projects_list: bool,
}

impl ExportPolicy {
    /// Whether the repository may be served, given what the adapter observed.
    ///
    /// Visible only when `HEAD` is linked and every *enabled* gate is satisfied;
    /// a disabled gate imposes no constraint — exactly gitweb's
    /// `check_export_ok && (!$strict_export || project_in_list)`.
    #[must_use]
    pub fn permits(&self, facts: &RepoFacts) -> bool {
        facts.head_linked
            && (!self.require_marker || facts.marker_present)
            && (!self.has_auth_hook || facts.auth_hook_allows)
            && (!self.strict || facts.in_projects_list)
    }
}
