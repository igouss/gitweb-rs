//! The `snapshot` handler: gitweb's `git_snapshot` (a downloadable archive).
//!
//! Glue only, and — like `blob_plain` — a handler with no render layer: gitweb
//! streams the archive bytes, so there is no HTML to assemble. It opens the
//! requested project, reads the site's enabled snapshot formats from the resolved
//! [`Settings`] (gitweb's `$feature{snapshot}{default}`), runs the
//! [`assemble_snapshot`] use case for the requested hash and format, and returns
//! the bytes under the format's media type, the download file name, and the
//! commit's `Last-Modified`. The branch ref directories that shape the archive
//! name are gitweb's `get_branch_refs`: `heads` plus the resolved
//! `extra-branch-refs` feature, so a ref under an extra branch directory is named
//! with its directory prefix instead of the literal `refs/<dir>/` form.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::branch_refs::get_branch_refs;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::model::timestamp::Timestamp;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::snapshot::{SnapshotView, assemble_snapshot};

use crate::dispatch::Handler;
use crate::response::View;

/// Serves a snapshot archive over a wired project store and the resolved settings.
pub struct SnapshotHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl SnapshotHandler {
    /// Wires the handler with the store it opens projects from and the settings it
    /// reads the enabled snapshot formats from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for SnapshotHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let configured: &[String] = self
            .settings
            .feature(FeatureName::Snapshot)
            .default_options();
        let requested: Option<&str> = request.snapshot_format.as_deref();
        let hash: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);

        // gitweb's get_branch_refs: heads plus the validated extra-branch-refs
        // feature. A malformed entry is gitweb's die_error(500).
        let extra_branch_refs: &[String] = self
            .settings
            .feature(FeatureName::ExtraBranchRefs)
            .default_options();
        let branch_refs: Vec<String> = get_branch_refs(extra_branch_refs)?;
        let branch_refs: Vec<&str> = branch_refs.iter().map(String::as_str).collect();

        let view: SnapshotView = assemble_snapshot(
            repository.as_ref(),
            project,
            configured,
            requested,
            hash,
            &branch_refs,
        )?;

        let content_type: &'static str = view.content_type();
        let content_disposition: String = view.content_disposition().to_owned();
        // The boundary formats the Last-Modified header and compares it against
        // the client's If-Modified-Since, so the use case hands over the time.
        let last_modified: Option<Timestamp> = view.last_modified().cloned();
        Ok(View::snapshot(
            content_type,
            content_disposition,
            last_modified,
            view.into_bytes(),
        ))
    }
}
