//! Markup primitives and the single raw-HTML safe sink.
//!
//! The render layer is built on [`maud`], a compile-time templating engine that
//! **escapes interpolated values by default**. That default is the security
//! property the conventions bead demands: any git-derived text written into a
//! template — a filename, a commit subject, an author name — is HTML-escaped
//! automatically, so it can never break out of its context.
//!
//! Raw HTML therefore has to be *opted into*, and it enters through exactly one
//! door: [`raw`]. gitweb has a handful of deliberately-raw spots — a rendered
//! `README.html`, and the visible control-character spans produced by the
//! [`crate::escape`] family — whose output is already safe HTML and must be
//! emitted verbatim rather than escaped a second time. Funnelling all of them
//! through one named, documented function keeps the raw-HTML surface auditable:
//! grep for `raw(` and every untrusted-becomes-trusted boundary is right there.
//!
//! The canonical maud items are re-exported here so the rest of the codebase has
//! a single import path (`gitweb_render::markup`) and never depends on `maud`
//! directly.

pub use maud::{DOCTYPE, Markup, PreEscaped, Render, html};

/// The one raw-HTML safe sink: emit an already-safe HTML string verbatim,
/// bypassing maud's auto-escaping.
///
/// # Safety contract
///
/// This does **not** sanitise. The caller guarantees the string is already safe
/// HTML — produced by [`crate::escape`], by a trusted template, or by a
/// sanitiser. Passing raw untrusted git/user input here reintroduces the very
/// injection the default-escaping path prevents, so don't.
#[must_use]
pub fn raw(html: impl Into<String>) -> Markup {
    PreEscaped(html.into())
}
