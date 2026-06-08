//! The `blob_plain` use case: gitweb's `git_blob_plain`, minus the streaming.
//!
//! `git_blob_plain` serves one file's bytes verbatim. It finds the blob exactly
//! as `git_blob` does — by id, or by resolving a path under the base revision
//! (HEAD by default), the shared [`resolve_blob`] rule — then derives the
//! Content-Type and Content-Disposition from the content and file name
//! ([`PlainHeaders`]). This use case returns those headers and the raw bytes; the
//! web boundary streams them. The bytes are never transformed, so a non-UTF-8
//! blob comes back untouched.

use crate::error::DomainError;
use crate::model::blob::Blob;
use crate::model::content_type::PlainHeaders;
use crate::model::object_id::ObjectId;
use crate::port::repository::Repository;
use crate::usecase::blob::resolve_blob;

/// The assembled raw-blob view: the derived headers and the verbatim bytes.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobPlainView {
    headers: PlainHeaders,
    bytes: Vec<u8>,
}

impl BlobPlainView {
    /// The `Content-Type` the bytes are served under.
    #[must_use]
    pub fn content_type(&self) -> &str {
        self.headers.content_type()
    }

    /// The `Content-Disposition` the bytes are served under.
    #[must_use]
    pub fn content_disposition(&self) -> &str {
        self.headers.content_disposition()
    }

    /// The raw bytes, borrowed.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the view, returning the raw bytes for the boundary to stream.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Assembles the raw-blob view over the [`Repository`] port. `base_rev` is the
/// base revision (gitweb's `$hash_base`); `hash` is a direct blob id (gitweb's
/// `$hash`); `file_name` is the path (gitweb's `$file_name`); `charset` is the
/// site's text charset (gitweb's `$default_text_plain_charset`, appended only to
/// `text/plain`); `prevent_xss` is the XSS-sandbox flag (gitweb's `$prevent_xss`).
///
/// # Errors
///
/// [`DomainError::Invalid`] (gitweb's `die_error(400)`) when neither `hash` nor
/// `file_name` is given; [`DomainError::NotFound`] (gitweb's `die_error(404)`)
/// when `file_name` names nothing, or names a non-blob, in the base; and any
/// error the repository raises reading the objects.
pub fn assemble_blob_plain(
    repo: &dyn Repository,
    base_rev: Option<&str>,
    hash: Option<&str>,
    file_name: Option<&str>,
    charset: Option<&str>,
    prevent_xss: bool,
) -> Result<BlobPlainView, DomainError> {
    let oid: ObjectId = resolve_blob(repo, base_rev, hash, file_name)?;
    let blob: Blob = repo.find_blob(&oid)?;
    let headers: PlainHeaders = PlainHeaders::resolve(
        file_name,
        blob.is_binary(),
        oid.as_str(),
        charset,
        prevent_xss,
    );
    Ok(BlobPlainView {
        headers,
        bytes: blob.into_bytes(),
    })
}
