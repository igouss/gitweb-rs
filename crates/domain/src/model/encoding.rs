//! Decoding git bytes into text.
//!
//! Mirrors gitweb's `to_utf8`: bytes that are already valid UTF-8 are kept as
//! they are; otherwise they are decoded through a fallback encoding. gitweb's
//! default fallback is latin1, and the actual fallback is configuration the
//! caller supplies — this primitive only owns the decode rule.

/// A fallback encoding used when bytes are not valid UTF-8.
///
/// Only latin1 (gitweb's default) is modelled today; the enum leaves room for
/// the configurable `$fallback_encoding` to grow without changing call sites.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum FallbackEncoding {
    /// ISO-8859-1: each byte maps to the code point of the same value.
    #[default]
    Latin1,
}

impl FallbackEncoding {
    /// Decodes bytes known not to be valid UTF-8 through this encoding.
    fn decode(self, bytes: &[u8]) -> String {
        match self {
            Self::Latin1 => bytes.iter().map(|&byte: &u8| byte as char).collect(),
        }
    }
}

/// Decodes git bytes to a `String`: the bytes verbatim if valid UTF-8,
/// otherwise via `fallback`.
#[must_use]
pub fn to_utf8(bytes: &[u8], fallback: FallbackEncoding) -> String {
    match std::str::from_utf8(bytes) {
        Ok(text) => text.to_owned(),
        Err(_) => fallback.decode(bytes),
    }
}
