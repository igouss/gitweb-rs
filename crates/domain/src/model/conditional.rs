//! Conditional-GET freshness: gitweb's `exit_if_unmodified_since` as a rule.
//!
//! gitweb dates its feeds and snapshots with a `Last-Modified` and, when the
//! client echoes it back in `If-Modified-Since`, compares the two: a resource
//! whose timestamp is no newer than the client's cached copy
//! (`$latest_epoch <= $since`) is *unmodified* and earns a bare `304 Not
//! Modified`; anything else is served afresh. A missing or unparseable header
//! always serves — gitweb only short-circuits when `$since` is defined.
//!
//! The comparison is the whole business rule and lives here; turning the verdict
//! into a `304`-vs-body HTTP response is the web boundary's job. The only
//! supporting work is parsing the client's HTTP-date back into an epoch, which
//! [`parse_http_date`] does for the RFC 1123 / RFC 2822 forms gitweb emits and
//! browsers send (a leading weekday is optional; `GMT`/`UTC` and numeric `+HHMM`
//! zones are honoured). Anything it cannot parse yields `None`, which the rule
//! treats as "serve".

use crate::model::timestamp::days_from_civil;

const SECONDS_PER_DAY: i64 = 86_400;

/// Whether a resource is unmodified relative to the client's cached copy.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Freshness {
    /// The client's copy is current — serve `304 Not Modified` with no body.
    NotModified,
    /// The resource changed (or no usable validator was sent) — serve it.
    Modified,
}

/// gitweb's `exit_if_unmodified_since`: a resource dated no later than the
/// client's `If-Modified-Since` is [`Freshness::NotModified`]; a missing or
/// unparseable header, or a resource newer than the cache, is
/// [`Freshness::Modified`].
#[must_use]
pub fn freshness(resource_epoch: i64, if_modified_since: Option<&str>) -> Freshness {
    match if_modified_since.and_then(parse_http_date) {
        Some(since) if resource_epoch <= since => Freshness::NotModified,
        _ => Freshness::Modified,
    }
}

/// Parses an HTTP `If-Modified-Since` date into a Unix epoch in UTC, or `None`
/// if it is not one of the recognised forms.
///
/// Handles the RFC 1123 / RFC 2822 shape gitweb emits and browsers send —
/// `[Wdy, ]DD Mon YYYY HH:MM:SS ZONE` — where the leading weekday is optional and
/// the zone is `GMT`/`UTC` (UTC) or a numeric `+HHMM`/`-HHMM` offset that is
/// normalised away. Legacy RFC 850 and asctime forms are not parsed; they fall
/// through to `None`, which [`freshness`] treats as "serve".
#[must_use]
pub fn parse_http_date(value: &str) -> Option<i64> {
    let trimmed: &str = value.trim();
    // Drop an optional leading weekday token ("Wed, ").
    let rest: &str = match trimmed.split_once(", ") {
        Some((_weekday, rest)) => rest,
        None => trimmed,
    };
    let mut parts = rest.split_whitespace();
    let day: u8 = parts.next()?.parse().ok()?;
    let month: u8 = month_number(parts.next()?)?;
    let year: i64 = parts.next()?.parse().ok()?;
    let (hour, minute, second): (i64, i64, i64) = parse_hms(parts.next()?)?;
    // A missing zone is read as UTC, as HTTP-date requires.
    let zone_offset: i64 = parts.next().map_or(0, parse_zone_offset);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    let days: i64 = days_from_civil(year, month, day);
    let local: i64 = days * SECONDS_PER_DAY + hour * 3600 + minute * 60 + second;
    Some(local - zone_offset)
}

/// Maps a three-letter English month abbreviation to its 1–12 number.
fn month_number(token: &str) -> Option<u8> {
    const MONTHS: [&str; 12] = [
        "jan", "feb", "mar", "apr", "may", "jun", "jul", "aug", "sep", "oct", "nov", "dec",
    ];
    let key: String = token.to_ascii_lowercase();
    MONTHS
        .iter()
        .position(|name: &&str| *name == key)
        .map(|index: usize| (index + 1) as u8)
}

/// Parses an `HH:MM:SS` (or `HH:MM`) time into its three components.
fn parse_hms(token: &str) -> Option<(i64, i64, i64)> {
    let mut fields = token.split(':');
    let hour: i64 = fields.next()?.parse().ok()?;
    let minute: i64 = fields.next()?.parse().ok()?;
    let second: i64 = fields
        .next()
        .map_or(Some(0), |field: &str| field.parse().ok())?;
    Some((hour, minute, second))
}

/// Parses a zone token into seconds east of UTC: `GMT`/`UTC`/`UT`/`Z` are zero,
/// `+HHMM`/`-HHMM` are their signed offset, and anything else is read as UTC.
fn parse_zone_offset(token: &str) -> i64 {
    let bytes: &[u8] = token.as_bytes();
    if bytes.len() == 5
        && (bytes[0] == b'+' || bytes[0] == b'-')
        && bytes[1..].iter().all(u8::is_ascii_digit)
    {
        let sign: i64 = if bytes[0] == b'-' { -1 } else { 1 };
        let hours: i64 = i64::from(bytes[1] - b'0') * 10 + i64::from(bytes[2] - b'0');
        let minutes: i64 = i64::from(bytes[3] - b'0') * 10 + i64::from(bytes[4] - b'0');
        return sign * (hours * 3600 + minutes * 60);
    }
    0
}
