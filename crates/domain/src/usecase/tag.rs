//! The `tag` use case: gitweb's `git_tag` (via `parse_tag`), minus the HTML.
//!
//! gitweb's `git_tag` takes the request hash, `cat-file tag`s it, and renders
//! the resulting tag object: a header naming the tag, a one-row object table
//! linking the *tagged* object by its kind, the tagger authorship when present,
//! and the message body. Anything the hash does not name a tag object for —
//! a missing object, or an object of another kind (a commit, or a lightweight
//! tag's object) — is gitweb's `die_error(404, "Unknown tag object")`.
//!
//! This is the orchestration over the [`Repository`] port. It resolves the hash,
//! classifies it with [`Repository::object_kind`] (so a non-tag is the exact
//! `Unknown tag object` 404 rather than a generic read failure), then reads the
//! tag and assembles a framework-free [`TagView`]. gitweb's `git_tag` renders the
//! tagger through `git_print_authorship_rows` → `format_timestamp_html`, i.e. the
//! tag's own *absolute* date, so this use case needs no clock at all.

use crate::error::DomainError;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::signature::Signature;
use crate::model::tag::Tag;
use crate::model::timestamp::Timestamp;
use crate::port::repository::Repository;

/// The tagger identity on a tag view: who tagged it, their email if the ident
/// carries one, and the absolute date they tagged it — gitweb's
/// `git_print_authorship_rows` for a tag's `tagger` line, whose
/// `format_timestamp_html` renders the tagger's own timestamp.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagAuthor {
    name: String,
    email: Option<String>,
    timestamp: Timestamp,
}

impl TagAuthor {
    /// The tagger's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The tagger's email, if the ident carries one (an explicit empty `<>`
    /// parses to `Some("")`, distinct from an absent email).
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// The absolute date the tag was made (gitweb's `format_timestamp_html` of
    /// the tagger line).
    #[must_use]
    pub fn timestamp(&self) -> &Timestamp {
        &self.timestamp
    }
}

/// One annotated tag, resolved and ready to render: gitweb's `%tag` for
/// `git_tag`, framework-free.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagView {
    name: String,
    tag_id: ObjectId,
    object: ObjectId,
    object_kind: ObjectKind,
    tagger: Option<TagAuthor>,
    message: String,
}

impl TagView {
    /// The tag's name, read from the tag object itself (gitweb's `$tag{'name'}`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The tag object's own id (gitweb's `$tag{'id'}`), for a selflink.
    #[must_use]
    pub fn tag_id(&self) -> &ObjectId {
        &self.tag_id
    }

    /// The id of the object the tag points at (gitweb's `$tag{'object'}`); the
    /// object row links here, via the object's kind.
    #[must_use]
    pub fn object(&self) -> &ObjectId {
        &self.object
    }

    /// The kind of the tagged object (gitweb's `$tag{'type'}`), which decides the
    /// action the object row links to.
    #[must_use]
    pub fn object_kind(&self) -> ObjectKind {
        self.object_kind
    }

    /// The tagger authorship, present only when the tag object carries a tagger
    /// line (gitweb prints the rows only `if defined($tag{'author'})`).
    #[must_use]
    pub fn tagger(&self) -> Option<&TagAuthor> {
        self.tagger.as_ref()
    }

    /// The tag message body (gitweb's `$tag{'comment'}` lines), newline-joined;
    /// the render layer splits it into display lines.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}

/// Resolves the requested `hash` to an annotated tag and assembles its view.
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] with gitweb's `Unknown tag object` message
/// when the hash names an object that is not a tag (a commit, or a lightweight
/// tag's object), and propagates the repository's not-found when the hash
/// resolves to nothing — both gitweb's `die_error(404)`.
pub fn show_tag(repo: &dyn Repository, hash: &str) -> Result<TagView, DomainError> {
    let oid: ObjectId = repo.resolve(hash)?;
    if repo.object_kind(&oid)? != ObjectKind::Tag {
        return Err(DomainError::NotFound("Unknown tag object".to_owned()));
    }
    let tag: Tag = repo.find_tag(&oid)?;
    Ok(view_of(&tag))
}

/// Assembles the view-model from a read tag object: its name and own id, the
/// tagged object and kind (which decides the object row's link), the tagger
/// authorship when present, and the message body.
fn view_of(tag: &Tag) -> TagView {
    TagView {
        name: tag.name().to_owned(),
        tag_id: tag.id().clone(),
        object: tag.object().clone(),
        object_kind: tag.object_kind(),
        tagger: tag.tagger().map(author_of),
        message: tag.message().to_owned(),
    }
}

/// Maps a tagger [`Signature`] to the view's authorship, carrying the tagger's
/// own absolute timestamp (gitweb's `parse_date($tag{author_epoch}, …_tz)`).
fn author_of(who: &Signature) -> TagAuthor {
    TagAuthor {
        name: who.name().to_owned(),
        email: who.email().map(str::to_owned),
        timestamp: Timestamp::from_signature(who),
    }
}
