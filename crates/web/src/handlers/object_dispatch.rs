//! The no-action object resolver: gitweb's `dispatch` `git_get_type` default.
//!
//! Glue only. When a request names no action but carries a hash — or a base ref
//! and a file — gitweb's `dispatch` sub does not redirect; it resolves the
//! referenced object's kind and serves the matching view *inline* for the same
//! request (unlike `git_object`, which 302-redirects — see [`ObjectHandler`]).
//! This resolver opens the requested project, runs the [`resolve_dispatch_action`]
//! use case to learn which action's view serves the object, and hands that
//! action back to the [`Dispatcher`], which invokes its registered handler with
//! the request unchanged.
//!
//! [`ObjectHandler`]: crate::handlers::ObjectHandler
//! [`Dispatcher`]: crate::dispatch::Dispatcher

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::object_dispatch::resolve_dispatch_action;

use crate::dispatch::ObjectKindResolver;

/// Resolves gitweb's no-action object-kind default over a wired project store.
pub struct DefaultObjectResolver {
    store: Arc<dyn ProjectStore + Send + Sync>,
}

impl DefaultObjectResolver {
    /// Wires the resolver with the store it opens projects from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>) -> Self {
        Self { store }
    }
}

impl ObjectKindResolver for DefaultObjectResolver {
    fn resolve(&self, request: &Request) -> Result<Action, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let hash: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let hash_base: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        let file_name: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        resolve_dispatch_action(repository.as_ref(), hash, hash_base, file_name)
    }
}
