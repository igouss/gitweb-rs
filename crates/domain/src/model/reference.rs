//! References: a named pointer to an object.
//!
//! Mirrors gitweb's ref lists (`git_get_refs_list`): a fully-qualified ref name
//! paired with the object it resolves to. The display shortening lives in
//! [`RefName`].

use crate::model::object_id::ObjectId;
use crate::model::ref_name::RefName;
use std::borrow::Cow;

/// A git reference: a name bound to a target object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reference {
    name: RefName,
    target: ObjectId,
}

impl Reference {
    /// Binds a ref name to its target object.
    #[must_use]
    pub fn new(name: RefName, target: ObjectId) -> Self {
        Self { name, target }
    }

    /// The fully-qualified ref name.
    #[must_use]
    pub fn name(&self) -> &RefName {
        &self.name
    }

    /// The object the ref points at.
    #[must_use]
    pub fn target(&self) -> &ObjectId {
        &self.target
    }

    /// The display short name (see [`RefName::short`]).
    #[must_use]
    pub fn short(&self) -> Cow<'_, str> {
        self.name.short()
    }
}

/// A reference resolved to the object it ultimately points at, the way
/// `git show-ref --dereference` reports it (gitweb's `git_get_references`). An
/// annotated tag appears here already peeled to its target object and flagged
/// [`indirect`](Self::indirect) — gitweb's `^{}` form, which `format_ref_marker`
/// keys off so a commit row is badged with the tag rather than the tag object.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct DereferencedRef {
    name: RefName,
    target: ObjectId,
    indirect: bool,
}

impl DereferencedRef {
    /// Binds a fully-qualified ref name to the object it ultimately resolves to,
    /// recording whether reaching that object went through a tag object.
    #[must_use]
    pub fn new(name: RefName, target: ObjectId, indirect: bool) -> Self {
        Self {
            name,
            target,
            indirect,
        }
    }

    /// The fully-qualified ref name (e.g. `refs/tags/v1.0`).
    #[must_use]
    pub fn name(&self) -> &RefName {
        &self.name
    }

    /// The object the ref ultimately decorates — for an annotated tag, the
    /// commit it peels to, not the tag object.
    #[must_use]
    pub fn target(&self) -> &ObjectId {
        &self.target
    }

    /// Whether the ref reached [`target`](Self::target) through a tag object —
    /// gitweb's `indirect` (the `^{}` peel). True only for annotated tags.
    #[must_use]
    pub fn indirect(&self) -> bool {
        self.indirect
    }
}
