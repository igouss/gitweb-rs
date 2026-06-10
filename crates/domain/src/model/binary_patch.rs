//! git's `GIT binary patch` body for a binary file in `format-patch --binary`.
//!
//! `git format-patch` embeds a binary blob as a `GIT binary patch` so the mail is
//! `git am`-able. For one file git's `emit_binary_diff` writes the `GIT binary
//! patch` header, then TWO blocks: a forward block that rebuilds the post-image
//! from the pre-image (what `git apply` consumes), then a reverse block that
//! rebuilds the pre-image from the post-image (what `git apply -R` consumes).
//!
//! Each block (`emit_binary_diff_body`) is whichever is shorter of
//! - `literal <post-image-size>` — the deflated post-image, or
//! - `delta <uncompressed-delta-size>` — the deflated binary delta pre→post,
//!   computed only when both sides are non-empty and kept only when its deflated
//!   form is strictly shorter than the deflated literal,
//!
//! followed by the chosen bytes in git's base85 framing ([`crate::model::base85`])
//! and a trailing blank line (the footer). The delta is git's `diff_delta`
//! ([`crate::model::delta`]) capped at the deflated literal's size; the deflate is
//! plain zlib level 1, the exact stream git's `deflate_it` produces.
//!
//! This module owns the FORMAT (the choice, the headers, the framing, the two
//! blocks); the [`Repository`](crate::port::repository::Repository) adapter owns
//! reading the two blobs and hands their raw bytes to
//! [`FileContent::Binary`](crate::model::patch::FileContent::Binary), which calls
//! [`render`] here.
//!
//! Pure, framework-free, no I/O, no `unsafe`.

use std::io::Write;

use flate2::Compression;
use flate2::write::ZlibEncoder;

use crate::model::base85::encode_framed;
use crate::model::delta::diff_delta_bounded;

/// Renders the `GIT binary patch` body for a binary file changing from `source`
/// (pre-image bytes) to `target` (post-image bytes): the `GIT binary patch` header,
/// then the forward block (rebuilds `target` from `source`) and the reverse block
/// (rebuilds `source` from `target`), exactly as git's `emit_binary_diff` writes
/// them. An empty `source` (a create) or empty `target` (a delete) simply forces a
/// literal in the affected block.
#[must_use]
pub fn render(source: &[u8], target: &[u8]) -> String {
    let mut out: String = String::from("GIT binary patch\n");
    out.push_str(&render_block(source, target));
    out.push_str(&render_block(target, source));
    out
}

/// One `emit_binary_diff_body` block: emits whichever is shorter of a `delta` (the
/// deflated binary delta `source -> target`, headed by its *uncompressed* size) or
/// a `literal` (the deflated `target`, headed by `target`'s size), then the chosen
/// bytes in base85 framing and a trailing blank line (the footer). The block
/// rebuilds `target` when applied to `source`.
fn render_block(source: &[u8], target: &[u8]) -> String {
    let deflated_literal: Vec<u8> = deflate(target);
    let (label, header, data): (&str, usize, Vec<u8>) =
        match shorter_delta(source, target, &deflated_literal) {
            Some((uncompressed_size, deflated_delta)) => {
                ("delta", uncompressed_size, deflated_delta)
            }
            None => ("literal", target.len(), deflated_literal),
        };
    let mut out: String = format!("{label} {header}\n");
    out.push_str(&encode_framed(&data));
    out.push('\n');
    out
}

/// The deflated binary delta `source -> target` paired with the delta's
/// *uncompressed* size (git's `delta <N>` header), but only when it strictly beats
/// the deflated literal git would otherwise emit. `None` keeps the literal: when no
/// delta exists (an empty side), when the delta outgrows the deflated-literal cap
/// ([`diff_delta_bounded`]), or when its deflated form does not actually win.
fn shorter_delta(
    source: &[u8],
    target: &[u8],
    deflated_literal: &[u8],
) -> Option<(usize, Vec<u8>)> {
    let raw_delta: Vec<u8> = diff_delta_bounded(source, target, deflated_literal.len())?;
    let deflated_delta: Vec<u8> = deflate(&raw_delta);
    if deflated_delta.len() < deflated_literal.len() {
        Some((raw_delta.len(), deflated_delta))
    } else {
        None
    }
}

/// git's `deflate_it`: plain zlib at level 1 (`Z_BEST_SPEED`), default strategy —
/// the exact byte stream git's binary patch base85-encodes. flate2's zlib-rs
/// backend reproduces it byte-for-byte (the `78 01` level-1 header and all). The
/// writer is an in-memory `Vec`, so neither the write nor the flush can fail.
fn deflate(data: &[u8]) -> Vec<u8> {
    let mut encoder: ZlibEncoder<Vec<u8>> = ZlibEncoder::new(Vec::new(), Compression::new(1));
    encoder
        .write_all(data)
        .expect("writing to an in-memory buffer never fails");
    encoder
        .finish()
        .expect("finishing an in-memory zlib stream never fails")
}
