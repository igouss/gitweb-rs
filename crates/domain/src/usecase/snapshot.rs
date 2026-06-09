//! The `snapshot` use case: gitweb's `git_snapshot`, minus the streaming.
//!
//! `git_snapshot` serves a downloadable archive of a tree-ish. It picks the
//! archive format from the request and the site's configured formats, resolves
//! the requested `hash` to an object, rejects a non-tree-ish, and streams
//! `git archive --prefix=<name>/ <hash>` under a content type and a download file
//! name derived from the format and the project. This use case does the same over
//! the [`Repository`] port: the pure [`select_format`] / [`snapshot_name`] rules
//! decide the format and the base name, the port resolves the object and produces
//! the bytes, and the [`SnapshotView`] carries the headers the boundary serves.
//!
//! gitweb stamps both the archive entries and the `Last-Modified` header with the
//! commit time when `hash` names (or peels to) a commit — a bare tree has no such
//! time. The bytes are reproducible: the same tree, format, and commit time
//! always yield the same archive.

use crate::error::DomainError;
use crate::model::commit::Commit;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::snapshot::{
    ArchiveFormat, ArchiveOptions, enabled_formats, select_format, snapshot_name,
};
use crate::model::timestamp::Timestamp;
use crate::port::repository::Repository;

/// The assembled snapshot: the archive bytes and the headers the boundary serves
/// them under.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SnapshotView {
    content_type: &'static str,
    content_disposition: String,
    last_modified: Option<Timestamp>,
    bytes: Vec<u8>,
}

impl SnapshotView {
    /// The `Content-Type` the archive is served under (the format's media type).
    #[must_use]
    pub fn content_type(&self) -> &'static str {
        self.content_type
    }

    /// The `Content-Disposition` offering the archive inline under its file name.
    #[must_use]
    pub fn content_disposition(&self) -> &str {
        &self.content_disposition
    }

    /// The commit time the snapshot is dated with, when `hash` named a commit —
    /// the boundary renders it as the `Last-Modified` header.
    #[must_use]
    pub fn last_modified(&self) -> Option<&Timestamp> {
        self.last_modified.as_ref()
    }

    /// The archive bytes, borrowed.
    #[must_use]
    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    /// Consumes the view, returning the archive bytes for the boundary to stream.
    #[must_use]
    pub fn into_bytes(self) -> Vec<u8> {
        self.bytes
    }
}

/// Assembles the snapshot view over the [`Repository`] port.
///
/// `project` is the project path (gitweb's `$project`), `configured_formats` the
/// site's `snapshot` feature options, `requested_format` the `sf` parameter,
/// `hash` the `h` parameter (a ref name or object id), and `branch_refs` the ref
/// directories that count as branches when naming the archive (gitweb's
/// `get_branch_refs`).
///
/// # Errors
///
/// The format-selection cascade's [`DomainError`] (gitweb's `die_error` 400/403);
/// [`NotFound`](DomainError::NotFound) when no `hash` is given or it resolves to
/// nothing (gitweb's `Object does not exist`); [`Invalid`](DomainError::Invalid)
/// when the object is not a tree-ish (a blob); and any error the repository raises
/// resolving the object or producing the bytes.
pub fn assemble_snapshot(
    repo: &dyn Repository,
    project: &str,
    configured_formats: &[String],
    requested_format: Option<&str>,
    hash: Option<&str>,
    branch_refs: &[&str],
) -> Result<SnapshotView, DomainError> {
    // gitweb validates the format before touching the object: an off feature is a
    // 403 regardless of the hash.
    let enabled: Vec<ArchiveFormat> = enabled_formats(configured_formats);
    let format: ArchiveFormat = select_format(requested_format, &enabled)?;

    let hash: &str =
        hash.ok_or_else(|| DomainError::NotFound("Object does not exist".to_owned()))?;
    let oid: ObjectId = repo.resolve(hash)?;

    // The commit behind the snapshot, when there is one: a commit names itself, an
    // annotated tag of a commit peels one level. It dates both the archive entries
    // and the Last-Modified header (gitweb's `parse_commit($hash)`); a bare tree
    // has neither.
    let commit: Option<Commit> = snapshot_commit(repo, &oid)?;
    let modification_time: i64 = commit
        .as_ref()
        .map_or(0, |commit: &Commit| commit.committer().epoch());
    let last_modified: Option<Timestamp> = commit
        .as_ref()
        .map(|commit: &Commit| Timestamp::from_signature(commit.committer()));

    let abbrev: usize = repo.abbrev_length()?;
    let short: &str = oid.abbreviated_to(abbrev);
    let name: String = snapshot_name(project, hash, short, branch_refs);

    // The archive itself: the port peels `oid` to its tree, rejecting a non-tree
    // (gitweb's `Object is not a tree-ish`), and wraps every entry in `name/`.
    let options: ArchiveOptions = ArchiveOptions {
        prefix: name.clone(),
        modification_time,
    };
    let bytes: Vec<u8> = repo.archive(&oid, format, &options)?;

    Ok(SnapshotView {
        content_type: format.content_type(),
        content_disposition: format!("inline; filename=\"{name}{}\"", format.suffix()),
        last_modified,
        bytes,
    })
}

/// The commit that dates a snapshot of `oid`, when there is one: `oid` itself if
/// it is a commit, or the target of an annotated tag of a commit. A tree, a blob,
/// or a tag of a non-commit has no commit, and so no date — gitweb's
/// `parse_commit($hash)` simply yields nothing.
fn snapshot_commit(repo: &dyn Repository, oid: &ObjectId) -> Result<Option<Commit>, DomainError> {
    match repo.object_kind(oid)? {
        ObjectKind::Commit => Ok(Some(repo.find_commit(oid)?)),
        ObjectKind::Tag => {
            let tag = repo.find_tag(oid)?;
            if tag.object_kind() == ObjectKind::Commit {
                Ok(Some(repo.find_commit(tag.object())?))
            } else {
                Ok(None)
            }
        }
        ObjectKind::Tree | ObjectKind::Blob => Ok(None),
    }
}
