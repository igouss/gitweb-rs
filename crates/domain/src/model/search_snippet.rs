//! Search-result snippet highlight — gitweb's `git_search_grep_body` line rule.
//!
//! When a commit/author/committer search lists a commit, gitweb walks the
//! commit-message lines and, for each line that matches the search pattern,
//! prints a compact highlighted fragment: the part before the match, the match
//! in a `<span class="match">`, and the part after. To keep the row short it
//! trims all three with [`chop_str`]:
//!
//! ```text
//! $match = chop_str($match, 70, 5, 'center');
//! my $contextlen = int((80 - length($match))/2);
//! $contextlen = 30 if ($contextlen > 30);
//! $lead  = chop_str($lead,  $contextlen, 10, 'left');
//! $trail = chop_str($trail, $contextlen, 10, 'right');
//! ```
//!
//! The match is centre-chopped to 70 characters; the surrounding context shares
//! what is left of an 80-character budget, half to each side, capped at 30 — the
//! lead trimmed from its left so its tail (next to the match) survives, the trail
//! from its right so its head survives. This rule produces the three trimmed
//! strings; the HTML escaping and the `<span>` are the render layer's job, and a
//! line that does not match yields no snippet at all.

use crate::model::chop::{ChopMode, chop_str};
use crate::model::search_pattern::SearchPattern;

/// gitweb's centre-chop of the match (`chop_str($match, 70, 5, 'center')`).
const MATCH_LEN: usize = 70;
/// The match chop's word slack.
const MATCH_SLACK: usize = 5;
/// The total context budget shared by lead and trail (`80 - length($match)`).
const BUDGET: usize = 80;
/// The per-side context cap (`$contextlen = 30 if ($contextlen > 30)`).
const MAX_CONTEXT: usize = 30;
/// The context chop's word slack (`chop_str($lead/$trail, .., 10, ..)`).
const CONTEXT_SLACK: usize = 10;

/// One highlighted commit-message line: the trimmed text before the match, the
/// trimmed match itself, and the trimmed text after it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct MatchSnippet {
    lead: String,
    matched: String,
    trail: String,
}

impl MatchSnippet {
    /// The trimmed text before the match (chopped from its left).
    #[must_use]
    pub fn lead(&self) -> &str {
        &self.lead
    }

    /// The trimmed matched fragment (centre-chopped).
    #[must_use]
    pub fn matched(&self) -> &str {
        &self.matched
    }

    /// The trimmed text after the match (chopped from its right).
    #[must_use]
    pub fn trail(&self) -> &str {
        &self.trail
    }
}

/// Highlights `line` against `pattern`: splits it at the first match and trims
/// each part the way `git_search_grep_body` does, or returns `None` when the line
/// does not match (so no snippet is shown for it).
#[must_use]
pub fn highlight_line(line: &str, pattern: &SearchPattern) -> Option<MatchSnippet> {
    let (start, end): (usize, usize) = pattern.first_match(line)?;
    let matched: String = chop_str(&line[start..end], MATCH_LEN, MATCH_SLACK, ChopMode::Center);
    let context: usize = context_len(matched.chars().count());
    Some(MatchSnippet {
        lead: chop_str(&line[..start], context, CONTEXT_SLACK, ChopMode::Left),
        trail: chop_str(&line[end..], context, CONTEXT_SLACK, ChopMode::Right),
        matched,
    })
}

/// The per-side context length: half the 80-character budget left after the
/// (already chopped) match, capped at 30. A match that fills or overflows the
/// budget leaves no room, so the context collapses to nothing.
fn context_len(match_chars: usize) -> usize {
    (BUDGET.saturating_sub(match_chars) / 2).min(MAX_CONTEXT)
}
