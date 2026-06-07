//! Per-project metadata: the description, owner, category, and clone URLs
//! gitweb shows for a repository on the projects list and summary pages.
//!
//! Identity (the name used to open a repository) lives in
//! [`Project`](crate::model::project::Project); this is the richer metadata
//! gitweb layers on top via `fill_project_list_info`. Each field is optional
//! because every source (a `$GIT_DIR/description` file, a `gitweb.*` config key,
//! the projects-list file) may be absent; the adapter resolves the sources and
//! hands the result here. The pure rules are the display derivations gitweb
//! computes from these fields: the chopped short description and the category
//! fallback.

use std::collections::BTreeMap;

use crate::model::chop::{ChopMode, chop_str};

/// Metadata for one project, with every field optional because its source may
/// be absent.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectInfo {
    name: String,
    description: Option<String>,
    owner: Option<String>,
    category: Option<String>,
    clone_urls: Vec<String>,
    last_activity: Option<i64>,
}

impl ProjectInfo {
    /// A project known only by name, with no metadata resolved yet.
    #[must_use]
    pub fn named(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            description: None,
            owner: None,
            category: None,
            clone_urls: Vec::new(),
            last_activity: None,
        }
    }

    /// Sets the long description (gitweb's `descr_long`).
    #[must_use]
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Sets the owner.
    #[must_use]
    pub fn with_owner(mut self, owner: impl Into<String>) -> Self {
        self.owner = Some(owner.into());
        self
    }

    /// Sets the category.
    #[must_use]
    pub fn with_category(mut self, category: impl Into<String>) -> Self {
        self.category = Some(category.into());
        self
    }

    /// Appends one clone URL, in list order.
    #[must_use]
    pub fn with_clone_url(mut self, url: impl Into<String>) -> Self {
        self.clone_urls.push(url.into());
        self
    }

    /// Sets the last-activity time: the committer timestamp (Unix epoch seconds)
    /// of the project's most recently updated branch, as gitweb's
    /// `git_get_last_activity` derives it. The value is the raw timestamp, not an
    /// age — the view turns it into a relative age via [`crate::model::age::Age`],
    /// so the clock stays out of the metadata.
    #[must_use]
    pub fn with_last_activity(mut self, epoch: i64) -> Self {
        self.last_activity = Some(epoch);
        self
    }

    /// The store-relative name used to open the project.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The full description, or `None` if the project has none.
    #[must_use]
    pub fn description(&self) -> Option<&str> {
        self.description.as_deref()
    }

    /// The owner, or `None` if unknown.
    #[must_use]
    pub fn owner(&self) -> Option<&str> {
        self.owner.as_deref()
    }

    /// The category, or `None` if uncategorized.
    #[must_use]
    pub fn category(&self) -> Option<&str> {
        self.category.as_deref()
    }

    /// The clone URLs, in list order.
    #[must_use]
    pub fn clone_urls(&self) -> &[String] {
        &self.clone_urls
    }

    /// The last-activity time (committer epoch seconds of the most recent
    /// branch), or `None` for a project with no branch commits — an unborn or
    /// empty repository, where gitweb shows no "last change".
    #[must_use]
    pub fn last_activity(&self) -> Option<i64> {
        self.last_activity
    }

    /// The short description shown in the projects list: the full description
    /// chopped to about `width` characters (gitweb's
    /// `chop_str($descr, $projects_list_description_width, 5)`), with a missing
    /// description treated as empty.
    #[must_use]
    pub fn descr_short(&self, width: usize) -> String {
        chop_str(
            self.description.as_deref().unwrap_or(""),
            width,
            5,
            ChopMode::Right,
        )
    }

    /// The category for grouping: this project's category, or `default` when it
    /// has none (gitweb's `git_get_project_category || $project_list_default_category`).
    #[must_use]
    pub fn category_or<'a>(&'a self, default: &'a str) -> &'a str {
        self.category.as_deref().unwrap_or(default)
    }
}

/// A category and the projects filed under it (gitweb's
/// `build_projlist_by_category`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CategoryGroup {
    name: String,
    projects: Vec<ProjectInfo>,
}

impl CategoryGroup {
    /// The category name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The projects in this category, in the order they were listed.
    #[must_use]
    pub fn projects(&self) -> &[ProjectInfo] {
        &self.projects
    }
}

/// Buckets `projects` by category, using `default` for any project without one.
/// Categories come out sorted by name for a stable listing; within a category
/// the projects keep their input order.
#[must_use]
pub fn group_by_category(projects: &[ProjectInfo], default: &str) -> Vec<CategoryGroup> {
    // BTreeMap keys the buckets by category name, so iteration is sorted; each
    // bucket keeps the projects in the order they were listed.
    let mut buckets: BTreeMap<String, Vec<ProjectInfo>> = BTreeMap::new();
    for project in projects {
        let category: String = project.category_or(default).to_owned();
        buckets.entry(category).or_default().push(project.clone());
    }
    buckets
        .into_iter()
        .map(|(name, projects): (String, Vec<ProjectInfo>)| CategoryGroup { name, projects })
        .collect()
}
