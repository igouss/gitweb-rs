//! Snapshot archive bytes for the gix adapter's `Repository::archive`.
//!
//! gix-archive renders a tree's worktree stream into tar, tar.gz, and zip
//! natively, all pure-Rust (flate2 over zlib-rs, rawzip). The two containers it
//! does not produce — tar.bz2 and tar.xz — are the raw tar run through bzip2 and
//! xz here, again pure-Rust, so the snapshot needs no external tool. Like
//! `git archive`, the stream carries only file content: regular files, symlinks
//! (stored as symlinks), executables (0755), files inside subtrees (as their
//! prefixed path), and nothing for a submodule gitlink, whose objects are not
//! present to recurse into.
//!
//! Every entry's modification time is pinned to a constant so the same tree
//! always yields the same bytes — reproducibility is a parity requirement, and
//! gitweb's snapshots are cache-validated on this time. The download filename
//! and the archive `--prefix` are gitweb presentation concerns resolved at the
//! snapshot endpoint, so the bytes produced here carry no path prefix.

use std::io::{Cursor, Write};
use std::sync::atomic::AtomicBool;

use gix::worktree::archive::{Format, Options};

use gitweb_domain::error::DomainError;
use gitweb_domain::port::repository::ArchiveFormat;

use crate::conv::backend;

/// The modification time stamped on every archived entry. The adapter is handed
/// a tree, not the commit whose date gitweb stamps, so it uses a fixed epoch for
/// reproducibility; the snapshot endpoint supplies the real commit time when it
/// lands and needs byte-exact parity.
const REPRODUCIBLE_MTIME: i64 = 0;

/// The snapshot archive of an already-resolved `tree_id` in `format`.
///
/// `tree_id` must name a tree — the caller resolves a commit or tree-ish to its
/// tree (and rejects a non-tree) before calling, so any failure here is a
/// backend fault, not bad input.
pub(crate) fn archive_bytes(
    repo: &gix::Repository,
    tree_id: gix::ObjectId,
    format: ArchiveFormat,
) -> Result<Vec<u8>, DomainError> {
    match format {
        ArchiveFormat::TarGz => write_archive(
            repo,
            tree_id,
            Format::TarGz {
                compression_level: None,
            },
        ),
        ArchiveFormat::Zip => write_archive(
            repo,
            tree_id,
            Format::Zip {
                compression_level: None,
            },
        ),
        ArchiveFormat::TarBz2 => {
            let tar: Vec<u8> = write_archive(repo, tree_id, Format::Tar)?;
            bzip2_bytes(&tar)
        }
        ArchiveFormat::TarXz => {
            let tar: Vec<u8> = write_archive(repo, tree_id, Format::Tar)?;
            xz_bytes(&tar)
        }
    }
}

/// Streams `tree_id` through gix-archive into an in-memory buffer. The buffer is
/// a [`Cursor`] because the zip writer needs `Seek`; tar formats only ever write
/// forward.
fn write_archive(
    repo: &gix::Repository,
    tree_id: gix::ObjectId,
    format: Format,
) -> Result<Vec<u8>, DomainError> {
    let (stream, _index): (gix::worktree::stream::Stream, gix::index::File) =
        repo.worktree_stream(tree_id).map_err(backend)?;
    let options: Options = Options {
        format,
        tree_prefix: None,
        modification_time: REPRODUCIBLE_MTIME,
    };
    let mut out: Cursor<Vec<u8>> = Cursor::new(Vec::new());
    let interrupt: AtomicBool = AtomicBool::new(false);
    repo.worktree_archive(
        stream,
        &mut out,
        gix::progress::Discard,
        &interrupt,
        options,
    )
    .map_err(backend)?;
    Ok(out.into_inner())
}

/// Compresses raw tar bytes with bzip2 — the `tbz2` container's compressor.
fn bzip2_bytes(tar: &[u8]) -> Result<Vec<u8>, DomainError> {
    let mut encoder: bzip2::write::BzEncoder<Vec<u8>> =
        bzip2::write::BzEncoder::new(Vec::new(), bzip2::Compression::default());
    encoder.write_all(tar).map_err(backend)?;
    encoder.finish().map_err(backend)
}

/// Compresses raw tar bytes with xz — the `txz` container's compressor.
fn xz_bytes(tar: &[u8]) -> Result<Vec<u8>, DomainError> {
    let options: lzma_rust2::XzOptions = lzma_rust2::XzOptions::with_preset(6);
    let mut encoder: lzma_rust2::XzWriter<Vec<u8>> =
        lzma_rust2::XzWriter::new(Vec::new(), options).map_err(backend)?;
    encoder.write_all(tar).map_err(backend)?;
    encoder.finish().map_err(backend)
}
