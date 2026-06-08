//! The `commitdiff_plain` handler: gitweb's `git_commitdiff -format plain`.
//!
//! Glue only, and one of the format-stable plain endpoints (its bytes must match
//! the original `gitweb.perl`, since `git am` parses them). It opens the
//! requested project, runs the [`assemble_commitdiff_plain`] use case for the
//! requested hash / explicit parent, and serves the framed patch as
//! `text/plain; charset=utf-8`. Two boundary jobs land here: the `X-Git-Url`
//! self-link in the body (gitweb's `$cgi->self_url()`, built with
//! [`self_url`] from the request's own parameters) and the inline filename
//! `<project basename>-<hash>.patch` in the Content-Disposition — both URLs/
//! headers, which the domain never builds.
//!
//! gitweb defaults the commit to `$hash || $hash_base || "HEAD"`, and stamps the
//! filename and the self-link with that *request* value (the literal `HEAD` when
//! neither is given), not the resolved id — reproduced here verbatim.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::commitdiff_plain::CommitdiffPlain;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::commitdiff_plain::assemble_commitdiff_plain;

use crate::dispatch::Handler;
use crate::response::View;
use crate::url::self_url;

/// Serves the `commitdiff_plain` patch over a wired project store. `base` is the
/// site URL (scheme and host, no trailing slash) the `X-Git-Url` self-link is
/// absolute against, the same base the feeds use.
pub struct CommitdiffPlainHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    base: String,
}

impl CommitdiffPlainHandler {
    /// Wires the handler with the store it opens projects from and the site
    /// `base` URL the self-link is built against.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, base: String) -> Self {
        Self { store, base }
    }
}

impl Handler for CommitdiffPlainHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;

        // gitweb's `$hash ||= $hash_base || "HEAD"`: the commit, then the base.
        // The use case defaults a `None` to HEAD; the filename and self-link use
        // the request value as-is (the literal "HEAD" when neither is given).
        let revision: Option<&str> = request
            .hash
            .as_ref()
            .or(request.hash_base.as_ref())
            .map(SafeRef::as_str);
        let explicit: Option<&str> = request.hash_parent.as_ref().map(SafeRef::as_str);

        let plain: CommitdiffPlain =
            assemble_commitdiff_plain(repository.as_ref(), revision, explicit)?;
        let body: String = plain.render(&self.self_link(project, request));
        let disposition: String = content_disposition(project, revision.unwrap_or("HEAD"));
        Ok(View::plain_attachment(disposition, body))
    }
}

impl CommitdiffPlainHandler {
    /// gitweb's `$cgi->self_url()` for this request: the site base, then the
    /// request's own parameters (`p`, `a=commitdiff_plain`, and whichever of
    /// `h` / `hp` / `hb` it carried). Reflecting the request — not the resolved
    /// commit — is gitweb's behaviour for the `X-Git-Url` line.
    fn self_link(&self, project: &str, request: &Request) -> String {
        let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", "commitdiff_plain")];
        if let Some(hash) = request.hash.as_ref() {
            params.push(("h", hash.as_str()));
        }
        if let Some(parent) = request.hash_parent.as_ref() {
            params.push(("hp", parent.as_str()));
        }
        if let Some(base) = request.hash_base.as_ref() {
            params.push(("hb", base.as_str()));
        }
        self_url(&self.base, &params)
    }
}

/// gitweb's `inline; filename="<project basename>-<hash>.patch"`. The project
/// basename is its last path segment (the bare repository name), and the hash is
/// the request value gitweb stamps the download with.
fn content_disposition(project: &str, hash: &str) -> String {
    let basename: &str = project.rsplit('/').next().unwrap_or(project);
    format!("inline; filename=\"{basename}-{hash}.patch\"")
}
