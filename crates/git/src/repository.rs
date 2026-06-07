//! [`GixRepository`]: the gix-backed [`Repository`] adapter.
//!
//! Each method is a faithful rendering of one port operation into gix calls,
//! with the gix→domain translation delegated to [`crate::conv`]. Failures are
//! mapped to the domain's vocabulary: a missing object or ref is
//! [`DomainError::NotFound`], an object of the wrong kind is
//! [`DomainError::Invalid`], and anything else is [`DomainError::Backend`].

use std::path::Path;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blame::Blame;
use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::Diff;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tree::{Tree, TreeEntry};
use gitweb_domain::port::repository::{ArchiveFormat, Page, Repository, SearchQuery};

use crate::conv::{
    backend, to_domain_oid, to_gix_oid, to_object_kind, to_signature, to_tree_entry,
};

/// Read access to one on-disk git repository, backed by gix.
#[derive(Debug)]
pub struct GixRepository {
    repo: gix::Repository,
}

impl GixRepository {
    /// Opens the repository rooted at `path` (a `.git` directory or a bare
    /// repository), failing with [`DomainError::Backend`] if gix cannot.
    pub fn open(path: &Path) -> Result<Self, DomainError> {
        let repo: gix::Repository = gix::open(path).map_err(backend)?;
        Ok(Self { repo })
    }

    /// Finds the object named by `oid`, distinguishing "absent" (→ `NotFound`)
    /// from a backend failure. The wrong-kind check is left to each caller.
    fn require_object(&self, oid: &ObjectId) -> Result<gix::Object<'_>, DomainError> {
        let gix_oid: gix::ObjectId = to_gix_oid(oid)?;
        match self.repo.try_find_object(gix_oid) {
            Ok(Some(object)) => Ok(object),
            Ok(None) => Err(DomainError::NotFound(oid.as_str().to_owned())),
            Err(error) => Err(backend(error)),
        }
    }
}

impl Repository for GixRepository {
    fn head(&self) -> Result<Reference, DomainError> {
        let head: gix::Head<'_> = self.repo.head().map_err(backend)?;
        let target: gix::ObjectId = head
            .id()
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?
            .detach();
        let name: String = match head.referent_name() {
            Some(full) => full.as_bstr().to_string(),
            None => "HEAD".to_owned(),
        };
        Ok(Reference::new(RefName::new(name), to_domain_oid(target)))
    }

    fn references(&self, prefix: &str) -> Result<Vec<Reference>, DomainError> {
        let platform: gix::reference::iter::Platform<'_> =
            self.repo.references().map_err(backend)?;
        let mut out: Vec<Reference> = Vec::new();
        for item in platform.all().map_err(backend)? {
            let reference: gix::Reference<'_> = item.map_err(backend)?;
            let full: String = reference.name().as_bstr().to_string();
            if !full.starts_with(prefix) {
                continue;
            }
            // Direct refs carry their target id; the rare symbolic ref under a
            // prefix is followed to the object it ultimately names.
            let target: ObjectId = match reference.try_id() {
                Some(id) => to_domain_oid(id.detach()),
                None => to_domain_oid(reference.into_fully_peeled_id().map_err(backend)?.detach()),
            };
            out.push(Reference::new(RefName::new(full), target));
        }
        Ok(out)
    }

    fn resolve(&self, rev: &str) -> Result<ObjectId, DomainError> {
        let id: gix::Id<'_> = self.repo.rev_parse_single(rev).map_err(
            |_error: gix::revision::spec::parse::single::Error| {
                DomainError::NotFound(rev.to_owned())
            },
        )?;
        Ok(to_domain_oid(id.detach()))
    }

    fn object_kind(&self, oid: &ObjectId) -> Result<ObjectKind, DomainError> {
        let object: gix::Object<'_> = self.require_object(oid)?;
        Ok(to_object_kind(object.kind))
    }

    fn find_commit(&self, oid: &ObjectId) -> Result<Commit, DomainError> {
        let object: gix::Object<'_> = self.require_object(oid)?;
        let commit: gix::Commit<'_> =
            object
                .try_into_commit()
                .map_err(|_error: gix::object::try_into::Error| {
                    DomainError::Invalid(format!("not a commit: {}", oid.as_str()))
                })?;
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

    fn find_tree(&self, oid: &ObjectId) -> Result<Tree, DomainError> {
        let object: gix::Object<'_> = self.require_object(oid)?;
        let tree: gix::Tree<'_> =
            object
                .try_into_tree()
                .map_err(|_error: gix::object::try_into::Error| {
                    DomainError::Invalid(format!("not a tree: {}", oid.as_str()))
                })?;
        let decoded: gix::objs::TreeRef<'_> = tree.decode().map_err(backend)?;
        let entries: Vec<TreeEntry> = decoded
            .entries
            .iter()
            .map(to_tree_entry)
            .collect::<Result<Vec<TreeEntry>, DomainError>>()?;
        Ok(Tree::new(entries))
    }

    fn find_blob(&self, oid: &ObjectId) -> Result<Blob, DomainError> {
        let object: gix::Object<'_> = self.require_object(oid)?;
        if object.kind != gix::object::Kind::Blob {
            return Err(DomainError::Invalid(format!(
                "not a blob: {}",
                oid.as_str()
            )));
        }
        Ok(Blob::new(object.detach().data))
    }

    fn find_tag(&self, oid: &ObjectId) -> Result<Tag, DomainError> {
        let object: gix::Object<'_> = self.require_object(oid)?;
        let tag: gix::Tag<'_> =
            object
                .try_into_tag()
                .map_err(|_error: gix::object::try_into::Error| {
                    DomainError::Invalid(format!("not a tag: {}", oid.as_str()))
                })?;
        let id: ObjectId = to_domain_oid(tag.id);
        let target: ObjectId = to_domain_oid(tag.target_id().map_err(backend)?.detach());
        let tagger: Option<Signature> = match tag.tagger().map_err(backend)? {
            Some(sig) => Some(to_signature(sig)?),
            None => None,
        };
        let decoded: gix::objs::TagRef<'_> = tag.decode().map_err(backend)?;
        let object_kind: ObjectKind = to_object_kind(decoded.target_kind);
        let name: String = decoded.name.to_string();
        let message: String = decoded.message.to_string();
        Ok(Tag::new(id, target, object_kind, name, tagger, message))
    }

    // --- Later slices of gitweb_in_rust-a10 ----------------------------------
    // History, diff, blame, archive, and search are the remaining adapter
    // operations. They are not yet implemented; no conformance scenario calls
    // them, so nothing falsely passes. Each returns a clearly-labelled backend
    // error until its own red→green slice lands.

    fn history(
        &self,
        _start: &ObjectId,
        _path: Option<&str>,
        _page: Page,
    ) -> Result<Vec<Commit>, DomainError> {
        Err(DomainError::Backend(
            "history: not yet implemented".to_owned(),
        ))
    }

    fn diff(&self, _from: Option<&ObjectId>, _to: &ObjectId) -> Result<Diff, DomainError> {
        Err(DomainError::Backend("diff: not yet implemented".to_owned()))
    }

    fn blame(&self, _at: &ObjectId, _path: &str) -> Result<Blame, DomainError> {
        Err(DomainError::Backend(
            "blame: not yet implemented".to_owned(),
        ))
    }

    fn archive(&self, _tree: &ObjectId, _format: ArchiveFormat) -> Result<Vec<u8>, DomainError> {
        Err(DomainError::Backend(
            "archive: not yet implemented".to_owned(),
        ))
    }

    fn search(&self, _query: &SearchQuery, _page: Page) -> Result<Vec<Commit>, DomainError> {
        Err(DomainError::Backend(
            "search: not yet implemented".to_owned(),
        ))
    }
}
