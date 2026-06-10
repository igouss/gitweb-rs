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

use std::collections::HashMap;

use crate::error::DomainError;
use crate::model::age::Age;
use crate::model::forks::{ProjectGroup, partition_forks};
use crate::model::project::Project;
use crate::model::project_filter::ProjectFilter;
use crate::model::project_info::ProjectInfo;
use crate::model::project_order::ProjectOrder;
use crate::model::settings::{FeatureName, Settings};
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
    fork_count: usize,
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

    /// The number of forks folded under this project (gitweb's
    /// `scalar @{$pr->{'forks'}}`). Always `0` when the forks feature is off; a
    /// positive count drives the leading `+` link to the project's forks view.
    #[must_use]
    pub fn fork_count(&self) -> usize {
        self.fork_count
    }
}

/// The assembled projects-list page: the ordered rows and the order they are in,
/// so the view can mark the active sort column.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectListView {
    rows: Vec<ProjectListRow>,
    order: ProjectOrder,
    forks_enabled: bool,
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

    /// Whether the forks feature is on for this listing (gitweb's
    /// `$check_forks`). When set, the view carries the leading fork column and a
    /// project's [`fork_count`](ProjectListRow::fork_count) is meaningful.
    #[must_use]
    pub fn forks_enabled(&self) -> bool {
        self.forks_enabled
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
/// projects are discoverable (gitweb's `404 No projects found`). When `filter`
/// is given, only the projects under its subdirectory are listed; an empty
/// result after filtering is the same `404` — exactly where gitweb dies.
pub fn assemble_project_list(
    store: &dyn ProjectStore,
    settings: &Settings,
    order: Option<&str>,
    filter: Option<&ProjectFilter>,
    now: i64,
) -> Result<ProjectListView, DomainError> {
    // gitweb's git_project_list: die_error(404, "No projects found").
    assemble_listing(store, settings, order, filter, "No projects found", now)
}

/// The shared projects-listing assembly behind both the landing list
/// ([`assemble_project_list`]) and the forks view
/// ([`assemble_forks`](crate::usecase::forks::assemble_forks)) — gitweb's
/// `git_project_list_body`. It lists the projects (scoped by `filter`), dies
/// `404 not_found` when the raw list is empty (the caller's message: "No
/// projects found" or "No forks found"), folds forks under their parents when
/// the forks feature is on, fills the surviving projects with their metadata, and
/// sorts by `order`. `now` is the request-time epoch the relative ages measure
/// against.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] for an unrecognized `order`, the store's own
/// error if discovery or metadata fails, and [`DomainError::NotFound`] with
/// `not_found` when no projects are discoverable.
pub(crate) fn assemble_listing(
    store: &dyn ProjectStore,
    settings: &Settings,
    order: Option<&str>,
    filter: Option<&ProjectFilter>,
    not_found: &str,
    now: i64,
) -> Result<ProjectListView, DomainError> {
    let order: ProjectOrder = resolve_order(order, settings)?;

    let projects: Vec<Project> = store.list(filter)?;
    if projects.is_empty() {
        return Err(DomainError::NotFound(not_found.to_owned()));
    }

    // gitweb's git_project_list_body: $check_forks = gitweb_check_feature('forks').
    let forks_enabled: bool = settings.feature(FeatureName::Forks).enabled();
    let names: Vec<String> = projects
        .iter()
        .map(|project: &Project| project.name().to_owned())
        .collect();
    let visible: Vec<(String, usize)> = fold_forks(&names, forks_enabled);
    let fork_counts: HashMap<&str, usize> = visible
        .iter()
        .map(|(name, count): &(String, usize)| (name.as_str(), *count))
        .collect();

    let mut infos: Vec<ProjectInfo> = visible
        .iter()
        .map(|(name, _): &(String, usize)| store.info(name))
        .collect::<Result<Vec<ProjectInfo>, DomainError>>()?;
    order.sort(&mut infos);

    let rows: Vec<ProjectListRow> = infos
        .iter()
        .map(|info: &ProjectInfo| {
            let fork_count: usize = fork_counts.get(info.name()).copied().unwrap_or(0);
            ProjectListRow::from_info(info, now, fork_count)
        })
        .collect();
    Ok(ProjectListView {
        rows,
        order,
        forks_enabled,
    })
}

/// Folds forks under their parents when the feature is on (gitweb's
/// `filter_forks_from_projects_list` inside `git_project_list_body`), pairing
/// each surviving project with the count of forks folded under it; input order
/// is preserved. With the feature off, every project stays visible with no fold.
fn fold_forks(names: &[String], forks_enabled: bool) -> Vec<(String, usize)> {
    if !forks_enabled {
        return names
            .iter()
            .map(|name: &String| (name.clone(), 0))
            .collect();
    }
    partition_forks(names)
        .into_iter()
        .map(|group: ProjectGroup| (group.name().to_owned(), group.forks().len()))
        .collect()
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
    /// into an age relative to `now` (gitweb computes `age = $now - $last`), and
    /// carrying the number of forks folded under it.
    fn from_info(info: &ProjectInfo, now: i64, fork_count: usize) -> Self {
        let age: Option<Age> = info
            .last_activity()
            .map(|epoch: i64| Age::from_seconds(now - epoch));
        Self {
            name: info.name().to_owned(),
            description: info.description().map(str::to_owned),
            owner: info.owner().map(str::to_owned),
            age,
            fork_count,
        }
    }
}
