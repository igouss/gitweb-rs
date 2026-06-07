//! The `Repository` port: read access to one git repository.
//!
//! This trait is the seam the use cases depend on, so they can be exercised
//! against in-memory fakes while the real implementation drives gix. It is the
//! framework-free contract behind every gitweb action that reads a single
//! repository — log, commit, tree, blob, diff, blame, snapshot, search.
//!
//! It models *what* gitweb asks git for, not *how*: rev resolution, ref
//! listing, object lookup, path-filtered history with pagination, tree-to-tree
//! diff, blame, archive bytes, and commit search. The data shapes are the
//! entities in [`crate::model`]; failures are [`DomainError`].

use crate::error::DomainError;
use crate::model::blame::Blame;
use crate::model::blob::Blob;
use crate::model::commit::Commit;
use crate::model::diff::{CombinedDiff, Diff};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::Patch;
use crate::model::reference::Reference;
use crate::model::tag::Tag;
use crate::model::tree::Tree;

/// A half-open window into a list: skip `skip`, then take at most `limit`.
///
/// gitweb paginates the log this way (`--skip=$page*100 --max-count=100`); the
/// page-number-to-skip arithmetic is a caller policy, captured by
/// [`Page::from_page`].
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Page {
    /// Number of entries to skip from the start.
    pub skip: usize,
    /// Maximum number of entries to return.
    pub limit: usize,
}

impl Page {
    /// A window with an explicit skip and limit.
    #[must_use]
    pub fn new(skip: usize, limit: usize) -> Self {
        Self { skip, limit }
    }

    /// The window for the zero-based `page`, each holding `per_page` entries.
    #[must_use]
    pub fn from_page(page: usize, per_page: usize) -> Self {
        Self {
            skip: page * per_page,
            limit: per_page,
        }
    }
}

/// A snapshot archive format, matching gitweb's enabled snapshot formats.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// gzip-compressed tar (`tgz`).
    TarGz,
    /// bzip2-compressed tar (`tbz2`).
    TarBz2,
    /// xz-compressed tar (`txz`).
    TarXz,
    /// zip (`zip`).
    Zip,
}

/// How a commit search interprets its pattern, matching gitweb's `searchtype`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    /// Match the commit message (`commit`).
    Commit,
    /// Match the author identity (`author`).
    Author,
    /// Match the committer identity (`committer`).
    Committer,
    /// Match file content with `git grep` (`grep`).
    Grep,
    /// Match commits that add or remove the pattern (`pickaxe`).
    Pickaxe,
}

/// A commit search request: what to match and the pattern to match it with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SearchQuery {
    /// Which facet of history to search.
    pub kind: SearchKind,
    /// The pattern to match.
    pub pattern: String,
}

/// Read access to a single git repository.
pub trait Repository {
    /// The repository's `HEAD`.
    fn head(&self) -> Result<Reference, DomainError>;

    /// All references whose full name starts with `prefix`
    /// (e.g. `"refs/heads/"`).
    fn references(&self, prefix: &str) -> Result<Vec<Reference>, DomainError>;

    /// Resolves a revision (ref name, full or abbreviated id, …) to an id.
    fn resolve(&self, rev: &str) -> Result<ObjectId, DomainError>;

    /// The kind of the object named by `oid`.
    fn object_kind(&self, oid: &ObjectId) -> Result<ObjectKind, DomainError>;

    /// Reads the commit named by `oid`.
    fn find_commit(&self, oid: &ObjectId) -> Result<Commit, DomainError>;

    /// Reads the tree named by `oid`.
    fn find_tree(&self, oid: &ObjectId) -> Result<Tree, DomainError>;

    /// Reads the blob named by `oid`.
    fn find_blob(&self, oid: &ObjectId) -> Result<Blob, DomainError>;

    /// Reads the annotated tag named by `oid`.
    fn find_tag(&self, oid: &ObjectId) -> Result<Tag, DomainError>;

    /// History reachable from `start`, optionally filtered to commits touching
    /// `path`, windowed by `page` (gitweb's `git rev-list` with `--skip` /
    /// `--max-count`).
    fn history(
        &self,
        start: &ObjectId,
        path: Option<&str>,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError>;

    /// The diff between two trees. `from` is `None` to diff against the empty
    /// tree, e.g. for a root commit.
    fn diff(&self, from: Option<&ObjectId>, to: &ObjectId) -> Result<Diff, DomainError>;

    /// The textual unified diff (patch) between two trees: the same change set
    /// as [`Repository::diff`], rendered as git's patch text with hunks the way
    /// `git diff-tree -p` emits and gitweb's `commitdiff_plain` / `patch`
    /// endpoints stream. `from` is `None` to diff against the empty tree (a root
    /// commit).
    fn patch(&self, from: Option<&ObjectId>, to: &ObjectId) -> Result<Patch, DomainError>;

    /// The combined diff of a merge `commit` against all its parents at once,
    /// the way gitweb renders `git diff-tree -c`/`--cc`: only paths that differ
    /// from *every* parent appear, each carrying one from-side per parent and a
    /// single merge-result to-side. The adapter reads the merge's parents
    /// itself, so the caller passes just the merge commit.
    fn combined_diff(&self, commit: &ObjectId) -> Result<CombinedDiff, DomainError>;

    /// Line-by-line blame of `path` as of commit `at`.
    fn blame(&self, at: &ObjectId, path: &str) -> Result<Blame, DomainError>;

    /// A snapshot archive of `tree` in the given `format`.
    fn archive(&self, tree: &ObjectId, format: ArchiveFormat) -> Result<Vec<u8>, DomainError>;

    /// Commits matching `query`, windowed by `page` (grep / pickaxe / ident /
    /// message search).
    fn search(&self, query: &SearchQuery, page: Page) -> Result<Vec<Commit>, DomainError>;
}
