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
