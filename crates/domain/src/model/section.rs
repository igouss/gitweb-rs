//! The summary-page section rule: show at most a cap, flag the overflow.
//!
//! gitweb's `git_summary` repeats one idiom at four sites — the shortlog, tags,
//! heads, and forks sections. It fetches one item more than it will show
//! (`git_get_tags_list(16)` returns up to 17) and then decides whether to offer
//! a "..." link to the full action with `$#list <= 15 ? undef : "..."`: show the
//! first 16, and link onward only when a seventeenth exists.
//!
//! That is one rule, not four. [`Section`] captures it: given the full list and
//! a cap, it keeps the first `cap` items and reports whether the list overran
//! that cap. The caller (the summary use case) applies it with gitweb's cap of
//! 16 to each list whose own use case returns everything; the shortlog reuses
//! its own paginated window instead, which already reports the same overflow.

/// A list trimmed to a display cap, remembering whether it overran.
///
/// `shown` is the first `cap` items in their given order; `truncated` is whether
/// the original list held more than `cap` — gitweb's "is there a `...`?".
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Section<T> {
    shown: Vec<T>,
    truncated: bool,
}

impl<T> Section<T> {
    /// Limits `items` to the first `cap`, recording whether any were dropped.
    /// A list at or under the cap is shown whole and is not truncated; a list
    /// over the cap is cut to `cap` items and flagged truncated.
    #[must_use]
    pub fn limited(mut items: Vec<T>, cap: usize) -> Self {
        let truncated: bool = items.len() > cap;
        items.truncate(cap);
        Self {
            shown: items,
            truncated,
        }
    }

    /// The items to display: the first `cap` of the original list.
    #[must_use]
    pub fn shown(&self) -> &[T] {
        &self.shown
    }

    /// Whether the original list overran the cap — gitweb's overflow that earns
    /// the section a "..." link to its full action.
    #[must_use]
    pub fn is_truncated(&self) -> bool {
        self.truncated
    }
}
