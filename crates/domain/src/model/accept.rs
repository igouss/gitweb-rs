//! Feed content-type negotiation: gitweb's `Accept`-driven `text/xml` fallback.
//!
//! gitweb serves a feed as `application/rss+xml` / `application/atom+xml`, but
//! `git_feed` downgrades to `text/xml` when the reader's `Accept` prefers it —
//! `$cgi->Accept('text/xml') > $cgi->Accept($content_type)`. CGI's `Accept`
//! scores a media type against the header's media ranges, honouring `q=` weights
//! and the `type/*` and `*/*` wildcards, most-specific range first. We mirror
//! that scoring in [`accept_quality`] and the strictly-greater comparison in
//! [`prefer_text_xml_feed`].
//!
//! A request with no `Accept` keeps the feed type (gitweb guards on
//! `defined $cgi->http('HTTP_ACCEPT')`); equal scores keep it too, since the
//! comparison is strict.

/// Whether the reader's `Accept` prefers `text/xml` over `feed_type`, i.e. the
/// feed should be downgraded. `None` (no `Accept` header) keeps the feed type,
/// and so does an equal score, since gitweb's comparison is strict.
#[must_use]
pub fn prefer_text_xml_feed(accept: Option<&str>, feed_type: &str) -> bool {
    match accept {
        None => false,
        Some(header) => accept_quality(header, "text/xml") > accept_quality(header, feed_type),
    }
}

/// CGI's `Accept($type)`: the quality the `Accept` header assigns `media_type`,
/// taking the most specific matching range (exact, then `type/*`, then `*/*`)
/// and `0.0` when nothing matches. Within one specificity the highest `q` wins.
#[must_use]
pub fn accept_quality(header: &str, media_type: &str) -> f64 {
    let (want_type, want_subtype): (&str, &str) = split_media_type(media_type);
    let mut exact: Option<f64> = None;
    let mut type_wildcard: Option<f64> = None;
    let mut full_wildcard: Option<f64> = None;
    for entry in header.split(',') {
        let mut fields = entry.split(';');
        let range: &str = fields.next().unwrap_or("").trim();
        if range.is_empty() {
            continue;
        }
        let quality: f64 = parse_quality(fields);
        let (range_type, range_subtype): (&str, &str) = split_media_type(range);
        let slot: &mut Option<f64> = if range_type == want_type && range_subtype == want_subtype {
            &mut exact
        } else if range_type == want_type && range_subtype == "*" {
            &mut type_wildcard
        } else if range_type == "*" && range_subtype == "*" {
            &mut full_wildcard
        } else {
            continue;
        };
        *slot = Some(slot.map_or(quality, |existing: f64| existing.max(quality)));
    }
    exact.or(type_wildcard).or(full_wildcard).unwrap_or(0.0)
}

/// Splits `"type/subtype"` into its parts; a token with no slash is all type and
/// an empty subtype.
fn split_media_type(media: &str) -> (&str, &str) {
    match media.trim().split_once('/') {
        Some((media_type, subtype)) => (media_type.trim(), subtype.trim()),
        None => (media.trim(), ""),
    }
}

/// Reads the `q=` weight from a media range's parameters, defaulting to `1.0`
/// (and clamping a malformed weight to `1.0`) as HTTP content negotiation does.
fn parse_quality<'a>(parameters: impl Iterator<Item = &'a str>) -> f64 {
    for parameter in parameters {
        let parameter: &str = parameter.trim();
        if let Some(value) = parameter.strip_prefix("q=") {
            return value.trim().parse::<f64>().unwrap_or(1.0);
        }
    }
    1.0
}
