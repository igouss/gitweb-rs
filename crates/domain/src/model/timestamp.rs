//! Absolute civil-timestamp formatting.
//!
//! Mirrors gitweb's `parse_date` + `format_timestamp_html`. The absolute-date
//! sibling of the relative [`crate::model::age::Age`] primitive: where `Age`
//! renders "3 days ago", this renders the calendar date itself.
//!
//! gitweb builds three representations off one epoch:
//!   * `rfc2822`  — `"Sun, 7 Jun 2026 14:30:45 +0000"`, always in UTC; feeds the
//!     RSS feed and the `Last-Modified` header.
//!   * `iso-8601` — `"2026-06-07T14:30:45Z"`, also UTC; feeds the Atom feed and
//!     any machine-readable value.
//!   * `iso-tz`   — `"2026-06-07 16:30:45 +0200"`, the *local* civil date in the
//!     commit's own timezone; gitweb's default human form (and the blame date).
//!
//! `format_timestamp_html` then decorates `rfc2822` with a `(HH:MM tz)` local
//! hint, highlighting the time when the local hour is before 06:00 (`atnight`).
//! That decoration is markup and belongs to the render layer; the domain owns
//! only the numbers and the < 06:00 rule — see [`Timestamp::is_at_night`].
//!
//! All civil-date arithmetic is done here with integer math (no calendar
//! crate). Both UTC and local fields come from the same days-from-epoch
//! computation, so negative epochs and zone offsets crossing day/month/year
//! boundaries are handled uniformly.

use crate::model::signature::Signature;

const SECONDS_PER_DAY: i64 = 86_400;
const SECONDS_PER_HOUR: i64 = 3_600;
const SECONDS_PER_MINUTE: i64 = 60;

/// gitweb falls back to `-0000` when no zone is supplied (`shift || "-0000"`).
const DEFAULT_TIMEZONE: &str = "-0000";

const MONTH_NAMES: [&str; 12] = [
    "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
];
const WEEKDAY_NAMES: [&str; 7] = ["Sun", "Mon", "Tue", "Wed", "Thu", "Fri", "Sat"];

/// An absolute commit/tag timestamp, renderable in the forms gitweb uses.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Timestamp {
    /// The originating Unix epoch — kept so the `Last-Modified` validator can
    /// compare it against the client's `If-Modified-Since` without re-parsing the
    /// formatted forms.
    epoch: i64,
    /// UTC civil fields — drive the RFC-2822 and ISO-8601 forms.
    utc: Civil,
    /// Civil fields shifted into the commit timezone — drive the local form.
    local: Civil,
    /// The displayed offset token (e.g. `"+0200"`, or `"-0000"` by default).
    timezone: String,
}

impl Timestamp {
    /// Builds a timestamp from a git epoch and a `"+HHMM"`/`"-HHMM"` offset.
    ///
    /// An empty zone falls back to `-0000`, matching gitweb's
    /// `my $tz = shift || "-0000"`. An unparseable token is displayed verbatim
    /// but contributes a zero offset, again as gitweb's regex would leave it.
    #[must_use]
    pub fn new(epoch: i64, tz: &str) -> Self {
        let timezone: &str = if tz.is_empty() { DEFAULT_TIMEZONE } else { tz };
        let offset: i64 = parse_offset_seconds(timezone);
        Self {
            epoch,
            utc: Civil::from_epoch(epoch),
            local: Civil::from_epoch(epoch + offset),
            timezone: timezone.to_owned(),
        }
    }

    /// The originating Unix epoch, in UTC seconds — the value gitweb's
    /// `exit_if_unmodified_since` compares against the client's cache time.
    #[must_use]
    pub fn epoch(&self) -> i64 {
        self.epoch
    }

    /// Builds a timestamp from a bare epoch (no zone), as the feeds do when they
    /// call gitweb's `parse_date($epoch)`.
    #[must_use]
    pub fn from_epoch(epoch: i64) -> Self {
        Self::new(epoch, "")
    }

    /// Builds a timestamp from a parsed identity line's epoch and timezone.
    #[must_use]
    pub fn from_signature(sig: &Signature) -> Self {
        Self::new(sig.epoch(), sig.timezone())
    }

    /// The RFC-2822 form in UTC, e.g. `"Sun, 7 Jun 2026 14:30:45 +0000"`.
    ///
    /// gitweb hard-codes the `+0000` suffix: this representation is always UTC.
    #[must_use]
    pub fn rfc2822(&self) -> String {
        format!(
            "{}, {} {} {:4} {:02}:{:02}:{:02} +0000",
            WEEKDAY_NAMES[self.utc.weekday as usize],
            self.utc.day,
            MONTH_NAMES[(self.utc.month - 1) as usize],
            self.utc.year,
            self.utc.hour,
            self.utc.minute,
            self.utc.second,
        )
    }

    /// The RFC-2822 form in the commit's *own* timezone, e.g.
    /// `"Sun, 7 Jun 2026 16:30:45 +0200"` — git's `DATE_RFC2822` (the email date
    /// `format-patch` stamps on the `Date:` line). Unlike [`Self::rfc2822`],
    /// which is always UTC with a hard `+0000`, this renders the *local* civil
    /// fields (weekday included) and trails the real offset token. The day is
    /// not zero-padded, matching git's `%-d`.
    #[must_use]
    pub fn rfc2822_local(&self) -> String {
        format!(
            "{}, {} {} {:4} {:02}:{:02}:{:02} {}",
            WEEKDAY_NAMES[self.local.weekday as usize],
            self.local.day,
            MONTH_NAMES[(self.local.month - 1) as usize],
            self.local.year,
            self.local.hour,
            self.local.minute,
            self.local.second,
            self.timezone,
        )
    }

    /// The ISO-8601 form in UTC, e.g. `"2026-06-07T14:30:45Z"`.
    #[must_use]
    pub fn iso8601(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}T{:02}:{:02}:{:02}Z",
            self.utc.year,
            self.utc.month,
            self.utc.day,
            self.utc.hour,
            self.utc.minute,
            self.utc.second,
        )
    }

    /// The civil form in the commit's own timezone, e.g.
    /// `"2026-06-07 16:30:45 +0200"` — gitweb's `iso-tz` / default human form.
    #[must_use]
    pub fn iso_tz(&self) -> String {
        format!(
            "{:04}-{:02}-{:02} {:02}:{:02}:{:02} {}",
            self.local.year,
            self.local.month,
            self.local.day,
            self.local.hour,
            self.local.minute,
            self.local.second,
            self.timezone,
        )
    }

    /// The bare UTC calendar date, `YYYY-MM-DD` — gitweb's `age_string_date`
    /// (`sprintf "%4i-%02u-%02i"` over `gmtime`). The absolute half of a
    /// commit-list date cell.
    #[must_use]
    pub fn utc_date(&self) -> String {
        format!(
            "{:04}-{:02}-{:02}",
            self.utc.year, self.utc.month, self.utc.day
        )
    }

    /// The local hour (commit timezone), as shown in the `(HH:MM tz)` hint.
    #[must_use]
    pub fn local_hour(&self) -> u8 {
        self.local.hour
    }

    /// The local minute (commit timezone), as shown in the `(HH:MM tz)` hint.
    #[must_use]
    pub fn local_minute(&self) -> u8 {
        self.local.minute
    }

    /// The displayed timezone offset token (gitweb's `tz_local`).
    #[must_use]
    pub fn timezone(&self) -> &str {
        &self.timezone
    }

    /// Whether the local hour is before 06:00 — gitweb's `atnight` predicate,
    /// which the render layer uses to highlight wee-hours commits.
    #[must_use]
    pub fn is_at_night(&self) -> bool {
        self.local.hour < 6
    }
}

/// Parses a `"+HHMM"`/`"-HHMM"` offset into signed seconds, returning 0 for any
/// token that does not match — exactly as gitweb's regex leaves a non-match.
fn parse_offset_seconds(tz: &str) -> i64 {
    let bytes: &[u8] = tz.as_bytes();
    if bytes.len() != 5 {
        return 0;
    }
    let sign: i64 = match bytes[0] {
        b'-' => -1,
        b'+' => 1,
        _ => return 0,
    };
    if !bytes[1..].iter().all(u8::is_ascii_digit) {
        return 0;
    }
    let hours: i64 = i64::from(bytes[1] - b'0') * 10 + i64::from(bytes[2] - b'0');
    let minutes: i64 = i64::from(bytes[3] - b'0') * 10 + i64::from(bytes[4] - b'0');
    sign * (hours * SECONDS_PER_HOUR + minutes * SECONDS_PER_MINUTE)
}

/// Civil (Gregorian) calendar fields for one instant — the result of applying a
/// `gmtime`-equivalent to a Unix epoch.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Civil {
    year: i64,
    /// Month of year, 1–12.
    month: u8,
    /// Day of month, 1–31.
    day: u8,
    hour: u8,
    minute: u8,
    second: u8,
    /// Day of week, 0 = Sunday … 6 = Saturday (Perl's `gmtime` `$wday`).
    weekday: u8,
}

impl Civil {
    /// Breaks a Unix epoch (seconds) into civil fields, the proleptic-Gregorian
    /// `gmtime`. Euclidean division makes negative epochs fall before 1970
    /// instead of truncating toward it.
    fn from_epoch(epoch: i64) -> Self {
        let days: i64 = epoch.div_euclid(SECONDS_PER_DAY);
        let seconds: i64 = epoch.rem_euclid(SECONDS_PER_DAY);
        // 1970-01-01 (day 0) was a Thursday; Perl's $wday counts Sunday as 0.
        let weekday: u8 = ((days.rem_euclid(7) + 4) % 7) as u8;
        let (year, month, day): (i64, u8, u8) = civil_from_days(days);
        Self {
            year,
            month,
            day,
            hour: (seconds / SECONDS_PER_HOUR) as u8,
            minute: ((seconds % SECONDS_PER_HOUR) / SECONDS_PER_MINUTE) as u8,
            second: (seconds % SECONDS_PER_MINUTE) as u8,
            weekday,
        }
    }
}

/// Converts a day count since the Unix epoch into `(year, month, day)`.
///
/// Howard Hinnant's `civil_from_days` (public-domain `date` algorithms): exact
/// integer arithmetic, valid for the whole `i64` range including days before the
/// epoch, with truncating division as Rust's `/` already provides.
fn civil_from_days(days: i64) -> (i64, u8, u8) {
    // Shift the era so the year starts in March (leap day lands at year's end).
    let z: i64 = days + 719_468;
    let era: i64 = if z >= 0 { z } else { z - 146_096 } / 146_097;
    let day_of_era: i64 = z - era * 146_097; // [0, 146096]
    let year_of_era: i64 =
        (day_of_era - day_of_era / 1460 + day_of_era / 36524 - day_of_era / 146_096) / 365; // [0, 399]
    let year: i64 = year_of_era + era * 400;
    let day_of_year: i64 = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100); // [0, 365]
    let month_shifted: i64 = (5 * day_of_year + 2) / 153; // [0, 11], March = 0
    let day: i64 = day_of_year - (153 * month_shifted + 2) / 5 + 1; // [1, 31]
    let month: i64 = if month_shifted < 10 {
        month_shifted + 3
    } else {
        month_shifted - 9
    }; // [1, 12]
    let year: i64 = if month <= 2 { year + 1 } else { year };
    (year, month as u8, day as u8)
}

/// Converts a civil `(year, month, day)` into a day count since the Unix epoch —
/// the inverse of [`civil_from_days`].
///
/// Howard Hinnant's `days_from_civil` (public-domain `date` algorithms): exact
/// integer arithmetic over the whole `i64` range, with the same March-rooted era
/// shift the forward conversion uses. The conditional-GET validator pairs it with
/// the time-of-day to turn a parsed HTTP date back into an epoch.
pub(crate) fn days_from_civil(year: i64, month: u8, day: u8) -> i64 {
    let month: i64 = i64::from(month);
    let day: i64 = i64::from(day);
    // Treat Jan/Feb as months 13/14 of the *previous* year so the leap day is last.
    let year: i64 = if month <= 2 { year - 1 } else { year };
    let era: i64 = if year >= 0 { year } else { year - 399 } / 400;
    let year_of_era: i64 = year - era * 400; // [0, 399]
    let day_of_year: i64 =
        (153 * (if month > 2 { month - 3 } else { month + 9 }) + 2) / 5 + day - 1; // [0, 365]
    let day_of_era: i64 = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year; // [0, 146096]
    era * 146_097 + day_of_era - 719_468
}
