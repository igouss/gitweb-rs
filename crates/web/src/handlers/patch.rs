//! The `patch` handler: gitweb's `git_patch` (`format-patch -single`).
//!
//! Glue only, and a format-stable endpoint (its bytes must match real gitweb,
//! since `git am` parses them). It opens the requested project, reads the
//! `patches` feature limit from the resolved [`Settings`] (gitweb's
//! `$feature{patches}{default}`, the `$patch_max` the 403 gate turns on), runs
//! the [`assemble_patch`] use case for the requested hash, and serves the
//! mailbox stream as `text/plain; charset=utf-8`, offered inline under
//! `<project basename>-<hash>.patch`.
//!
//! gitweb defaults the commit to `$hash || $hash_base || "HEAD"` and stamps the
//! filename with that *request* value (the literal `HEAD` when neither is
//! given), not the resolved id — reproduced here. Unlike `commitdiff_plain`,
//! the format-patch body carries no `X-Git-Url`, so there is no self-link to
//! build; the only boundary value is the git version on the mail's signature,
//! which the composition root configures.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::format_patch::FormatPatch;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::patch::assemble_patch;

use crate::dispatch::Handler;
use crate::response::View;

/// Serves the `patch` mailbox stream over a wired project store and the resolved
/// settings. `git_version` is the version stamped on the mail's `-- ` signature
/// (git format-patch's own, which a standalone gitweb-rs configures rather than
/// shelling out for).
pub struct PatchHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
    git_version: String,
}

impl PatchHandler {
    /// Wires the handler with the store it opens projects from, the settings it
    /// reads the `patches` limit from, and the git version its signature carries.
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

impl Handler for PatchHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;

        // gitweb's `$hash ||= $hash_base || "HEAD"`: the commit, then the base.
        // The use case defaults a `None` to HEAD; the filename uses the request
        // value as-is (the literal "HEAD" when neither is given).
        let revision: Option<&str> = request
            .hash
            .as_ref()
            .or(request.hash_base.as_ref())
            .map(SafeRef::as_str);

        let stream: FormatPatch = assemble_patch(
            repository.as_ref(),
            revision,
            patches_max(&self.settings),
            &self.git_version,
        )?;
        let disposition: String = content_disposition(project, revision.unwrap_or("HEAD"));
        Ok(View::plain_attachment(disposition, stream.render()))
    }
}

/// gitweb's `($patch_max) = gitweb_get_feature('patches')`: the first `patches`
/// option as an integer (the built-in `16`), `0` when the feature is off or its
/// value is not a positive number — the value the use case's 403 gate reads.
fn patches_max(settings: &Settings) -> usize {
    settings
        .feature(FeatureName::Patches)
        .default_options()
        .first()
        .and_then(|option: &String| option.parse::<usize>().ok())
        .unwrap_or(0)
}

/// gitweb's `inline; filename="<project basename>-<hash>.patch"`. The basename is
/// the project's last path segment, the hash the request value gitweb stamps the
/// download with.
fn content_disposition(project: &str, hash: &str) -> String {
    let basename: &str = project.rsplit('/').next().unwrap_or(project);
    format!("inline; filename=\"{basename}-{hash}.patch\"")
}
