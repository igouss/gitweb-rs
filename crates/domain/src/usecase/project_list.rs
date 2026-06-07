//! The `project_list` use case: gitweb's `git_project_list` page logic, minus
//! the HTML.
//!
//! gitweb discovers the projects under the root, fills each with its display
//! metadata, validates and applies the requested sort order, and dies
//! `404 No projects found` when the root is empty. This is that orchestration
//! over the [`ProjectStore`] port, producing a framework-free [`ProjectListView`]
//! the render adapter turns into a table. The clock is injected (`now`) so the
//! relative ages are computed here, once, and the view-model stays free of any
//! time dependency.

use crate::error::DomainError;
use crate::model::age::Age;
use crate::model::project::Project;
use crate::model::project_info::ProjectInfo;
use crate::model::project_order::ProjectOrder;
use crate::model::settings::Settings;
use crate::port::project_store::ProjectStore;

/// One project as it appears on the listing: its identity, the metadata gitweb
/// shows, and its last-change age relative to the request time (absent for a
/// project with no commits).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListRow {
    name: String,
    description: Option<String>,
    owner: Option<String>,
    age: Option<Age>,
}

impl ProjectListRow {
    /// The store-relative project path, used to link the row and shown as its
    /// name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The full project description, or `None` when it has none.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The project owner, or `None` when unknown.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// The age of the project's last change relative to the request time, or
    /// `None` for a project with no commits (gitweb's "No commits").
    #[must_use]
    pub fn age(&self) -> Option<Age> {
        self.age
    }
}

/// The assembled projects-list page: the ordered rows and the order they are in,
/// so the view can mark the active sort column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListView {
    rows: Vec<ProjectListRow>,
    order: ProjectOrder,
}

impl ProjectListView {
    /// The projects, in display order.
    #[must_use]
    pub fn rows(&self) -> &[ProjectListRow] {
        &self.rows
    }

    /// The order the rows are sorted by, for marking the active sort column.
    #[must_use]
    pub fn order(&self) -> ProjectOrder {
        self.order
    }
}

/// Assembles the projects-list page (gitweb's `git_project_list`): list the
/// projects, fill each with its metadata, and sort by `order` (defaulting to the
/// configured order). `now` is the request-time epoch the relative ages are
/// measured against.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] for an unrecognized `order`, the store's own
/// error if discovery or metadata fails, and [`DomainError::NotFound`] when no
/// projects are discoverable (gitweb's `404 No projects found`).
pub fn assemble_project_list(
    store: &dyn ProjectStore,
    settings: &Settings,
    order: Option<&str>,
    now: i64,
) -> Result<ProjectListView, DomainError> {
    let order: ProjectOrder = resolve_order(order, settings)?;

    let projects: Vec<Project> = store.list()?;
    if projects.is_empty() {
        // gitweb's git_project_list: die_error(404, "No projects found").
        return Err(DomainError::NotFound("No projects found".to_owned()));
    }

    let mut infos: Vec<ProjectInfo> = projects
        .iter()
        .map(|project: &Project| store.info(project.name()))
        .collect::<Result<Vec<ProjectInfo>, DomainError>>()?;
    order.sort(&mut infos);

    let rows: Vec<ProjectListRow> = infos
        .iter()
        .map(|info: &ProjectInfo| ProjectListRow::from_info(info, now))
        .collect();
    Ok(ProjectListView { rows, order })
}

/// Resolves the effective sort order: the request's `order` is validated and any
/// bad value rejected (gitweb's `die_error(400, "Unknown order parameter")`); an
/// absent `order` falls back to the configured default. gitweb does not
/// re-validate `$default_projects_order`, so a misconfigured default simply
/// leaves the list unsorted rather than failing the request.
fn resolve_order(order: Option<&str>, settings: &Settings) -> Result<ProjectOrder, DomainError> {
    match order {
        Some(token) => ProjectOrder::parse(token),
        None => Ok(
            ProjectOrder::parse(settings.default_projects_order()).unwrap_or(ProjectOrder::None)
        ),
    }
}

impl ProjectListRow {
    /// Builds a row from a project's metadata, turning its last-activity epoch
    /// into an age relative to `now` (gitweb computes `age = $now - $last`).
    fn from_info(info: &ProjectInfo, now: i64) -> Self {
        let age: Option<Age> = info
            .last_activity()
            .map(|epoch: i64| Age::from_seconds(now - epoch));
        Self {
            name: info.name().to_owned(),
            description: info.description().map(str::to_owned),
            owner: info.owner().map(str::to_owned),
            age,
        }
    }
}
