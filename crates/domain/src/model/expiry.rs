//! By-oid HTTP cache freshness (gitweb's `$expires = "+1d"`).
//!
//! gitweb stamps an `Expires` header — a one-day freshness window — on a response
//! whose primary hash is a literal object id, because content addressed by an
//! immutable oid may safely be cached for a day:
//! `if ($hash =~ m/^$oid_regex$/) { $expires = "+1d"; }`. A symbolic ref (HEAD, a
//! branch name), an abbreviated id, or an absent hash earns no `Expires` — such
//! content can move under the same URL and must be revalidated.
//!
//! This is the pure decision; the boundary turns an [`Expiry::OneDay`] into the
//! absolute `Expires` date (the request clock plus the window) the way CGI.pm
//! resolves `"+1d"` at header-emit time.

use crate::model::object_id::ObjectId;

/// Seconds in gitweb's `"+1d"` freshness window (one day).
const ONE_DAY_SECONDS: i64 = 24 * 60 * 60;

/// The cache freshness lifetime a served body carries (gitweb's `$expires`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Expiry {
    /// No freshness window — ref-named, abbreviated, or unresolved content, which
    /// can change under the same URL and must be revalidated.
    #[default]
    None,
    /// One day — gitweb's `"+1d"` for content addressed by a literal object id,
    /// which is immutable and may be held by a cache for a day.
    OneDay,
}

impl Expiry {
    /// gitweb's by-oid rule: the freshness a view carries given the hash it is
    /// addressed by (the `$hash` its `Expires` site tests). A literal object id
    /// (`$hash =~ /^$oid_regex$/` — a full 40- or 64-character hex id) earns one
    /// day; a ref name, an abbreviation, or no hash earns none.
    #[must_use]
    pub fn for_hash(hash: Option<&str>) -> Self {
        match hash {
            Some(hash) if ObjectId::parse(hash).is_some() => Self::OneDay,
            _ => Self::None,
        }
    }

    /// The freshness window in seconds — one day for [`Expiry::OneDay`], `None`
    /// when no `Expires` is carried.
    #[must_use]
    pub fn seconds(self) -> Option<i64> {
        match self {
            Self::OneDay => Some(ONE_DAY_SECONDS),
            Self::None => None,
        }
    }
}
