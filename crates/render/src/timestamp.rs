//! The absolute-timestamp badge — gitweb's `format_timestamp_html`, modernized.
//!
//! gitweb prints a commit/tag time as its RFC-2822 string followed by a
//! `(HH:MM tz)` hint in the event's own timezone, highlighting the time when the
//! local hour is before 06:00 (the `atnight` class). Several views render exactly
//! this — the single-tag tagger row, the verbose log's authorship line, and the
//! commit view — so the markup lives here once.
//!
//! Modernized: the RFC-2822 text is wrapped in a machine-readable
//! `<time datetime>` carrying the ISO-8601 instant, so the `javascript-timezone`
//! feature (gitweb_in_rust-h2t) can rewrite it to the viewer's zone. The domain
//! owns the numbers and the < 06:00 rule ([`Timestamp::is_at_night`]); this only
//! places them.

use gitweb_domain::model::timestamp::Timestamp;

use crate::markup::{Markup, html};

/// Renders an absolute timestamp: the UTC date as the visible text inside a
/// `<time datetime>`, then gitweb's `(HH:MM tz)` local hint, flagged `atnight`
/// when the local hour is before 06:00.
#[must_use]
pub fn timestamp_badge(timestamp: &Timestamp) -> Markup {
    let local_class: &str = if timestamp.is_at_night() {
        "local-time atnight"
    } else {
        "local-time"
    };
    let local_time: String = format!(
        "{:02}:{:02}",
        timestamp.local_hour(),
        timestamp.local_minute()
    );
    html! {
        time class="commit-date" datetime=(timestamp.iso8601()) { (timestamp.rfc2822()) }
        " (" span class=(local_class) { (local_time) } " " (timestamp.timezone()) ")"
    }
}
