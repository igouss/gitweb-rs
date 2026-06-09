//! The three-state age of a tag ref on the tags listing.
//!
//! gitweb's `git_get_tags_list` records a creation time (`$ref_item{'age'}`)
//! only when the ref's top-level object is a tag or a commit — the kinds that
//! carry a creator date. From that it derives three distinct renderings, which
//! `git_tags_body` keeps strictly apart:
//!
//! * a recorded, non-zero time -> the relative age (`age_string`),
//! * a recorded but zero time -> the literal `"unknown"`,
//! * no recorded time at all (a lightweight tag of a blob or a tree, whose
//!   object type is neither tag nor commit) -> no age field, so gitweb prints an
//!   empty `<td></td>`.
//!
//! An earlier slice collapsed the last two into one "unknown", which is wrong
//! for a blob/tree tag. This entity is the rule that keeps all three apart: the
//! use case feeds it the ref's creation time (`None` when the ref carries none),
//! and the render adapter maps each variant to its own cell.

use crate::model::age::Age;

/// How a tag's creation age renders on the tags listing — gitweb's three states.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagAge {
    /// The ref records a non-zero creation time: its relative age.
    Known(Age),
    /// The ref records a creation time, but it is zero — gitweb's "unknown".
    Unknown,
    /// The ref records no creation time at all: a lightweight tag of a blob or a
    /// tree. gitweb leaves the age undefined and prints an empty cell.
    Absent,
}

impl TagAge {
    /// Classifies a tag's age from its recorded creation time, mirroring
    /// gitweb's `git_get_tags_list`: `creation` is `Some(epoch)` only for a tag
    /// or commit ref (the kinds that carry a creator date), and `None` for any
    /// other object. A zero epoch is gitweb's falsy creator date, rendered
    /// "unknown"; a real epoch ages against `now`.
    #[must_use]
    pub fn classify(creation: Option<i64>, now: i64) -> Self {
        match creation {
            None => Self::Absent,
            Some(0) => Self::Unknown,
            Some(epoch) => Self::Known(Age::from_seconds(now - epoch)),
        }
    }
}
