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
use crate::model::grep::GrepResults;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::Patch;
use crate::model::reference::Reference;
use crate::model::remote::Remote;
use crate::model::tag::Tag;
use crate::model::tree::Tree;

// The snapshot value objects are owned by `model::snapshot` (the format table and
// its rules); the port re-exports the two its `archive` signature names.
pub use crate::model::snapshot::{ArchiveFormat, ArchiveOptions};

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

/// How a commit search interprets its pattern, matching gitweb's `searchtype`.
///
/// These are the facets that *list commits*: gitweb's `commit` / `author` /
/// `committer` (`git log --grep= / --author= / --committer=`) and `pickaxe`
/// (`git log -S`). gitweb's fifth facet, `grep`, is `git grep` listing
/// file/line hits at a single revision — it returns lines, not commits, so it is
/// a separate capability with its own port shape, not a variant here.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    /// Match the commit message (`commit`).
    Commit,
    /// Match the author identity (`author`).
    Author,
    /// Match the committer identity (`committer`).
    Committer,
    /// Match commits that change the number of occurrences of the pattern in
    /// some file (`pickaxe`).
    Pickaxe,
}

/// How aggressively a two-tree diff detects moved and copied content, matching
/// gitweb's site-configured `@diff_opts` (its "rename detection options").
///
/// gitweb's default is `('-M')` — renames only; `('-C')` and
/// `('-C', '--find-copies-harder')` are opt-in and progressively costlier. The
/// three settings differ only in which *sources* a newly added file may be a
/// copy of:
/// - [`RenamesOnly`](RenameDetection::RenamesOnly) (`-M`): no copies at all; a
///   copied file reads as a plain addition.
/// - [`Copies`](RenameDetection::Copies) (`-C`): a copy is found only when its
///   source was itself changed in the same diff (the "modified" set).
/// - [`CopiesHarder`](RenameDetection::CopiesHarder) (`-C --find-copies-harder`):
///   a copy may be found from *any* file in the source tree, including ones the
///   diff leaves untouched.
///
/// Renames are detected at every level (gitweb's `-C` implies `-M`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum RenameDetection {
    /// Renames only (`-M`), gitweb's default.
    #[default]
    RenamesOnly,
    /// Renames plus copies whose source was modified in the same diff (`-C`).
    Copies,
    /// Renames plus copies from any source-tree file (`-C --find-copies-harder`).
    CopiesHarder,
}

impl RenameDetection {
    /// Whether this level looks for copies at all (`-C` or harder).
    #[must_use]
    pub fn detects_copies(self) -> bool {
        matches!(self, Self::Copies | Self::CopiesHarder)
    }

    /// Whether copies may come from files the diff leaves unchanged
    /// (`--find-copies-harder`), as opposed to only the modified set.
    #[must_use]
    pub fn finds_copies_harder(self) -> bool {
        matches!(self, Self::CopiesHarder)
    }
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

    /// The configured remotes, each with the fetch and push URLs `git remote -v`
    /// reports (gitweb's `git_get_remotes_list`), in name order. The
    /// remote-tracking branches are read separately via [`Self::references`] under
    /// `refs/remotes/<name>/`, the way gitweb's `fill_remote_heads` enriches them.
    fn remotes(&self) -> Result<Vec<Remote>, DomainError>;

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

    /// The byte size of the object named by `oid`, read from its header without
    /// inflating its content — gitweb's `git ls-tree -l` size column, which
    /// reports each blob's size. The tree view asks this per file entry; it is the
    /// cheap size lookup, not a `find_blob` that would decode the whole object.
    fn object_size(&self, oid: &ObjectId) -> Result<u64, DomainError>;

    /// The tag name that names `oid`, the way gitweb's `git_get_rev_name_tags`
    /// (`git name-rev --tags <oid>`) supplies the `X-Git-Tag` line on
    /// `commitdiff_plain` / `patch`. `None` when no tag names the commit.
    ///
    /// The returned string is the captured `tags/<name>` body — gitweb's regex
    /// `^<hash> tags/(.*)$` — so a hierarchical tag keeps its path (`release/v1`)
    /// and, mirroring `name-rev`'s one-dereference marker, an *annotated* tag
    /// whose object peels to the commit reads as `<name>^0` while a *lightweight*
    /// tag reads as the bare `<name>`.
    ///
    /// This is `name-rev`'s *distance-zero* case — a tag whose tip is exactly the
    /// commit — which is what a tagged (release) commit stamps. `name-rev`'s
    /// ancestor-distance naming (`<name>~N` for a commit some generations behind a
    /// tag) is deliberately out of scope; an untagged commit reachable only as
    /// such an ancestor reports `None` here rather than a suffixed name. When more
    /// than one tag's tip is the commit, the first in ref-name order wins (git's
    /// own multi-tag tie-break is not replicated). The corpus and the adapter
    /// conformance exercise the lightweight, annotated and no-tag cases.
    fn rev_name_tag(&self, oid: &ObjectId) -> Result<Option<String>, DomainError>;

    /// Reads the annotated tag named by `oid`.
    fn find_tag(&self, oid: &ObjectId) -> Result<Tag, DomainError>;

    /// The object id recorded at `path` within the tree of the tree-ish `at`
    /// (gitweb's `git_get_hash_by_path` / `git ls-tree <at> -- <path>`), or
    /// `None` when `path` is absent there. `at` may name a commit or a tree; it
    /// is peeled to its tree first. The id of a directory `path` is its subtree,
    /// of a file its blob — the caller asks [`Self::object_kind`] for which it
    /// got. This is the seam the per-path history walk resolves a file's type and
    /// its blob at each commit through.
    fn path_id(&self, at: &ObjectId, path: &str) -> Result<Option<ObjectId>, DomainError>;

    /// History reachable from `start`, optionally filtered to commits touching
    /// `path`, windowed by `page` (gitweb's `git rev-list` with `--skip` /
    /// `--max-count`).
    fn history(
        &self,
        start: &ObjectId,
        path: Option<&str>,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError>;

    /// The diff between two trees, with rename/copy detection at the given
    /// `detection` level (gitweb's `@diff_opts`). `from` is `None` to diff
    /// against the empty tree, e.g. for a root commit.
    fn diff(
        &self,
        from: Option<&ObjectId>,
        to: &ObjectId,
        detection: RenameDetection,
    ) -> Result<Diff, DomainError>;

    /// The textual unified diff (patch) between two trees: the same change set
    /// as [`Repository::diff`] at the given `detection` level, rendered as git's
    /// patch text with hunks the way `git diff-tree -p` emits and gitweb's
    /// `commitdiff_plain` / `patch` endpoints stream. `from` is `None` to diff
    /// against the empty tree (a root commit).
    fn patch(
        &self,
        from: Option<&ObjectId>,
        to: &ObjectId,
        detection: RenameDetection,
    ) -> Result<Patch, DomainError>;

    /// The combined diff of a merge `commit` against all its parents at once,
    /// the way gitweb renders `git diff-tree -c`/`--cc`: only paths that differ
    /// from *every* parent appear, each carrying one from-side per parent and a
    /// single merge-result to-side. The adapter reads the merge's parents
    /// itself, so the caller passes just the merge commit.
    fn combined_diff(&self, commit: &ObjectId) -> Result<CombinedDiff, DomainError>;

    /// The repository's default object-id abbreviation length, in hex
    /// characters — git's `core.abbrev` (auto-scaled by object count, never
    /// below 7). The `commitdiff_plain` / `patch` endpoints abbreviate their
    /// `index` lines to this width, matching the short ids bare `git diff-tree
    /// -p` writes, via [`Patch::render_abbreviated`](crate::model::patch::Patch::render_abbreviated).
    fn abbrev_length(&self) -> Result<usize, DomainError>;

    /// Line-by-line blame of `path` as of commit `at`.
    fn blame(&self, at: &ObjectId, path: &str) -> Result<Blame, DomainError>;

    /// A snapshot archive of `tree` in the given `format`, with the top-level
    /// directory and entry modification time `options` carries (gitweb's
    /// `git archive --prefix=<name>/`, stamped with the commit time). `tree` may
    /// be any tree-ish — a commit, tag, or tree — which the adapter peels.
    fn archive(
        &self,
        tree: &ObjectId,
        format: ArchiveFormat,
        options: &ArchiveOptions,
    ) -> Result<Vec<u8>, DomainError>;

    /// Commits matching `query`, rooted at `HEAD` and windowed by `page`
    /// (message / author / committer / pickaxe search). gitweb roots its search
    /// at the current view's revision; with no revision selected that is `HEAD`,
    /// which is the base this port searches from.
    fn search(&self, query: &SearchQuery, page: Page) -> Result<Vec<Commit>, DomainError>;

    /// Content matches for the literal `pattern` over the regular files of
    /// `revision`'s tree, mirroring gitweb's `git_search_files` (`git grep -n -z
    /// -F <pattern> <tree>`): each text-file line containing the pattern (its
    /// path and 1-based number) and each binary file whose bytes contain it
    /// (reported with no line text). `revision` is a tree-ish — gitweb's
    /// `$hash` / `$co{'tree'}` — so a commit or a tree id both name the tree to
    /// search. Matches come out grouped by file in tree order; the result is
    /// capped at [`crate::model::grep::GREP_MATCH_LIMIT`], reporting `trimmed`
    /// when the cap drops further matches.
    fn grep(&self, revision: &ObjectId, pattern: &str) -> Result<GrepResults, DomainError>;
}
