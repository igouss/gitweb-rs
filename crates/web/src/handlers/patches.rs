//! The `patches` handler: gitweb's `git_patches` (`format-patch` range).
//!
//! Glue only, and a format-stable endpoint (its bytes must match real gitweb,
//! since `git am` parses them). It opens the requested project, reads the
//! `patches` feature limit from the resolved [`Settings`] (gitweb's
//! `$feature{patches}{default}`, the `$patch_max` the 403 gate and the range cap
//! share), runs the [`assemble_patches`] use case for the requested tip, and
//! serves the numbered mailbox stream as `text/plain; charset=utf-8`, offered
//! inline under `<project basename>-<hash>.patch`.
//!
//! This is the `patch` handler's sibling: the single form streams one commit, the
//! range form streams up to the feature limit, oldest-first, each `Subject`
//! numbered `[PATCH i/N]`. The two share their feature-limit and filename glue
//! (the [`patch`](crate::handlers::patch) module's `pub(crate)` helpers); only the
//! use case differs. Like the single form, the filename is stamped with the
//! *request* hash value (the literal `HEAD` when neither hash nor base is given),
//! and the only boundary value is the git version on each mail's signature.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::format_patch::FormatPatch;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::patch::assemble_patches;

use crate::dispatch::Handler;
use crate::handlers::patch::{content_disposition, patches_max};
use crate::response::View;

/// Serves the `patches` mailbox stream over a wired project store and the resolved
/// settings. `git_version` is the version stamped on each mail's `-- ` signature
/// (git format-patch's own, which a standalone gitweb-rs configures rather than
/// shelling out for).
pub struct PatchesHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
    git_version: String,
}

impl PatchesHandler {
    /// Wires the handler with the store it opens projects from, the settings it
    /// reads the `patches` limit from, and the git version its signatures carry.
    #[must_use]
    pub fn new(
        store: Arc<dyn ProjectStore + Send + Sync>,
        settings: Arc<Settings>,
        git_version: String,
    ) -> Self {
        Self {
            store,
            settings,
            git_version,
        }
    }
}

impl Handler for PatchesHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;

        // gitweb's `$hash ||= $hash_base || "HEAD"`: the tip, then the base. The
        // use case defaults a `None` to HEAD; the filename uses the request value
        // as-is (the literal "HEAD" when neither is given).
        let revision: Option<&str> = request
            .hash
            .as_ref()
            .or(request.hash_base.as_ref())
            .map(SafeRef::as_str);

        let stream: FormatPatch = assemble_patches(
            repository.as_ref(),
            revision,
            patches_max(&self.settings),
            &self.git_version,
        )?;
        let disposition: String = content_disposition(project, revision.unwrap_or("HEAD"));
        // gitweb's `$hash =~ /^$oid_regex$/` (after `$hash ||= $hash_base || "HEAD"`):
        // a format-patch range addressed by a literal oid tip is cacheable for a day.
        Ok(View::plain_attachment(disposition, stream.render())
            .with_expiry(Expiry::for_hash(revision)))
    }
}
