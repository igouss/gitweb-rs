//! Scoping a project listing to a subdirectory (gitweb's `$project_filter`).
//!
//! gitweb threads the `pf` parameter through `git_get_projects_list`, keeping
//! only the projects whose path lies under the named subdirectory. In file mode
//! (and the paranoid directory walk) this is the regex `$path !~ m!^\Qfilter\E/!`;
//! in the plain directory walk it scopes the search root to `$dir/$filter`, but
//! the surviving paths are identical — every kept path begins with `filter/`.
//! This is that pure include rule, applied once at the [`ProjectStore::list`]
//! seam so all three listings (HTML list, project_index, opml) inherit it.
//!
//! Matching is by whole path component, so `foo` does not scope to `foobar.git`,
//! and a project whose path is exactly the filter (no deeper component) is not
//! under it.
//!
//! [`ProjectStore::list`]: crate::port::project_store::ProjectStore::list

/// A subdirectory a project listing is scoped to (gitweb's `$project_filter`).
///
/// The subdirectory is a validated, pathname-safe value — the request boundary
/// rejects an unsafe one with gitweb's `404 Invalid project_filter parameter`
/// before constructing this — so the rule itself only decides containment.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectFilter {
    subdir: String,
}

impl ProjectFilter {
    /// A filter scoping a listing to the subdirectory `subdir`.
    #[must_use]
    pub fn new(subdir: impl Into<String>) -> Self {
        Self {
            subdir: subdir.into(),
        }
    }

    /// The subdirectory this filter scopes to.
    #[must_use]
    pub fn subdir(&self) -> &str {
        &self.subdir
    }

    /// Whether a project at store-relative `path` lies under this filter's
    /// subdirectory — gitweb's `^\Qfilter\E/`: `path` stripped of the
    /// subdirectory must continue with a `/`, so the match is by whole component
    /// (`foo` does not contain `foobar.git`) and the subdirectory itself is not
    /// a member.
    #[must_use]
    pub fn include(&self, path: &str) -> bool {
        path.strip_prefix(&self.subdir)
            .is_some_and(|rest: &str| rest.starts_with('/'))
    }
}
