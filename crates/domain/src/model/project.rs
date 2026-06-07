//! Projects: a repository discoverable under the project root.
//!
//! Identity only — the name [`ProjectStore`] uses to open the repository, which
//! gitweb derives from the repository's path relative to `$projectroot`. The
//! richer metadata gitweb shows (description, owner, last change, category) is
//! layered on separately by the project-metadata capability, not here.
//!
//! [`ProjectStore`]: crate::port::project_store::ProjectStore

/// A repository known to the project store.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    name: String,
}

impl Project {
    /// Names a project by its store-relative identifier.
    #[must_use]
    pub fn new(name: String) -> Self {
        Self { name }
    }

    /// The identifier used to open the project.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
}
