//! The `blob_plain` handler: gitweb's `git_blob_plain` (one file, served raw).
//!
//! Glue only, and the one handler with no render layer: gitweb streams the blob
//! verbatim, so there is no HTML to assemble. It opens the requested project
//! through the wired [`ProjectStore`], runs the [`assemble_blob_plain`] use case
//! for the requested hash / base / file name, and returns the raw bytes under the
//! Content-Type and Content-Disposition the use case derived. The XSS-sandbox
//! flag is read here, at the boundary, from the resolved [`Settings`] (gitweb's
//! `$prevent_xss`); the text charset is gitweb's default of none.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::blob_plain::{BlobPlainView, assemble_blob_plain};

use crate::dispatch::Handler;
use crate::response::View;

/// Serves a blob's raw bytes over a wired project store and the resolved settings.
pub struct BlobPlainHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl BlobPlainHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the `prevent_xss` flag from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for BlobPlainHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let base: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        let hash: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let file: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        let view: BlobPlainView = assemble_blob_plain(
            repository.as_ref(),
            base,
            hash,
            file,
            None,
            self.settings.prevent_xss(),
        )?;
        let content_type: String = view.content_type().to_owned();
        let content_disposition: String = view.content_disposition().to_owned();
        Ok(View::raw_blob(
            content_type,
            content_disposition,
            view.into_bytes(),
        ))
    }
}
