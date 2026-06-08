//! The `object` action's two pure rules: which lookup, and which destination.
//!
//! gitweb's `git_object` is the generic object dispatcher. It does not render a
//! view itself — it works out what *kind* of object a request names and issues a
//! `302` redirect to the view for that kind. Two decisions in it are pure, and
//! live here; the repository lookups that sit between them are the use case's job
//! (`crate::usecase::object`).
//!
//! [`resolution`] is gitweb's `if ($hash || ($hash_base && !defined $file_name))
//! … elsif ($hash_base && defined $file_name) … else` ladder: a hash (or the
//! base ref standing in for it when no hash is given) is a direct id lookup; a
//! base ref plus a file is a path lookup under the base; neither is the `400`
//! "Not enough information to find object". The base+file form strips trailing
//! slashes off the file exactly as gitweb's `$file_name =~ s,/+$,,` does.
//!
//! [`target_action`] is gitweb's `action => $type`: the cat-file/ls-tree object
//! type maps one-to-one to the action whose view shows it.

use crate::error::DomainError;
use crate::model::action::Action;
use crate::model::object_kind::ObjectKind;

/// Which lookup the `object` action must perform to learn an object's kind —
/// gitweb's `git_object` `if`/`elsif` branches.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Resolution {
    /// Look the id up directly (gitweb's `cat-file -t $object_id`, where
    /// `$object_id = $hash || $hash_base`).
    ById {
        /// The revision to resolve and read the kind of.
        id: String,
    },
    /// Look a path up under a base revision (gitweb's `ls-tree $hash_base --
    /// $file_name`), with any trailing slashes already stripped from the file.
    ByBasePath {
        /// The base revision the path is resolved against.
        base: String,
        /// The path within the base's tree, trailing slashes stripped.
        file_name: String,
    },
}

/// Classifies an `object` request into the lookup it asks for — gitweb's
/// `git_object` parameter ladder.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] with gitweb's "Not enough information to
/// find object" (a `400`) when the request carries neither a hash nor a base
/// ref.
pub fn resolution(
    hash: Option<&str>,
    hash_base: Option<&str>,
    file_name: Option<&str>,
) -> Result<Resolution, DomainError> {
    // gitweb: `if ($hash || ($hash_base && !defined $file_name))`, with
    // `$object_id = $hash || $hash_base`. A hash always wins; absent one, a base
    // ref with no file to scope it stands in as the id.
    let direct_id: Option<&str> = match (hash, file_name) {
        (Some(hash), _) => Some(hash),
        (None, None) => hash_base,
        (None, Some(_)) => None,
    };
    if let Some(id) = direct_id {
        return Ok(Resolution::ById { id: id.to_owned() });
    }
    // gitweb: `elsif ($hash_base && defined $file_name)` — a base ref plus a file
    // is a path lookup, with trailing slashes stripped (`s,/+$,,`).
    if let (Some(base), Some(file)) = (hash_base, file_name) {
        return Ok(Resolution::ByBasePath {
            base: base.to_owned(),
            file_name: file.trim_end_matches('/').to_owned(),
        });
    }
    // gitweb: `else { die_error(400, "Not enough information to find object") }`.
    Err(DomainError::Invalid(
        "Not enough information to find object".to_owned(),
    ))
}

/// Maps a resolved object kind to the action whose view renders it — gitweb's
/// `action => $type`.
#[must_use]
pub fn target_action(kind: ObjectKind) -> Action {
    match kind {
        ObjectKind::Commit => Action::Commit,
        ObjectKind::Tree => Action::Tree,
        ObjectKind::Blob => Action::Blob,
        ObjectKind::Tag => Action::Tag,
    }
}
