//! The `object` handler: gitweb's `git_object` (generic object dispatch).
//!
//! Glue only. gitweb's `git_object` does not render a view — it learns the kind
//! of the object a request names and `302`-redirects to the view for that kind.
//! This handler opens the requested project, runs the [`assemble_object_redirect`]
//! use case to resolve the destination action and the parameters that name the
//! object, builds the absolute redirect URL (gitweb's `href(-full => 1, …)`), and
//! returns it as a [`View::redirect`]. URLs are the boundary's job, so the use
//! case hands back the action and parameters and this assembles the `Location`.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::object::{ObjectRedirect, assemble_object_redirect};

use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href_full;

/// Resolves the `object` action to a redirect over a wired project store,
/// against the site `base` URL the absolute `Location` is built from.
pub struct ObjectHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    base: String,
}

impl ObjectHandler {
    /// Wires the handler with the store it opens projects from and the site
    /// `base` URL (scheme and host, no trailing slash) the redirect target is
    /// absolute against — gitweb's `-full => 1`.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, base: String) -> Self {
        Self { store, base }
    }

    /// Builds the absolute redirect target gitweb's `href(-full => 1, action =>
    /// …, hash => …, hash_base => …, file_name => …)` produces: the project, the
    /// resolved action, and whichever of the object parameters the redirect
    /// carries. [`href_full`] orders the pairs canonically, so the `Location` is
    /// stable regardless of push order.
    fn location(&self, project: &str, redirect: &ObjectRedirect) -> String {
        let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", redirect.action.as_str())];
        if let Some(hash) = redirect.hash.as_deref() {
            params.push(("h", hash));
        }
        if let Some(base) = redirect.hash_base.as_deref() {
            params.push(("hb", base));
        }
        if let Some(file) = redirect.file_name.as_deref() {
            params.push(("f", file));
        }
        href_full(&self.base, &params)
    }
}

impl Handler for ObjectHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let hash: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let hash_base: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        let file_name: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        let redirect: ObjectRedirect =
            assemble_object_redirect(repository.as_ref(), hash, hash_base, file_name)?;
        Ok(View::redirect(self.location(project, &redirect)))
    }
}
