//! The `project_index` use case: gitweb's `git_project_index` data, minus the
//! text formatting.
//!
//! gitweb's machine-readable `index.aux` is one `path owner` line per project
//! under the root. This is that orchestration over the [`ProjectStore`] port: it
//! lists the projects, resolves each one's owner through the store's metadata
//! (the same file/config/filesystem precedence the listing uses), and dies
//! `404 No projects found` when the root is empty — exactly where gitweb does,
//! before any line is produced. It does not sort (gitweb emits discovery order)
//! and it does not quote: the `path owner` fields stay raw, and the render
//! adapter applies gitweb's CGI quoting.

use crate::error::DomainError;
use crate::model::project::Project;
use crate::port::project_store::ProjectStore;

/// One project in the index: its store-relative path and its resolved owner,
/// both raw (gitweb's `$path $owner`, before the CGI quoting the render layer
/// applies). The owner is empty when none can be resolved.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIndexRow {
    path: String,
    owner: String,
}

impl ProjectIndexRow {
    /// The store-relative project path (gitweb's `$pr->{'path'}`).
    #[must_use]
    pub fn path(&self) -> &str {
        &self.path
    }

    /// The resolved project owner, or `""` when none could be resolved.
    #[must_use]
    pub fn owner(&self) -> &str {
        &self.owner
    }
}

/// The machine-readable project index (gitweb's `git_project_index`): every
/// discovered project as a `path owner` row, in discovery order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectIndex {
    rows: Vec<ProjectIndexRow>,
}

impl ProjectIndex {
    /// The project rows, in discovery order.
    #[must_use]
    pub fn rows(&self) -> &[ProjectIndexRow] {
        &self.rows
    }
}

/// Assembles the machine-readable project index (gitweb's `git_project_index`):
/// list the projects, resolve each one's owner, and die `404 No projects found`
/// when the root is empty.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] when no projects are discoverable (gitweb's
/// `404 No projects found`), and the store's own error if discovery or metadata
/// fails.
pub fn assemble_project_index(store: &dyn ProjectStore) -> Result<ProjectIndex, DomainError> {
    let projects: Vec<Project> = store.list()?;
    if projects.is_empty() {
        // gitweb's git_project_index: die_error(404, "No projects found").
        return Err(DomainError::NotFound("No projects found".to_owned()));
    }
    let rows: Vec<ProjectIndexRow> = projects
        .iter()
        .map(|project: &Project| index_row(store, project))
        .collect::<Result<Vec<ProjectIndexRow>, DomainError>>()?;
    Ok(ProjectIndex { rows })
}

/// Builds one index row: the project's path verbatim and the owner the store
/// resolves for it, flattened to `""` when none is set.
fn index_row(store: &dyn ProjectStore, project: &Project) -> Result<ProjectIndexRow, DomainError> {
    let owner: String = store
        .info(project.name())?
        .owner()
        .unwrap_or_default()
        .to_owned();
    Ok(ProjectIndexRow {
        path: project.name().to_owned(),
        owner,
    })
}
