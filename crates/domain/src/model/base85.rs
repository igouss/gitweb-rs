//! git's base85 encoder for the `GIT binary patch` body.
//!
//! `git diff --binary` / `git format-patch --binary` encode the deflated literal
//! (or delta) of a binary blob in git's base85 variant — NOT a standard Ascii85.
//! It is an 85-character alphabet (`base85.c`'s `en85`), data packed big-endian
//! four bytes to five digits (a short final group zero-padded but still five
//! digits), wrapped in a line framing unique to the patch body: each line carries
//! at most [`LINE_BYTES`] data bytes, is prefixed by a length marker (`1..=26` →
//! `A..=Z`, `27..=52` → `a..=z`), and is newline-terminated (the last line too).
//!
//! This module owns that codec and its framing; the deflate, the literal-vs-delta
//! choice, and the surrounding `literal`/`delta` header line live in
//! [`crate::model::binary_patch`].
//!
//! Pure, framework-free, no I/O, no `unsafe` — a byte-exact port of git's
//! `encode_85` (`base85.c`) plus `emit_binary_diff_body`'s line loop (`diff.c`).

/// git's 85-character alphabet (`base85.c` `en85`): digits, uppercase, lowercase,
/// then a fixed run of punctuation. Index `0..=84` is the character a base85 digit
/// of that value encodes to.
const EN85: &[u8; 85] =
    b"0123456789ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz!#$%&()*+-;<=>?@^_`{|}~";

/// The most data bytes git packs onto one base85 line of the patch body
/// (`emit_binary_diff_body`'s `bytes = (52 < data_size) ? 52 : data_size`).
const LINE_BYTES: usize = 52;

/// Encodes `data` in git's base85 patch framing: one newline-terminated line per
/// up to [`LINE_BYTES`] source bytes, each line a length marker followed by the
/// base85 digits of that line's bytes. The result ends in a newline (every line is
/// terminated, the last included); an empty input yields an empty string.
#[must_use]
pub fn encode_framed(data: &[u8]) -> String {
    let mut out: String = String::new();
    for line in data.chunks(LINE_BYTES) {
        out.push(line_marker(line.len()));
        encode_digits(line, &mut out);
        out.push('\n');
    }
    out
}

/// The length-marker character for a line of `bytes` data bytes: `1..=26` map to
/// `A..=Z`, `27..=52` to `a..=z` (`emit_binary_diff_body`).
fn line_marker(bytes: usize) -> char {
    let marker: u8 = if bytes <= 26 {
        (bytes as u8) + b'A' - 1
    } else {
        (bytes as u8) - 26 + b'a' - 1
    };
    marker as char
}

/// git's `encode_85`: packs `data` four bytes at a time, big-endian, into a 32-bit
/// accumulator, then appends five base85 digits most-significant-first per group.
/// A final group of fewer than four bytes is zero-padded but still emits five
/// digits (the line marker records how many of the decoded bytes are real).
fn encode_digits(data: &[u8], out: &mut String) {
    for quad in data.chunks(4) {
        let mut acc: u32 = 0;
        for (index, &byte) in quad.iter().enumerate() {
            acc |= u32::from(byte) << (24 - 8 * index);
        }
        let mut digits: [u8; 5] = [0; 5];
        let mut value: u32 = acc;
        for digit in digits.iter_mut().rev() {
            *digit = EN85[(value % 85) as usize];
            value /= 85;
        }
        for &digit in &digits {
            out.push(digit as char);
        }
    }
}
