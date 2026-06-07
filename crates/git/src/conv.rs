//! Translation between gix's read types and the domain's git entities.
//!
//! This is the only module that speaks both vocabularies. Keeping it apart lets
//! the adapter proper read as a description of *what* the port asks for, not as
//! a mess of `.to_string()` and byte juggling. Everything here is pure.

use gitweb_domain::error::DomainError;
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
