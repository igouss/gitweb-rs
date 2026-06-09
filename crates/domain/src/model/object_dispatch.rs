//! gitweb's no-action default object lookup (the `dispatch` sub's `git_get_type`
//! branch).
//!
//! gitweb's `dispatch` sub, when a request names no action, does not redirect:
//! if it carries a hash — or a base ref plus a file — it learns the object's
//! kind with `git_get_type` and serves the matching view *inline* for the same
//! request. This rule is the pure half of that: which object `git_get_type` is
//! asked about. gitweb's `if (defined $hash) … elsif (defined $hash_base &&
//! defined $file_name) …` — a hash names the object directly; a base ref plus a
//! file names the object at that path under the base.
//!
//! This is *not* `git_object`'s [`resolution`](crate::model::object_redirect)
//! ladder, and the differences are the point:
//! - A hash is tested first and outranks a base ref.
//! - A base ref *alone* names no object here. `git_object` lets a bare base ref
//!   stand in as an id; `dispatch` does not — a bare project routes to the
//!   summary, so object resolution is only ever reached with a hash or a
//!   base+file (see [`route`](crate::model::routing::route)). The classifier
//!   reports [`None`] for the bare-base case rather than inventing an id.
//! - No trailing slash is stripped off the file (`git_object` does `s,/+$,,`;
//!   `dispatch` passes `"$hash_base:$file_name"` through verbatim).
//!
//! The kind-to-action half is identical to `git_object`'s, so the use case
//! reuses [`target_action`](crate::model::object_redirect::target_action); only
//! the lookup choice and the `git_get_type` miss strings differ, and those are
//! the use case's job (`crate::usecase::object_dispatch`).

/// Which object gitweb's `dispatch` asks `git_get_type` about when no action was
/// named — gitweb's `if (defined $hash) … elsif (defined $hash_base && defined
/// $file_name) …`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DispatchLookup {
    /// The object the hash names directly (gitweb's `git_get_type($hash)`).
    ByHash {
        /// The revision to read the kind of.
        hash: String,
    },
    /// The object at a path under a base revision (gitweb's
    /// `git_get_type("$hash_base:$file_name")`), with the file passed through
    /// verbatim — no trailing-slash stripping.
    ByBasePath {
        /// The base revision the path is resolved against.
        base: String,
        /// The path within the base's tree, exactly as given.
        file_name: String,
    },
}

/// Classifies which object gitweb's no-action `dispatch` resolves the kind of —
/// gitweb's `if (defined $hash) … elsif (defined $hash_base && defined
/// $file_name) …`. A hash outranks a base ref; a base ref alone (or a file with
/// no base, or nothing) names no object to resolve and yields [`None`] — the
/// routing rule never reaches this with such a request, so it is a no-op the
/// classifier reports rather than guesses around.
#[must_use]
pub fn dispatch_lookup(
    hash: Option<&str>,
    hash_base: Option<&str>,
    file_name: Option<&str>,
) -> Option<DispatchLookup> {
    // gitweb: `if (defined $hash)` — a hash names the object, outranking a base
    // ref, exactly as dispatch tests `$hash` before `$hash_base`.
    if let Some(hash) = hash {
        return Some(DispatchLookup::ByHash {
            hash: hash.to_owned(),
        });
    }
    // gitweb: `elsif (defined $hash_base && defined $file_name)` — a base ref
    // plus a file is `git_get_type("$hash_base:$file_name")`, the file verbatim.
    if let (Some(base), Some(file)) = (hash_base, file_name) {
        return Some(DispatchLookup::ByBasePath {
            base: base.to_owned(),
            file_name: file.to_owned(),
        });
    }
    // A base ref alone, a file with no base, or nothing names no object —
    // dispatch's outer ladder never routes such a request here.
    None
}
