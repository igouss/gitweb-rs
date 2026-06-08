//! A commit's date cell: gitweb's two-week age/date swap.
//!
//! gitweb's `parse_commit_text` derives two views of a commit's committer time —
//! the relative age (`age_string`, e.g. `"3 days ago"`) and the absolute UTC
//! calendar date (`sprintf "%4i-%02u-%02i"` over `gmtime`, e.g. `"2026-06-04"`) —
//! and decides per commit which one a list body shows in the cell and which
//! becomes the cell's `title` tooltip (`age_string_date` / `age_string_age`):
//!
//!   * older than two weeks -> the cell shows the absolute date, age as tooltip;
//!   * two weeks or newer    -> the cell shows the relative age, date as tooltip.
//!
//! The threshold is strict (`$age > 60*60*24*7*2`), so exactly two weeks still
//! counts as recent. This rule is shared by every commit-list body — shortlog,
//! log, history, commit, commitdiff — which is why it lives here once rather than
//! in any single use case.
//!
//! The relative age reuses [`Age`] and the absolute date reuses the UTC form of
//! [`Timestamp`]; this entity owns only the swap.

use crate::model::age::Age;
use crate::model::timestamp::Timestamp;

/// gitweb's `60*60*24*7*2` — two weeks in seconds. A commit strictly older than
/// this shows its absolute date instead of its relative age.
const TWO_WEEKS: i64 = 60 * 60 * 24 * 7 * 2;

/// A commit's resolved date cell: the text shown and the tooltip behind it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitDate {
    displayed: String,
    tooltip: String,
}

impl CommitDate {
    /// Builds the date cell for a commit dated at `committer_epoch`, viewed at
    /// `now`. Both are Unix epoch seconds; the age is their difference and the
    /// absolute date is the committer epoch in UTC.
    #[must_use]
    pub fn new(committer_epoch: i64, now: i64) -> Self {
        let age: i64 = now - committer_epoch;
        let relative: String = Age::from_seconds(age).humanized();
        let absolute: String = Timestamp::from_epoch(committer_epoch).utc_date();
        if age > TWO_WEEKS {
            Self {
                displayed: absolute,
                tooltip: relative,
            }
        } else {
            Self {
                displayed: relative,
                tooltip: absolute,
            }
        }
    }

    /// The text shown in the date cell (gitweb's `age_string_date`).
    #[must_use]
    pub fn displayed(&self) -> &str {
        &self.displayed
    }

    /// The text shown as the date cell's hover tooltip (gitweb's
    /// `age_string_age`).
    #[must_use]
    pub fn tooltip(&self) -> &str {
        &self.tooltip
    }
}
