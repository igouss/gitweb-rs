//! [`GixRepository`]: the gix-backed [`Repository`] adapter.
//!
//! Each method is a faithful rendering of one port operation into gix calls,
//! with the gix→domain translation delegated to [`crate::conv`]. Failures are
//! mapped to the domain's vocabulary: a missing object or ref is
//! [`DomainError::NotFound`], an object of the wrong kind is
//! [`DomainError::Invalid`], and anything else is [`DomainError::Backend`].

use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use gix::revision::walk::Sorting;
use gix::traverse::commit::simple::CommitTimeOrder;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blame::Blame;
use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::change::{ChangeKind, ChangeStatus};
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::{CombinedDiff, CombinedDiffEntry, CombinedParent, Diff};
use gitweb_domain::model::file_mode::{FileKind, FileMode};
use gitweb_domain::model::grep::{GREP_MATCH_LIMIT, GrepMatch, GrepResults, file_matches};
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::patch::{FilePatch, Patch};
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::{DereferencedRef, Reference};
use gitweb_domain::model::remote::Remote;
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tree::{Tree, TreeEntry};
use gitweb_domain::port::repository::{
    ArchiveFormat, ArchiveOptions, Page, RenameDetection, Repository, SearchKind, SearchQuery,
};

use gitweb_domain::model::diff::DiffEntry;

use crate::conv::{
    absent_mode, backend, is_null_oid, read_commit, rewrite_similarity, to_blame, to_diff_entry,
    to_domain_oid, to_file_mode, to_file_patch, to_gix_oid, to_object_kind, to_signature,
    to_tree_entry,
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

    /// Loads the commit named by `id` straight from the object database, for the
    /// history walk where the id is already known to name a commit.
    fn commit_at(&self, id: gix::ObjectId) -> Result<gix::Commit<'_>, DomainError> {
        self.repo
            .find_object(id)
            .map_err(backend)?
            .try_into_commit()
            .map_err(|_error: gix::object::try_into::Error| {
                backend(format!("rev-list yielded a non-commit: {id}"))
            })
    }

    /// The object id recorded at `path` in `commit`'s tree, or `None` if the
    /// path is absent there.
    fn entry_oid_at(
        &self,
        commit: &gix::Commit<'_>,
        path: &str,
    ) -> Result<Option<gix::ObjectId>, DomainError> {
        let tree: gix::Tree<'_> = commit.tree().map_err(backend)?;
        let entry: Option<gix::object::tree::Entry<'_>> =
            tree.lookup_entry_by_path(path).map_err(backend)?;
        Ok(entry.map(|found: gix::object::tree::Entry<'_>| found.id().detach()))
    }

    /// Peels the object named by `oid` to its tree — accepting a commit or a
    /// tree id, as `git diff-tree` does. A missing object is
    /// [`DomainError::NotFound`]; an object that cannot be a tree (e.g. a blob)
    /// is [`DomainError::Invalid`].
    fn require_tree(&self, oid: &ObjectId) -> Result<gix::Tree<'_>, DomainError> {
        let object: gix::Object<'_> = self.require_object(oid)?;
        object
            .peel_to_tree()
            .map_err(|_error: gix::object::peel::to_kind::Error| {
                DomainError::Invalid(format!("not a tree-ish: {}", oid.as_str()))
            })
    }

    /// The raw bytes of one side of a changed path, for diffing.
    ///
    /// The absent side of an addition or deletion (the null id) has no blob, so
    /// it is the empty buffer. A gitlink (submodule) entry points at a commit in
    /// another repository that is not present here, so git's synthetic
    /// `Subproject commit <id>` line stands in for its "content". Otherwise the
    /// blob is read straight from the object database.
    fn side_content(&self, oid: &ObjectId, mode: FileMode) -> Result<Vec<u8>, DomainError> {
        if is_null_oid(oid) {
            return Ok(Vec::new());
        }
        if matches!(mode.kind(), FileKind::Submodule) {
            return Ok(format!("Subproject commit {}\n", oid.as_str()).into_bytes());
        }
        let object: gix::Object<'_> = self.require_object(oid)?;
        if object.kind != gix::object::Kind::Blob {
            return Err(DomainError::Invalid(format!(
                "not a blob: {}",
                oid.as_str()
            )));
        }
        Ok(object.detach().data)
    }

    /// Whether `commit` changed `path` relative to its first parent — gitweb's
    /// path-limited `rev-list <start> -- <path>` over the linear (non-merge)
    /// history its file logs walk. A root commit counts as touching `path` when
    /// the path is present in it; any difference (add, modify, or delete)
    /// against the first parent counts otherwise.
    fn commit_touches(&self, commit: &gix::Commit<'_>, path: &str) -> Result<bool, DomainError> {
        let here: Option<gix::ObjectId> = self.entry_oid_at(commit, path)?;
        match commit.parent_ids().next() {
            None => Ok(here.is_some()),
            Some(first_parent) => {
                let parent: gix::Commit<'_> = self.commit_at(first_parent.detach())?;
                let there: Option<gix::ObjectId> = self.entry_oid_at(&parent, path)?;
                Ok(here != there)
            }
        }
    }

    /// Every leaf file in `tree_ish`'s tree, keyed by full path — the recursive
    /// `git ls-tree -r` view git compares per path. `tree_ish` may name a commit
    /// or a tree (peeled via [`Self::require_tree`]); subtree entries are
    /// dropped, leaving only the blob/symlink/gitlink leaves a combined diff
    /// compares.
    fn tree_leaves(&self, tree_ish: &ObjectId) -> Result<BTreeMap<String, Side>, DomainError> {
        let tree: gix::Tree<'_> = self.require_tree(tree_ish)?;
        let records: Vec<gix::traverse::tree::recorder::Entry> =
            tree.traverse().breadthfirst.files().map_err(backend)?;
        let mut leaves: BTreeMap<String, Side> = BTreeMap::new();
        for record in &records {
            if record.mode.is_tree() {
                continue;
            }
            let mode: FileMode = to_file_mode(record.mode)?;
            let oid: ObjectId = to_domain_oid(record.oid);
            leaves.insert(record.filepath.to_string(), (mode, oid));
        }
        Ok(leaves)
    }

    /// The revision gitweb roots a search at: the object `HEAD` resolves to.
    fn search_base(&self) -> Result<ObjectId, DomainError> {
        Ok(self.head()?.target().clone())
    }

    /// History reachable from `start`, newest first, keeping the commits for
    /// which `keep` holds, then windowed by `page`. The walk mirrors
    /// [`Repository::history`] so search ordering and pagination match the rest
    /// of the adapter; the predicate decides membership over the already-parsed
    /// domain [`Commit`].
    fn walk_matching<F>(
        &self,
        start: &ObjectId,
        page: Page,
        mut keep: F,
    ) -> Result<Vec<Commit>, DomainError>
    where
        F: FnMut(&Commit) -> Result<bool, DomainError>,
    {
        let start_oid: gix::ObjectId = to_gix_oid(start)?;
        let order: Sorting = Sorting::ByCommitTime(CommitTimeOrder::NewestFirst);
        let walk: gix::revision::Walk<'_> = self
            .repo
            .rev_walk([start_oid])
            .sorting(order)
            .all()
            .map_err(backend)?;

        let mut out: Vec<Commit> = Vec::new();
        let mut skipped: usize = 0;
        for step in walk {
            if out.len() >= page.limit {
                break;
            }
            let info: gix::revision::walk::Info<'_> = step.map_err(backend)?;
            let commit: gix::Commit<'_> = self.commit_at(info.id)?;
            let domain: Commit = read_commit(&commit)?;
            if !keep(&domain)? {
                continue;
            }
            if skipped < page.skip {
                skipped += 1;
                continue;
            }
            out.push(domain);
        }
        Ok(out)
    }

    /// Whether `commit` changes the number of occurrences of `pattern` in some
    /// file against its first parent — git's pickaxe (`-S`) over the rename-aware
    /// diff (`-M`). A root commit is diffed against the empty tree, so a pattern
    /// in its tree counts as added. The match is byte-exact and case-sensitive,
    /// unlike the message/author/committer searches.
    fn pickaxe_matches(&self, commit: &Commit, pattern: &str) -> Result<bool, DomainError> {
        let needle: &[u8] = pattern.as_bytes();
        let from: Option<&ObjectId> = commit.parents().first();
        let diff: Diff = self.diff(from, commit.id(), RenameDetection::RenamesOnly)?;
        for entry in diff.entries() {
            let before: Vec<u8> = self.side_content(entry.from_oid(), entry.from_mode())?;
            let after: Vec<u8> = self.side_content(entry.to_oid(), entry.to_mode())?;
            if count_occurrences(&before, needle) != count_occurrences(&after, needle) {
                return Ok(true);
            }
        }
        Ok(false)
    }

    /// The leaf changes between two trees under the given gix rewrite options,
    /// mapped to domain entries. Shared by the renames-only base pass and any
    /// later pass; directory entries are dropped by [`to_diff_entry`].
    fn changed_entries(
        &self,
        from_tree: Option<&gix::Tree<'_>>,
        to_tree: &gix::Tree<'_>,
        rewrites: gix::diff::Rewrites,
    ) -> Result<Vec<DiffEntry>, DomainError> {
        let options: gix::diff::Options =
            gix::diff::Options::default().with_rewrites(Some(rewrites));
        let changes: Vec<gix::object::tree::diff::ChangeDetached> = self
            .repo
            .diff_tree_to_tree(from_tree, Some(to_tree), Some(options))
            .map_err(backend)?;
        let mut entries: Vec<DiffEntry> = Vec::new();
        for change in &changes {
            if let Some(entry) = to_diff_entry(change)? {
                entries.push(entry);
            }
        }
        Ok(entries)
    }

    /// The copies gix detects between two trees at the given level: each one's
    /// destination path, the source it was copied from, and gix's similarity
    /// hint. Only the copy pairs are kept — renames and modifications from this
    /// pass are discarded, since the renames-only base already holds them
    /// faithfully (gix's copy pass would shadow a modified source's own change).
    fn copy_hints(
        &self,
        from_tree: &gix::Tree<'_>,
        to_tree: &gix::Tree<'_>,
        detection: RenameDetection,
    ) -> Result<Vec<CopyHint>, DomainError> {
        let options: gix::diff::Options =
            gix::diff::Options::default().with_rewrites(Some(rewrites_for(detection)));
        let changes: Vec<gix::object::tree::diff::ChangeDetached> = self
            .repo
            .diff_tree_to_tree(Some(from_tree), Some(to_tree), Some(options))
            .map_err(backend)?;
        Ok(changes.iter().filter_map(copy_hint).collect())
    }

    /// Turns the additions gix flagged as copies into [`ChangeKind::Copied`]
    /// entries, in place. Each copy's from-side is the source's *pre-image* in
    /// the old tree (not gix's source id, which for a modified source is its
    /// post-image); a byte-identical pre-image is an exact copy at 100%, else
    /// gix's similarity hint stands (an approximation, like rename similarity).
    fn reclassify_copies(
        &self,
        entries: &mut [DiffEntry],
        hints: &[CopyHint],
        from_tree: &gix::Tree<'_>,
    ) -> Result<(), DomainError> {
        for hint in hints {
            self.reclassify_one_copy(entries, hint, from_tree)?;
        }
        Ok(())
    }

    /// Reclassifies the single addition matching one copy hint. A hint whose
    /// source is absent from the old tree, or whose destination is not an
    /// addition in the base set, is left alone — nothing to rewrite.
    fn reclassify_one_copy(
        &self,
        entries: &mut [DiffEntry],
        hint: &CopyHint,
        from_tree: &gix::Tree<'_>,
    ) -> Result<(), DomainError> {
        let source: gix::object::tree::Entry<'_> = match from_tree
            .lookup_entry_by_path(hint.source_path.as_str())
            .map_err(backend)?
        {
            Some(entry) => entry,
            None => return Ok(()),
        };
        let from_mode: FileMode = to_file_mode(source.mode())?;
        let from_oid: ObjectId = to_domain_oid(source.id().detach());

        let index: usize = match entries.iter().position(|entry: &DiffEntry| {
            entry.to_path() == hint.dest_path && entry.status().kind() == ChangeKind::Added
        }) {
            Some(index) => index,
            None => return Ok(()),
        };

        let (to_mode, to_oid): (FileMode, ObjectId) = {
            let added: &DiffEntry = &entries[index];
            (added.to_mode(), added.to_oid().clone())
        };
        let similarity: u8 = if from_oid == to_oid {
            100
        } else {
            hint.similarity
        };
        entries[index] = DiffEntry::new(
            ChangeStatus::copied(similarity),
            from_mode,
            to_mode,
            from_oid,
            to_oid,
            hint.source_path.clone(),
            hint.dest_path.clone(),
        );
        Ok(())
    }
}

/// One copy gix detected: where the copy landed, the path it was copied from,
/// and gix's similarity hint (used only when the copy is not byte-exact).
struct CopyHint {
    source_path: String,
    dest_path: String,
    similarity: u8,
}

/// The gix rewrite options for a detection level. Renames track at git's default
/// 50% similarity at every level; copies are off for `RenamesOnly`, drawn from
/// the modified set for `Copies` (`-C`), and from every source-tree file for
/// `CopiesHarder` (`-C --find-copies-harder`).
/// git's auto `core.abbrev`: roughly half the bits needed to count the objects,
/// floored at 7 — the same `len.div_ceil(2).max(7)` over the packed-object count
/// that gix's `Id::shorten` uses, so our abbreviated `index` ids match the width
/// `git diff-tree -p` writes for the same repository.
fn auto_hex_len(num_packed_objects: u64) -> usize {
    let bits: u32 = u64::BITS - num_packed_objects.leading_zeros();
    bits.div_ceil(2).max(7) as usize
}

fn rewrites_for(detection: RenameDetection) -> gix::diff::Rewrites {
    let copies: Option<gix::diff::rewrites::Copies> = detection.detects_copies().then(|| {
        let source: gix::diff::rewrites::CopySource = if detection.finds_copies_harder() {
            gix::diff::rewrites::CopySource::FromSetOfModifiedFilesAndAllSources
        } else {
            gix::diff::rewrites::CopySource::FromSetOfModifiedFiles
        };
        gix::diff::rewrites::Copies {
            source,
            percentage: Some(0.5),
        }
    });
    gix::diff::Rewrites {
        copies,
        ..gix::diff::Rewrites::default()
    }
}

/// The copy hint for one change, or `None` if it is not a copy. Renames
/// (`copy: false`) are excluded — the base set already carries them.
fn copy_hint(change: &gix::object::tree::diff::ChangeDetached) -> Option<CopyHint> {
    match change {
        gix::object::tree::diff::ChangeDetached::Rewrite {
            copy: true,
            source_location,
            location,
            diff,
            ..
        } => Some(CopyHint {
            source_path: source_location.to_string(),
            dest_path: location.to_string(),
            similarity: rewrite_similarity(diff.as_ref()),
        }),
        _ => None,
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

    fn dereferenced_references(&self) -> Result<Vec<DereferencedRef>, DomainError> {
        // gitweb's `git_get_references`: `show-ref --dereference`. Every ref is
        // followed to the object it ultimately names; an annotated tag's ref
        // points at a tag object that peels to that object, so it arrives
        // `indirect`. A symbolic ref (no direct id, e.g. `refs/remotes/*/HEAD`)
        // is followed too but is not a tag, so it is never flagged indirect.
        let platform: gix::reference::iter::Platform<'_> =
            self.repo.references().map_err(backend)?;
        let mut out: Vec<DereferencedRef> = Vec::new();
        for item in platform.all().map_err(backend)? {
            let reference: gix::Reference<'_> = item.map_err(backend)?;
            let full: String = reference.name().as_bstr().to_string();
            let direct: Option<gix::ObjectId> =
                reference.try_id().map(|id: gix::Id<'_>| id.detach());
            let peeled: gix::ObjectId = reference.into_fully_peeled_id().map_err(backend)?.detach();
            let indirect: bool = direct.is_some_and(|id: gix::ObjectId| id != peeled);
            out.push(DereferencedRef::new(
                RefName::new(full),
                to_domain_oid(peeled),
                indirect,
            ));
        }
        // gitweb badges refs in show-ref order (sorted by full name); gix yields
        // them packed-then-loose, so sort to keep the badges deterministic.
        out.sort_by(|a: &DereferencedRef, b: &DereferencedRef| {
            a.name().full().cmp(b.name().full())
        });
        Ok(out)
    }

    fn remotes(&self) -> Result<Vec<Remote>, DomainError> {
        // gitweb parses `git remote -v`; gix reads the same `[remote "<name>"]`
        // config. `remote_names()` is a BTreeSet, so the remotes come out in name
        // order. The URLs are read WITHOUT insteadOf rewriting, to mirror what
        // `git remote -v` shows; gix's push URL falls back to the fetch url, the
        // same fallback git's push line uses, so a url-only remote reports an equal
        // fetch and push (gitweb collapses it to one "URL" row).
        let mut out: Vec<Remote> = Vec::new();
        for name in self.repo.remote_names() {
            let remote: gix::Remote<'_> =
                match self.repo.try_find_remote_without_url_rewrite(name.as_ref()) {
                    Some(result) => result.map_err(backend)?,
                    None => continue,
                };
            let fetch: Option<String> = remote
                .url(gix::remote::Direction::Fetch)
                .map(|url: &gix::Url| url.to_bstring().to_string());
            let push: Option<String> = remote
                .url(gix::remote::Direction::Push)
                .map(|url: &gix::Url| url.to_bstring().to_string());
            out.push(Remote::new(name.to_string(), fetch, push));
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
        read_commit(&commit)
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

    fn object_size(&self, oid: &ObjectId) -> Result<u64, DomainError> {
        // `find_header` reads only the object's type and length from the odb — no
        // inflation of the content, mirroring `git ls-tree -l`'s size column.
        let gix_oid: gix::ObjectId = to_gix_oid(oid)?;
        let header: gix::odb::find::Header = self.repo.find_header(gix_oid).map_err(backend)?;
        Ok(header.size())
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

    fn rev_name_tag(&self, oid: &ObjectId) -> Result<Option<String>, DomainError> {
        // gitweb's `git_get_rev_name_tags`: `git name-rev --tags <oid>`, keeping
        // the captured `tags/<name>`. We implement name-rev's distance-zero case
        // — a tag whose tip is exactly this commit. A lightweight tag points
        // straight at the commit (its ref id is the commit), so it names it by the
        // bare `<name>`; an annotated tag's ref points at a tag object that peels
        // to the commit, which name-rev marks with one dereference, `<name>^0`.
        // Ancestor-distance naming (`<name>~N`) and git's multi-tag tie-break are
        // out of scope (see the port doc); candidates are gathered in ref-name
        // order so the first match is deterministic.
        let target: gix::ObjectId = to_gix_oid(oid)?;
        let platform: gix::reference::iter::Platform<'_> =
            self.repo.references().map_err(backend)?;
        // The winning tag, keyed by its short ref name so ties break in ref-name
        // order regardless of gix's iteration order.
        let mut best: Option<(String, String)> = None;
        for item in platform.all().map_err(backend)? {
            let reference: gix::Reference<'_> = item.map_err(backend)?;
            let full: String = reference.name().as_bstr().to_string();
            let Some(short) = full.strip_prefix("refs/tags/") else {
                continue;
            };
            // The ref's own target distinguishes the tag shape: equal to the
            // peeled commit means lightweight; a tag object in between means
            // annotated.
            let direct: Option<gix::ObjectId> =
                reference.try_id().map(|id: gix::Id<'_>| id.detach());
            let peeled: gix::ObjectId = reference.into_fully_peeled_id().map_err(backend)?.detach();
            if peeled != target {
                continue;
            }
            let annotated: bool = direct != Some(peeled);
            let name: String = if annotated {
                format!("{short}^0")
            } else {
                short.to_owned()
            };
            match &best {
                Some((best_short, _)) if best_short.as_str() <= short => {}
                _ => best = Some((short.to_owned(), name)),
            }
        }
        Ok(best.map(|(_, name): (String, String)| name))
    }

    fn path_id(&self, at: &ObjectId, path: &str) -> Result<Option<ObjectId>, DomainError> {
        // gitweb's `git_get_hash_by_path` / `git ls-tree <at> -- <path>`: peel the
        // tree-ish to its tree, then look the path up. An absent path is `None`,
        // not an error — the per-path history walk relies on that to skip the
        // commits that deleted the file.
        let tree: gix::Tree<'_> = self.require_tree(at)?;
        let entry: Option<gix::object::tree::Entry<'_>> =
            tree.lookup_entry_by_path(path).map_err(backend)?;
        Ok(entry.map(|found: gix::object::tree::Entry<'_>| to_domain_oid(found.id().detach())))
    }

    // --- Later slices of gitweb_in_rust-a10 ----------------------------------
    // History, diff, blame, archive, and search are the remaining adapter
    // operations. They are not yet implemented; no conformance scenario calls
    // them, so nothing falsely passes. Each returns a clearly-labelled backend
    // error until its own red→green slice lands.

    fn history(
        &self,
        start: &ObjectId,
        path: Option<&str>,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError> {
        // gitweb's `git rev-list --header --max-count --skip <start> -- [path]`:
        // newest first by commit time, optionally path-limited, then windowed.
        let start_oid: gix::ObjectId = to_gix_oid(start)?;
        let order: Sorting = Sorting::ByCommitTime(CommitTimeOrder::NewestFirst);
        let walk: gix::revision::Walk<'_> = self
            .repo
            .rev_walk([start_oid])
            .sorting(order)
            .all()
            .map_err(backend)?;

        let mut out: Vec<Commit> = Vec::new();
        let mut skipped: usize = 0;
        for step in walk {
            if out.len() >= page.limit {
                break;
            }
            let info: gix::revision::walk::Info<'_> = step.map_err(backend)?;
            let commit: gix::Commit<'_> = self.commit_at(info.id)?;
            if let Some(wanted) = path
                && !self.commit_touches(&commit, wanted)?
            {
                continue;
            }
            if skipped < page.skip {
                skipped += 1;
                continue;
            }
            out.push(read_commit(&commit)?);
        }
        Ok(out)
    }

    fn diff(
        &self,
        from: Option<&ObjectId>,
        to: &ObjectId,
        detection: RenameDetection,
    ) -> Result<Diff, DomainError> {
        // gitweb's `git diff-tree -r -M`: recurse to leaf files, detect renames
        // at git's default 50% similarity. A `None` from-side (a root commit) is
        // gitweb's `--root`, i.e. a diff against the empty tree, so gix's empty
        // tree is left implicit. This renames-only pass is always the base set —
        // never gix's own copy pass, which would shadow a copy source's *own*
        // modification (see `reclassify_copies`).
        let to_tree: gix::Tree<'_> = self.require_tree(to)?;
        let from_tree: Option<gix::Tree<'_>> = match from {
            Some(oid) => Some(self.require_tree(oid)?),
            None => None,
        };
        let mut entries: Vec<DiffEntry> = self.changed_entries(
            from_tree.as_ref(),
            &to_tree,
            rewrites_for(RenameDetection::RenamesOnly),
        )?;

        // Copy detection (gitweb `-C` / `-C --find-copies-harder`) is layered on
        // top of the faithful renames-only base. A second gix pass — with copies
        // enabled — tells us which additions are copies and from where; we then
        // recompute each copy's from-side against the *old* tree (its pre-image)
        // and reclassify the matching addition, leaving every base modification
        // untouched. Copies need a from-side, so a root commit has none.
        if detection.detects_copies()
            && let Some(from_tree) = from_tree.as_ref()
        {
            let hints: Vec<CopyHint> = self.copy_hints(from_tree, &to_tree, detection)?;
            self.reclassify_copies(&mut entries, &hints, from_tree)?;
        }

        // gix's traversal interleaves directory and leaf changes; sort by path so
        // the diff is stable regardless of traversal order.
        entries.sort_by(|a: &DiffEntry, b: &DiffEntry| {
            a.to_path()
                .cmp(b.to_path())
                .then_with(|| a.from_path().cmp(b.from_path()))
        });
        Ok(Diff::new(entries))
    }

    fn patch(
        &self,
        from: Option<&ObjectId>,
        to: &ObjectId,
        detection: RenameDetection,
    ) -> Result<Patch, DomainError> {
        // The structured change set is exactly what `diff` computes; this adds
        // the per-file blob reads and the hunk computation, then hands each file
        // to the domain's patch formatter. `diff` already maps a missing object
        // to `NotFound` and a `None` from-side to the empty tree (gitweb
        // `--root`), so a root commit and an invalid id are handled there.
        let diff: Diff = self.diff(from, to, detection)?;
        let mut files: Vec<FilePatch> = Vec::with_capacity(diff.entries().len());
        for entry in diff.entries() {
            let before: Vec<u8> = self.side_content(entry.from_oid(), entry.from_mode())?;
            let after: Vec<u8> = self.side_content(entry.to_oid(), entry.to_mode())?;
            files.push(to_file_patch(entry, &before, &after));
        }
        Ok(Patch::new(files))
    }

    fn abbrev_length(&self) -> Result<usize, DomainError> {
        // gitweb leaves `core.abbrev` unset, so git auto-scales the default from
        // the object count, never going below 7. gix's own `Id::shorten` floors
        // its auto length the same way (`calculate_auto_hex_len`); reproduce that
        // floor over the packed-object count so the `index` ids we abbreviate
        // match the width `git diff-tree -p` writes. A configured `core.abbrev`
        // is not honoured (gitweb does not set one); the default is what the
        // format-stable endpoints need.
        let packed: u64 = self.repo.objects.packed_object_count().map_err(backend)?;
        Ok(auto_hex_len(packed))
    }

    fn combined_diff(&self, commit: &ObjectId) -> Result<CombinedDiff, DomainError> {
        // gitweb's `git diff-tree -c <merge>`: compare the merge against every
        // parent at once and show only paths that differ from EVERY parent — a
        // path matching even one parent was taken verbatim from it, so the merge
        // resolved nothing there. gix has no combined diff, so we compose one
        // from per-parent, path-keyed tree comparisons.
        let merge_object: gix::Object<'_> = self.require_object(commit)?;
        let merge: gix::Commit<'_> =
            merge_object
                .try_into_commit()
                .map_err(|_error: gix::object::try_into::Error| {
                    DomainError::Invalid(format!("not a commit: {}", commit.as_str()))
                })?;
        let parent_ids: Vec<ObjectId> = merge
            .parent_ids()
            .map(|id: gix::Id<'_>| to_domain_oid(id.detach()))
            .collect();

        let merge_leaves: BTreeMap<String, Side> = self.tree_leaves(commit)?;
        let mut parent_leaves: Vec<BTreeMap<String, Side>> = Vec::with_capacity(parent_ids.len());
        for parent in &parent_ids {
            parent_leaves.push(self.tree_leaves(parent)?);
        }

        let null_oid: ObjectId = commit.null_like();
        let entries: Vec<CombinedDiffEntry> = combine(&merge_leaves, &parent_leaves, &null_oid);
        Ok(CombinedDiff::new(entries))
    }

    fn blame(&self, at: &ObjectId, path: &str) -> Result<Blame, DomainError> {
        // gitweb's blame first parses the base commit (404 if absent) and looks
        // up the file at it (404 if absent); mirror those two checks so missing
        // commit and missing path are clean `NotFound`s, not opaque backend
        // failures buried in gix-blame's traversal.
        let object: gix::Object<'_> = self.require_object(at)?;
        let commit: gix::Commit<'_> =
            object
                .try_into_commit()
                .map_err(|_error: gix::object::try_into::Error| {
                    DomainError::Invalid(format!("not a commit: {}", at.as_str()))
                })?;
        if self.entry_oid_at(&commit, path)?.is_none() {
            return Err(DomainError::NotFound(path.to_owned()));
        }
        let suspect: gix::ObjectId = commit.id;

        // `git blame` follows a file across a whole-file rename by default, so
        // rename tracking is enabled to match gitweb's plain `git blame -p`.
        let options: gix::repository::blame_file::Options = gix::repository::blame_file::Options {
            rewrites: Some(gix::diff::Rewrites::default()),
            ..Default::default()
        };
        let outcome: gix::blame::Outcome = self
            .repo
            .blame_file(gix::bstr::BStr::new(path), suspect, options)
            .map_err(backend)?;
        Ok(to_blame(outcome))
    }

    fn archive(
        &self,
        tree: &ObjectId,
        format: ArchiveFormat,
        options: &ArchiveOptions,
    ) -> Result<Vec<u8>, DomainError> {
        // gitweb's git_snapshot archives `$hash` (a commit or tree-ish), peeling
        // to its tree; a non-tree-ish is a 400. `require_tree` gives the same
        // mapping: a missing object is NotFound, a blob or other non-tree-ish is
        // Invalid. The resolved tree id is what gix-archive streams.
        let resolved: gix::Tree<'_> = self.require_tree(tree)?;
        let tree_id: gix::ObjectId = resolved.id().detach();
        crate::archive::archive_bytes(&self.repo, tree_id, format, options)
    }

    fn search(&self, query: &SearchQuery, page: Page) -> Result<Vec<Commit>, DomainError> {
        // gitweb roots search at the current view's revision; with none selected
        // that is HEAD. Message/author/committer are a case-insensitive substring
        // (gitweb's `--regexp-ignore-case --fixed-strings`); pickaxe is git's
        // `-S`, a case-sensitive change in a pattern's occurrence count.
        let start: ObjectId = self.search_base()?;
        let pattern: &str = &query.pattern;
        match query.kind {
            SearchKind::Commit => self.walk_matching(&start, page, |commit: &Commit| {
                Ok(contains_ci(commit.message(), pattern))
            }),
            SearchKind::Author => self.walk_matching(&start, page, |commit: &Commit| {
                Ok(contains_ci(&ident_string(commit.author()), pattern))
            }),
            SearchKind::Committer => self.walk_matching(&start, page, |commit: &Commit| {
                Ok(contains_ci(&ident_string(commit.committer()), pattern))
            }),
            SearchKind::Pickaxe => self.walk_matching(&start, page, |commit: &Commit| {
                self.pickaxe_matches(commit, pattern)
            }),
        }
    }

    fn grep(&self, revision: &ObjectId, pattern: &str) -> Result<GrepResults, DomainError> {
        // gitweb's `git grep -n -z -F <pattern> <tree>`: gix has no git-grep, so
        // walk the tree's leaves (path order, recursing into subtrees) and grep
        // each *regular file* blob. git greps only regular files, so symlinks
        // (a blob, but a link target) and gitlinks (a commit, no content) are
        // skipped. The per-file matching is the domain rule; here we read blobs
        // and apply gitweb's output cap: once more than the limit's worth of
        // matches accrue, the listing is trimmed to the limit and reported so.
        let leaves: BTreeMap<String, Side> = self.tree_leaves(revision)?;
        let mut found: Vec<GrepMatch> = Vec::new();
        for (path, (mode, oid)) in &leaves {
            if !matches!(mode.kind(), FileKind::Regular | FileKind::Executable) {
                continue;
            }
            let content: Vec<u8> = self.side_content(oid, *mode)?;
            found.extend(file_matches(path, &content, pattern));
            if found.len() > GREP_MATCH_LIMIT {
                found.truncate(GREP_MATCH_LIMIT);
                return Ok(GrepResults::new(found, true));
            }
        }
        Ok(GrepResults::new(found, false))
    }
}

/// One side of a path in a tree: its file mode and object id.
type Side = (FileMode, ObjectId);

/// Composes a combined diff from the merge's leaves and each parent's leaves,
/// applying git's `diff-tree -c` rule: a path is shown only if it differs from
/// *every* parent. Paths come out sorted because the working set is a
/// [`BTreeSet`]. With no parents there is nothing to combine, so the diff is
/// empty rather than reporting every path as a phantom change.
fn combine(
    merge: &BTreeMap<String, Side>,
    parents: &[BTreeMap<String, Side>],
    null_oid: &ObjectId,
) -> Vec<CombinedDiffEntry> {
    if parents.is_empty() {
        return Vec::new();
    }
    let mut paths: BTreeSet<&String> = BTreeSet::new();
    paths.extend(merge.keys());
    for parent in parents {
        paths.extend(parent.keys());
    }
    let mut entries: Vec<CombinedDiffEntry> = Vec::new();
    for path in paths {
        if let Some(entry) = combine_path(path, merge.get(path), parents, null_oid) {
            entries.push(entry);
        }
    }
    entries
}

/// Builds the combined entry for one path, or `None` when the path matches some
/// parent (and so is uninteresting in a combined diff). The to-side is the merge
/// result; an absent merge side is a deletion (absent mode, null id).
fn combine_path(
    path: &str,
    merge_side: Option<&Side>,
    parents: &[BTreeMap<String, Side>],
    null_oid: &ObjectId,
) -> Option<CombinedDiffEntry> {
    let differs_from_all: bool = parents
        .iter()
        .all(|parent: &BTreeMap<String, Side>| parent.get(path) != merge_side);
    if !differs_from_all {
        return None;
    }
    let combined_parents: Vec<CombinedParent> = parents
        .iter()
        .map(|parent: &BTreeMap<String, Side>| {
            combined_parent(parent.get(path), merge_side, null_oid)
        })
        .collect();
    let (to_mode, to_oid): (FileMode, ObjectId) = match merge_side {
        Some(side) => (side.0, side.1.clone()),
        None => (absent_mode(), null_oid.clone()),
    };
    Some(CombinedDiffEntry::new(
        combined_parents,
        to_mode,
        to_oid,
        path.to_owned(),
    ))
}

/// One parent's from-side for an included path: its mode and id (absent/null
/// when the path is new to that parent) and the status relative to the merge.
fn combined_parent(
    parent_side: Option<&Side>,
    merge_side: Option<&Side>,
    null_oid: &ObjectId,
) -> CombinedParent {
    let (from_mode, from_oid): (FileMode, ObjectId) = match parent_side {
        Some(side) => (side.0, side.1.clone()),
        None => (absent_mode(), null_oid.clone()),
    };
    CombinedParent::new(
        combined_status(parent_side, merge_side),
        from_mode,
        from_oid,
    )
}

/// The status of a path relative to one parent, mirroring git's per-parent
/// status letter: absent-in-parent is an add, absent-in-merge is a delete, and
/// a present-on-both pair is a modification or a type change (by mode).
fn combined_status(parent_side: Option<&Side>, merge_side: Option<&Side>) -> ChangeStatus {
    match (parent_side, merge_side) {
        (None, Some(_)) => ChangeStatus::added(),
        (Some(_), None) => ChangeStatus::deleted(),
        (Some(from), Some(to)) => ChangeStatus::from_modification(from.0, to.0),
        // Unreachable for an included path: a path absent from both sides does
        // not differ from this parent, so it is never combined.
        (None, None) => ChangeStatus::from_modification(absent_mode(), absent_mode()),
    }
}

/// Case-insensitive substring test, matching git's `--regexp-ignore-case
/// --fixed-strings`: ASCII case folding over a literal (non-regex) pattern.
fn contains_ci(haystack: &str, needle: &str) -> bool {
    haystack
        .to_ascii_lowercase()
        .contains(&needle.to_ascii_lowercase())
}

/// The `Name <email>` identity string git matches `--author=` / `--committer=`
/// against; an identity with no email is just the name.
fn ident_string(signature: &Signature) -> String {
    match signature.email() {
        Some(email) => format!("{} <{}>", signature.name(), email),
        None => signature.name().to_owned(),
    }
}

/// The number of non-overlapping occurrences of `needle` in `haystack` — the
/// count git's pickaxe compares between a change's two sides. An empty needle
/// never occurs, so it never reports a change.
fn count_occurrences(haystack: &[u8], needle: &[u8]) -> usize {
    if needle.is_empty() {
        return 0;
    }
    let mut count: usize = 0;
    let mut index: usize = 0;
    while index + needle.len() <= haystack.len() {
        if &haystack[index..index + needle.len()] == needle {
            count += 1;
            index += needle.len();
        } else {
            index += 1;
        }
    }
    count
}
