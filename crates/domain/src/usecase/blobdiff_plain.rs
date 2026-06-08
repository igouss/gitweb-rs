//! The `blobdiff_plain` use case: one file's diff as a bare unified patch.
//!
//! gitweb's `git_blobdiff('plain')` streams `git diff-tree -r -p
//! <hash_parent_base> <hash_base> -- <file>` for a single path, framed with a
//! lone `X-Git-Url` line. This use case assembles that body from the
//! [`Repository`] port: it reads the two-tree patch between the parent base and
//! the base (gitweb's site-default rename detection, `-M`), then renders just
//! the one file selected by its new-side path, with the repository's default
//! `index` abbreviation — the short-id form bare `git diff-tree -p` writes,
//! since the plain format does not pass `--full-index`.
//!
//! A path the diff does not touch is gitweb's `die_error(404, "Blob diff not
//! found")`. The `self_url` for the `X-Git-Url` line is the boundary's job, so
//! the produced [`BlobdiffPlain`] carries everything but that link, exactly as
//! [`commitdiff_plain`](crate::usecase::commitdiff_plain) does.
//!
//! [`Repository`]: crate::port::repository::Repository

use crate::error::DomainError;
use crate::model::blobdiff_plain::BlobdiffPlain;
use crate::model::object_id::ObjectId;
use crate::model::patch::Patch;
use crate::port::repository::{RenameDetection, Repository};

/// gitweb's site-default rename detection (`@diff_opts = ('-M')`): renames only,
/// the same default the `commitdiff` / `commitdiff_plain` views use, so a renamed
/// path is paired old→new and its single-file patch carries the rename headers.
const DETECTION: RenameDetection = RenameDetection::RenamesOnly;

/// Assembles the `blobdiff_plain` body for `file_name` between the two bases:
/// the patch between `hash_parent_base` (from) and `hash_base` (to), narrowed to
/// the one file whose new-side path is `file_name`, abbreviated the way the plain
/// format streams it.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] with gitweb's `Blob diff not found` message
/// when no file in the diff has that new-side path, and propagates the
/// repository's failures (an unresolvable base, a patch read, the abbreviation
/// length).
pub fn assemble_blobdiff_plain(
    repo: &dyn Repository,
    hash_base: &str,
    hash_parent_base: &str,
    file_name: &str,
) -> Result<BlobdiffPlain, DomainError> {
    let to: ObjectId = repo.resolve(hash_base)?;
    let from: ObjectId = repo.resolve(hash_parent_base)?;
    let patch: Patch = repo.patch(Some(&from), &to, DETECTION)?;
    let abbrev: usize = repo.abbrev_length()?;
    let body: String = patch
        .render_file_abbreviated(file_name, abbrev)
        .ok_or_else(|| DomainError::NotFound("Blob diff not found".to_owned()))?;
    Ok(BlobdiffPlain::new(body))
}
