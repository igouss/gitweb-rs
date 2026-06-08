//! The `blob` use case: gitweb's `git_blob`, minus the HTML.
//!
//! gitweb shows one file of a revision. The blob is located either directly by
//! its id (gitweb's `$hash`, the way a tree entry links to it) or by resolving a
//! path within the base revision (gitweb's `git_get_hash_by_path($hash_base ||
//! HEAD, $file_name, "blob")`); a request that names neither is
//! `die_error(400, "No file name defined")`, and a path absent from the base is
//! `die_error(404, "Cannot find file")`.
//!
//! How the content is presented is the entity's call ([`Blob::display_kind`]):
//! text is decoded to UTF-8 with the caller's fallback encoding and split into
//! lines, an image is marked for inline display, and any other binary carries no
//! lines because it is download-only. The commit subject heads the page only
//! when the base revision was named and resolves to a commit (gitweb's
//! `if (defined $hash_base && (my %co = parse_commit($hash_base)))`).

use crate::error::DomainError;
use crate::model::blob::{Blob, BlobDisplay};
use crate::model::encoding::{FallbackEncoding, to_utf8};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::port::repository::Repository;

/// The assembled blob view: the resolved blob id (the title when no commit
/// subject is shown), the commit subject for the header, how the blob is to be
/// displayed, and — for a text blob — its lines.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct BlobView {
    blob_id: String,
    commit_title: Option<String>,
    display: BlobDisplay,
    lines: Vec<String>,
}

impl BlobView {
    /// The resolved id of the blob being shown. gitweb prints this as the title
    /// when the base names no commit.
    #[must_use]
    pub fn blob_id(&self) -> &str {
        &self.blob_id
    }

    /// The commit subject shown in the header, or `None` when the base revision
    /// was not named (or does not resolve to a commit), gitweb's loose-blob path.
    #[must_use]
    pub fn commit_title(&self) -> Option<&str> {
        self.commit_title.as_deref()
    }

    /// How the blob should be presented (text, image, or download-only binary).
    #[must_use]
    pub fn display(&self) -> BlobDisplay {
        self.display
    }

    /// The decoded text lines, for a text blob; empty for an image or binary.
    #[must_use]
    pub fn lines(&self) -> &[String] {
        &self.lines
    }
}

/// Assembles the blob view over the [`Repository`] port. `base_rev` is the base
/// revision (gitweb's `$hash_base`); `hash` is a direct blob id (gitweb's
/// `$hash`); `file_name` is the path (gitweb's `$file_name`); `fallback` decodes
/// non-UTF-8 text.
///
/// # Errors
///
/// [`DomainError::Invalid`] (gitweb's `die_error(400)`) when neither `hash` nor
/// `file_name` is given; [`DomainError::NotFound`] (gitweb's `die_error(404)`)
/// when `file_name` names nothing, or names a non-blob, in the base; and any
/// error the repository raises reading the objects.
pub fn assemble_blob(
    repo: &dyn Repository,
    base_rev: Option<&str>,
    hash: Option<&str>,
    file_name: Option<&str>,
    fallback: FallbackEncoding,
) -> Result<BlobView, DomainError> {
    let oid: ObjectId = resolve_blob(repo, base_rev, hash, file_name)?;
    let blob: Blob = repo.find_blob(&oid)?;
    let display: BlobDisplay = blob.display_kind(file_name);
    let lines: Vec<String> = match display {
        BlobDisplay::Text => decode_lines(&blob, fallback),
        BlobDisplay::Image | BlobDisplay::Binary => Vec::new(),
    };
    Ok(BlobView {
        blob_id: oid.as_str().to_owned(),
        commit_title: header_title(repo, base_rev)?,
        display,
        lines,
    })
}

/// Resolves the blob to show: directly by `hash` (gitweb's `$hash`), or by
/// looking `file_name` up under the base revision (`base_rev` or `HEAD`), the
/// way `git_get_hash_by_path($hash_base || HEAD, $file_name, "blob")` does. A
/// request that names neither is `die_error(400, "No file name defined")`; a
/// path that is absent, or that names something other than a blob, is
/// `die_error(404, "Cannot find file")`.
fn resolve_blob(
    repo: &dyn Repository,
    base_rev: Option<&str>,
    hash: Option<&str>,
    file_name: Option<&str>,
) -> Result<ObjectId, DomainError> {
    if let Some(hash) = hash {
        return repo.resolve(hash);
    }
    let file_name: &str =
        file_name.ok_or_else(|| DomainError::Invalid("No file name defined".to_owned()))?;
    let base: ObjectId = repo.resolve(base_rev.unwrap_or("HEAD"))?;
    let at: ObjectId = repo
        .path_id(&base, file_name)?
        .ok_or_else(|| DomainError::NotFound("Cannot find file".to_owned()))?;
    if repo.object_kind(&at)? != ObjectKind::Blob {
        return Err(DomainError::NotFound("Cannot find file".to_owned()));
    }
    Ok(at)
}

/// The commit subject for the header, shown only when the base revision was
/// named and resolves to a commit (gitweb's
/// `if (defined $hash_base && (my %co = parse_commit($hash_base)))`); `None`
/// otherwise, the loose-blob path where gitweb prints the blob hash instead.
fn header_title(
    repo: &dyn Repository,
    base_rev: Option<&str>,
) -> Result<Option<String>, DomainError> {
    let Some(base_rev) = base_rev else {
        return Ok(None);
    };
    let base: ObjectId = repo.resolve(base_rev)?;
    if repo.object_kind(&base)? == ObjectKind::Commit {
        Ok(Some(repo.find_commit(&base)?.title()))
    } else {
        Ok(None)
    }
}

/// Decodes a text blob to UTF-8 (with `fallback`) and splits it into the lines
/// gitweb numbers — `str::lines`, which drops the empty segment after a trailing
/// newline exactly as gitweb's `while (<$fd>)` loop does.
fn decode_lines(blob: &Blob, fallback: FallbackEncoding) -> Vec<String> {
    to_utf8(blob.bytes(), fallback)
        .lines()
        .map(str::to_owned)
        .collect()
}
