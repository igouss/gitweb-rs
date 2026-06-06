//! Relative-age formatting and classification.
//!
//! Mirrors gitweb's `age_string` and `age_class`. The original uses
//! factor-of-two thresholds, so the smallest count rendered for any unit is
//! always two (gitweb never prints "1 min ago"); that quirk is observable and
//! is preserved here deliberately.

/// CSS-classification bucket for how recently something changed.
///
/// Maps to gitweb's `age_class` return values (`noage`/`age0`/`age1`/`age2`).
/// The CSS string itself is a presentation concern and lives in the render
/// adapter; the domain only owns the classification rule.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgeClass {
    /// Age is unknown.
    Unknown,
    /// Less than two hours old.
    Fresh,
    /// Less than two days old.
    Recent,
    /// Two days or older.
    Old,
}

impl AgeClass {
    /// Classifies a possibly-unknown age. An absent age is `Unknown`, mirroring
    /// gitweb's `age_class(undef) == "noage"`.
    #[must_use]
    pub fn from_age(age: Option<Age>) -> Self {
        match age {
            None => Self::Unknown,
            Some(age) => age.class(),
        }
    }
}

/// A span between an event and "now", measured in whole seconds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Age {
    seconds: i64,
}

// Time units in seconds, so the thresholds below read as "2 weeks", "2 days"
// rather than as opaque second counts. MONTH is gitweb's 365/12-day month
// (86400 * 365 / 12 == 2_628_000 exactly).
const MINUTE: i64 = 60;
const HOUR: i64 = 60 * MINUTE;
const DAY: i64 = 24 * HOUR;
const WEEK: i64 = 7 * DAY;
const MONTH: i64 = DAY * 365 / 12;
const YEAR: i64 = 365 * DAY;

impl Age {
    /// Constructs an age from a whole-second count.
    #[must_use]
    pub const fn from_seconds(seconds: i64) -> Self {
        Self { seconds }
    }

    /// Renders the age the way gitweb does, e.g. `"3 sec ago"` or `"right now"`.
    ///
    /// Each threshold is two units wide, so the smallest count shown for a unit
    /// is two — matching gitweb, which never prints "1 min ago".
    #[must_use]
    pub fn humanized(self) -> String {
        let age: i64 = self.seconds;
        if age > 2 * YEAR {
            format!("{} years ago", age / YEAR)
        } else if age > 2 * MONTH {
            format!("{} months ago", age / MONTH)
        } else if age > 2 * WEEK {
            format!("{} weeks ago", age / WEEK)
        } else if age > 2 * DAY {
            format!("{} days ago", age / DAY)
        } else if age > 2 * HOUR {
            format!("{} hours ago", age / HOUR)
        } else if age > 2 * MINUTE {
            format!("{} min ago", age / MINUTE)
        } else if age > 2 {
            format!("{age} sec ago")
        } else {
            "right now".to_owned()
        }
    }

    /// Buckets the age for CSS styling, mirroring gitweb's `age_class`.
    #[must_use]
    pub fn class(self) -> AgeClass {
        if self.seconds < 2 * HOUR {
            AgeClass::Fresh
        } else if self.seconds < 2 * DAY {
            AgeClass::Recent
        } else {
            AgeClass::Old
        }
    }
}
