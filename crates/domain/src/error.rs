//! The error vocabulary the ports fail with.
//!
//! These are the domain-level failure modes a [`Repository`] or
//! [`ProjectStore`] can return, independent of any transport. Turning them into
//! gitweb's `die_error` HTTP status codes and messages is a separate concern
//! handled at the web boundary, not here.
//!
//! [`Repository`]: crate::port::repository::Repository
//! [`ProjectStore`]: crate::port::project_store::ProjectStore

use std::fmt;

/// A failure surfaced by a domain port.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DomainError {
    /// A requested object, ref, or project does not exist.
    NotFound(String),
    /// Input named an object or ref but it was malformed or the wrong kind.
    Invalid(String),
    /// Access to the resource is denied by policy (export rules, a disabled
    /// feature, an unmet precondition) — gitweb's `die_error(403, ...)` cases.
    Forbidden(String),
    /// The underlying git backend failed for a reason outside the domain.
    Backend(String),
}

impl fmt::Display for DomainError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(what) => write!(f, "not found: {what}"),
            Self::Invalid(what) => write!(f, "invalid: {what}"),
            Self::Forbidden(what) => write!(f, "forbidden: {what}"),
            Self::Backend(why) => write!(f, "backend error: {why}"),
        }
    }
}

impl std::error::Error for DomainError {}
