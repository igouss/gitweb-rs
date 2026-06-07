//! Annotated tag objects.
//!
//! Mirrors the fields gitweb reads in `parse_tag`: the tagged object and its
//! type, the tag name, the optional tagger identity, and the message. A
//! lightweight tag is not a tag object and never reaches this entity.

use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::signature::Signature;

/// A git annotated tag.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Tag {
    id: ObjectId,
    object: ObjectId,
    object_kind: ObjectKind,
    name: String,
    tagger: Option<Signature>,
    message: String,
}

impl Tag {
    /// Assembles a tag from its structured fields.
    #[must_use]
    pub fn new(
        id: ObjectId,
        object: ObjectId,
        object_kind: ObjectKind,
        name: String,
        tagger: Option<Signature>,
        message: String,
    ) -> Self {
        Self {
            id,
            object,
            object_kind,
            name,
            tagger,
            message,
        }
    }

    /// The tag object's own id.
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// The id of the object this tag points at.
    #[must_use]
    pub fn object(&self) -> &ObjectId {
        &self.object
    }

    /// The kind of the tagged object.
    #[must_use]
    pub fn object_kind(&self) -> ObjectKind {
        self.object_kind
    }

    /// The tag's name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The tagger identity, if the tag carries one.
    #[must_use]
    pub fn tagger(&self) -> Option<&Signature> {
        self.tagger.as_ref()
    }

    /// The tag message.
    #[must_use]
    pub fn message(&self) -> &str {
        &self.message
    }
}
