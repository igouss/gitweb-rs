//! Ordering the projects list (gitweb's `sort_projects_list` + the `order`
//! validation in `git_project_list`).
//!
//! gitweb validates the `order` parameter against a fixed set
//! (`none|project|descr|owner|age`) and otherwise dies `400 Unknown order
//! parameter`; an absent order falls back to `$default_projects_order`. The
//! chosen key then drives `sort_projects_list`: a string comparison for
//! `project` / `descr` / `owner`, and a defined-before-undefined numeric
//! comparison for `age`. This is that pure rule over [`ProjectInfo`].

use std::cmp::Ordering;

use crate::error::DomainError;
use crate::model::project_info::ProjectInfo;

/// The validated sort order for the projects list, mirroring gitweb's `order`
/// parameter values.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectOrder {
    /// Input order, left untouched (gitweb's `none`).
    None,
    /// By store-relative project path (gitweb's `project`).
    Project,
    /// By full description (gitweb's `descr`, comparing `descr_long`).
    Description,
    /// By owner (gitweb's `owner`).
    Owner,
    /// By last change: most recent first, projects with no commits last
    /// (gitweb's `age`).
    Age,
}

impl ProjectOrder {
    /// Parses gitweb's `order` token, rejecting anything outside
    /// `none|project|descr|owner|age` the way `git_project_list` does
    /// (`die_error(400, "Unknown order parameter")`).
    ///
    /// # Errors
    ///
    /// Returns [`DomainError::Invalid`] for an unrecognized order.
    pub fn parse(value: &str) -> Result<Self, DomainError> {
        match value {
            "none" => Ok(Self::None),
            "project" => Ok(Self::Project),
            "descr" => Ok(Self::Description),
            "owner" => Ok(Self::Owner),
            "age" => Ok(Self::Age),
            _ => Err(DomainError::Invalid("Unknown order parameter".to_owned())),
        }
    }

    /// gitweb's token for this order, the inverse of [`parse`](Self::parse).
    #[must_use]
    pub fn as_str(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Project => "project",
            Self::Description => "descr",
            Self::Owner => "owner",
            Self::Age => "age",
        }
    }

    /// Sorts `projects` in place by this order (gitweb's `sort_projects_list`).
    /// [`None`](Self::None) leaves the input order untouched. The sort is
    /// stable, so equal-keyed projects keep their input order, matching Perl's
    /// stable `sort`.
    pub fn sort(self, projects: &mut [ProjectInfo]) {
        match self {
            // gitweb's %orderings has no 'none' key, so $ordering is undef and
            // the list comes back unsorted.
            Self::None => {}
            Self::Project => {
                projects.sort_by(|a: &ProjectInfo, b: &ProjectInfo| a.name().cmp(b.name()))
            }
            Self::Description => {
                projects.sort_by(|a: &ProjectInfo, b: &ProjectInfo| {
                    str_key(a.description()).cmp(str_key(b.description()))
                });
            }
            Self::Owner => {
                projects.sort_by(|a: &ProjectInfo, b: &ProjectInfo| {
                    str_key(a.owner()).cmp(str_key(b.owner()))
                });
            }
            Self::Age => projects.sort_by(age_then_undef),
        }
    }
}

/// A string sort key, treating a missing field as empty — gitweb's `cmp` on an
/// undefined `descr_long` / `owner`, which Perl stringifies to `""`.
fn str_key(value: Option<&str>) -> &str {
    value.unwrap_or("")
}

/// gitweb's `order_num_then_undef('age')`: projects with commits sort by age
/// ascending (most recent first), and projects with none sink to the bottom.
///
/// gitweb's `age` is `now - last_activity`, so a smaller age is a more recent
/// commit. The relative order of two real ages is independent of `now`
/// (`age_a < age_b` exactly when `epoch_a > epoch_b`), so ordering by last
/// activity *descending* reproduces it without consulting any clock.
fn age_then_undef(a: &ProjectInfo, b: &ProjectInfo) -> Ordering {
    match (a.last_activity(), b.last_activity()) {
        (Some(left), Some(right)) => right.cmp(&left),
        (Some(_), None) => Ordering::Less,
        (None, Some(_)) => Ordering::Greater,
        (None, None) => Ordering::Equal,
    }
}
