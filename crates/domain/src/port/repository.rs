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
use crate::model::grep_pattern::GrepPattern;
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::patch::Patch;
use crate::model::pickaxe::PickaxeMatch;
use crate::model::pickaxe_pattern::PickaxePattern;
use crate::model::reference::{DereferencedRef, Reference};
use crate::model::remote::Remote;
use crate::model::search_pattern::SearchPattern;
use crate::model::tag::Tag;
use crate::model::tree::{Tree, TreeEntry};

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

/// How a commit-message search interprets its pattern, matching gitweb's
/// `searchtype`.
///
/// These are the facets `git_search_message` serves: gitweb's `commit` /
/// `author` / `committer` (`git log --grep= / --author= / --committer=`), all of
/// which list commits by a case-insensitive match over the message or an
/// identity. gitweb's two other facets each return a *different* shape and so are
/// their own capabilities, not variants here: `grep` (`git grep`) lists file/line
/// hits at one revision ([`Repository::grep`]), and `pickaxe` (`git log -S`) lists
/// commits *with their changed files* ([`Repository::pickaxe`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SearchKind {
    /// Match the commit message (`commit`).
    Commit,
    /// Match the author identity (`author`).
    Author,
    /// Match the committer identity (`committer`).
    Committer,
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

/// A commit-message search request: which facet to search and the pattern to
/// match it with. The pattern is the already-validated [`SearchPattern`], so the
/// fixed-vs-regexp / case rules (gitweb's `search_use_regexp`) live in one place
/// and the adapter just applies it.
#[derive(Debug, Clone)]
pub struct SearchQuery {
    /// Which facet of history to search.
    pub kind: SearchKind,
    /// The pattern to match.
    pub pattern: SearchPattern,
}

/// Read access to a single git repository.
pub trait Repository {
    /// The repository's `HEAD`.
    fn head(&self) -> Result<Reference, DomainError>;

    /// All references whose full name starts with `prefix`
    /// (e.g. `"refs/heads/"`).
    fn references(&self, prefix: &str) -> Result<Vec<Reference>, DomainError>;

    /// Every reference resolved to the object it ultimately points at, the way
    /// `git show-ref --dereference` reports it (gitweb's `git_get_references`):
    /// an annotated tag arrives peeled to its target object and flagged
    /// indirect. Feeds the ref badges (`format_ref_marker`) on commit rows, so
    /// the order is the refs' name order, deterministically.
    fn dereferenced_references(&self) -> Result<Vec<DereferencedRef>, DomainError>;

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
    /// `commitdiff_plain` / `patch`. `None` when no tag reaches the commit.
    ///
    /// The returned string is the captured `tags/<name>` body — gitweb's regex
    /// `^<hash> tags/(.*)$` — so a hierarchical tag keeps its path (`release/v1`)
    /// and, mirroring `name-rev`'s one-dereference marker, an *annotated* tag
    /// whose tip is exactly the commit reads as `<name>^0` while a *lightweight*
    /// one reads as the bare `<name>`.
    ///
    /// This is full `name-rev --tags`, not just the distance-zero (tagged-tip)
    /// case: a commit some generations behind a tag is named by the path
    /// `name-rev` walks from that tag's tip — `~N` per first-parent generation,
    /// `^N` onto a merge's Nth parent, with the `^0` dereference marker stripped
    /// before any `~N` (so a commit three first-parent generations behind
    /// annotated `v1.0` is `v1.0~3`). The nearest tag wins, ties going to the
    /// older tag, matching git byte-for-byte; the naming itself is the pure
    /// [`name_rev`] port, this method only supplies the tips and ancestry.
    ///
    /// [`name_rev`]: crate::model::name_rev
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

    /// The full tree entry recorded at `path` within the tree of the tree-ish
    /// `at` — its mode, name, and object id — the way `git ls-tree <at> -- <path>`
    /// reports a row, or `None` when `path` is absent there. `at` is peeled to its
    /// tree first, exactly like [`Self::path_id`] (of which this is the richer
    /// form: `path_id` is this id alone).
    ///
    /// The mode is what classifies the entry, mirroring `ls-tree`'s type column,
    /// which git derives from the entry mode and not from the object: a directory
    /// is a tree, a regular/exec/symlink entry a blob, and — crucially — a
    /// gitlink (mode `160000`) a *commit*, named without the referenced commit
    /// needing to exist in this repository. The `object` action and the no-action
    /// dispatch read the destination view from [`FileMode::object_kind`] of this
    /// mode for exactly that reason: a submodule entry resolves to the commit view
    /// even though its commit object is absent, where reading the kind off the
    /// recorded id (gitweb's `cat-file -t`) would `404`.
    ///
    /// [`FileMode::object_kind`]: crate::model::file_mode::FileMode::object_kind
    fn path_entry(&self, at: &ObjectId, path: &str) -> Result<Option<TreeEntry>, DomainError>;

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

    /// Commits matching `query`, reachable from `base` and windowed by `page`
    /// (message / author / committer search). gitweb roots its search at the
    /// current view's revision (`$hash`); resolving that — defaulting to `HEAD`
    /// when no revision is selected — is the caller's policy, so the base arrives
    /// already resolved rather than being baked into the adapter.
    fn search(
        &self,
        base: &ObjectId,
        query: &SearchQuery,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError>;

    /// The pickaxe matches reachable from `base`: every commit that changes the
    /// number of occurrences of `pattern` in some file, each paired with the
    /// files whose count it changed — gitweb's `git_search_changes`
    /// (`git log -S<text> --raw`, plus `--pickaxe-regex` in regexp mode). The
    /// commits come out newest-first, the way `git log` walks; unlike the message
    /// search, gitweb does not page pickaxe (`git log -S` has no count limit), so
    /// every match reachable from `base` is returned. Without `--pickaxe-all` git
    /// lists, per commit, only the paths that *touch* the pattern, so a match's
    /// [`changes`](crate::model::pickaxe::PickaxeMatch::changes) are exactly the
    /// count-changing files — including deletions, which the caller skips when it
    /// lays out the page (gitweb's `next if is_deleted`).
    fn pickaxe(
        &self,
        base: &ObjectId,
        pattern: &PickaxePattern,
    ) -> Result<Vec<PickaxeMatch>, DomainError>;

    /// Content matches for `pattern` over the regular files of `revision`'s tree,
    /// mirroring gitweb's `git_search_files` (`git grep -n -z <pattern> <tree>`):
    /// each text-file line that matches (its path and 1-based number) and each
    /// binary file whose bytes match (reported with no line text). The
    /// [`GrepPattern`] carries which of git-grep's modes is in play (`-F`
    /// literal/case-sensitive or `-E -i` regexp/case-insensitive). `revision` is a
    /// tree-ish — gitweb's `$hash` / `$co{'tree'}` — so a commit or a tree id both
    /// name the tree to search. Matches come out grouped by file in tree order;
    /// the result is capped at [`crate::model::grep::GREP_MATCH_LIMIT`], reporting
    /// `trimmed` when the cap drops further matches.
    fn grep(&self, revision: &ObjectId, pattern: &GrepPattern) -> Result<GrepResults, DomainError>;
}
