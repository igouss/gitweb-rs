//! Validation of untrusted request parameters before any filesystem or
//! repository access.
//!
//! Mirrors gitweb's `is_valid_pathname`, `is_valid_ref_format` and
//! `is_valid_refname`. These are the front line against directory traversal
//! (`../`) and malformed-ref attacks: every `f=` path and `h=`/`hb=` ref or
//! hash coming off the wire must pass through here. The resulting newtypes are
//! the only way to name a path or ref to the rest of the system, so "validate
//! before access" is enforced by the type system, not by discipline.

use crate::model::object_id::ObjectId;

/// A request path that is safe to resolve against a tree or the filesystem.
///
/// Construction *is* the validation: a `SafePath` exists only if the input had
/// no empty, `.` or `..` path element (which also rejects leading, trailing and
/// doubled slashes) and no NUL byte — gitweb's `is_valid_pathname`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafePath {
    value: String,
}

impl SafePath {
    /// Validates an untrusted path, returning `None` if it is unsafe.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        if is_valid_pathname(input) {
            Some(Self {
                value: input.to_owned(),
            })
        } else {
            None
        }
    }

    /// The validated path text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// A revision selector that is safe to resolve: either a full object id, or a
/// ref name that is a safe path and satisfies git-check-ref-format.
///
/// Mirrors gitweb's `is_valid_refname`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SafeRef {
    value: String,
}

impl SafeRef {
    /// Validates an untrusted ref or hash, returning `None` if it is unsafe.
    #[must_use]
    pub fn parse(input: &str) -> Option<Self> {
        if is_valid_refname(input) {
            Some(Self {
                value: input.to_owned(),
            })
        } else {
            None
        }
    }

    /// The validated ref text.
    #[must_use]
    pub fn as_str(&self) -> &str {
        &self.value
    }
}

/// gitweb's `is_valid_pathname`: no empty, `.` or `..` path element, no NUL.
///
/// Splitting on `/` makes leading, trailing and doubled slashes show up as
/// empty elements, so a single element check covers all of gitweb's regex
/// cases. Note that `..foo` is fine — only a *whole* element of `.` or `..` is
/// rejected, not dots that merely begin a name.
fn is_valid_pathname(input: &str) -> bool {
    if input.contains('\0') {
        return false;
    }
    !input
        .split('/')
        .any(|element: &str| element.is_empty() || element == "." || element == "..")
}

/// gitweb's `is_valid_ref_format`: the git-check-ref-format restrictions —
/// no component starting with `.` (`/.`), no `..` anywhere, no trailing slash,
/// and none of the control/space/glob/ref-special bytes.
fn is_valid_ref_format(input: &str) -> bool {
    if input.contains("/.") || input.contains("..") || input.ends_with('/') {
        return false;
    }
    !input.bytes().any(|byte: u8| {
        byte <= 0x20 || byte == 0x7f || matches!(byte, b'~' | b'^' | b':' | b'?' | b'*' | b'[')
    })
}

/// gitweb's `is_valid_refname`: a full object id is always valid; otherwise the
/// name must be a valid pathname that also satisfies git-check-ref-format.
fn is_valid_refname(input: &str) -> bool {
    if ObjectId::parse(input).is_some() {
        return true;
    }
    is_valid_pathname(input) && is_valid_ref_format(input)
}
