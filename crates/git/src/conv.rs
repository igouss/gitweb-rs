//! Translation between gix's read types and the domain's git entities.
//!
//! This is the only module that speaks both vocabularies. Keeping it apart lets
//! the adapter proper read as a description of *what* the port asks for, not as
//! a mess of `.to_string()` and byte juggling. Everything here is pure.

use gix::object::tree::diff::ChangeDetached;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::change::ChangeStatus;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::DiffEntry;
use gitweb_domain::model::file_mode::FileMode;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tree::TreeEntry;

/// Wraps any gix error as an opaque backend failure — the catch-all for
/// failures that are neither "not found" nor "wrong kind".
pub(crate) fn backend(error: impl std::fmt::Display) -> DomainError {
    DomainError::Backend(error.to_string())
}

/// Converts a gix object id into the domain's.
///
/// A git object id is, by construction, valid domain hex (40 or 64 chars), so
/// the parse cannot fail; the `expect` records that invariant rather than
/// pushing a `Result` onto every call site.
pub(crate) fn to_domain_oid(oid: gix::ObjectId) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// Parses a domain object id back into gix's form for lookups.
pub(crate) fn to_gix_oid(oid: &ObjectId) -> Result<gix::ObjectId, DomainError> {
    gix::ObjectId::from_hex(oid.as_str().as_bytes()).map_err(|_error: gix::hash::decode::Error| {
        DomainError::Invalid(format!("object id: {}", oid.as_str()))
    })
}

/// Translates gix's object kind into the domain's.
pub(crate) fn to_object_kind(kind: gix::object::Kind) -> ObjectKind {
    match kind {
        gix::object::Kind::Commit => ObjectKind::Commit,
        gix::object::Kind::Tree => ObjectKind::Tree,
        gix::object::Kind::Blob => ObjectKind::Blob,
        gix::object::Kind::Tag => ObjectKind::Tag,
    }
}

/// Rebuilds a git ident line from a gix signature and parses it with the
/// domain's own parser.
///
/// gix already splits a signature into name, email, and a raw `"<epoch> <tz>"`
/// time string, so reassembling the canonical line and handing it to
/// [`Signature::parse`] keeps identity parsing in exactly one place — the
/// domain — instead of duplicating gitweb's quirks here.
pub(crate) fn to_signature(sig: gix::actor::SignatureRef<'_>) -> Result<Signature, DomainError> {
    let line: String = format!("{} <{}> {}", sig.name, sig.email, sig.time);
    Signature::parse(&line)
        .ok_or_else(|| DomainError::Backend(format!("unparseable identity: {line}")))
}

/// Translates a gix commit into the domain's, deriving the same fields gitweb
/// parses out of `git rev-list --header` output.
///
/// Shared by the single-commit read and the history walk, so the gix→domain
/// commit mapping lives in exactly one place.
pub(crate) fn read_commit(commit: &gix::Commit<'_>) -> Result<Commit, DomainError> {
    let id: ObjectId = to_domain_oid(commit.id);
    let tree: ObjectId = to_domain_oid(commit.tree_id().map_err(backend)?.detach());
    let parents: Vec<ObjectId> = commit
        .parent_ids()
        .map(|parent: gix::Id<'_>| to_domain_oid(parent.detach()))
        .collect();
    let author: Signature = to_signature(commit.author().map_err(backend)?)?;
    let committer: Signature = to_signature(commit.committer().map_err(backend)?)?;
    let message: String = commit.message_raw_sloppy().to_string();
    Ok(Commit::new(id, tree, parents, author, committer, message))
}

/// Translates one gix tree entry into the domain's, carrying its mode through
/// the domain's own octal parser so file-mode classification stays in one place.
pub(crate) fn to_tree_entry(
    entry: &gix::objs::tree::EntryRef<'_>,
) -> Result<TreeEntry, DomainError> {
    let octal: String = entry.mode.kind().as_octal_str().to_string();
    let mode: FileMode = FileMode::from_octal(&octal)
        .ok_or_else(|| backend(format!("invalid file mode: {octal}")))?;
    let name: String = entry.filename.to_string();
    let oid: ObjectId = to_domain_oid(entry.oid.to_owned());
    Ok(TreeEntry::new(mode, name, oid))
}

/// A gix tree-entry mode as the domain's [`FileMode`], via the domain's own
/// octal parser so mode classification stays in one place.
fn to_file_mode(mode: gix::objs::tree::EntryMode) -> Result<FileMode, DomainError> {
    let octal: String = mode.kind().as_octal_str().to_string();
    FileMode::from_octal(&octal).ok_or_else(|| backend(format!("invalid file mode: {octal}")))
}

/// The absent-side mode (`000000`) gitweb shows for a created or deleted path.
fn absent_mode() -> FileMode {
    FileMode::from_octal("000000").expect("000000 is a valid octal mode")
}

/// The null object id of `oid`'s hash kind — the all-zero id gitweb shows for
/// the missing side of an addition or deletion.
fn null_oid_like(oid: &gix::ObjectId) -> ObjectId {
    to_domain_oid(oid.kind().null())
}

/// The rename's similarity as a percentage. A perfect rename has no blob diff
/// (`source_id == id`), so it is 100%; gix's ratio covers the inexact case.
///
/// gix's ratio is not byte-identical to git's similarity index, but the only
/// format-stable consumer (the `similarity index` line in patch output) is
/// produced elsewhere, so the approximation is harmless here.
fn rewrite_similarity(diff: Option<&gix::diff::blob::DiffLineStats>) -> u8 {
    match diff {
        None => 100,
        Some(stats) => (stats.similarity * 100.0).round().clamp(0.0, 100.0) as u8,
    }
}

/// Translates one gix tree-diff change into the domain's [`DiffEntry`], or
/// `None` for a directory entry.
///
/// gitweb runs `git diff-tree -r`, which recurses into subtrees and reports
/// only leaf files; gix instead reports the directory entry *and* its leaves, so
/// directory (tree) changes are dropped here. Copies are out of scope, so a
/// rewrite is always a rename.
pub(crate) fn to_diff_entry(change: &ChangeDetached) -> Result<Option<DiffEntry>, DomainError> {
    if change.entry_mode().is_tree() {
        return Ok(None);
    }
    let entry: DiffEntry = match change {
        ChangeDetached::Addition {
            location,
            entry_mode,
            id,
            ..
        } => DiffEntry::new(
            ChangeStatus::added(),
            absent_mode(),
            to_file_mode(*entry_mode)?,
            null_oid_like(id),
            to_domain_oid(*id),
            location.to_string(),
            location.to_string(),
        ),
        ChangeDetached::Deletion {
            location,
            entry_mode,
            id,
            ..
        } => DiffEntry::new(
            ChangeStatus::deleted(),
            to_file_mode(*entry_mode)?,
            absent_mode(),
            to_domain_oid(*id),
            null_oid_like(id),
            location.to_string(),
            location.to_string(),
        ),
        ChangeDetached::Modification {
            location,
            previous_entry_mode,
            previous_id,
            entry_mode,
            id,
        } => {
            let from_mode: FileMode = to_file_mode(*previous_entry_mode)?;
            let to_mode: FileMode = to_file_mode(*entry_mode)?;
            DiffEntry::new(
                ChangeStatus::from_modification(from_mode, to_mode),
                from_mode,
                to_mode,
                to_domain_oid(*previous_id),
                to_domain_oid(*id),
                location.to_string(),
                location.to_string(),
            )
        }
        ChangeDetached::Rewrite {
            source_location,
            source_entry_mode,
            source_id,
            entry_mode,
            id,
            location,
            diff,
            ..
        } => DiffEntry::new(
            ChangeStatus::renamed(rewrite_similarity(diff.as_ref())),
            to_file_mode(*source_entry_mode)?,
            to_file_mode(*entry_mode)?,
            to_domain_oid(*source_id),
            to_domain_oid(*id),
            source_location.to_string(),
            location.to_string(),
        ),
    };
    Ok(Some(entry))
}
