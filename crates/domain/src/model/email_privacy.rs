//! Email-address redaction for the email-privacy feature.
//!
//! Mirrors gitweb's `hide_mailaddrs_if_private`: replace every
//! `<local@domain>` with `<redacted>`. Whether redaction runs at all is a
//! configuration decision the caller makes; this function is only the rule.

use std::borrow::Cow;

/// Replaces every `<local@domain>` with `<redacted>`.
///
/// Matches gitweb's `s/<[^@>]+@[^>]+>/<redacted>/g`: the local part is one or
/// more characters other than `@` or `>`, and the domain one or more other
/// than `>`. Returns the input borrowed when it contains no address.
#[must_use]
pub fn redact(line: &str) -> Cow<'_, str> {
    let mut search_from: usize = 0;
    let mut result: Option<String> = None;
    while let Some((start, end)) = find_address(line, search_from) {
        let output: &mut String = result.get_or_insert_with(String::new);
        output.push_str(&line[search_from..start]);
        output.push_str("<redacted>");
        search_from = end;
    }
    match result {
        Some(mut output) => {
            output.push_str(&line[search_from..]);
            Cow::Owned(output)
        }
        None => Cow::Borrowed(line),
    }
}

/// Finds the next `<[^@>]+@[^>]+>` at or after `from`, as a byte range.
fn find_address(line: &str, from: usize) -> Option<(usize, usize)> {
    let bytes: &[u8] = line.as_bytes();
    let mut open: usize = from;
    while let Some(offset) = line[open..].find('<') {
        let start: usize = open + offset;
        if let Some(end) = match_address_body(bytes, start) {
            return Some((start, end));
        }
        open = start + 1;
    }
    None
}

/// Validates `<local@domain>` starting at the `<` at `start`; returns the byte
/// index just past the closing `>` on success.
///
/// Delimiters are ASCII, so byte scanning never lands mid-character; the local
/// and domain parts may contain any non-delimiter bytes.
fn match_address_body(bytes: &[u8], start: usize) -> Option<usize> {
    let mut i: usize = start + 1; // skip '<'
    let local_start: usize = i;
    while i < bytes.len() && bytes[i] != b'@' && bytes[i] != b'>' {
        i += 1;
    }
    let local_nonempty: bool = i > local_start;
    if !local_nonempty || i >= bytes.len() || bytes[i] != b'@' {
        return None;
    }
    i += 1; // skip '@'
    let domain_start: usize = i;
    while i < bytes.len() && bytes[i] != b'>' {
        i += 1;
    }
    let domain_nonempty: bool = i > domain_start;
    if !domain_nonempty || i >= bytes.len() {
        return None;
    }
    Some(i + 1) // include the closing '>'
}
