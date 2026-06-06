//! Binary-vs-text content detection.
//!
//! gitweb decides text-vs-binary with Perl's `-T` operator (used by
//! `blob_mimetype`). `-T` is a fuzzy heuristic — NUL byte *or* more than ~30%
//! high-bit/control characters — and it misclassifies valid non-UTF-8 text as
//! binary. We deliberately use git's own canonical rule instead: content is
//! binary iff it contains a NUL byte in the examined window. That is what the
//! "Binary files differ" decision is actually based on in git itself.

/// Bytes examined when looking for a NUL, matching git's `FIRST_FEW_BYTES`.
/// A NUL beyond this window does not flip a long otherwise-text blob.
const WINDOW: usize = 8000;

/// Returns whether `content` should be treated as binary.
#[must_use]
pub fn is_binary(content: &[u8]) -> bool {
    content.iter().take(WINDOW).any(|&byte: &u8| byte == 0)
}
