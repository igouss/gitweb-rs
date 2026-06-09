//! Tab expansion for display — gitweb's `untabify`.
//!
//! gitweb pre-formats grep lines (and other fixed-width text) by replacing every
//! tab with the spaces that reach the next eight-column tab stop, so the text
//! lines up the way an editor with `tabstop=8` would show it. The width a tab
//! takes depends on where it sits: a tab `column` characters into the (already
//! expanded) line is widened to `8 - (column % 8)` spaces — a tab at the start of
//! the line fills a whole eight columns, while one sitting exactly on a stop
//! fills another full eight. The column is counted in characters, matching
//! gitweb's `index`-based loop over the decoded line, so the rule is a pure
//! string transform with no notion of display width beyond that naive count.

/// The tab stop width gitweb expands to (`8 - ($pos % 8)`).
const TAB_STOP: usize = 8;

/// Expands every tab in `line` to the spaces that reach the next eight-column tab
/// stop, counting columns in characters from the start of the line.
#[must_use]
pub fn untabify(line: &str) -> String {
    let mut expanded: String = String::with_capacity(line.len());
    let mut column: usize = 0;
    for character in line.chars() {
        if character == '\t' {
            let width: usize = TAB_STOP - (column % TAB_STOP);
            expanded.extend(std::iter::repeat_n(' ', width));
            column += width;
        } else {
            expanded.push(character);
            column += 1;
        }
    }
    expanded
}
