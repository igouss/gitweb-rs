//! Building gitweb URLs (gitweb's `href()`), the boundary's job.
//!
//! gitweb assembles every internal link from named parameters via `href()`;
//! reading them back is [`crate::request`]'s job. This is the write side: a
//! query-form URL (`/?p=…&a=…`) whose pairs are percent-encoded with the same
//! `application/x-www-form-urlencoded` rules the inbound decoder parses, so the
//! links round-trip exactly. The render layer never builds URLs — it takes the
//! finished strings this produces.
//!
//! Two dialects live here. [`href`] is the *modernized* relative link the HTML
//! pages use (`&`-joined, `form_urlencoded`). [`href_full`] / [`self_url`] are
//! the *byte-faithful* gitweb forms the format-stable feeds emit: absolute
//! (`http://host/?…`), parameters in gitweb's canonical `@cgi_param_mapping`
//! order, joined with `;`, each value percent-encoded with gitweb's `esc_param`
//! — so the syndication output matches real gitweb to the byte.

use gitweb_render::escape::esc_param;

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

/// gitweb's `@cgi_param_mapping`: the short CGI names in declaration order.
/// `href_full` emits whatever `params` it is given in this order — never call
/// order — so a feed URL is byte-stable regardless of how the caller listed the
/// pairs. (Mirrors `gitweb.perl`'s `for (my $i = 0; $i < @cgi_param_mapping; …)`
/// emission loop.)
const PARAM_ORDER: [&str; 19] = [
    "p", "a", "f", "fp", "h", "hp", "hb", "hpb", "pg", "o", "s", "st", "sf", "opt", "sr", "by_tag",
    "ds", "pf", "js",
];

/// The position of `key` in gitweb's parameter order; unknown keys sort after
/// every known one (keeping their relative order), so a stray param never
/// reorders the known ones.
fn param_rank(key: &str) -> usize {
    PARAM_ORDER
        .iter()
        .position(|known: &&str| *known == key)
        .unwrap_or(PARAM_ORDER.len())
}

/// The gitweb query string for `params`: the pairs ordered by
/// [`PARAM_ORDER`], each value `esc_param`-encoded, joined with `;` — gitweb's
/// `join(';', @result)`. Empty when `params` is empty.
fn query_string(params: &[(&str, &str)]) -> String {
    let mut ordered: Vec<(&str, &str)> = params.to_vec();
    // A stable sort by rank keeps gitweb's declaration order and leaves
    // same-rank (e.g. repeated) params in call order.
    ordered.sort_by_key(|(key, _): &(&str, &str)| param_rank(key));
    ordered
        .iter()
        .map(|(key, value): &(&str, &str)| format!("{key}={}", esc_param(value)))
        .collect::<Vec<String>>()
        .join(";")
}

/// Builds gitweb's `href(-full => 1, …)`: an absolute URL rooted at `base` (a
/// scheme-and-host with no trailing slash, e.g. `http://localhost`), then `/?`
/// and the [`query_string`] of `params`. With no params it is just `base` plus
/// `/` — gitweb's `$cgi->url()` (`http://localhost/`). This is the form every
/// link inside an RSS/Atom feed takes.
#[must_use]
pub fn href_full(base: &str, params: &[(&str, &str)]) -> String {
    if params.is_empty() {
        return format!("{base}/");
    }
    format!("{base}/?{}", query_string(params))
}

/// gitweb's `$cgi->self_url()` for a feed request: the absolute URL of the
/// request itself, which CGI.pm renders as the host with *no* trailing slash,
/// then `?` and the request's own parameters (`http://localhost?p=…;a=atom`).
/// `params` are the request's CGI params; the host quirk (no slash, unlike
/// [`href_full`]) is reproduced verbatim so the Atom `<link rel="self">` matches
/// gitweb to the byte.
#[must_use]
pub fn self_url(base: &str, params: &[(&str, &str)]) -> String {
    format!("{base}?{}", query_string(params))
}
