//! The no-action default object dispatch: gitweb's `dispatch` `git_get_type`.
//!
//! gitweb's `dispatch` sub, when a request names no action but carries a hash —
//! or a base ref plus a file — resolves the referenced object's *kind* with
//! `git_get_type` and serves the matching view inline (NOT a redirect; that is
//! `git_object`'s job, [`crate::usecase::object`]). This use case does the
//! repository half: it picks the lookup ([`dispatch_lookup`]), reads the kind
//! over the [`Repository`] port, and returns the action whose view shows it
//! (reusing [`target_action`], identical to `git_object`'s kind-to-action map).
//!
//! The misses are gitweb's `dispatch` strings — distinct from `git_object`'s:
//! - a hash that names nothing is `404` "Object does not exist";
//! - a base+file that resolves to nothing — whether the base or the path is the
//!   miss — is the one `404` "File or directory does not exist". gitweb collapses
//!   both into a single `git_get_type("$hash_base:$file_name")` that returns
//!   undef, so there is no separate "base does not exist" here.

use crate::error::DomainError;
use crate::model::action::Action;
use crate::model::object_dispatch::{DispatchLookup, dispatch_lookup};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::object_redirect::target_action;
use crate::port::repository::Repository;

/// Resolves gitweb's no-action default to the action whose view serves the
/// referenced object — gitweb's `dispatch` `$action = git_get_type(...)`.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] with gitweb's `dispatch` strings: "Object
/// does not exist" when a hash names nothing, "File or directory does not exist"
/// when a base+file resolves to nothing.
pub fn resolve_dispatch_action(
    repo: &dyn Repository,
    hash: Option<&str>,
    hash_base: Option<&str>,
    file_name: Option<&str>,
) -> Result<Action, DomainError> {
    let kind: ObjectKind = match dispatch_lookup(hash, hash_base, file_name) {
        Some(DispatchLookup::ByHash { hash }) => kind_of_id(repo, &hash)?,
        Some(DispatchLookup::ByBasePath { base, file_name }) => {
            kind_of_base_path(repo, &base, &file_name)?
        }
        // Unreachable via the routing rule (a bare project routes to the summary,
        // never here); if nothing names an object, nothing exists.
        None => return Err(object_missing()),
    };
    Ok(target_action(kind))
}

/// gitweb's `git_get_type($hash)`: resolve the revision and read its kind. Any
/// miss is the `dispatch` `404` "Object does not exist".
fn kind_of_id(repo: &dyn Repository, id: &str) -> Result<ObjectKind, DomainError> {
    let oid: ObjectId = repo.resolve(id).map_err(|_| object_missing())?;
    repo.object_kind(&oid).map_err(|_| object_missing())
}

/// gitweb's `git_get_type("$hash_base:$file_name")`: resolve the base, look the
/// path up under it, read the found object's kind. The base miss, the path miss,
/// and a kind read that fails all collapse to the one `dispatch` `404` "File or
/// directory does not exist".
fn kind_of_base_path(
    repo: &dyn Repository,
    base: &str,
    file_name: &str,
) -> Result<ObjectKind, DomainError> {
    let base_oid: ObjectId = repo.resolve(base).map_err(|_| file_missing())?;
    let object_oid: ObjectId = repo
        .path_id(&base_oid, file_name)
        .map_err(|_| file_missing())?
        .ok_or_else(file_missing)?;
    repo.object_kind(&object_oid).map_err(|_| file_missing())
}

/// gitweb's `dispatch` `die_error(404, "Object does not exist")`.
fn object_missing() -> DomainError {
    DomainError::NotFound("Object does not exist".to_owned())
}

/// gitweb's `dispatch` `die_error(404, "File or directory does not exist")`.
fn file_missing() -> DomainError {
    DomainError::NotFound("File or directory does not exist".to_owned())
}
