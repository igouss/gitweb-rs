//! The `opml` use case: gitweb's `git_opml` data, minus the XML.
//!
//! gitweb's OPML outline lists every project under the root that has a resolvable
//! `HEAD` (`git_opml` skips a project whose `git_get_head_hash` is undefined).
//! This is that orchestration over the [`ProjectStore`] port: list the projects,
//! die `404 No projects found` when the root is empty — before the per-project
//! HEAD filter, exactly where gitweb does — then keep only the projects that
//! open and resolve a HEAD. The link building (absolute `rss` / `summary` URLs)
//! and the title are the boundary's and render's job; this layer yields only the
//! surviving project paths, in discovery order.

use crate::error::DomainError;
use crate::model::project::Project;
use crate::port::project_store::ProjectStore;

/// One project in the OPML outline: its store-relative path. Only projects with a
/// resolvable HEAD reach here.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OpmlProject {
    path: String,
}

impl OpmlProject {
    /// The store-relative project path (gitweb's `$proj{'path'}`).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }
}

/// The OPML outline data (gitweb's `git_opml`): the projects with a HEAD, in
/// discovery order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Opml {
    projects: Vec<OpmlProject>,
}

impl Opml {
    /// The projects in the outline, in discovery order.
    #[must_use]
    pub fn projects(&self) -> &[OpmlProject] {
        &self.projects
    }
}

/// Assembles the OPML outline data (gitweb's `git_opml`): list the projects, die
/// `404 No projects found` when the root is empty, then keep only those with a
/// resolvable HEAD.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] when no projects are discoverable (gitweb's
/// `404 No projects found`), and the store's own error if discovery fails. A
/// non-empty root whose projects all lack a HEAD is *not* an error: it yields an
/// empty outline, as gitweb does.
pub fn assemble_opml(store: &dyn ProjectStore) -> Result<Opml, DomainError> {
    let projects: Vec<Project> = store.list()?;
    if projects.is_empty() {
        // gitweb's git_opml: die_error(404, "No projects found").
        return Err(DomainError::NotFound("No projects found".to_owned()));
    }
    let projects: Vec<OpmlProject> = projects
        .iter()
        .filter_map(|project: &Project| project_with_head(store, project.name()))
        .collect();
    Ok(Opml { projects })
}

/// Whether `name` resolves to a project with a HEAD, yielding its outline entry.
/// gitweb skips a project whose `git_get_head_hash` is undefined; here, a project
/// that fails to open or whose HEAD does not resolve is the same `None`.
fn project_with_head(store: &dyn ProjectStore, name: &str) -> Option<OpmlProject> {
    let repository = store.open(name).ok()?;
    repository.head().ok()?;
    Some(OpmlProject {
        path: name.to_owned(),
    })
}
