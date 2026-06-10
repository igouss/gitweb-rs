//! The `forks` use case: gitweb's `git_forks` page logic, minus the HTML.
//!
//! gitweb's fork convention puts the forks of `repo.git` in the sibling `repo/`
//! directory. The forks action lists them: it scopes a project listing to that
//! subdirectory (the project path with one trailing `.git` removed) and dies
//! `404 No forks found` when none exist. It reuses `git_project_list_body`
//! wholesale, so — with the forks feature on — forks-of-forks fold under their
//! parent exactly as on the landing list.
//!
//! This is that orchestration: build the forks subdirectory filter and delegate
//! to the shared [`assemble_listing`] core, differing from the landing list only
//! in the filter source and the not-found message. The result is the same
//! [`ProjectListView`] the render adapter turns into a table, under the `$project
//! forks` header the boundary supplies.

use crate::error::DomainError;
use crate::model::forks::forks_subdirectory;
use crate::model::project_filter::ProjectFilter;
use crate::model::settings::Settings;
use crate::port::project_store::ProjectStore;
use crate::usecase::project_list::{ProjectListView, assemble_listing};

/// Assembles the forks page (gitweb's `git_forks`): list the projects under
/// `project`'s forks subdirectory (its path with one trailing `.git` removed),
/// folding forks-of-forks when the feature is on, and sort by `order`. `now` is
/// the request-time epoch the relative ages measure against.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] for an unrecognized `order`, the store's own
/// error if discovery or metadata fails, and [`DomainError::NotFound`] with
/// gitweb's `No forks found` when the project has no forks.
pub fn assemble_forks(
    store: &dyn ProjectStore,
    settings: &Settings,
    project: &str,
    order: Option<&str>,
    now: i64,
) -> Result<ProjectListView, DomainError> {
    let filter: ProjectFilter = ProjectFilter::new(forks_subdirectory(project));
    assemble_listing(store, settings, order, Some(&filter), "No forks found", now)
}
