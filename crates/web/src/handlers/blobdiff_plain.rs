//! The `blobdiff_plain` handler: gitweb's `git_blobdiff('plain')`.
//!
//! Glue only, and one of the format-stable plain endpoints — its bytes must
//! match the original `gitweb.perl` so the single-file diff applies. It opens
//! the requested project, runs the [`assemble_blobdiff_plain`] use case for the
//! two bases and file, and serves the framed patch as `text/plain;
//! charset=utf-8`. Two boundary jobs land here: the `X-Git-Url` self-link in the
//! body (gitweb's `$cgi->self_url()`, built with [`self_url`] from the request's
//! own parameters) and the inline filename `<file_name>.patch` in the
//! Content-Disposition — both URLs/headers, which the domain never builds.
//!
//! This serves gitweb's new-style blobdiff URI: `hb` (base) + `hpb` (parent
//! base) + `f` (file), with an optional `fp` (the rename's from-path) reflected
//! in the self-link. gitweb 404s when a base is missing and 400s when the file
//! is missing — replicated here. The by-hash form that resolves a filename from
//! a raw blob hash is the HTML viewer's URI-resolution concern, tracked
//! separately; blobdiff_plain links always carry `f`.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blobdiff_plain::BlobdiffPlain;
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::blobdiff_plain::assemble_blobdiff_plain;

use crate::dispatch::Handler;
use crate::response::View;
use crate::url::self_url;

/// Serves the `blobdiff_plain` patch over a wired project store. `base` is the
/// site URL (scheme and host, no trailing slash) the `X-Git-Url` self-link is
/// absolute against, the same base the feeds and `commitdiff_plain` use.
pub struct BlobdiffPlainHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    base: String,
}

impl BlobdiffPlainHandler {
    /// Wires the handler with the store it opens projects from and the site
    /// `base` URL the self-link is built against.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, base: String) -> Self {
        Self { store, base }
    }
}

impl Handler for BlobdiffPlainHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;

        // gitweb's git_blobdiff: a missing base is a 404, a missing file (with
        // the bases present) is a 400 — both "Missing one of the blob diff
        // parameters".
        let hash_base: &str = request
            .hash_base
            .as_ref()
            .map(SafeRef::as_str)
            .ok_or_else(|| {
                DomainError::NotFound("Missing one of the blob diff parameters".to_owned())
            })?;
        let hash_parent_base: &str = request
            .hash_parent_base
            .as_ref()
            .map(SafeRef::as_str)
            .ok_or_else(|| {
                DomainError::NotFound("Missing one of the blob diff parameters".to_owned())
            })?;
        let file_name: &str = request
            .file_name
            .as_ref()
            .map(SafePath::as_str)
            .ok_or_else(|| {
                DomainError::Invalid("Missing one of the blob diff parameters".to_owned())
            })?;

        let repository: Box<dyn Repository> = self.store.open(project)?;
        let plain: BlobdiffPlain =
            assemble_blobdiff_plain(repository.as_ref(), hash_base, hash_parent_base, file_name)?;
        let body: String = plain.render(&self.self_link(project, request));
        let disposition: String = format!("inline; filename=\"{file_name}.patch\"");
        // gitweb's git_blobdiff('plain') shares the html path's dual-oid Expires
        // gate: a one-day window only when both bases are literal object ids.
        Ok(
            View::plain_attachment(disposition, body).with_expiry(Expiry::for_hashes(&[
                Some(hash_base),
                Some(hash_parent_base),
            ])),
        )
    }
}

impl BlobdiffPlainHandler {
    /// gitweb's `$cgi->self_url()` for this request: the site base, then the
    /// request's own parameters (`p`, `a=blobdiff_plain`, `f`, the optional `fp`,
    /// `hb`, `hpb`). [`self_url`] orders them by gitweb's `@cgi_param_mapping`,
    /// the order the capture's query string is written in.
    fn self_link(&self, project: &str, request: &Request) -> String {
        let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", "blobdiff_plain")];
        if let Some(file) = request.file_name.as_ref() {
            params.push(("f", file.as_str()));
        }
        if let Some(file_parent) = request.file_parent.as_ref() {
            params.push(("fp", file_parent.as_str()));
        }
        if let Some(base) = request.hash_base.as_ref() {
            params.push(("hb", base.as_str()));
        }
        if let Some(parent_base) = request.hash_parent_base.as_ref() {
            params.push(("hpb", parent_base.as_str()));
        }
        self_url(&self.base, &params)
    }
}
