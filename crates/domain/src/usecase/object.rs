//! The `object` use case: gitweb's `git_object` redirect target.
//!
//! `git_object` is the generic object dispatcher. It does not render anything —
//! it learns the *kind* of the object a request names and `302`-redirects to the
//! view for that kind. This use case carries out the repository half of that:
//! it classifies the request with [`resolution`], performs the lookup the
//! classification calls for, and returns the [`ObjectRedirect`] the boundary
//! turns into a `Location`. The pure decisions — which lookup, and which action
//! a kind maps to — live in [`crate::model::object_redirect`]; this is only the
//! orchestration over the [`Repository`] port.
//!
//! The lookup mirrors gitweb exactly:
//! - a direct id (gitweb's `cat-file -t $object_id`) resolves the revision and
//!   reads its kind; either miss is the `404` "Object does not exist".
//! - a base+path (gitweb's `cat-file -e $hash_base` then `ls-tree`) resolves the
//!   base (`404` "Base object does not exist"), looks the path up under it
//!   (`404` "File or directory for given base does not exist"), and takes the
//!   entry's kind from its mode — `ls-tree`'s type column — so a gitlink reads as
//!   a commit without its commit object existing here.
//!
//! The redirect carries the request's original hash / base / file through
//! unchanged, except the base+path form replaces the hash with the object id the
//! path resolved to — exactly as gitweb sets `$hash = $3` from the `ls-tree` line
//! while leaving `$hash_base` and `$file_name` as given.

use crate::error::DomainError;
use crate::model::action::Action;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::object_redirect::{Resolution, resolution, target_action};
use crate::model::tree::TreeEntry;
use crate::port::repository::Repository;

/// Where the `object` action redirects: the destination action and the
/// parameters that name the object under it. Each parameter is `None` when the
/// request did not carry it, so the boundary emits only the ones gitweb's
/// `href` would.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ObjectRedirect {
    /// The action whose view renders the resolved object kind.
    pub action: Action,
    /// The `h` parameter — the request's hash, or the path's resolved id for the
    /// base+path form.
    pub hash: Option<String>,
    /// The `hb` parameter — the request's base ref, carried through unchanged.
    pub hash_base: Option<String>,
    /// The `f` parameter — the request's file, trailing slashes stripped for the
    /// base+path form.
    pub file_name: Option<String>,
}

/// Resolves an `object` request to its redirect target over the [`Repository`]
/// port — gitweb's `git_object`.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] (gitweb's `400`) when the request names
/// neither a hash nor a base ref ("Not enough information to find object"), and
/// [`DomainError::NotFound`] (gitweb's `404`) when the id, the base, or the path
/// under the base does not resolve.
pub fn assemble_object_redirect(
    repo: &dyn Repository,
    hash: Option<&str>,
    hash_base: Option<&str>,
    file_name: Option<&str>,
) -> Result<ObjectRedirect, DomainError> {
    match resolution(hash, hash_base, file_name)? {
        Resolution::ById { id } => redirect_by_id(repo, &id, hash, hash_base, file_name),
        Resolution::ByBasePath { base, file_name } => {
            redirect_by_base_path(repo, &base, &file_name)
        }
    }
}

/// gitweb's `cat-file -t $object_id` branch: resolve the id and read its kind,
/// carrying the request's hash / base / file through to the redirect. Any miss
/// is the `404` "Object does not exist".
fn redirect_by_id(
    repo: &dyn Repository,
    id: &str,
    hash: Option<&str>,
    hash_base: Option<&str>,
    file_name: Option<&str>,
) -> Result<ObjectRedirect, DomainError> {
    let kind: ObjectKind = kind_of_id(repo, id).map_err(|_| object_missing())?;
    Ok(ObjectRedirect {
        action: target_action(kind),
        hash: hash.map(str::to_owned),
        hash_base: hash_base.map(str::to_owned),
        file_name: file_name.map(str::to_owned),
    })
}

/// gitweb's `cat-file -e $hash_base` + `ls-tree` branch: resolve the base, look
/// the path up under it, and take the entry's kind from its mode — `ls-tree`'s
/// type column, which git derives from the mode, so a gitlink reads as a commit
/// without its commit object existing here. The redirect carries the resolved
/// object id as its hash (gitweb's `$hash = $3`), with the base and the
/// (slash-stripped) file unchanged. The base miss and the path miss are the two
/// distinct `404`s; the kind never fails, since it is read from the mode rather
/// than the (possibly-absent) object.
fn redirect_by_base_path(
    repo: &dyn Repository,
    base: &str,
    file_name: &str,
) -> Result<ObjectRedirect, DomainError> {
    let base_oid: ObjectId = repo.resolve(base).map_err(|_| base_missing())?;
    let entry: TreeEntry = repo
        .path_entry(&base_oid, file_name)?
        .ok_or_else(path_missing)?;
    // gitweb classifies the path by `ls-tree`'s mode-derived type column, not by
    // `cat-file`-ing the object: a gitlink (mode 160000) names a commit that
    // lives in the submodule and is absent here, yet still redirects to the
    // commit view. Reading the recorded id would 404 instead.
    let kind: ObjectKind = entry.mode().object_kind();
    Ok(ObjectRedirect {
        action: target_action(kind),
        hash: Some(entry.oid().as_str().to_owned()),
        hash_base: Some(base.to_owned()),
        file_name: Some(file_name.to_owned()),
    })
}

/// Resolves a revision and reads the kind of what it names (gitweb's
/// `cat-file -t`).
fn kind_of_id(repo: &dyn Repository, id: &str) -> Result<ObjectKind, DomainError> {
    let oid: ObjectId = repo.resolve(id)?;
    repo.object_kind(&oid)
}

/// gitweb's `git_object` `die_error(404, "Object does not exist")`.
fn object_missing() -> DomainError {
    DomainError::NotFound("Object does not exist".to_owned())
}

/// gitweb's `git_object` `die_error(404, "Base object does not exist")`.
fn base_missing() -> DomainError {
    DomainError::NotFound("Base object does not exist".to_owned())
}

/// gitweb's `git_object`
/// `die_error(404, "File or directory for given base does not exist")`.
fn path_missing() -> DomainError {
    DomainError::NotFound("File or directory for given base does not exist".to_owned())
}
