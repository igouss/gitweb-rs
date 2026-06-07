//! The `ProjectStore` port: discovering and opening repositories.
//!
//! Mirrors gitweb's project discovery under `$projectroot`
//! (`git_get_projects_list`) and the act of opening one repository to serve a
//! request. Listing yields project identities; opening yields a [`Repository`]
//! the use cases then read through.

use crate::error::DomainError;
use crate::model::project::Project;
use crate::port::repository::Repository;

/// Discovery and opening of the repositories under a project root.
pub trait ProjectStore {
    /// All projects discoverable under the project root.
    fn list(&self) -> Result<Vec<Project>, DomainError>;

    /// Opens one project by name for reading.
    fn open(&self, name: &str) -> Result<Box<dyn Repository>, DomainError>;
}
