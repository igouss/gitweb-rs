//! The `tags` use case: gitweb's `git_tags` / `git_get_tags_list`, minus the
//! HTML.
//!
//! gitweb lists the refs under `refs/tags/` with `for-each-ref
//! --sort=-creatordate`, so the rows come out newest-creation-first, then by ref
//! name (git's implicit final key). Each ref is classified the way gitweb's
//! `%(objecttype)` switch does:
//!
//! * an **annotated** tag — its ref points at a tag *object* — is peeled to the
//!   object it tags. That object's id becomes the row's `refid` and its kind the
//!   row's `reftype` (the `*objectname` / `*objecttype` of gitweb), the tag
//!   message's subject is shown, and the row ages by the tagger's time.
//! * a **lightweight** tag points straight at its object: its `reftype` is that
//!   object's own kind, its `refid` is the object itself, it has no subject, and
//!   a lightweight tag of a commit ages by the commit's committer time. A
//!   lightweight tag of a blob or tree carries no time at all.
//!
//! This is the orchestration over the [`Repository`] port, producing a
//! framework-free [`TagsView`] the render adapter turns into a table. The clock
//! is injected (`now`), so ages are computed here, once, and the view-model
//! stays free of any time dependency.

use std::cmp::Ordering;

use crate::error::DomainError;
use crate::model::age::Age;
use crate::model::commit::Commit;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::reference::Reference;
use crate::model::tag::Tag;
use crate::port::repository::Repository;

/// One tag as it appears on the tags listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagRow {
    name: String,
    full_name: String,
    tag_id: ObjectId,
    reftype: ObjectKind,
    refid: ObjectId,
    subject: Option<String>,
    annotated: bool,
    age: Option<Age>,
}

impl TagRow {
    /// The display short tag name (e.g. `v1.0`), `refs/tags/` stripped.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The fully-qualified ref name (`refs/tags/<name>`), for link building.
    #[must_use]
    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    /// The id of the ref's direct target (gitweb's `$tag{'id'}`): the tag object
    /// for an annotated tag, the tagged object for a lightweight one. The single
    /// `tag` view links to this.
    #[must_use]
    pub fn tag_id(&self) -> &ObjectId {
        &self.tag_id
    }

    /// The kind of the object the tag ultimately names (gitweb's `reftype`): the
    /// peeled object's kind for an annotated tag, the object's own kind for a
    /// lightweight one. The name links to this object via its kind's action.
    #[must_use]
    pub fn reftype(&self) -> ObjectKind {
        self.reftype
    }

    /// The id of the object the tag ultimately names (gitweb's `refid`).
    #[must_use]
    pub fn refid(&self) -> &ObjectId {
        &self.refid
    }

    /// The tag message's subject, present only for annotated tags (gitweb only
    /// sets `subject` when the ref is a tag object).
    #[must_use]
    pub fn subject(&self) -> Option<&str> {
        self.subject.as_deref()
    }

    /// Whether the ref points at a tag object (gitweb's `type eq "tag"`); only
    /// then does the row carry a subject and a `tag` selflink.
    #[must_use]
    pub fn annotated(&self) -> bool {
        self.annotated
    }

    /// The tag's age relative to the request time, or `None` when it carries no
    /// creation time (gitweb's "unknown").
    #[must_use]
    pub fn age(&self) -> Option<Age> {
        self.age
    }
}

/// The assembled tags page: the tags in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagsView {
    rows: Vec<TagRow>,
}

impl TagsView {
    /// The tags, in display order.
    #[must_use]
    pub fn rows(&self) -> &[TagRow] {
        &self.rows
    }
}

/// A tag enriched with the bits that decide its order, before it becomes a
/// display row: its creation epoch (the `-creatordate` sort key, also the age
/// source) and the display fields.
struct TagEntry {
    name: String,
    full_name: String,
    tag_id: ObjectId,
    reftype: ObjectKind,
    refid: ObjectId,
    subject: Option<String>,
    annotated: bool,
    epoch: i64,
}

/// Assembles the tags page over the [`Repository`] port. `now` is the
/// request-time epoch the relative ages are measured against.
///
/// # Errors
///
/// Propagates the repository's error if ref listing, classifying a ref, or
/// peeling a tag fails.
pub fn assemble_tags(repo: &dyn Repository, now: i64) -> Result<TagsView, DomainError> {
    let references: Vec<Reference> = repo.references("refs/tags/")?;
    let mut entries: Vec<TagEntry> = references
        .iter()
        .map(|reference: &Reference| entry_for(repo, reference))
        .collect::<Result<Vec<TagEntry>, DomainError>>()?;
    entries.sort_by(order_tags);

    let rows: Vec<TagRow> = entries
        .into_iter()
        .map(|entry: TagEntry| TagRow {
            name: entry.name,
            full_name: entry.full_name,
            tag_id: entry.tag_id,
            reftype: entry.reftype,
            refid: entry.refid,
            subject: entry.subject,
            annotated: entry.annotated,
            age: age_of(entry.epoch, now),
        })
        .collect();
    Ok(TagsView { rows })
}

/// Classifies one tag ref. An annotated tag (the ref names a tag object) is
/// peeled; a lightweight tag (the ref names its object directly) is read in
/// place — a commit for its committer time, a blob or tree with no time at all.
fn entry_for(repo: &dyn Repository, reference: &Reference) -> Result<TagEntry, DomainError> {
    let target: &ObjectId = reference.target();
    match repo.object_kind(target)? {
        ObjectKind::Tag => {
            let tag: Tag = repo.find_tag(target)?;
            Ok(annotated_entry(reference, &tag))
        }
        ObjectKind::Commit => {
            let commit: Commit = repo.find_commit(target)?;
            Ok(lightweight_entry(
                reference,
                ObjectKind::Commit,
                commit.committer().epoch(),
            ))
        }
        // A lightweight tag of a blob or tree carries no creation time: gitweb
        // only reads one for a tag object or a commit.
        other => Ok(lightweight_entry(reference, other, 0)),
    }
}

/// Builds the display fields of an annotated tag from its peeled [`Tag`]. The
/// row's reftype/refid point at the *tagged* object, the tag object's own id
/// (the ref target) is kept for the `tag` selflink, and the subject is the tag
/// message's first line (gitweb's `%(subject)`).
fn annotated_entry(reference: &Reference, tag: &Tag) -> TagEntry {
    TagEntry {
        name: reference.short().into_owned(),
        full_name: reference.name().full().to_owned(),
        tag_id: reference.target().clone(),
        reftype: tag.object_kind(),
        refid: tag.object().clone(),
        subject: Some(subject_line(tag.message())),
        annotated: true,
        epoch: tag.tagger().map_or(0, |who| who.epoch()),
    }
}

/// Builds the display fields of a lightweight tag: its reftype is the object's
/// own kind, its refid is the object itself, and it carries no subject.
fn lightweight_entry(reference: &Reference, kind: ObjectKind, epoch: i64) -> TagEntry {
    let target: ObjectId = reference.target().clone();
    TagEntry {
        name: reference.short().into_owned(),
        full_name: reference.name().full().to_owned(),
        tag_id: target.clone(),
        reftype: kind,
        refid: target,
        subject: None,
        annotated: false,
        epoch,
    }
}

/// The first non-empty line of a tag message — gitweb's `%(subject)`. An empty
/// message yields an empty subject (still shown, since the tag is annotated).
fn subject_line(message: &str) -> String {
    message
        .lines()
        .find(|line: &&str| !line.is_empty())
        .unwrap_or("")
        .to_owned()
}

/// gitweb's `--sort=-creatordate` order: newest creation first, then ref name
/// ascending (git's implicit final key).
fn order_tags(a: &TagEntry, b: &TagEntry) -> Ordering {
    b.epoch
        .cmp(&a.epoch)
        .then_with(|| a.full_name.cmp(&b.full_name))
}

/// The tag's age relative to `now`, or `None` when it carries no creation time
/// (gitweb renders this as "unknown").
fn age_of(epoch: i64, now: i64) -> Option<Age> {
    if epoch == 0 {
        None
    } else {
        Some(Age::from_seconds(now - epoch))
    }
}
