//! Word-aware string truncation.
//!
//! Mirrors gitweb's `chop_str`: shorten a string to roughly `len` characters
//! for display, keeping a straddling word whole when it fits within `add_len`
//! extra characters, and marking the cut with an ellipsis filler. All counting
//! is character-based so multi-byte UTF-8 sequences are never split.

/// Which end of the string is truncated.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ChopMode {
    /// Drop the tail, append `"... "` (gitweb default).
    Right,
    /// Drop the head, prepend `" ..."`.
    Left,
    /// Drop the middle, join the ends with `" ... "`.
    Center,
}

/// Truncates `text` to about `len` characters from the given end.
///
/// `add_len` is the slack: a word straddling the cut is kept whole if it ends
/// within that many extra characters. gitweb passes `10` by default.
#[must_use]
pub fn chop_str(text: &str, len: usize, add_len: usize, mode: ChopMode) -> String {
    let chars: Vec<char> = text.chars().collect();
    let total: usize = chars.len();

    // gitweb returns the string untouched when chopping wouldn't make it
    // shorter than the filler it would add (length 4, or 5 for center).
    match mode {
        ChopMode::Center => {
            if len + 5 >= total {
                return text.to_owned();
            }
        }
        ChopMode::Left | ChopMode::Right => {
            if len + 4 >= total {
                return text.to_owned();
            }
        }
    }

    match mode {
        // Center distributes the budget across both ends (gitweb halves $len).
        ChopMode::Center => {
            let half: usize = len / 2;
            let left_end: usize = head_word_boundary(&chars, half, add_len);
            let left: String = chars[..left_end].iter().collect();

            let remainder: &[char] = &chars[left_end..];
            let body_start: usize = tail_word_boundary(remainder, half, add_len);
            let middle: &[char] = &remainder[..body_start];
            let right: String = remainder[body_start..].iter().collect();
            let filler: String = if middle.len() > 5 {
                " ... ".to_owned()
            } else {
                middle.iter().collect()
            };
            left + &filler + &right
        }
        ChopMode::Right => {
            let body_end: usize = head_word_boundary(&chars, len, add_len);
            let body: String = chars[..body_end].iter().collect();
            let tail: &[char] = &chars[body_end..];
            let filler: String = if tail.len() > 4 {
                "... ".to_owned()
            } else {
                tail.iter().collect()
            };
            body + &filler
        }
        ChopMode::Left => {
            let body_start: usize = tail_word_boundary(&chars, len, add_len);
            let lead: &[char] = &chars[..body_start];
            let body: String = chars[body_start..].iter().collect();
            let filler: String = if lead.len() > 4 {
                " ...".to_owned()
            } else {
                lead.iter().collect()
            };
            filler + &body
        }
    }
}

/// Returns whether `c` is a regex `\w` character (letter, digit, or `_`).
fn is_word(c: char) -> bool {
    c.is_alphanumeric() || c == '_'
}

/// Index where the head ends: the first `len` characters extended forward over
/// up to `add_len` word characters (gitweb's `.{len}\w{0,add_len}`).
fn head_word_boundary(chars: &[char], len: usize, add_len: usize) -> usize {
    let mut end: usize = len;
    let mut taken: usize = 0;
    while end < chars.len() && taken < add_len && is_word(chars[end]) {
        end += 1;
        taken += 1;
    }
    end
}

/// Index where the tail begins: the last `len` characters extended backward
/// over up to `add_len` word characters (gitweb's `\w{0,add_len}.{len}$`).
fn tail_word_boundary(chars: &[char], len: usize, add_len: usize) -> usize {
    let mut start: usize = chars.len().saturating_sub(len);
    let mut taken: usize = 0;
    while start > 0 && taken < add_len && is_word(chars[start - 1]) {
        start -= 1;
        taken += 1;
    }
    start
}
