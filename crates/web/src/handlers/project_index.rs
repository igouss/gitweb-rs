//! The `project_index` handler: gitweb's `git_project_index`.
//!
//! Glue only. It runs the [`assemble_project_index`] use case over the wired
//! [`ProjectStore`], maps the resulting index into the render layer's view-model,
//! and serves the serialized `index.aux` body. gitweb sets `text/plain;
//! charset=utf-8` and offers it inline under that fixed filename; both are fixed
//! per endpoint, so the [`View`] carries them as `'static`. The action takes no
//! project and no parameters, so the request is ignored.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project_filter::ProjectFilter;
use gitweb_domain::model::request::Request;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::usecase::project_index::{
    ProjectIndex, ProjectIndexRow, assemble_project_index,
};
use gitweb_render::project_index::{ProjectIndexEntry, ProjectIndexView, project_index};

use crate::dispatch::Handler;
use crate::response::View;

/// gitweb's `text/plain; charset=utf-8` for the `index.aux` body. Public so the
/// golden conformance test can assert it against gitweb's captured header.
pub const INDEX_CONTENT_TYPE: &str = "text/plain; charset=utf-8";
/// gitweb's `-content_disposition` for `git_project_index`. Public so the golden
/// conformance test can assert it against gitweb's captured header.
pub const INDEX_DISPOSITION: &str = r#"inline; filename="index.aux""#;

/// Maps the domain index into the render layer's view-model — the raw `path` and
/// `owner` of every project, which the render serializer CGI-quotes. Shared by
/// the handler and the golden conformance test so both produce identical bytes.
#[must_use]
pub fn assemble_project_index_view(index: &ProjectIndex) -> ProjectIndexView {
    ProjectIndexView {
        entries: index.rows().iter().map(entry_view).collect(),
    }
}

/// Maps one index row to its render entry.
fn entry_view(row: &ProjectIndexRow) -> ProjectIndexEntry {
    ProjectIndexEntry {
        path: row.path().to_owned(),
        owner: row.owner().to_owned(),
    }
}

/// Serves the machine-readable project index over a wired project store.
pub struct ProjectIndexHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
}

impl ProjectIndexHandler {
    /// Wires the handler with the store it lists.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>) -> Self {
        Self { store }
    }
}

impl Handler for ProjectIndexHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let filter: Option<ProjectFilter> = request.filter();
        let index: ProjectIndex = assemble_project_index(self.store.as_ref(), filter.as_ref())?;
        let view: ProjectIndexView = assemble_project_index_view(&index);
        Ok(View::text_attachment(
            INDEX_CONTENT_TYPE,
            INDEX_DISPOSITION,
            project_index(&view),
        ))
    }
}
