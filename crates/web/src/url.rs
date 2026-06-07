//! Building gitweb URLs (gitweb's `href()`), the boundary's job.
//!
//! gitweb assembles every internal link from named parameters via `href()`;
//! reading them back is [`crate::request`]'s job. This is the write side: a
//! query-form URL (`/?p=…&a=…`) whose pairs are percent-encoded with the same
//! `application/x-www-form-urlencoded` rules the inbound decoder parses, so the
//! links round-trip exactly. The render layer never builds URLs — it takes the
//! finished strings this produces.

/// Builds a query-form gitweb URL from `params` (gitweb's short CGI names, e.g.
/// `p`, `a`, `o`), percent-encoding each pair. An empty `params` yields `/`.
#[must_use]
pub fn href(params: &[(&str, &str)]) -> String {
    if params.is_empty() {
        return "/".to_owned();
    }
    let query: String = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.iter().copied())
        .finish();
    format!("/?{query}")
}
