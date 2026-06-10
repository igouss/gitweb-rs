//! The `ProjectStore` port: discovering and opening repositories.
//!
//! Mirrors gitweb's project discovery under `$projectroot`
//! (`git_get_projects_list`) and the act of opening one repository to serve a
//! request. Listing yields project identities; opening yields a [`Repository`]
//! the use cases then read through.

use crate::error::DomainError;
use crate::model::project::Project;
use crate::model::project_filter::ProjectFilter;
use crate::model::project_info::ProjectInfo;
use crate::port::repository::Repository;

/// Discovery and opening of the repositories under a project root.
pub trait ProjectStore {
    /// The projects discoverable under the project root, scoped to `filter`'s
    /// subdirectory when one is given (gitweb's `$project_filter` argument to
    /// `git_get_projects_list`). A `None` filter lists the whole root.
    fn list(&self, filter: Option<&ProjectFilter>) -> Result<Vec<Project>, DomainError>;

    /// Opens one project by name for reading.
    fn open(&self, name: &str) -> Result<Box<dyn Repository>, DomainError>;

    /// Whether `subdir` exists as a directory under the project root — gitweb's
    /// `-d "$projectroot/$path"` guard in `filter_forks_from_projects_list`,
    /// where `$path` is a project's [`fork_container`]. A fork-capable project
    /// whose sibling container directory exists, even with zero forks inside,
    /// earns gitweb's `forks => []` empty-container state on the listing.
    ///
    /// Like gitweb's `-d`, this is an infallible predicate: a missing directory,
    /// an unreadable one, or a path that is not a directory all answer `false`
    /// rather than failing the request.
    ///
    /// [`fork_container`]: crate::model::forks::fork_container
    fn container_exists(&self, subdir: &str) -> bool;

    /// The display metadata for one project — description, owner, category, and
    /// clone URLs — resolved from its filesystem files and `gitweb.*` config
    /// (gitweb's `git_get_project_*` family). A name that does not resolve to a
    /// repository fails the same way [`open`](Self::open) does.
    fn info(&self, name: &str) -> Result<ProjectInfo, DomainError>;

    /// The raw contents of the project's `$GIT_DIR/README.html`, when the file
    /// exists and is non-empty (gitweb's `-s` test on the summary page); `None`
    /// when it is absent or empty.
    ///
    /// This is a DELIBERATE raw-HTML sink: gitweb's `git_summary` injects these
    /// bytes verbatim with `insert_file`, doing no escaping. The caller is the
    /// guard — gitweb only reaches this when `$prevent_xss` is off — so a
    /// consumer must apply that gate before emitting the result. A name that does
    /// not resolve to a repository fails the same way [`open`](Self::open) does;
    /// a missing or empty README is not a failure, just `None`.
    fn readme_html(&self, name: &str) -> Result<Option<String>, DomainError>;
}
