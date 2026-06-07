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
use gitweb_domain::model::change::ChangeStatus;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::{CombinedDiff, CombinedDiffEntry, CombinedParent, Diff};
use gitweb_domain::model::file_mode::{FileKind, FileMode};
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::patch::{FilePatch, Patch};
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tree::{Tree, TreeEntry};
use gitweb_domain::port::repository::{ArchiveFormat, Page, Repository, SearchKind, SearchQuery};

use gitweb_domain::model::diff::DiffEntry;

use crate::conv::{
    absent_mode, backend, is_null_oid, read_commit, to_blame, to_diff_entry, to_domain_oid,
    to_file_mode, to_file_patch, to_gix_oid, to_object_kind, to_signature, to_tree_entry,
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
        let diff: Diff = self.diff(from, commit.id())?;
        for entry in diff.entries() {
            let before: Vec<u8> = self.side_content(entry.from_oid(), entry.from_mode())?;
            let after: Vec<u8> = self.side_content(entry.to_oid(), entry.to_mode())?;
            if count_occurrences(&before, needle) != count_occurrences(&after, needle) {
                return Ok(true);
            }
        }
        Ok(false)
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

    fn diff(&self, from: Option<&ObjectId>, to: &ObjectId) -> Result<Diff, DomainError> {
        // gitweb's `git diff-tree -r -M`: recurse to leaf files, detect renames
        // at git's default 50% similarity. A `None` from-side (a root commit) is
        // gitweb's `--root`, i.e. a diff against the empty tree, so gix's empty
        // tree is left implicit.
        let to_tree: gix::Tree<'_> = self.require_tree(to)?;
        let from_tree: Option<gix::Tree<'_>> = match from {
            Some(oid) => Some(self.require_tree(oid)?),
            None => None,
        };
        let options: gix::diff::Options =
            gix::diff::Options::default().with_rewrites(Some(gix::diff::Rewrites::default()));
        let changes: Vec<gix::object::tree::diff::ChangeDetached> = self
            .repo
            .diff_tree_to_tree(from_tree.as_ref(), Some(&to_tree), Some(options))
            .map_err(backend)?;

        let mut entries: Vec<DiffEntry> = Vec::new();
        for change in &changes {
            if let Some(entry) = to_diff_entry(change)? {
                entries.push(entry);
            }
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

    fn patch(&self, from: Option<&ObjectId>, to: &ObjectId) -> Result<Patch, DomainError> {
        // The structured change set is exactly what `diff` computes; this adds
        // the per-file blob reads and the hunk computation, then hands each file
        // to the domain's patch formatter. `diff` already maps a missing object
        // to `NotFound` and a `None` from-side to the empty tree (gitweb
        // `--root`), so a root commit and an invalid id are handled there.
        let diff: Diff = self.diff(from, to)?;
        let mut files: Vec<FilePatch> = Vec::with_capacity(diff.entries().len());
        for entry in diff.entries() {
            let before: Vec<u8> = self.side_content(entry.from_oid(), entry.from_mode())?;
            let after: Vec<u8> = self.side_content(entry.to_oid(), entry.to_mode())?;
            files.push(to_file_patch(entry, &before, &after));
        }
        Ok(Patch::new(files))
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

    fn archive(&self, _tree: &ObjectId, _format: ArchiveFormat) -> Result<Vec<u8>, DomainError> {
        Err(DomainError::Backend(
            "archive: not yet implemented".to_owned(),
        ))
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
