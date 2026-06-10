//! Gherkin-driven BDD harness for the domain use cases.
//!
//! Use cases orchestrate the ports, so the `Given` builds an in-memory fake
//! [`ProjectStore`] (no adapter, no gix) holding hand-written [`ProjectInfo`],
//! the `When` runs the use case against it, and each `Then` asserts one fact
//! about the resulting view-model or the failure. Verifying the orchestration
//! through a fake keeps it fast and isolated; the gix adapter realizes the same
//! port contract under its own conformance specs, and the web layer exercises
//! the two together end-to-end. cucumber supplies its own `main`, so this target
//! sets `harness = false`.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::blame::Blame;
use gitweb_domain::model::blob::{Blob, BlobDisplay};
use gitweb_domain::model::blobdiff_plain::BlobdiffPlain;
use gitweb_domain::model::branch_refs::get_branch_refs;
use gitweb_domain::model::change::{ChangeKind, ChangeStatus};
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::commitdiff_plain::CommitdiffPlain;
use gitweb_domain::model::diff::{
    CombinedDiff, CombinedDiffEntry, CombinedParent, Diff, DiffEntry,
};
use gitweb_domain::model::encoding::FallbackEncoding;
use gitweb_domain::model::feed::{Feed, FeedEntry, FeedFile};
use gitweb_domain::model::file_mode::{FileKind, FileMode};
use gitweb_domain::model::forks::ForkState;
use gitweb_domain::model::format_patch::FormatPatch;
use gitweb_domain::model::grep::{GrepMatch, GrepResults};
use gitweb_domain::model::grep_pattern::GrepPattern;
use gitweb_domain::model::message_body::LogLine;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::patch::{FileContent, FilePatch, Patch};
use gitweb_domain::model::pickaxe::{PickaxeChange, PickaxeMatch};
use gitweb_domain::model::pickaxe_pattern::PickaxePattern;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_filter::ProjectFilter;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::ref_marker::{MarkerView, RefMarker};
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::{DereferencedRef, Reference};
use gitweb_domain::model::remote::{Remote, RemoteUrl};
use gitweb_domain::model::search_help::SearchHelpTopic;
use gitweb_domain::model::settings::{FeatureLayer, FeatureName, Settings, SettingsLayer};
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tag_age::TagAge;
use gitweb_domain::model::tree::{Tree, TreeEntry};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::{
    ArchiveFormat, ArchiveOptions, Page, RenameDetection, Repository, SearchKind, SearchQuery,
};
use gitweb_domain::usecase::blob::{BlobView, assemble_blob};
use gitweb_domain::usecase::blob_plain::{BlobPlainView, assemble_blob_plain};
use gitweb_domain::usecase::blobdiff::{BlobdiffView, assemble_blobdiff};
use gitweb_domain::usecase::blobdiff_plain::assemble_blobdiff_plain;
use gitweb_domain::usecase::commit::{ChangedFiles, CommitView, assemble_commit};
use gitweb_domain::usecase::commitdiff::assemble_commit_diff;
use gitweb_domain::usecase::commitdiff_plain::assemble_commitdiff_plain;
use gitweb_domain::usecase::feed::assemble_feed;
use gitweb_domain::usecase::forks::assemble_forks;
use gitweb_domain::usecase::grep::{GrepFileView, GrepLine, GrepRow, GrepView, assemble_grep};
use gitweb_domain::usecase::heads::{HeadRow, HeadsView, assemble_heads};
use gitweb_domain::usecase::history::{HistoryRow, HistoryView, assemble_history};
use gitweb_domain::usecase::log::{LogRow, LogView, assemble_log};
use gitweb_domain::usecase::object::{ObjectRedirect, assemble_object_redirect};
use gitweb_domain::usecase::object_dispatch::resolve_dispatch_action;
use gitweb_domain::usecase::opml::{Opml, OpmlProject, assemble_opml};
use gitweb_domain::usecase::patch::{assemble_patch, assemble_patches};
use gitweb_domain::usecase::pickaxe::{PickaxeRow, PickaxeView, assemble_pickaxe};
use gitweb_domain::usecase::project_index::{
    ProjectIndex, ProjectIndexRow, assemble_project_index,
};
use gitweb_domain::usecase::project_list::{
    ProjectListRow, ProjectListView, assemble_project_list,
};
use gitweb_domain::usecase::ref_markers::{RefMarkerIndex, assemble_ref_markers};
use gitweb_domain::usecase::remotes::{RemoteBlock, RemotesView, assemble_remotes};
use gitweb_domain::usecase::search::{SearchCriteria, SearchRow, SearchView, assemble_search};
use gitweb_domain::usecase::search_help::{SearchHelpView, assemble_search_help};
use gitweb_domain::usecase::shortlog::{ShortlogRow, ShortlogView, assemble_shortlog};
use gitweb_domain::usecase::snapshot::{SnapshotView, assemble_snapshot};
use gitweb_domain::usecase::summary::{SummaryView, assemble_summary};
use gitweb_domain::usecase::tag::{TagView, show_tag};
use gitweb_domain::usecase::tags::{TagRow, TagsView, assemble_tags};
use gitweb_domain::usecase::tree::{TreeRow, TreeView, assemble_tree};

/// An in-memory [`ProjectStore`] over a fixed set of projects. It serves
/// `list` and `info` from its metadata; `open` is never reached by the
/// project-list use case, so it is left unimplemented. `containers` names the
/// directories that exist on the notional filesystem, so `container_exists`
/// (gitweb's `-d`) can answer for the fork empty-container state.
struct FakeStore {
    projects: Vec<ProjectInfo>,
    containers: Vec<String>,
}

impl ProjectStore for FakeStore {
    fn list(&self, filter: Option<&ProjectFilter>) -> Result<Vec<Project>, DomainError> {
        Ok(self
            .projects
            .iter()
            .map(|info: &ProjectInfo| Project::new(info.name().to_owned()))
            .filter(|project: &Project| {
                filter.is_none_or(|filter: &ProjectFilter| filter.include(project.name()))
            })
            .collect())
    }

    fn open(&self, _name: &str) -> Result<Box<dyn Repository>, DomainError> {
        unimplemented!("the project-list use case never opens a repository")
    }

    fn container_exists(&self, subdir: &str) -> bool {
        self.containers.iter().any(|dir: &String| dir == subdir)
    }

    fn info(&self, name: &str) -> Result<ProjectInfo, DomainError> {
        self.projects
            .iter()
            .find(|info: &&ProjectInfo| info.name() == name)
            .cloned()
            .ok_or_else(|| DomainError::NotFound(format!("no project {name}")))
    }

    fn readme_html(&self, _name: &str) -> Result<Option<String>, DomainError> {
        // The summary use case takes its README as a direct input (the boundary
        // reads it from the store), so no use case drives this through the fake.
        Ok(None)
    }

    fn description(&self, name: &str) -> Result<Option<String>, DomainError> {
        // The footer description is a web-boundary read, never driven through a
        // domain use case; serve it from the same metadata `info` resolves.
        Ok(self.info(name)?.description().map(str::to_owned))
    }
}

/// An in-memory [`ProjectStore`] for the opml use case: each project either has a
/// HEAD (included) or does not (skipped), mirroring gitweb's per-project
/// `git_get_head_hash` check. Only `list` and `open` are reached; `open` yields a
/// [`FakeRepository`] whose `head` resolves exactly when the project has one.
struct FakeOpmlStore {
    projects: Vec<(String, bool)>,
}

impl ProjectStore for FakeOpmlStore {
    fn list(&self, filter: Option<&ProjectFilter>) -> Result<Vec<Project>, DomainError> {
        Ok(self
            .projects
            .iter()
            .map(|(name, _): &(String, bool)| Project::new(name.clone()))
            .filter(|project: &Project| {
                filter.is_none_or(|filter: &ProjectFilter| filter.include(project.name()))
            })
            .collect())
    }

    fn open(&self, name: &str) -> Result<Box<dyn Repository>, DomainError> {
        let has_head: bool = self
            .projects
            .iter()
            .find(|(candidate, _): &&(String, bool)| candidate == name)
            .map(|(_, has_head): &(String, bool)| *has_head)
            .ok_or_else(|| DomainError::NotFound(format!("no project {name}")))?;
        let repository: FakeRepository = FakeRepository {
            head_commit: has_head.then(|| fake_oid(name)),
            ..FakeRepository::default()
        };
        Ok(Box::new(repository))
    }

    fn container_exists(&self, _subdir: &str) -> bool {
        unimplemented!("the opml use case never folds forks")
    }

    fn info(&self, _name: &str) -> Result<ProjectInfo, DomainError> {
        unimplemented!("the opml use case never reads project metadata")
    }

    fn readme_html(&self, _name: &str) -> Result<Option<String>, DomainError> {
        unimplemented!("the opml use case never reads a README")
    }

    fn description(&self, _name: &str) -> Result<Option<String>, DomainError> {
        unimplemented!("the opml use case never reads a project description")
    }
}

#[derive(Debug, Default, World)]
struct UsecaseWorld {
    projects: Vec<ProjectInfo>,
    containers: Vec<String>,
    now: i64,
    settings: Settings,
    result: Option<Result<ProjectListView, DomainError>>,
    index_result: Option<Result<ProjectIndex, DomainError>>,
    opml_projects: Vec<(String, bool)>,
    opml_result: Option<Result<Opml, DomainError>>,
    head: Option<String>,
    branches: Vec<FakeBranch>,
    extra_branch_refs: Vec<String>,
    heads_result: Option<Result<HeadsView, DomainError>>,
    tags: Vec<FakeTag>,
    tags_result: Option<Result<TagsView, DomainError>>,
    tag_result: Option<Result<TagView, DomainError>>,
    commits: Vec<FakeCommit>,
    search_hits: Vec<FakeCommit>,
    search_result: Option<Result<SearchView, DomainError>>,
    grep_matches: Vec<GrepMatch>,
    grep_trimmed: bool,
    grep_result: Option<Result<GrepView, DomainError>>,
    pickaxe_hits: Vec<FakePickaxeMatch>,
    pickaxe_result: Option<Result<PickaxeView, DomainError>>,
    head_commit: Option<ObjectId>,
    shortlog_result: Option<Result<ShortlogView, DomainError>>,
    marker_index: Option<Result<RefMarkerIndex, DomainError>>,
    log_result: Option<Result<LogView, DomainError>>,
    history_result: Option<Result<HistoryView, DomainError>>,
    summary_name: Option<String>,
    summary_description: Option<String>,
    summary_owner: Option<String>,
    summary_clone_urls: Vec<String>,
    summary_readme: Option<String>,
    summary_omit_owner: bool,
    summary_prevent_xss: bool,
    summary_base_urls: Vec<String>,
    summary_result: Option<Result<SummaryView, DomainError>>,
    remotes: Vec<Remote>,
    remote_branches: Vec<FakeRemoteBranch>,
    remote_heads_enabled: bool,
    remotes_result: Option<Result<RemotesView, DomainError>>,
    tree_commit_title: Option<String>,
    tree_nodes: Vec<FakeTreeNode>,
    tree_show_sizes: bool,
    tree_result: Option<Result<TreeView, DomainError>>,
    blob_base_title: Option<String>,
    blob_files: Vec<FakeBlobFile>,
    blob_result: Option<Result<BlobView, DomainError>>,
    blob_plain_result: Option<Result<BlobPlainView, DomainError>>,
    feed_result: Option<Result<Feed, DomainError>>,
    commit_fixture: Option<FakeCommitFixture>,
    commit_result: Option<Result<CommitView, DomainError>>,
    commitdiff_result: Option<Result<String, DomainError>>,
    commitdiff_plain_result: Option<Result<CommitdiffPlain, DomainError>>,
    patch_result: Option<Result<FormatPatch, DomainError>>,
    /// The `patches` range fixture (its commits, oldest-first) and the result of
    /// assembling the numbered stream over it.
    patch_series: Option<FakePatchSeries>,
    patches_result: Option<Result<FormatPatch, DomainError>>,
    /// The file patches accumulated by the blobdiff_plain scenarios; the When
    /// folds them into the fixture's whole-tree patch before assembling.
    blobdiff_files: Vec<FilePatch>,
    blobdiff_plain_result: Option<Result<BlobdiffPlain, DomainError>>,
    blobdiff_result: Option<Result<BlobdiffView, DomainError>>,
    object_result: Option<Result<ObjectRedirect, DomainError>>,
    dispatch_action_result: Option<Result<Action, DomainError>>,
    snapshot: Option<FakeSnapshot>,
    snapshot_project: String,
    snapshot_configured: Vec<String>,
    snapshot_result: Option<Result<SnapshotView, DomainError>>,
    search_help_grep_disabled: bool,
    search_help_pickaxe_disabled: bool,
    search_help_view: Option<SearchHelpView>,
}

/// One directory in the fake repository's tree, listed by the `tree` use case.
/// `path` is the path that resolves to this node (gitweb's
/// `git_get_hash_by_path`); the root directory has `None`. Its object id is
/// derived from its `label`, and `entries` are its children.
#[derive(Debug, Clone)]
struct FakeTreeNode {
    label: String,
    path: Option<String>,
    entries: Vec<FakeTreeEntry>,
}

/// One entry of a fake tree node: its octal mode, leaf name, the `ls-tree -l`
/// byte size (used for file entries), and, for a symlink, the path its blob
/// points to.
#[derive(Debug, Clone)]
struct FakeTreeEntry {
    mode: String,
    name: String,
    size: u64,
    target: Option<String>,
}

/// The fixed id the fake's `HEAD` (a commit) resolves to when a tree is declared.
fn tree_head_oid() -> ObjectId {
    fake_oid("tree-head")
}

/// The id of the tree node labelled `label`.
fn tree_node_oid(label: &str) -> ObjectId {
    fake_oid(&format!("treenode-{label}"))
}

/// The id of the entry named `name` within the node labelled `label`.
fn tree_entry_oid(label: &str, name: &str) -> ObjectId {
    fake_oid(&format!("treeentry-{label}-{name}"))
}

/// One blob the `blob` use case can read, by its label (the id a by-hash request
/// names) or by its `path` within the blob base (the path a by-name request
/// resolves), holding the raw `content` the view classifies and decodes.
#[derive(Debug, Clone)]
struct FakeBlobFile {
    label: String,
    path: Option<String>,
    content: Vec<u8>,
}

/// The id the blob base resolves to — a commit, so a by-name request peels it
/// and `HEAD` resolves to it while blob fixtures are declared.
fn blob_base_oid() -> ObjectId {
    fake_oid("blob-base")
}

/// The id of the blob fixture labelled `label`.
fn blob_file_oid(label: &str) -> ObjectId {
    fake_oid(&format!("blobfile-{label}"))
}

/// Parses space-separated hex byte pairs into the raw bytes they name.
fn parse_hex_bytes(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .map(|byte: &str| u8::from_str_radix(byte, 16).expect("a valid hex byte"))
        .collect()
}

/// One remote-tracking branch in the fake repository: which remote it belongs to,
/// its branch name (without the remote prefix), the id of its tip commit, and
/// that commit's committer epoch. Listed under `refs/remotes/<remote>/<name>`.
#[derive(Debug, Clone)]
struct FakeRemoteBranch {
    remote: String,
    name: String,
    tip: ObjectId,
    epoch: i64,
}

/// One branch in the fake repository: the directory under `refs/` it lives in
/// (`heads` for an ordinary branch, an extra-branch-refs directory otherwise),
/// its short name, the id of its tip commit, and that commit's committer epoch.
#[derive(Debug, Clone)]
struct FakeBranch {
    dir: String,
    name: String,
    tip: ObjectId,
    epoch: i64,
}

/// One tag in the fake repository. `ref_target` is what `refs/tags/<name>`
/// points at directly — a tag object for an annotated tag, the tagged object
/// itself for a lightweight one. `object_kind` is the kind of the object the tag
/// ultimately names, and `epoch` is its creation time (the tagger time for an
/// annotated tag, the committer time for a lightweight tag of a commit, ignored
/// for a lightweight tag of a blob or tree). `message` is the full annotated-tag
/// message (its first non-empty line is the listing subject); `has_tagger` is
/// whether the tag object carries a tagger line.
#[derive(Debug, Clone)]
struct FakeTag {
    full_name: String,
    ref_target: ObjectId,
    object: ObjectId,
    object_kind: ObjectKind,
    annotated: bool,
    epoch: i64,
    message: String,
    has_tagger: bool,
}

/// One commit in the fake repository's linear history (newest first as declared):
/// its id, its committer epoch, the author name, and the subject line. For the
/// per-path history, `touches` names the paths this commit changed (the
/// path-limited walk keeps it for any of these) and `present` is the object each
/// path resolves to in this commit's tree — a deleted path is touched but absent.
#[derive(Debug, Clone)]
struct FakeCommit {
    id: ObjectId,
    epoch: i64,
    author: String,
    title: String,
    touches: Vec<String>,
    present: Vec<FakePathEntry>,
}

/// One path present in a fake commit's tree: the path, the object it resolves to,
/// and that object's kind (a blob for a file, a tree for a directory).
#[derive(Debug, Clone)]
struct FakePathEntry {
    path: String,
    oid: ObjectId,
    kind: ObjectKind,
}

/// A pickaxe hit the fake returns: the matching commit and the files whose
/// occurrence count it changed, declared by a scenario. The matching itself is the
/// adapter's conformance, so the fake just replays this.
#[derive(Debug, Clone)]
struct FakePickaxeMatch {
    commit: FakeCommit,
    changes: Vec<FakePickaxeChange>,
}

/// One count-changing file in a [`FakePickaxeMatch`]: its path and the seed of its
/// post-change blob id, or `None` for a deletion (gitweb's `to_id` is null, the row
/// the page skips).
#[derive(Debug, Clone)]
struct FakePickaxeChange {
    path: String,
    blob_seed: Option<String>,
}

/// A deterministic 40-hex object id derived from `seed`, so distinct branch
/// names get distinct tips while a test can still alias two branches onto one
/// commit by deriving both from the same seed.
fn fake_oid(seed: &str) -> ObjectId {
    let mut hex: String = seed.bytes().map(|byte: u8| format!("{byte:02x}")).collect();
    hex.truncate(40);
    while hex.len() < 40 {
        hex.push('0');
    }
    ObjectId::parse(&hex).expect("a 40-character hex object id")
}

/// Builds a domain [`Commit`] from a declared [`FakeCommit`]: a synthetic
/// `Name <a@example.com> epoch +0000` signature for both author and committer
/// and the title as the whole message. Shared by `history` (the log walk) and
/// `search` (the result list) so both render rows from the same shape.
fn commit_from_fake(commit: &FakeCommit) -> Commit {
    let who: Signature = Signature::parse(&format!(
        "{} <a@example.com> {} +0000",
        commit.author, commit.epoch
    ))
    .expect("a valid fixture signature");
    Commit::new(
        commit.id.clone(),
        fake_oid("tree"),
        Vec::new(),
        who.clone(),
        who,
        format!("{}\n", commit.title),
    )
}

/// An in-memory [`Repository`] over a fixed set of branches and tags. It serves
/// the reads the heads and tags use cases make — `head`, `references`,
/// `object_kind`, `find_commit`, `find_tag` — and leaves every other port method
/// unimplemented, since those use cases never reach them.
#[derive(Default)]
struct FakeRepository {
    head: Option<String>,
    head_commit: Option<ObjectId>,
    branches: Vec<FakeBranch>,
    tags: Vec<FakeTag>,
    commits: Vec<FakeCommit>,
    /// The commits `search` returns (windowed by the page it is asked for). Kept
    /// separate from `commits` so a search result set is independent of the log
    /// walk, and so the base commit the search roots at is not itself a hit.
    search_hits: Vec<FakeCommit>,
    /// The matches `grep` returns, already grouped by file in tree order, and
    /// whether the cap trimmed the listing — the adapter's conformance, so the
    /// fake just hands back what the scenario configured.
    grep_matches: Vec<GrepMatch>,
    grep_trimmed: bool,
    /// The matches `pickaxe` returns, each a commit with its count-changing files
    /// (deletions included, as git's `--raw` lists them) — the adapter's
    /// conformance, so the fake just hands back what the scenario configured.
    pickaxe_hits: Vec<FakePickaxeMatch>,
    remotes: Vec<Remote>,
    remote_branches: Vec<FakeRemoteBranch>,
    tree_commit_title: Option<String>,
    tree_nodes: Vec<FakeTreeNode>,
    blob_base_title: Option<String>,
    blob_files: Vec<FakeBlobFile>,
    commit_fixture: Option<FakeCommitFixture>,
    snapshot: Option<FakeSnapshot>,
    patch_series: Option<FakePatchSeries>,
}

/// The object a `snapshot` request resolves and archives: the revision name it
/// answers to (`resolve`), its id and kind (`object_kind`), and, for a commit, the
/// committer whose time dates the archive (`find_commit`). The `archive` call
/// rejects a blob, as the gix adapter's `require_tree` does, and otherwise echoes
/// the [`ArchiveOptions`] it was handed into the bytes, so the use case's prefix
/// and modification-time threading is observable.
#[derive(Debug, Clone)]
struct FakeSnapshot {
    rev: String,
    id: ObjectId,
    kind: ObjectKind,
    committer: Option<Signature>,
}

/// A single commit the `commit` use case resolves and reads: the revision name it
/// answers to, its identity and ancestry, its authorship and message, the object
/// kind it reports (so a non-commit drives the `Unknown commit object` 404), and
/// its changed-files set — ordinary entries for a single-parent/root commit, or
/// combined entries for a merge.
#[derive(Debug, Clone)]
struct FakeCommitFixture {
    rev: String,
    id: ObjectId,
    tree: ObjectId,
    parents: Vec<ObjectId>,
    author: Signature,
    committer: Signature,
    message: String,
    kind: ObjectKind,
    diff: Vec<DiffEntry>,
    combined: Vec<CombinedDiffEntry>,
    patch: Patch,
    /// The byte sizes of the blobs the patch's binary files diff across, keyed by
    /// object id — what [`Repository::object_size`] reports so the `patch` use
    /// case's diffstat renders a binary `Bin <old> -> <new> bytes` row.
    binary_sizes: Vec<(ObjectId, u64)>,
    /// The `name-rev --tags` name of this commit, when tag-named; drives the
    /// commitdiff_plain `X-Git-Tag` line through [`Repository::rev_name_tag`].
    rev_name_tag: Option<String>,
}

impl FakeCommitFixture {
    /// The declared byte size of the blob `oid`, if this commit's patch diffs a
    /// binary file across it.
    fn binary_size(&self, oid: &ObjectId) -> Option<u64> {
        self.binary_sizes
            .iter()
            .find(|(id, _): &&(ObjectId, u64)| id == oid)
            .map(|(_, size): &(ObjectId, u64)| *size)
    }
}

/// The `patches` range fixture: the commits of the series, declared oldest-first
/// (the natural reading order), and whether the tip resolves to a commit (so a
/// non-commit tip drives the `Unknown commit object` 404). [`Repository::history`]
/// returns the series newest-first, the way `git rev-list` does, and the use case
/// reverses it back to oldest-first for the `[PATCH i/N]` stream.
#[derive(Debug, Clone)]
struct FakePatchSeries {
    author: Signature,
    commits: Vec<FakePatchCommit>,
    tip_is_commit: bool,
}

/// One commit in a [`FakePatchSeries`]: its id, its first parent (the diff base —
/// the previous, older commit, or `None` for the root), the subject its `Subject:`
/// line carries, and the patch its mail's body renders.
#[derive(Debug, Clone)]
struct FakePatchCommit {
    id: ObjectId,
    parent: Option<ObjectId>,
    subject: String,
    patch: Patch,
}

impl FakePatchSeries {
    /// The tip (newest) commit of the series, the one `HEAD` resolves to.
    fn tip(&self) -> Option<&FakePatchCommit> {
        self.commits.last()
    }

    /// The series commit whose id is `oid`, if any (the `patch` lookup keys on the
    /// diff's to-side, which is each commit's own id).
    fn commit(&self, oid: &ObjectId) -> Option<&FakePatchCommit> {
        self.commits
            .iter()
            .find(|commit: &&FakePatchCommit| &commit.id == oid)
    }
}

impl FakeRepository {
    /// Whether a tree is declared (so `HEAD` resolves to the tree's commit).
    fn has_tree(&self) -> bool {
        self.tree_commit_title.is_some()
    }

    /// Whether a blob base is declared (so `HEAD` resolves to the base commit).
    fn has_blob_base(&self) -> bool {
        self.blob_base_title.is_some()
    }

    /// The blob fixture whose id is `oid`, if any.
    fn blob_file_by_oid(&self, oid: &ObjectId) -> Option<&FakeBlobFile> {
        self.blob_files
            .iter()
            .find(|file: &&FakeBlobFile| &blob_file_oid(&file.label) == oid)
    }

    /// The node whose id is `oid`, if any.
    fn tree_node_by_oid(&self, oid: &ObjectId) -> Option<&FakeTreeNode> {
        self.tree_nodes
            .iter()
            .find(|node: &&FakeTreeNode| &tree_node_oid(&node.label) == oid)
    }

    /// The entry whose id is `oid`, if any. The id is derived from its node's
    /// label and the entry's name, so the search walks every (node, entry) pair.
    fn tree_entry_by_oid(&self, oid: &ObjectId) -> Option<&FakeTreeEntry> {
        self.tree_nodes.iter().find_map(|node: &FakeTreeNode| {
            node.entries
                .iter()
                .find(|entry: &&FakeTreeEntry| &tree_entry_oid(&node.label, &entry.name) == oid)
        })
    }

    fn branch_ref(branch: &FakeBranch) -> Reference {
        Reference::new(
            RefName::new(format!("refs/{}/{}", branch.dir, branch.name)),
            branch.tip.clone(),
        )
    }

    fn remote_branch_ref(branch: &FakeRemoteBranch) -> Reference {
        Reference::new(
            RefName::new(format!("refs/remotes/{}/{}", branch.remote, branch.name)),
            branch.tip.clone(),
        )
    }

    fn tag_ref(tag: &FakeTag) -> Reference {
        Reference::new(RefName::new(tag.full_name.clone()), tag.ref_target.clone())
    }

    /// The committer epoch of whatever commit `oid` names — a branch tip, a
    /// lightweight tag's commit, or a commit declared in this fake's history (so
    /// a HEAD pointed straight at such a commit resolves, as the summary's
    /// last-change read needs).
    fn commit_epoch(&self, oid: &ObjectId) -> Option<i64> {
        self.branches
            .iter()
            .find(|branch: &&FakeBranch| &branch.tip == oid)
            .map(|branch: &FakeBranch| branch.epoch)
            .or_else(|| {
                self.remote_branches
                    .iter()
                    .find(|branch: &&FakeRemoteBranch| &branch.tip == oid)
                    .map(|branch: &FakeRemoteBranch| branch.epoch)
            })
            .or_else(|| {
                self.tags
                    .iter()
                    .find(|tag: &&FakeTag| !tag.annotated && &tag.ref_target == oid)
                    .map(|tag: &FakeTag| tag.epoch)
            })
            .or_else(|| {
                self.commits
                    .iter()
                    .find(|commit: &&FakeCommit| &commit.id == oid)
                    .map(|commit: &FakeCommit| commit.epoch)
            })
    }
}

impl Repository for FakeRepository {
    fn head(&self) -> Result<Reference, DomainError> {
        if let Some(oid) = &self.head_commit {
            return Ok(Reference::new(RefName::new("HEAD".to_owned()), oid.clone()));
        }
        let name: &String = self
            .head
            .as_ref()
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?;
        let branch: &FakeBranch = self
            .branches
            .iter()
            .find(|branch: &&FakeBranch| &branch.name == name)
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?;
        Ok(Self::branch_ref(branch))
    }

    fn references(&self, prefix: &str) -> Result<Vec<Reference>, DomainError> {
        let branches: Vec<Reference> = self.branches.iter().map(Self::branch_ref).collect();
        let tags: Vec<Reference> = self.tags.iter().map(Self::tag_ref).collect();
        let remote_branches: Vec<Reference> = self
            .remote_branches
            .iter()
            .map(Self::remote_branch_ref)
            .collect();
        Ok(branches
            .into_iter()
            .chain(tags)
            .chain(remote_branches)
            .filter(|reference: &Reference| reference.name().full().starts_with(prefix))
            .collect())
    }

    fn dereferenced_references(&self) -> Result<Vec<DereferencedRef>, DomainError> {
        // show-ref --dereference: branches and remote-tracking refs point straight
        // at their commit (direct); a tag is peeled to the object it ultimately
        // names (`object`) and is indirect exactly when it is annotated.
        let branches = self.branches.iter().map(|branch: &FakeBranch| {
            DereferencedRef::new(
                RefName::new(format!("refs/heads/{}", branch.name)),
                branch.tip.clone(),
                false,
            )
        });
        let remote_branches = self
            .remote_branches
            .iter()
            .map(|branch: &FakeRemoteBranch| {
                DereferencedRef::new(
                    RefName::new(format!("refs/remotes/{}/{}", branch.remote, branch.name)),
                    branch.tip.clone(),
                    false,
                )
            });
        let tags = self.tags.iter().map(|tag: &FakeTag| {
            DereferencedRef::new(
                RefName::new(tag.full_name.clone()),
                tag.object.clone(),
                tag.annotated,
            )
        });
        let mut out: Vec<DereferencedRef> = branches.chain(remote_branches).chain(tags).collect();
        out.sort_by(|a: &DereferencedRef, b: &DereferencedRef| {
            a.name().full().cmp(b.name().full())
        });
        Ok(out)
    }

    fn remotes(&self) -> Result<Vec<Remote>, DomainError> {
        Ok(self.remotes.clone())
    }

    fn find_commit(&self, oid: &ObjectId) -> Result<Commit, DomainError> {
        if let Some(snapshot) = self.snapshot.as_ref().filter(|s| &s.id == oid) {
            let committer: &Signature = snapshot
                .committer
                .as_ref()
                .ok_or_else(|| DomainError::Invalid(format!("not a commit: {}", oid.as_str())))?;
            return Ok(Commit::new(
                snapshot.id.clone(),
                fake_oid("snapshot-tree"),
                Vec::new(),
                committer.clone(),
                committer.clone(),
                "snapshot commit\n".to_owned(),
            ));
        }
        if let Some(fixture) = self.commit_fixture.as_ref().filter(|f| &f.id == oid) {
            return Ok(Commit::new(
                fixture.id.clone(),
                fixture.tree.clone(),
                fixture.parents.clone(),
                fixture.author.clone(),
                fixture.committer.clone(),
                fixture.message.clone(),
            ));
        }
        if self.has_blob_base() && oid == &blob_base_oid() {
            let title: &str = self.blob_base_title.as_deref().expect("a blob base title");
            let who: Signature = Signature::parse("Tester <t@example.com> 1000 +0000")
                .expect("a valid fixture signature");
            return Ok(Commit::new(
                oid.clone(),
                fake_oid("blob-base-tree"),
                Vec::new(),
                who.clone(),
                who,
                format!("{title}\n"),
            ));
        }
        if self.has_tree() && oid == &tree_head_oid() {
            let title: &str = self
                .tree_commit_title
                .as_deref()
                .expect("a tree commit title");
            let who: Signature = Signature::parse("Tester <t@example.com> 1000 +0000")
                .expect("a valid fixture signature");
            return Ok(Commit::new(
                oid.clone(),
                tree_node_oid("root"),
                Vec::new(),
                who.clone(),
                who,
                format!("{title}\n"),
            ));
        }
        // A declared commit carries its title and author, so return it faithfully
        // (gitweb's parse_commit -> %co, whose subject heads the grep results).
        if let Some(fake) = self
            .commits
            .iter()
            .find(|commit: &&FakeCommit| &commit.id == oid)
        {
            return Ok(commit_from_fake(fake));
        }
        let epoch: i64 = self
            .commit_epoch(oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let who: Signature = Signature::parse(&format!("Tester <t@example.com> {epoch} +0000"))
            .expect("a valid fixture signature");
        Ok(Commit::new(
            oid.clone(),
            fake_oid("tree"),
            Vec::new(),
            who.clone(),
            who,
            "msg\n".to_owned(),
        ))
    }

    fn resolve(&self, rev: &str) -> Result<ObjectId, DomainError> {
        if let Some(snapshot) = self.snapshot.as_ref().filter(|s| s.rev == rev) {
            return Ok(snapshot.id.clone());
        }
        if let Some(fixture) = self.commit_fixture.as_ref().filter(|f| f.rev == rev) {
            return Ok(fixture.id.clone());
        }
        if rev == "HEAD"
            && let Some(tip) = self.patch_series.as_ref().and_then(FakePatchSeries::tip)
        {
            return Ok(tip.id.clone());
        }
        if rev == "HEAD" && self.has_tree() {
            return Ok(tree_head_oid());
        }
        if rev == "HEAD" && self.has_blob_base() {
            return Ok(blob_base_oid());
        }
        if let Some(file) = self
            .blob_files
            .iter()
            .find(|file: &&FakeBlobFile| file.label == rev)
        {
            return Ok(blob_file_oid(&file.label));
        }
        let full: String = format!("refs/tags/{rev}");
        if let Some(tag) = self
            .tags
            .iter()
            .find(|tag: &&FakeTag| tag.full_name == full)
        {
            return Ok(tag.ref_target.clone());
        }
        if let Some(branch) = self
            .branches
            .iter()
            .find(|branch: &&FakeBranch| branch.name == rev)
        {
            return Ok(branch.tip.clone());
        }
        // A full object id resolves to itself, the way `git rev-parse <oid>` does
        // — how the commit use case resolves an explicit parent passed as an oid.
        if let Some(oid) = ObjectId::parse(rev) {
            return Ok(oid);
        }
        Err(DomainError::NotFound(rev.to_owned()))
    }

    fn object_kind(&self, oid: &ObjectId) -> Result<ObjectKind, DomainError> {
        if let Some(snapshot) = self.snapshot.as_ref().filter(|s| &s.id == oid) {
            return Ok(snapshot.kind);
        }
        if let Some(fixture) = self.commit_fixture.as_ref().filter(|f| &f.id == oid) {
            return Ok(fixture.kind);
        }
        if let Some(series) = self.patch_series.as_ref()
            && series.tip().map(|tip: &FakePatchCommit| &tip.id) == Some(oid)
        {
            return Ok(if series.tip_is_commit {
                ObjectKind::Commit
            } else {
                ObjectKind::Blob
            });
        }
        if self.has_tree() && oid == &tree_head_oid() {
            return Ok(ObjectKind::Commit);
        }
        if self.has_blob_base() && oid == &blob_base_oid() {
            return Ok(ObjectKind::Commit);
        }
        if self.blob_file_by_oid(oid).is_some() {
            return Ok(ObjectKind::Blob);
        }
        if self.tree_node_by_oid(oid).is_some() {
            return Ok(ObjectKind::Tree);
        }
        if let Some(entry) = self.tree_entry_by_oid(oid) {
            let mode: FileMode = FileMode::from_octal(&entry.mode).expect("a valid octal mode");
            // A gitlink records a commit that lives in the submodule, not here, so
            // `cat-file -t` on it misses — exactly as the gix adapter 404s. The
            // entry's mode still classifies it (via `path_entry`); reading the
            // recorded id does not.
            if mode.kind() == FileKind::Submodule {
                return Err(DomainError::NotFound(oid.as_str().to_owned()));
            }
            return Ok(mode.object_kind());
        }
        if let Some(tag) = self
            .tags
            .iter()
            .find(|tag: &&FakeTag| &tag.ref_target == oid)
        {
            return Ok(if tag.annotated {
                ObjectKind::Tag
            } else {
                tag.object_kind
            });
        }
        if self
            .branches
            .iter()
            .any(|branch: &FakeBranch| &branch.tip == oid)
        {
            return Ok(ObjectKind::Commit);
        }
        if let Some(entry) = self
            .commits
            .iter()
            .flat_map(|commit: &FakeCommit| &commit.present)
            .find(|entry: &&FakePathEntry| &entry.oid == oid)
        {
            return Ok(entry.kind);
        }
        Err(DomainError::NotFound(oid.as_str().to_owned()))
    }

    fn find_tree(&self, oid: &ObjectId) -> Result<Tree, DomainError> {
        let node: &FakeTreeNode = self
            .tree_node_by_oid(oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let entries: Vec<TreeEntry> = node
            .entries
            .iter()
            .map(|entry: &FakeTreeEntry| {
                let mode: FileMode = FileMode::from_octal(&entry.mode).expect("a valid octal mode");
                TreeEntry::new(
                    mode,
                    entry.name.clone(),
                    tree_entry_oid(&node.label, &entry.name),
                )
            })
            .collect();
        Ok(Tree::new(entries))
    }

    fn find_blob(&self, oid: &ObjectId) -> Result<Blob, DomainError> {
        if let Some(file) = self.blob_file_by_oid(oid) {
            return Ok(Blob::new(file.content.clone()));
        }
        let entry: &FakeTreeEntry = self
            .tree_entry_by_oid(oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let target: &str = entry
            .target
            .as_deref()
            .ok_or_else(|| DomainError::Invalid(format!("not a symlink: {}", oid.as_str())))?;
        Ok(Blob::new(target.as_bytes().to_vec()))
    }

    fn object_size(&self, oid: &ObjectId) -> Result<u64, DomainError> {
        // A binary file in a commit fixture's patch declares its blob sizes
        // directly (no tree node to hang them off), so the diffstat's `Bin <old>
        // -> <new> bytes` row resolves; every other caller reads a tree entry.
        if let Some(size) = self
            .commit_fixture
            .as_ref()
            .and_then(|fixture: &FakeCommitFixture| fixture.binary_size(oid))
        {
            return Ok(size);
        }
        let entry: &FakeTreeEntry = self
            .tree_entry_by_oid(oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        Ok(entry.size)
    }

    fn find_tag(&self, oid: &ObjectId) -> Result<Tag, DomainError> {
        let tag: &FakeTag = self
            .tags
            .iter()
            .find(|tag: &&FakeTag| tag.annotated && &tag.ref_target == oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let tagger: Option<Signature> = tag.has_tagger.then(|| {
            Signature::parse(&format!("Tagger <tagger@example.com> {} +0000", tag.epoch))
                .expect("a valid fixture tagger signature")
        });
        let name: String = RefName::new(tag.full_name.clone()).short().into_owned();
        Ok(Tag::new(
            tag.ref_target.clone(),
            tag.object.clone(),
            tag.object_kind,
            name,
            tagger,
            tag.message.clone(),
        ))
    }

    fn rev_name_tag(&self, oid: &ObjectId) -> Result<Option<String>, DomainError> {
        Ok(self
            .commit_fixture
            .as_ref()
            .filter(|fixture: &&FakeCommitFixture| &fixture.id == oid)
            .and_then(|fixture: &FakeCommitFixture| fixture.rev_name_tag.clone()))
    }

    fn history(
        &self,
        _start: &ObjectId,
        path: Option<&str>,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError> {
        if let Some(series) = self.patch_series.as_ref() {
            // git rev-list order is newest-first, the reverse of the declared
            // (oldest-first) series; the `patches` use case reverses it back.
            let commits: Vec<Commit> = series
                .commits
                .iter()
                .rev()
                .skip(page.skip)
                .take(page.limit)
                .map(|commit: &FakePatchCommit| {
                    Commit::new(
                        commit.id.clone(),
                        fake_oid(&format!("patches-tree-{}", commit.subject)),
                        commit.parent.iter().cloned().collect(),
                        series.author.clone(),
                        series.author.clone(),
                        format!("{}\n", commit.subject),
                    )
                })
                .collect();
            return Ok(commits);
        }
        let commits: Vec<Commit> = self
            .commits
            .iter()
            .filter(|commit: &&FakeCommit| match path {
                None => true,
                Some(wanted) => commit.touches.iter().any(|p: &String| p == wanted),
            })
            .skip(page.skip)
            .take(page.limit)
            .map(commit_from_fake)
            .collect();
        Ok(commits)
    }

    fn path_id(&self, at: &ObjectId, path: &str) -> Result<Option<ObjectId>, DomainError> {
        // The id alone of the `ls-tree` row `path_entry` resolves.
        Ok(self
            .path_entry(at, path)?
            .map(|entry: TreeEntry| entry.oid().clone()))
    }

    fn path_entry(&self, at: &ObjectId, path: &str) -> Result<Option<TreeEntry>, DomainError> {
        let name: String = path.rsplit('/').next().unwrap_or(path).to_owned();
        if self.has_blob_base() && at == &blob_base_oid() {
            let found: Option<TreeEntry> = self
                .blob_files
                .iter()
                .find(|file: &&FakeBlobFile| file.path.as_deref() == Some(path))
                .map(|file: &FakeBlobFile| {
                    TreeEntry::new(
                        regular_file_mode(),
                        name.clone(),
                        blob_file_oid(&file.label),
                    )
                });
            return Ok(found);
        }
        if self.has_tree() && at == &tree_head_oid() {
            if let Some(node) = self
                .tree_nodes
                .iter()
                .find(|node: &&FakeTreeNode| node.path.as_deref() == Some(path))
            {
                let mode: FileMode = FileMode::from_octal("040000").expect("a valid tree mode");
                return Ok(Some(TreeEntry::new(mode, name, tree_node_oid(&node.label))));
            }
            let root_entry: Option<TreeEntry> = self
                .tree_nodes
                .iter()
                .find(|node: &&FakeTreeNode| node.label == "root")
                .and_then(|root: &FakeTreeNode| {
                    root.entries
                        .iter()
                        .find(|entry: &&FakeTreeEntry| entry.name == path)
                        .map(|entry: &FakeTreeEntry| {
                            let mode: FileMode =
                                FileMode::from_octal(&entry.mode).expect("a valid octal mode");
                            TreeEntry::new(
                                mode,
                                entry.name.clone(),
                                tree_entry_oid("root", &entry.name),
                            )
                        })
                });
            return Ok(root_entry);
        }
        let entry: Option<TreeEntry> = self
            .commits
            .iter()
            .find(|commit: &&FakeCommit| &commit.id == at)
            .and_then(|commit: &FakeCommit| {
                commit
                    .present
                    .iter()
                    .find(|entry: &&FakePathEntry| entry.path == path)
                    .map(|entry: &FakePathEntry| {
                        TreeEntry::new(mode_for_kind(entry.kind), name.clone(), entry.oid.clone())
                    })
            });
        Ok(entry)
    }

    fn diff(
        &self,
        _from: Option<&ObjectId>,
        to: &ObjectId,
        _detection: RenameDetection,
    ) -> Result<Diff, DomainError> {
        if let Some(fixture) = self.commit_fixture.as_ref().filter(|f| &f.id == to) {
            return Ok(Diff::new(fixture.diff.clone()));
        }
        // The feed use case diffs each commit against its parent; the fake's
        // commits carry the paths they touch, so synthesize one added-file entry
        // per touched path (the to-side id from the path's blob when present).
        let file_mode: FileMode = FileMode::from_octal("100644").expect("a valid file mode");
        let absent: FileMode = FileMode::from_octal("000000").expect("a valid absent mode");
        let entries: Vec<DiffEntry> = self
            .commits
            .iter()
            .find(|commit: &&FakeCommit| &commit.id == to)
            .map(|commit: &FakeCommit| {
                commit
                    .touches
                    .iter()
                    .map(|path: &String| {
                        let to_oid: ObjectId = commit
                            .present
                            .iter()
                            .find(|entry: &&FakePathEntry| &entry.path == path)
                            .map(|entry: &FakePathEntry| entry.oid.clone())
                            .unwrap_or_else(|| fake_oid(&format!("gone-{path}")));
                        DiffEntry::new(
                            ChangeStatus::added(),
                            absent,
                            file_mode,
                            to_oid.null_like(),
                            to_oid,
                            path.clone(),
                            path.clone(),
                        )
                    })
                    .collect()
            })
            .unwrap_or_default();
        Ok(Diff::new(entries))
    }

    fn patch(
        &self,
        _from: Option<&ObjectId>,
        to: &ObjectId,
        _detection: RenameDetection,
    ) -> Result<Patch, DomainError> {
        if let Some(fixture) = self.commit_fixture.as_ref().filter(|f| &f.id == to) {
            return Ok(fixture.patch.clone());
        }
        if let Some(commit) = self
            .patch_series
            .as_ref()
            .and_then(|series: &FakePatchSeries| series.commit(to))
        {
            return Ok(commit.patch.clone());
        }
        unimplemented!("only the commitdiff use case's fixture builds a patch")
    }

    fn abbrev_length(&self) -> Result<usize, DomainError> {
        // git's floor for a small repository; the commitdiff_plain use case
        // abbreviates its `index` ids to this width.
        Ok(7)
    }

    fn combined_diff(&self, commit: &ObjectId) -> Result<CombinedDiff, DomainError> {
        if let Some(fixture) = self.commit_fixture.as_ref().filter(|f| &f.id == commit) {
            return Ok(CombinedDiff::new(fixture.combined.clone()));
        }
        unimplemented!("only the commit use case's merge fixture builds a combined diff")
    }

    fn blame(&self, _at: &ObjectId, _path: &str) -> Result<Blame, DomainError> {
        unimplemented!("the heads use case never blames")
    }

    fn archive(
        &self,
        tree: &ObjectId,
        format: ArchiveFormat,
        options: &ArchiveOptions,
    ) -> Result<Vec<u8>, DomainError> {
        let snapshot: &FakeSnapshot = self
            .snapshot
            .as_ref()
            .filter(|s: &&FakeSnapshot| &s.id == tree)
            .unwrap_or_else(|| unimplemented!("only the snapshot fixture archives"));
        // The gix adapter peels to a tree and rejects a blob (require_tree); the
        // fake mirrors that so the use case's tree-ish guard is exercised.
        if snapshot.kind == ObjectKind::Blob {
            return Err(DomainError::Invalid(format!(
                "not a tree-ish: {}",
                tree.as_str()
            )));
        }
        // Echo the format and the threaded options so the orchestration is
        // observable without a real archive.
        Ok(format!(
            "fmt={};prefix={};mtime={}",
            format.key(),
            options.prefix,
            options.modification_time
        )
        .into_bytes())
    }

    fn search(
        &self,
        _base: &ObjectId,
        _query: &SearchQuery,
        page: Page,
    ) -> Result<Vec<Commit>, DomainError> {
        // The matching and rooting are the adapter's conformance; this fake just
        // returns the configured hit set windowed by the page, so the use case's
        // own logic (gating, base resolution, paging, row assembly) is exercised.
        let hits: Vec<Commit> = self
            .search_hits
            .iter()
            .skip(page.skip)
            .take(page.limit)
            .map(commit_from_fake)
            .collect();
        Ok(hits)
    }

    fn grep(
        &self,
        _revision: &ObjectId,
        _pattern: &GrepPattern,
    ) -> Result<GrepResults, DomainError> {
        // The matching, rooting, and cap are the adapter's conformance; this fake
        // returns the scenario's configured matches so the use case's own logic
        // (gating, base resolution, per-file grouping, untabify + highlight) is
        // exercised.
        Ok(GrepResults::new(
            self.grep_matches.clone(),
            self.grep_trimmed,
        ))
    }

    fn pickaxe(
        &self,
        _base: &ObjectId,
        _pattern: &PickaxePattern,
    ) -> Result<Vec<PickaxeMatch>, DomainError> {
        // The counting, rooting, and per-commit file set are the adapter's
        // conformance; this fake returns the scenario's configured matches so the
        // use case's own logic (gating, base resolution, deletion skip, row
        // assembly) is exercised.
        let matches: Vec<PickaxeMatch> = self
            .pickaxe_hits
            .iter()
            .map(|hit: &FakePickaxeMatch| {
                let changes: Vec<PickaxeChange> = hit
                    .changes
                    .iter()
                    .map(|change: &FakePickaxeChange| match &change.blob_seed {
                        Some(seed) => PickaxeChange::changed(change.path.clone(), fake_oid(seed)),
                        None => PickaxeChange::deleted(change.path.clone()),
                    })
                    .collect();
                PickaxeMatch::new(commit_from_fake(&hit.commit), changes)
            })
            .collect();
        Ok(matches)
    }
}

/// The successful view, or a panic if the scenario produced an error.
fn view(world: &UsecaseWorld) -> &ProjectListView {
    world
        .result
        .as_ref()
        .expect("assemble the list first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The failure, or a panic if the scenario produced a success.
fn error(world: &UsecaseWorld) -> &DomainError {
    match world.result.as_ref().expect("assemble the list first") {
        Ok(_) => panic!("expected assembly to fail"),
        Err(failure) => failure,
    }
}

/// The assembled row for `name`, or a panic if it is absent.
fn row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a ProjectListRow {
    view(world)
        .rows()
        .iter()
        .find(|row: &&ProjectListRow| row.name() == name)
        .unwrap_or_else(|| panic!("no row for {name}"))
}

// --- Givens ------------------------------------------------------------------

#[given(regex = r#"^the store has project "([^"]*)"$"#)]
fn store_has_project(world: &mut UsecaseWorld, name: String) {
    world.projects.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the store has the directory "([^"]*)"$"#)]
fn store_has_directory(world: &mut UsecaseWorld, subdir: String) {
    world.containers.push(subdir);
}

#[given(regex = r#"^the store has project "([^"]*)" last changed at (\d+)$"#)]
fn store_has_aged_project(world: &mut UsecaseWorld, name: String, epoch: i64) {
    world
        .projects
        .push(ProjectInfo::named(name).with_last_activity(epoch));
}

#[given(regex = r#"^the store has project "([^"]*)" with no commits$"#)]
fn store_has_uncommitted_project(world: &mut UsecaseWorld, name: String) {
    world.projects.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the store has project "([^"]*)" described as "(.*)" owned by "([^"]*)"$"#)]
fn store_has_described_project(
    world: &mut UsecaseWorld,
    name: String,
    description: String,
    owner: String,
) {
    world.projects.push(
        ProjectInfo::named(name)
            .with_description(description)
            .with_owner(owner),
    );
}

#[given(regex = r"^the current time is (\d+)$")]
fn current_time_is(world: &mut UsecaseWorld, now: i64) {
    world.now = now;
}

#[given("the forks feature is enabled")]
fn forks_feature_enabled(world: &mut UsecaseWorld) {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Forks,
        FeatureLayer {
            default: Some(vec!["1".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    world.settings = Settings::resolve(&[layer]);
}

// --- Whens -------------------------------------------------------------------

#[when("I assemble the project list")]
fn assemble_default(world: &mut UsecaseWorld) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
        containers: world.containers.clone(),
    };
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        None,
        None,
        world.now,
    ));
}

#[when(regex = r#"^I assemble the project list ordered by "([^"]*)"$"#)]
fn assemble_ordered(world: &mut UsecaseWorld, order: String) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
        containers: world.containers.clone(),
    };
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        Some(&order),
        None,
        world.now,
    ));
}

#[when(regex = r#"^I assemble the project list filtered by "([^"]*)"$"#)]
fn assemble_filtered(world: &mut UsecaseWorld, subdir: String) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
        containers: world.containers.clone(),
    };
    let filter: ProjectFilter = ProjectFilter::new(subdir);
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        None,
        Some(&filter),
        world.now,
    ));
}

#[when(regex = r#"^I assemble the forks of "([^"]*)"$"#)]
fn assemble_the_forks(world: &mut UsecaseWorld, project: String) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
        containers: world.containers.clone(),
    };
    world.result = Some(assemble_forks(
        &store,
        &world.settings,
        &project,
        None,
        world.now,
    ));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r#"^the listed projects are "(.*)"$"#)]
fn listed_projects_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = view(world)
        .rows()
        .iter()
        .map(|row: &ProjectListRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("assembling fails as not found")]
fn fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(error(world), DomainError::NotFound(_)));
}

#[then(regex = r#"^the not-found message is "([^"]*)"$"#)]
fn not_found_message_is(world: &mut UsecaseWorld, expected: String) {
    let DomainError::NotFound(message) = error(world) else {
        panic!("expected a not-found failure");
    };
    assert_eq!(message, &expected);
}

#[then("the listing has the fork column")]
fn listing_has_fork_column(world: &mut UsecaseWorld) {
    assert!(view(world).forks_enabled());
}

#[then("the listing has no fork column")]
fn listing_has_no_fork_column(world: &mut UsecaseWorld) {
    assert!(!view(world).forks_enabled());
}

#[then(regex = r#"^the project "([^"]*)" reports (\d+) forks?$"#)]
fn project_reports_forks(world: &mut UsecaseWorld, name: String, count: usize) {
    assert_eq!(row(world, &name).fork_count(), count);
}

#[then(regex = r#"^the project "([^"]*)" has an empty fork container$"#)]
fn project_has_empty_container(world: &mut UsecaseWorld, name: String) {
    assert_eq!(row(world, &name).fork_state(), ForkState::EmptyContainer);
}

#[then(regex = r#"^the project "([^"]*)" is not fork-capable$"#)]
fn project_is_not_forkable(world: &mut UsecaseWorld, name: String) {
    assert_eq!(row(world, &name).fork_state(), ForkState::NotForkable);
}

#[then(regex = r#"^the project "([^"]*)" is forked$"#)]
fn project_is_forked(world: &mut UsecaseWorld, name: String) {
    assert!(matches!(
        row(world, &name).fork_state(),
        ForkState::Forked(_)
    ));
}

#[then("assembling fails as invalid")]
fn fails_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(error(world), DomainError::Invalid(_)));
}

#[then(regex = r#"^the project "([^"]*)" shows description "(.*)"$"#)]
fn project_shows_description(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(row(world, &name).description(), Some(expected.as_str()));
}

#[then(regex = r#"^the project "([^"]*)" shows owner "([^"]*)"$"#)]
fn project_shows_owner(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(row(world, &name).owner(), Some(expected.as_str()));
}

#[then(regex = r#"^the project "([^"]*)" shows the age "([^"]*)"$"#)]
fn project_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = row(world, &name)
        .age()
        .expect("the project has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the project "([^"]*)" has no age$"#)]
fn project_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(row(world, &name).age(), None);
}

// --- project_index: When / accessors / Thens ---------------------------------

#[when("I assemble the project index")]
fn assemble_index(world: &mut UsecaseWorld) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
        containers: world.containers.clone(),
    };
    world.index_result = Some(assemble_project_index(&store, None));
}

#[when(regex = r#"^I assemble the project index filtered by "([^"]*)"$"#)]
fn assemble_index_filtered(world: &mut UsecaseWorld, subdir: String) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
        containers: world.containers.clone(),
    };
    let filter: ProjectFilter = ProjectFilter::new(subdir);
    world.index_result = Some(assemble_project_index(&store, Some(&filter)));
}

/// The assembled index, or a panic if the scenario produced an error.
fn index(world: &UsecaseWorld) -> &ProjectIndex {
    world
        .index_result
        .as_ref()
        .expect("assemble the index first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The index failure, or a panic if the scenario produced a success.
fn index_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .index_result
        .as_ref()
        .expect("assemble the index first")
    {
        Ok(_) => panic!("expected assembly to fail"),
        Err(failure) => failure,
    }
}

/// The index row for `path`, or a panic if it is absent.
fn index_row<'a>(world: &'a UsecaseWorld, path: &str) -> &'a ProjectIndexRow {
    index(world)
        .rows()
        .iter()
        .find(|row: &&ProjectIndexRow| row.path() == path)
        .unwrap_or_else(|| panic!("no index row for {path}"))
}

#[then("assembling the index fails as not found")]
fn index_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(index_error(world), DomainError::NotFound(_)));
}

#[then(regex = r#"^the index row for "([^"]*)" has owner "([^"]*)"$"#)]
fn index_row_has_owner(world: &mut UsecaseWorld, path: String, expected: String) {
    assert_eq!(index_row(world, &path).owner(), expected);
}

#[then(regex = r#"^the indexed projects are "(.*)"$"#)]
fn indexed_projects_are(world: &mut UsecaseWorld, expected: String) {
    let paths: Vec<&str> = index(world)
        .rows()
        .iter()
        .map(|row: &ProjectIndexRow| row.path())
        .collect();
    assert_eq!(paths.join(", "), expected);
}

// --- opml: Givens / When / accessors / Thens ---------------------------------

#[given(regex = r#"^the opml store has project "([^"]*)" with a head$"#)]
fn opml_store_has_headed_project(world: &mut UsecaseWorld, name: String) {
    world.opml_projects.push((name, true));
}

#[given(regex = r#"^the opml store has project "([^"]*)" without a head$"#)]
fn opml_store_has_headless_project(world: &mut UsecaseWorld, name: String) {
    world.opml_projects.push((name, false));
}

#[when("I assemble the opml")]
fn assemble_opml_outline(world: &mut UsecaseWorld) {
    let store: FakeOpmlStore = FakeOpmlStore {
        projects: world.opml_projects.clone(),
    };
    world.opml_result = Some(assemble_opml(&store, None));
}

#[when(regex = r#"^I assemble the opml filtered by "([^"]*)"$"#)]
fn assemble_opml_filtered(world: &mut UsecaseWorld, subdir: String) {
    let store: FakeOpmlStore = FakeOpmlStore {
        projects: world.opml_projects.clone(),
    };
    let filter: ProjectFilter = ProjectFilter::new(subdir);
    world.opml_result = Some(assemble_opml(&store, Some(&filter)));
}

/// The assembled outline, or a panic if the scenario produced an error.
fn opml(world: &UsecaseWorld) -> &Opml {
    world
        .opml_result
        .as_ref()
        .expect("assemble the opml first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The opml failure, or a panic if the scenario produced a success.
fn opml_error(world: &UsecaseWorld) -> &DomainError {
    match world.opml_result.as_ref().expect("assemble the opml first") {
        Ok(_) => panic!("expected assembly to fail"),
        Err(failure) => failure,
    }
}

#[then("assembling the opml fails as not found")]
fn opml_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(opml_error(world), DomainError::NotFound(_)));
}

#[then(regex = r#"^the opml projects are "(.*)"$"#)]
fn opml_projects_are(world: &mut UsecaseWorld, expected: String) {
    let paths: Vec<&str> = opml(world)
        .projects()
        .iter()
        .map(|project: &OpmlProject| project.path())
        .collect();
    assert_eq!(paths.join(", "), expected);
}

#[then("the opml has no projects")]
fn opml_has_no_projects(world: &mut UsecaseWorld) {
    assert!(opml(world).projects().is_empty());
}

// --- heads: accessors --------------------------------------------------------

/// The assembled heads view, or a panic if the scenario produced an error.
fn heads_view(world: &UsecaseWorld) -> &HeadsView {
    world
        .heads_result
        .as_ref()
        .expect("assemble the heads first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled head row for `name`, or a panic if it is absent.
fn head_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a HeadRow {
    heads_view(world)
        .rows()
        .iter()
        .find(|row: &&HeadRow| row.name() == name)
        .unwrap_or_else(|| panic!("no head row for {name}"))
}

/// The committer epoch of an already-declared branch, for aliasing a second
/// branch onto the same commit.
fn branch_epoch(world: &UsecaseWorld, name: &str) -> i64 {
    world
        .branches
        .iter()
        .find(|branch: &&FakeBranch| branch.name == name)
        .unwrap_or_else(|| panic!("no branch {name} declared yet"))
        .epoch
}

// --- heads: Givens -----------------------------------------------------------

#[given(regex = r#"^the repository HEAD is branch "([^"]*)"$"#)]
fn head_is_branch(world: &mut UsecaseWorld, name: String) {
    world.head = Some(name);
}

#[given(regex = r#"^the repository HEAD is the unborn branch "([^"]*)"$"#)]
fn head_is_unborn(world: &mut UsecaseWorld, name: String) {
    world.head = Some(name);
}

#[given(regex = r#"^the repository has branch "([^"]*)" committed at (\d+)$"#)]
fn repo_has_branch(world: &mut UsecaseWorld, name: String, epoch: i64) {
    let tip: ObjectId = fake_oid(&name);
    world.branches.push(FakeBranch {
        dir: "heads".to_owned(),
        name,
        tip,
        epoch,
    });
}

#[given(regex = r#"^the repository has branch "([^"]*)" at the same commit as "([^"]*)"$"#)]
fn repo_has_aliased_branch(world: &mut UsecaseWorld, name: String, other: String) {
    let tip: ObjectId = fake_oid(&other);
    let epoch: i64 = branch_epoch(world, &other);
    world.branches.push(FakeBranch {
        dir: "heads".to_owned(),
        name,
        tip,
        epoch,
    });
}

#[given(regex = r#"^the "extra-branch-refs" feature lists "([^"]*)"$"#)]
fn extra_branch_refs_lists(world: &mut UsecaseWorld, dir: String) {
    world.extra_branch_refs.push(dir);
}

#[given(regex = r#"^the repository has a ref "([^"]*)" under "([^"]*)" committed at (\d+)$"#)]
fn repo_has_ref_under_dir(world: &mut UsecaseWorld, name: String, dir: String, epoch: i64) {
    let tip: ObjectId = fake_oid(&format!("{dir}/{name}"));
    world.branches.push(FakeBranch {
        dir,
        name,
        tip,
        epoch,
    });
}

// --- heads: When -------------------------------------------------------------

#[when("I assemble the heads")]
fn assemble_the_heads(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    let branch_refs: Vec<String> =
        get_branch_refs(&world.extra_branch_refs).expect("valid extra-branch-refs");
    let branch_refs: Vec<&str> = branch_refs.iter().map(String::as_str).collect();
    world.heads_result = Some(assemble_heads(&repo, world.now, &branch_refs));
}

// --- heads: Thens ------------------------------------------------------------

#[then("no heads are listed")]
fn no_heads_listed(world: &mut UsecaseWorld) {
    assert!(heads_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed heads are "(.*)"$"#)]
fn listed_heads_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = heads_view(world)
        .rows()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the head "([^"]*)" is current$"#)]
fn head_is_current(world: &mut UsecaseWorld, name: String) {
    assert!(head_row(world, &name).current());
}

#[then(regex = r#"^the head "([^"]*)" is not current$"#)]
fn head_is_not_current(world: &mut UsecaseWorld, name: String) {
    assert!(!head_row(world, &name).current());
}

#[then(regex = r#"^the head "([^"]*)" shows the age "([^"]*)"$"#)]
fn head_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = head_row(world, &name)
        .age()
        .expect("the head has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the head "([^"]*)" has no age$"#)]
fn head_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(head_row(world, &name).age(), None);
}

// --- tags: accessors ---------------------------------------------------------

/// The assembled tags view, or a panic if the scenario produced an error.
fn tags_view(world: &UsecaseWorld) -> &TagsView {
    world
        .tags_result
        .as_ref()
        .expect("assemble the tags first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled tag row for `name`, or a panic if it is absent.
fn tag_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a TagRow {
    tags_view(world)
        .rows()
        .iter()
        .find(|row: &&TagRow| row.name() == name)
        .unwrap_or_else(|| panic!("no tag row for {name}"))
}

/// Derives the two ids an annotated tag needs — the tag object the ref points at
/// and the object that tag peels to — from its name, so each stays distinct.
fn annotated_ids(name: &str) -> (ObjectId, ObjectId) {
    (
        fake_oid(&format!("tagobj-{name}")),
        fake_oid(&format!("target-{name}")),
    )
}

// --- tags: Givens ------------------------------------------------------------

#[given(
    regex = r#"^an annotated tag "([^"]*)" of a (commit|blob|tree) tagged at (\d+) with subject "(.*)"$"#
)]
fn repo_has_annotated_tag(
    world: &mut UsecaseWorld,
    name: String,
    kind: String,
    epoch: i64,
    subject: String,
) {
    let (ref_target, object): (ObjectId, ObjectId) = annotated_ids(&name);
    let object_kind: ObjectKind = ObjectKind::parse(&kind).expect("a valid object kind");
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target,
        object,
        object_kind,
        annotated: true,
        epoch,
        message: format!("{subject}\n"),
        has_tagger: true,
    });
}

#[given(regex = r#"^an annotated tag "([^"]*)" of a commit with no tagger$"#)]
fn repo_has_taggerless_annotated_tag(world: &mut UsecaseWorld, name: String) {
    let (ref_target, object): (ObjectId, ObjectId) = annotated_ids(&name);
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target,
        object,
        object_kind: ObjectKind::Commit,
        annotated: true,
        epoch: 0,
        message: "anonymous tag\n".to_owned(),
        has_tagger: false,
    });
}

#[given(regex = r#"^an annotated tag "([^"]*)" of a commit with a two-line message$"#)]
fn repo_has_multiline_annotated_tag(world: &mut UsecaseWorld, name: String) {
    let (ref_target, object): (ObjectId, ObjectId) = annotated_ids(&name);
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target,
        object,
        object_kind: ObjectKind::Commit,
        annotated: true,
        epoch: 999_400,
        message: "First line\nSecond line\n".to_owned(),
        has_tagger: true,
    });
}

#[given(regex = r#"^a lightweight tag "([^"]*)" on a commit at (\d+)$"#)]
fn repo_has_lightweight_commit_tag(world: &mut UsecaseWorld, name: String, epoch: i64) {
    let target: ObjectId = fake_oid(&format!("lw-{name}"));
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target: target.clone(),
        object: target,
        object_kind: ObjectKind::Commit,
        annotated: false,
        epoch,
        message: String::new(),
        has_tagger: false,
    });
}

#[given(regex = r#"^a lightweight tag "([^"]*)" on a (blob|tree)$"#)]
fn repo_has_lightweight_object_tag(world: &mut UsecaseWorld, name: String, kind: String) {
    let target: ObjectId = fake_oid(&format!("lw-{name}"));
    let object_kind: ObjectKind = ObjectKind::parse(&kind).expect("a valid object kind");
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target: target.clone(),
        object: target,
        object_kind,
        annotated: false,
        epoch: 0,
        message: String::new(),
        has_tagger: false,
    });
}

// --- tags: When --------------------------------------------------------------

#[when("I assemble the tags")]
fn assemble_the_tags(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.tags_result = Some(assemble_tags(&repo, world.now));
}

// --- tags: Thens -------------------------------------------------------------

#[then("no tags are listed")]
fn no_tags_listed(world: &mut UsecaseWorld) {
    assert!(tags_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed tags are "(.*)"$"#)]
fn listed_tags_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = tags_view(world)
        .rows()
        .iter()
        .map(|row: &TagRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the tag "([^"]*)" is annotated$"#)]
fn tag_is_annotated(world: &mut UsecaseWorld, name: String) {
    assert!(tag_row(world, &name).annotated());
}

#[then(regex = r#"^the tag "([^"]*)" is not annotated$"#)]
fn tag_is_not_annotated(world: &mut UsecaseWorld, name: String) {
    assert!(!tag_row(world, &name).annotated());
}

#[then(regex = r#"^the tag "([^"]*)" has subject "(.*)"$"#)]
fn tag_has_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(tag_row(world, &name).subject(), Some(expected.as_str()));
}

#[then(regex = r#"^the tag "([^"]*)" has no subject$"#)]
fn tag_has_no_subject(world: &mut UsecaseWorld, name: String) {
    assert_eq!(tag_row(world, &name).subject(), None);
}

#[then(regex = r#"^the tag "([^"]*)" has reftype "([^"]*)"$"#)]
fn tag_has_reftype(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(tag_row(world, &name).reftype().as_str(), expected);
}

#[then(regex = r#"^the tag "([^"]*)" shows the age "([^"]*)"$"#)]
fn tag_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let TagAge::Known(age) = tag_row(world, &name).age() else {
        panic!("expected a known tag age");
    };
    assert_eq!(age.humanized(), expected);
}

#[then(regex = r#"^the tag "([^"]*)" has an unknown age$"#)]
fn tag_has_unknown_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(tag_row(world, &name).age(), TagAge::Unknown);
}

#[then(regex = r#"^the tag "([^"]*)" has no age cell$"#)]
fn tag_has_no_age_cell(world: &mut UsecaseWorld, name: String) {
    assert_eq!(tag_row(world, &name).age(), TagAge::Absent);
}

// --- single tag view: accessors ----------------------------------------------

/// The resolved single-tag view, or a panic if the scenario produced an error.
fn tag_show_view(world: &UsecaseWorld) -> &TagView {
    world
        .tag_result
        .as_ref()
        .expect("show the tag first")
        .as_ref()
        .expect("the tag resolved")
}

/// The single-tag failure, or a panic if the scenario produced a success.
fn tag_show_error(world: &UsecaseWorld) -> &DomainError {
    match world.tag_result.as_ref().expect("show the tag first") {
        Ok(_) => panic!("expected showing the tag to fail"),
        Err(failure) => failure,
    }
}

// --- single tag view: When ---------------------------------------------------

#[when(regex = r#"^I show the tag "([^"]*)"$"#)]
fn show_the_single_tag(world: &mut UsecaseWorld, hash: String) {
    let repo: FakeRepository = fake_repo(world);
    world.tag_result = Some(show_tag(&repo, &hash));
}

// --- single tag view: Thens --------------------------------------------------

#[then(regex = r#"^the tag view name is "([^"]*)"$"#)]
fn tag_view_name_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_view(world).name(), expected);
}

#[then(regex = r#"^the tag view points at a "([^"]*)"$"#)]
fn tag_view_points_at(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_view(world).object_kind().as_str(), expected);
}

#[then("the tag view has a tagger")]
fn tag_view_has_tagger(world: &mut UsecaseWorld) {
    assert!(tag_show_view(world).tagger().is_some());
}

#[then("the tag view has no tagger")]
fn tag_view_has_no_tagger(world: &mut UsecaseWorld) {
    assert!(tag_show_view(world).tagger().is_none());
}

#[then(regex = r#"^the tag view tagger shows the date "([^"]*)"$"#)]
fn tag_view_tagger_date(world: &mut UsecaseWorld, expected: String) {
    let date: String = tag_show_view(world)
        .tagger()
        .expect("the tag view has a tagger")
        .timestamp()
        .rfc2822();
    assert_eq!(date, expected);
}

#[then(regex = r#"^the tag view message is "([^"]*)"$"#)]
fn tag_view_message_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_view(world).message().trim_end(), expected);
}

#[then(regex = r"^the tag view message has (\d+) lines$")]
fn tag_view_message_line_count(world: &mut UsecaseWorld, expected: usize) {
    assert_eq!(tag_show_view(world).message().lines().count(), expected);
}

#[then(regex = r#"^the tag view message line (\d+) is "([^"]*)"$"#)]
fn tag_view_message_line_is(world: &mut UsecaseWorld, number: usize, expected: String) {
    let line: &str = tag_show_view(world)
        .message()
        .lines()
        .nth(number - 1)
        .expect("the message has that line");
    assert_eq!(line, expected);
}

#[then(regex = r#"^showing the tag fails with "([^"]*)"$"#)]
fn tag_view_fails_with(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tag_show_error(world).message(), expected);
}

#[then("showing the tag fails as not found")]
fn tag_view_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(tag_show_error(world), DomainError::NotFound(_)));
}

// --- shortlog: accessors -----------------------------------------------------

/// The assembled shortlog view, or a panic if the scenario produced an error.
fn shortlog_view(world: &UsecaseWorld) -> &ShortlogView {
    world
        .shortlog_result
        .as_ref()
        .expect("assemble the shortlog first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled shortlog row for the commit declared as `name`, or a panic if
/// it is absent. The fake derives each commit id from its declared name.
fn shortlog_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a ShortlogRow {
    let id: ObjectId = fake_oid(name);
    shortlog_view(world)
        .rows()
        .iter()
        .find(|row: &&ShortlogRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no shortlog row for {name}"))
}

// --- shortlog: Givens --------------------------------------------------------

#[given(regex = r#"^the repository HEAD is at commit "([^"]*)"$"#)]
fn head_is_at_commit(world: &mut UsecaseWorld, name: String) {
    world.head_commit = Some(fake_oid(&name));
}

#[given(regex = r#"^a commit "([^"]*)" at epoch (\d+) by "([^"]*)" titled "(.*)"$"#)]
fn repo_has_commit(
    world: &mut UsecaseWorld,
    name: String,
    epoch: i64,
    author: String,
    title: String,
) {
    world.commits.push(FakeCommit {
        id: fake_oid(&name),
        epoch,
        author,
        title,
        touches: Vec::new(),
        present: Vec::new(),
    });
}

// --- shortlog: Whens ---------------------------------------------------------

#[when(regex = r"^I assemble the shortlog of the default branch with page size (\d+)$")]
fn assemble_default_shortlog(world: &mut UsecaseWorld, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.shortlog_result = Some(assemble_shortlog(
        &repo,
        None,
        world.now,
        Page::new(0, size),
    ));
}

#[when(regex = r#"^I assemble the shortlog of "([^"]*)" with page size (\d+)$"#)]
fn assemble_rev_shortlog(world: &mut UsecaseWorld, rev: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.shortlog_result = Some(assemble_shortlog(
        &repo,
        Some(&rev),
        world.now,
        Page::new(0, size),
    ));
}

// --- shortlog: Thens ---------------------------------------------------------

#[then("no commits are listed")]
fn no_commits_listed(world: &mut UsecaseWorld) {
    assert!(shortlog_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed commits are "(.*)"$"#)]
fn listed_commits_are(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = shortlog_view(world)
        .rows()
        .iter()
        .map(|row: &ShortlogRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then("the shortlog has a further page")]
fn shortlog_has_further_page(world: &mut UsecaseWorld) {
    assert!(shortlog_view(world).has_more());
}

#[then("the shortlog has no further page")]
fn shortlog_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!shortlog_view(world).has_more());
}

#[then(regex = r#"^the commit "([^"]*)" is by "([^"]*)"$"#)]
fn commit_is_by(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).author(), expected);
}

#[then(regex = r#"^the commit "([^"]*)" author shortens to "(.*)"$"#)]
fn commit_author_shortens_to(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).author_short(), expected);
}

#[then(regex = r#"^the commit "([^"]*)" shows the subject "(.*)"$"#)]
fn commit_shows_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).title(), expected);
}

#[then(regex = r#"^the commit "([^"]*)" date cell shows "(.*)"$"#)]
fn commit_date_cell_shows(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(shortlog_row(world, &name).date().displayed(), expected);
}

#[then(regex = r#"^the row for commit "([^"]*)" carries the marker "(.*)"$"#)]
fn row_carries_marker(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(
        format_markers(shortlog_row(world, &name).markers()),
        expected
    );
}

// --- search: accessors -------------------------------------------------------

fn search_view(world: &UsecaseWorld) -> &SearchView {
    world
        .search_result
        .as_ref()
        .expect("assemble the search first")
        .as_ref()
        .expect("assembly succeeded")
}

fn search_error(world: &UsecaseWorld) -> &DomainError {
    world
        .search_result
        .as_ref()
        .expect("assemble the search first")
        .as_ref()
        .expect_err("assembly failed")
}

/// The assembled search row for the commit declared as `name`, by its fake id.
fn search_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a SearchRow {
    let id: ObjectId = fake_oid(name);
    search_view(world)
        .rows()
        .iter()
        .find(|row: &&SearchRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no search row for {name}"))
}

/// Runs the search use case with the given gates and page, recording the result.
fn run_assemble_search(
    world: &mut UsecaseWorld,
    enabled: bool,
    base_rev: Option<&str>,
    text: &str,
    use_regexp: bool,
    page: Page,
) {
    let repo: FakeRepository = fake_repo(world);
    let criteria: SearchCriteria = SearchCriteria {
        kind: SearchKind::Commit,
        text: text.to_owned(),
        use_regexp,
    };
    world.search_result = Some(assemble_search(
        &repo, enabled, base_rev, &criteria, world.now, page,
    ));
}

// --- search: Givens ----------------------------------------------------------

#[given(regex = r#"^a search hit "([^"]*)" at epoch (\d+) by "([^"]*)" titled "(.*)"$"#)]
fn repo_has_search_hit(
    world: &mut UsecaseWorld,
    name: String,
    epoch: i64,
    author: String,
    title: String,
) {
    world.search_hits.push(FakeCommit {
        id: fake_oid(&name),
        epoch,
        author,
        title,
        touches: Vec::new(),
        present: Vec::new(),
    });
}

// --- search: Whens -----------------------------------------------------------

#[when(regex = r#"^I assemble a message search for "([^"]*)"$"#)]
fn assemble_message_search(world: &mut UsecaseWorld, text: String) {
    run_assemble_search(world, true, None, &text, false, Page::new(0, 100));
}

#[when(regex = r#"^I assemble a disabled message search for "([^"]*)"$"#)]
fn assemble_disabled_search(world: &mut UsecaseWorld, text: String) {
    run_assemble_search(world, false, None, &text, false, Page::new(0, 100));
}

#[when(regex = r#"^I assemble a regexp message search for "([^"]*)"$"#)]
fn assemble_regexp_search(world: &mut UsecaseWorld, text: String) {
    run_assemble_search(world, true, None, &text, true, Page::new(0, 100));
}

#[when(regex = r#"^I assemble a message search for "([^"]*)" rooted at "([^"]*)"$"#)]
fn assemble_rooted_search(world: &mut UsecaseWorld, text: String, base: String) {
    run_assemble_search(world, true, Some(&base), &text, false, Page::new(0, 100));
}

#[when(regex = r#"^I assemble a message search for "([^"]*)" with page size (\d+)$"#)]
fn assemble_paged_search(world: &mut UsecaseWorld, text: String, size: usize) {
    run_assemble_search(world, true, None, &text, false, Page::new(0, size));
}

// --- search: Thens -----------------------------------------------------------

#[then("the search is forbidden")]
fn search_is_forbidden(world: &mut UsecaseWorld) {
    assert!(matches!(search_error(world), DomainError::Forbidden(_)));
}

#[then("the search is invalid")]
fn search_is_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(search_error(world), DomainError::Invalid(_)));
}

#[then("the search reports an unknown commit object")]
fn search_unknown_commit(world: &mut UsecaseWorld) {
    assert!(
        matches!(search_error(world), DomainError::NotFound(what) if what == "Unknown commit object")
    );
}

#[then("no commits are listed in the search")]
fn search_lists_nothing(world: &mut UsecaseWorld) {
    assert!(search_view(world).rows().is_empty());
}

#[then(regex = r"^the search lists (\d+) commits$")]
fn search_lists_count(world: &mut UsecaseWorld, count: usize) {
    assert_eq!(search_view(world).rows().len(), count);
}

#[then("the search has a further page")]
fn search_has_further_page(world: &mut UsecaseWorld) {
    assert!(search_view(world).has_more());
}

#[then("the search has no further page")]
fn search_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!search_view(world).has_more());
}

#[then(regex = r#"^search row "([^"]*)" shows the subject "(.*)"$"#)]
fn search_row_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(search_row(world, &name).title(), expected);
}

#[then(regex = r#"^search row "([^"]*)" highlights "(.*)"$"#)]
fn search_row_highlights(world: &mut UsecaseWorld, name: String, expected: String) {
    let matched: &str = search_row(world, &name)
        .snippets()
        .first()
        .expect("a highlighted snippet")
        .matched();
    assert_eq!(matched, expected);
}

#[then(regex = r#"^search row "([^"]*)" author shortens to "(.*)"$"#)]
fn search_row_author_short(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(search_row(world, &name).author_short(), expected);
}

// --- grep: accessors ---------------------------------------------------------

fn grep_view(world: &UsecaseWorld) -> &GrepView {
    world
        .grep_result
        .as_ref()
        .expect("assemble the grep first")
        .as_ref()
        .expect("assembly succeeded")
}

fn grep_error(world: &UsecaseWorld) -> &DomainError {
    world
        .grep_result
        .as_ref()
        .expect("assemble the grep first")
        .as_ref()
        .expect_err("assembly failed")
}

fn grep_file(world: &UsecaseWorld, index: usize) -> &GrepFileView {
    &grep_view(world).files()[index]
}

fn grep_row(world: &UsecaseWorld, file: usize, row: usize) -> &GrepRow {
    &grep_file(world, file).rows()[row]
}

/// Runs the grep use case with the given gates, base, and mode.
fn run_assemble_grep(
    world: &mut UsecaseWorld,
    search_enabled: bool,
    grep_enabled: bool,
    base_rev: Option<&str>,
    text: &str,
    use_regexp: bool,
) {
    let repo: FakeRepository = fake_repo(world);
    world.grep_result = Some(assemble_grep(
        &repo,
        search_enabled,
        grep_enabled,
        base_rev,
        text,
        use_regexp,
    ));
}

// --- grep: Givens ------------------------------------------------------------

#[given(regex = r#"^a grep line match in "([^"]*)" at line (\d+) with text "(.*)"$"#)]
fn repo_has_grep_line(world: &mut UsecaseWorld, path: String, line: usize, text: String) {
    let decoded: String = text.replace("\\t", "\t");
    world
        .grep_matches
        .push(GrepMatch::line(path, line, decoded));
}

#[given(regex = r#"^a grep binary match in "([^"]*)"$"#)]
fn repo_has_grep_binary(world: &mut UsecaseWorld, path: String) {
    world.grep_matches.push(GrepMatch::binary(path));
}

#[given("the grep listing is trimmed")]
fn repo_grep_trimmed(world: &mut UsecaseWorld) {
    world.grep_trimmed = true;
}

// --- grep: Whens -------------------------------------------------------------

#[when(regex = r#"^I assemble a grep for "([^"]*)"$"#)]
fn assemble_plain_grep(world: &mut UsecaseWorld, text: String) {
    run_assemble_grep(world, true, true, None, &text, false);
}

#[when(regex = r#"^I assemble a search-disabled grep for "([^"]*)"$"#)]
fn assemble_search_disabled_grep(world: &mut UsecaseWorld, text: String) {
    run_assemble_grep(world, false, true, None, &text, false);
}

#[when(regex = r#"^I assemble a grep-disabled grep for "([^"]*)"$"#)]
fn assemble_grep_disabled_grep(world: &mut UsecaseWorld, text: String) {
    run_assemble_grep(world, true, false, None, &text, false);
}

#[when(regex = r#"^I assemble a regexp grep for "([^"]*)"$"#)]
fn assemble_regexp_grep(world: &mut UsecaseWorld, text: String) {
    run_assemble_grep(world, true, true, None, &text, true);
}

#[when(regex = r#"^I assemble a grep for "([^"]*)" rooted at "([^"]*)"$"#)]
fn assemble_rooted_grep(world: &mut UsecaseWorld, text: String, base: String) {
    run_assemble_grep(world, true, true, Some(&base), &text, false);
}

// --- grep: Thens -------------------------------------------------------------

#[then(regex = r#"^the grep is forbidden as "(.*)"$"#)]
fn grep_is_forbidden(world: &mut UsecaseWorld, message: String) {
    assert!(matches!(grep_error(world), DomainError::Forbidden(what) if *what == message));
}

#[then("the grep is invalid")]
fn grep_is_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(grep_error(world), DomainError::Invalid(_)));
}

#[then("the grep reports an unknown commit object")]
fn grep_unknown_commit(world: &mut UsecaseWorld) {
    assert!(
        matches!(grep_error(world), DomainError::NotFound(what) if what == "Unknown commit object")
    );
}

#[then(regex = r#"^the grep header title is "(.*)"$"#)]
fn grep_header_title(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(grep_view(world).title(), expected);
}

#[then(regex = r#"^the grep roots blob links at "([^"]*)"$"#)]
fn grep_roots_at(world: &mut UsecaseWorld, name: String) {
    assert_eq!(grep_view(world).base_id(), fake_oid(&name).as_str());
}

#[then(regex = r"^the grep lists (\d+) files?$")]
fn grep_lists_files(world: &mut UsecaseWorld, count: usize) {
    assert_eq!(grep_view(world).files().len(), count);
}

#[then(regex = r#"^grep file (\d+) is "([^"]*)" with (\d+) rows?$"#)]
fn grep_file_is(world: &mut UsecaseWorld, index: usize, path: String, rows: usize) {
    let file: &GrepFileView = grep_file(world, index);
    assert_eq!(file.path(), path);
    assert_eq!(file.rows().len(), rows);
}

#[then(
    regex = r#"^grep file (\d+) row (\d+) is line (\d+) highlighting lead "(.*)" match "(.*)" trail "(.*)"$"#
)]
fn grep_row_highlighting(
    world: &mut UsecaseWorld,
    file: usize,
    row: usize,
    line: usize,
    lead: String,
    matched: String,
    trail: String,
) {
    assert_highlighted(grep_row(world, file, row), line, &lead, &matched, &trail);
}

#[then(regex = r#"^grep file (\d+) row (\d+) is line (\d+) plain "(.*)"$"#)]
fn grep_row_plain(world: &mut UsecaseWorld, file: usize, row: usize, line: usize, text: String) {
    assert_plain(grep_row(world, file, row), line, &text);
}

#[then(regex = r"^grep file (\d+) row (\d+) is a binary file$")]
fn grep_row_binary(world: &mut UsecaseWorld, file: usize, row: usize) {
    assert!(matches!(grep_row(world, file, row), GrepRow::Binary));
}

#[then("the grep result is trimmed")]
fn grep_result_trimmed(world: &mut UsecaseWorld) {
    assert!(grep_view(world).trimmed());
}

#[then("the grep result is not trimmed")]
fn grep_result_not_trimmed(world: &mut UsecaseWorld) {
    assert!(!grep_view(world).trimmed());
}

/// Asserts a row is a highlighted line with the given number and lead/match/trail.
fn assert_highlighted(row: &GrepRow, line: usize, lead: &str, matched: &str, trail: &str) {
    let GrepRow::Line {
        line_no,
        line: content,
    } = row
    else {
        panic!("expected a line row, found a binary row");
    };
    assert_eq!(*line_no, line);
    let GrepLine::Highlighted(snippet) = content else {
        panic!("expected a highlighted line, found a plain one");
    };
    assert_eq!(snippet.lead(), lead);
    assert_eq!(snippet.matched(), matched);
    assert_eq!(snippet.trail(), trail);
}

/// Asserts a row is a plain (unhighlighted) line with the given number and text.
fn assert_plain(row: &GrepRow, line: usize, text: &str) {
    let GrepRow::Line {
        line_no,
        line: content,
    } = row
    else {
        panic!("expected a line row, found a binary row");
    };
    assert_eq!(*line_no, line);
    let GrepLine::Plain(whole) = content else {
        panic!("expected a plain line, found a highlighted one");
    };
    assert_eq!(whole, text);
}

// --- pickaxe: accessors ------------------------------------------------------

fn pickaxe_view(world: &UsecaseWorld) -> &PickaxeView {
    world
        .pickaxe_result
        .as_ref()
        .expect("assemble a pickaxe first")
        .as_ref()
        .expect("the pickaxe succeeded")
}

fn pickaxe_error(world: &UsecaseWorld) -> &DomainError {
    world
        .pickaxe_result
        .as_ref()
        .expect("assemble a pickaxe first")
        .as_ref()
        .expect_err("the pickaxe failed")
}

/// The assembled row for the named commit (its id is `fake_oid(name)`).
fn pickaxe_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a PickaxeRow {
    let target: String = fake_oid(name).as_str().to_owned();
    pickaxe_view(world)
        .rows()
        .iter()
        .find(|row: &&PickaxeRow| row.id() == target)
        .unwrap_or_else(|| panic!("no assembled pickaxe row for {name}"))
}

/// The declared hit for the named commit, so a following step can attach its
/// count-changing files.
fn pickaxe_hit_mut<'a>(world: &'a mut UsecaseWorld, name: &str) -> &'a mut FakePickaxeMatch {
    let target: ObjectId = fake_oid(name);
    world
        .pickaxe_hits
        .iter_mut()
        .find(|hit: &&mut FakePickaxeMatch| hit.commit.id == target)
        .unwrap_or_else(|| panic!("declare pickaxe hit {name} first"))
}

fn run_assemble_pickaxe(
    world: &mut UsecaseWorld,
    search_enabled: bool,
    pickaxe_enabled: bool,
    base: Option<&str>,
    text: &str,
    use_regexp: bool,
) {
    let repo: FakeRepository = fake_repo(world);
    world.pickaxe_result = Some(assemble_pickaxe(
        &repo,
        search_enabled,
        pickaxe_enabled,
        base,
        text,
        use_regexp,
        world.now,
    ));
}

// --- pickaxe: Givens ---------------------------------------------------------

#[given(regex = r#"^a pickaxe hit "([^"]*)" at epoch (\d+) by "([^"]*)" titled "(.*)"$"#)]
fn repo_has_pickaxe_hit(
    world: &mut UsecaseWorld,
    name: String,
    epoch: i64,
    author: String,
    title: String,
) {
    world.pickaxe_hits.push(FakePickaxeMatch {
        commit: FakeCommit {
            id: fake_oid(&name),
            epoch,
            author,
            title,
            touches: Vec::new(),
            present: Vec::new(),
        },
        changes: Vec::new(),
    });
}

#[given(regex = r#"^the pickaxe hit "([^"]*)" changed file "([^"]*)" at blob "([^"]*)"$"#)]
fn pickaxe_hit_changed_file(world: &mut UsecaseWorld, name: String, path: String, blob: String) {
    pickaxe_hit_mut(world, &name)
        .changes
        .push(FakePickaxeChange {
            path,
            blob_seed: Some(blob),
        });
}

#[given(regex = r#"^the pickaxe hit "([^"]*)" deleted file "([^"]*)"$"#)]
fn pickaxe_hit_deleted_file(world: &mut UsecaseWorld, name: String, path: String) {
    pickaxe_hit_mut(world, &name)
        .changes
        .push(FakePickaxeChange {
            path,
            blob_seed: None,
        });
}

// --- pickaxe: Whens ----------------------------------------------------------

#[when(regex = r#"^I assemble a pickaxe for "([^"]*)"$"#)]
fn assemble_pickaxe_when(world: &mut UsecaseWorld, text: String) {
    run_assemble_pickaxe(world, true, true, None, &text, false);
}

#[when(regex = r#"^I assemble a search-disabled pickaxe for "([^"]*)"$"#)]
fn assemble_search_disabled_pickaxe(world: &mut UsecaseWorld, text: String) {
    run_assemble_pickaxe(world, false, true, None, &text, false);
}

#[when(regex = r#"^I assemble a pickaxe-disabled pickaxe for "([^"]*)"$"#)]
fn assemble_pickaxe_disabled_pickaxe(world: &mut UsecaseWorld, text: String) {
    run_assemble_pickaxe(world, true, false, None, &text, false);
}

#[when(regex = r#"^I assemble a regexp pickaxe for "([^"]*)"$"#)]
fn assemble_regexp_pickaxe(world: &mut UsecaseWorld, text: String) {
    run_assemble_pickaxe(world, true, true, None, &text, true);
}

#[when(regex = r#"^I assemble a pickaxe for "([^"]*)" rooted at "([^"]*)"$"#)]
fn assemble_pickaxe_rooted(world: &mut UsecaseWorld, text: String, base: String) {
    run_assemble_pickaxe(world, true, true, Some(&base), &text, false);
}

// --- pickaxe: Thens ----------------------------------------------------------

#[then(regex = r#"^the pickaxe is forbidden as "([^"]*)"$"#)]
fn pickaxe_is_forbidden(world: &mut UsecaseWorld, message: String) {
    assert!(matches!(pickaxe_error(world), DomainError::Forbidden(m) if m == &message));
}

#[then("the pickaxe is invalid")]
fn pickaxe_is_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(pickaxe_error(world), DomainError::Invalid(_)));
}

#[then("the pickaxe reports an unknown commit object")]
fn pickaxe_unknown_commit(world: &mut UsecaseWorld) {
    assert!(
        matches!(pickaxe_error(world), DomainError::NotFound(m) if m == "Unknown commit object")
    );
}

#[then(regex = r#"^the pickaxe header title is "([^"]*)"$"#)]
fn pickaxe_header_title(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(pickaxe_view(world).title(), expected);
}

#[then(regex = r#"^the pickaxe roots at "([^"]*)"$"#)]
fn pickaxe_roots_at(world: &mut UsecaseWorld, name: String) {
    assert_eq!(pickaxe_view(world).base_id(), fake_oid(&name).as_str());
}

#[then(regex = r"^the pickaxe lists (\d+) commits?$")]
fn pickaxe_lists_commits(world: &mut UsecaseWorld, count: usize) {
    assert_eq!(pickaxe_view(world).rows().len(), count);
}

#[then(regex = r#"^pickaxe commit (\d+) is "([^"]*)"$"#)]
fn pickaxe_commit_is(world: &mut UsecaseWorld, index: usize, name: String) {
    assert_eq!(
        pickaxe_view(world).rows()[index].id(),
        fake_oid(&name).as_str()
    );
}

#[then(regex = r#"^pickaxe commit "([^"]*)" lists (\d+) files?$"#)]
fn pickaxe_commit_lists_files(world: &mut UsecaseWorld, name: String, count: usize) {
    assert_eq!(pickaxe_row(world, &name).files().len(), count);
}

#[then(regex = r#"^pickaxe commit "([^"]*)" file (\d+) is "([^"]*)" linking blob "([^"]*)"$"#)]
fn pickaxe_commit_file_is(
    world: &mut UsecaseWorld,
    name: String,
    index: usize,
    path: String,
    blob: String,
) {
    let file = &pickaxe_row(world, &name).files()[index];
    assert_eq!(file.path(), path);
    assert_eq!(file.blob(), fake_oid(&blob).as_str());
}

#[then(regex = r#"^pickaxe commit "([^"]*)" author shortens to "(.*)"$"#)]
fn pickaxe_commit_author_short(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(pickaxe_row(world, &name).author_short(), expected);
}

#[then(regex = r#"^pickaxe commit "([^"]*)" full author is "([^"]*)"$"#)]
fn pickaxe_commit_full_author(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(pickaxe_row(world, &name).author(), expected);
}

// --- ref markers: accessors --------------------------------------------------

/// Renders a marker list as `kind[*]:name` items (the `*` flags an indirect,
/// annotated-tag marker), so one assertion pins kind, indirect and name at once.
fn format_markers(markers: &[RefMarker]) -> String {
    markers
        .iter()
        .map(|marker: &RefMarker| {
            let indirect: &str = if marker.indirect() { "*" } else { "" };
            format!(
                "{}{}:{}",
                marker.kind().class_token(),
                indirect,
                marker.name()
            )
        })
        .collect::<Vec<String>>()
        .join(", ")
}

fn marker_view_of(name: &str) -> MarkerView {
    match name {
        "shortlog" => MarkerView::Shortlog,
        "log" => MarkerView::Log,
        "history" => MarkerView::History,
        "tag" => MarkerView::Tag,
        _ => MarkerView::Other,
    }
}

fn marker_index(world: &UsecaseWorld) -> &RefMarkerIndex {
    world
        .marker_index
        .as_ref()
        .expect("index the ref markers first")
        .as_ref()
        .expect("indexing succeeded")
}

// --- ref markers: Givens -----------------------------------------------------

#[given(regex = r#"^branch "([^"]*)" points at commit "([^"]*)"$"#)]
fn branch_points_at(world: &mut UsecaseWorld, name: String, commit: String) {
    world.branches.push(FakeBranch {
        dir: "heads".to_owned(),
        name,
        tip: fake_oid(&commit),
        epoch: 0,
    });
}

#[given(regex = r#"^a lightweight tag "([^"]*)" points at commit "([^"]*)"$"#)]
fn lightweight_tag_points_at(world: &mut UsecaseWorld, name: String, commit: String) {
    let target: ObjectId = fake_oid(&commit);
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target: target.clone(),
        object: target,
        object_kind: ObjectKind::Commit,
        annotated: false,
        epoch: 0,
        message: String::new(),
        has_tagger: false,
    });
}

#[given(regex = r#"^an annotated tag "([^"]*)" points at commit "([^"]*)"$"#)]
fn annotated_tag_points_at(world: &mut UsecaseWorld, name: String, commit: String) {
    world.tags.push(FakeTag {
        full_name: format!("refs/tags/{name}"),
        ref_target: fake_oid(&format!("tagobj-{name}")),
        object: fake_oid(&commit),
        object_kind: ObjectKind::Commit,
        annotated: true,
        epoch: 0,
        message: "release\n".to_owned(),
        has_tagger: true,
    });
}

// --- ref markers: When -------------------------------------------------------

#[when(regex = r#"^I index ref markers for the "([^"]*)" view$"#)]
fn index_ref_markers(world: &mut UsecaseWorld, view: String) {
    let repo: FakeRepository = fake_repo(world);
    world.marker_index = Some(assemble_ref_markers(&repo, marker_view_of(&view)));
}

// --- ref markers: Thens ------------------------------------------------------

#[then(regex = r#"^commit "([^"]*)" has no markers$"#)]
fn commit_has_no_markers(world: &mut UsecaseWorld, commit: String) {
    assert!(
        marker_index(world)
            .for_commit(&fake_oid(&commit))
            .is_empty()
    );
}

#[then(regex = r#"^the markers for commit "([^"]*)" are "(.*)"$"#)]
fn markers_for_commit_are(world: &mut UsecaseWorld, commit: String, expected: String) {
    let markers: Vec<RefMarker> = marker_index(world).for_commit(&fake_oid(&commit));
    assert_eq!(format_markers(&markers), expected);
}

// --- log: accessors ----------------------------------------------------------

/// The assembled log view, or a panic if the scenario produced an error.
fn log_view(world: &UsecaseWorld) -> &LogView {
    world
        .log_result
        .as_ref()
        .expect("assemble the log first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled log row for the commit declared as `name`, or a panic if it is
/// absent. The fake derives each commit id from its declared name.
fn log_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a LogRow {
    let id: ObjectId = fake_oid(name);
    log_view(world)
        .rows()
        .iter()
        .find(|row: &&LogRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no log row for {name}"))
}

/// Builds the fake repository from the world's declared refs and commits.
fn fake_repo(world: &UsecaseWorld) -> FakeRepository {
    FakeRepository {
        head: world.head.clone(),
        head_commit: world.head_commit.clone(),
        branches: world.branches.clone(),
        tags: world.tags.clone(),
        commits: world.commits.clone(),
        search_hits: world.search_hits.clone(),
        grep_matches: world.grep_matches.clone(),
        grep_trimmed: world.grep_trimmed,
        pickaxe_hits: world.pickaxe_hits.clone(),
        remotes: world.remotes.clone(),
        remote_branches: world.remote_branches.clone(),
        tree_commit_title: world.tree_commit_title.clone(),
        tree_nodes: world.tree_nodes.clone(),
        blob_base_title: world.blob_base_title.clone(),
        blob_files: world.blob_files.clone(),
        commit_fixture: world.commit_fixture.clone(),
        snapshot: world.snapshot.clone(),
        patch_series: world.patch_series.clone(),
    }
}

// --- log: Whens --------------------------------------------------------------

#[when(regex = r"^I assemble the log of the default branch with page size (\d+)$")]
fn assemble_default_log(world: &mut UsecaseWorld, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.log_result = Some(assemble_log(&repo, None, world.now, Page::new(0, size)));
}

#[when(regex = r#"^I assemble the log of "([^"]*)" with page size (\d+)$"#)]
fn assemble_rev_log(world: &mut UsecaseWorld, rev: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.log_result = Some(assemble_log(
        &repo,
        Some(&rev),
        world.now,
        Page::new(0, size),
    ));
}

// --- log: Thens --------------------------------------------------------------

#[then("no log entries are listed")]
fn no_log_entries(world: &mut UsecaseWorld) {
    assert!(log_view(world).rows().is_empty());
}

#[then(regex = r#"^the logged commits are "(.*)"$"#)]
fn logged_commits_are(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = log_view(world)
        .rows()
        .iter()
        .map(|row: &LogRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then("the log has a further page")]
fn log_has_further_page(world: &mut UsecaseWorld) {
    assert!(log_view(world).has_more());
}

#[then("the log has no further page")]
fn log_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!log_view(world).has_more());
}

#[then(regex = r#"^the log entry "([^"]*)" is by "([^"]*)"$"#)]
fn log_entry_is_by(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).author(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" shows the title "(.*)"$"#)]
fn log_entry_shows_title(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).title(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" shows the age "(.*)"$"#)]
fn log_entry_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).age(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" was authored on "([^"]*)"$"#)]
fn log_entry_authored_on(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(log_row(world, &name).timestamp().utc_date(), expected);
}

#[then(regex = r#"^the log entry "([^"]*)" body begins "(.*)"$"#)]
fn log_entry_body_begins(world: &mut UsecaseWorld, name: String, expected: String) {
    let first: &LogLine = log_row(world, &name)
        .comment()
        .first()
        .expect("the log body has at least one line");
    assert_eq!(first, &LogLine::Text(expected));
}

#[then(regex = r#"^the log entry "([^"]*)" carries the marker "(.*)"$"#)]
fn log_entry_carries_marker(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(format_markers(log_row(world, &name).markers()), expected);
}

// --- summary: accessors ------------------------------------------------------

/// The assembled summary view, or a panic if the scenario produced an error.
fn summary_view(world: &UsecaseWorld) -> &SummaryView {
    world
        .summary_result
        .as_ref()
        .expect("assemble the summary first")
        .as_ref()
        .expect("assembly succeeded")
}

// --- summary: Givens ---------------------------------------------------------

#[given(regex = r#"^the project is named "([^"]*)"$"#)]
fn summary_project_named(world: &mut UsecaseWorld, name: String) {
    world.summary_name = Some(name);
}

#[given(regex = r#"^the project is described as "(.*)"$"#)]
fn summary_project_described(world: &mut UsecaseWorld, text: String) {
    world.summary_description = Some(text);
}

#[given(regex = r#"^the project is owned by "([^"]*)"$"#)]
fn summary_project_owned_by(world: &mut UsecaseWorld, owner: String) {
    world.summary_owner = Some(owner);
}

#[given(regex = r#"^the project has clone url "([^"]*)"$"#)]
fn summary_project_clone_url(world: &mut UsecaseWorld, url: String) {
    world.summary_clone_urls.push(url);
}

#[given(regex = r#"^the project README is "(.*)"$"#)]
fn summary_project_readme(world: &mut UsecaseWorld, contents: String) {
    world.summary_readme = Some(contents);
}

#[given("owner display is omitted")]
fn summary_omit_owner(world: &mut UsecaseWorld) {
    world.summary_omit_owner = true;
}

#[given("XSS prevention is on")]
fn summary_prevent_xss(world: &mut UsecaseWorld) {
    world.summary_prevent_xss = true;
}

#[given(regex = r#"^the git base URL is "([^"]*)"$"#)]
fn summary_base_url(world: &mut UsecaseWorld, base: String) {
    world.summary_base_urls.push(base);
}

// --- summary: When -----------------------------------------------------------

#[when("I assemble the summary")]
fn assemble_the_summary(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    let name: String = world
        .summary_name
        .clone()
        .unwrap_or_else(|| "repo".to_owned());
    let mut info: ProjectInfo = ProjectInfo::named(name);
    if let Some(description) = &world.summary_description {
        info = info.with_description(description.clone());
    }
    if let Some(owner) = &world.summary_owner {
        info = info.with_owner(owner.clone());
    }
    for url in &world.summary_clone_urls {
        info = info.with_clone_url(url.clone());
    }
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::ExtraBranchRefs,
        FeatureLayer {
            default: Some(world.extra_branch_refs.clone()),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        omit_owner: Some(world.summary_omit_owner),
        prevent_xss: Some(world.summary_prevent_xss),
        git_base_url_list: Some(world.summary_base_urls.clone()),
        features,
        ..SettingsLayer::default()
    };
    let settings: Settings = Settings::resolve(&[layer]);
    world.summary_result = Some(assemble_summary(
        &repo,
        &info,
        world.summary_readme.as_deref(),
        &settings,
        world.now,
    ));
}

// --- summary: Thens ----------------------------------------------------------

#[then(regex = r#"^the summary description is "(.*)"$"#)]
fn summary_description_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).description(), expected);
}

#[then(regex = r#"^the summary shows the owner "([^"]*)"$"#)]
fn summary_shows_owner(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).owner(), Some(expected.as_str()));
}

#[then("the summary shows no owner")]
fn summary_shows_no_owner(world: &mut UsecaseWorld) {
    assert_eq!(summary_view(world).owner(), None);
}

#[then(regex = r#"^the summary last change date is "([^"]*)"$"#)]
fn summary_last_change_date(world: &mut UsecaseWorld, expected: String) {
    let date: String = summary_view(world)
        .last_change()
        .expect("the summary has a last change")
        .utc_date();
    assert_eq!(date, expected);
}

#[then("the summary shows no last change")]
fn summary_shows_no_last_change(world: &mut UsecaseWorld) {
    assert_eq!(summary_view(world).last_change(), None);
}

#[then(regex = r#"^the summary clone urls are "(.*)"$"#)]
fn summary_clone_urls_are(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).clone_urls().join(", "), expected);
}

#[then("the summary advertises no clone urls")]
fn summary_advertises_no_clone_urls(world: &mut UsecaseWorld) {
    assert!(summary_view(world).clone_urls().is_empty());
}

#[then(regex = r#"^the summary README is "(.*)"$"#)]
fn summary_readme_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(summary_view(world).readme(), Some(expected.as_str()));
}

#[then("the summary includes no README")]
fn summary_includes_no_readme(world: &mut UsecaseWorld) {
    assert_eq!(summary_view(world).readme(), None);
}

#[then(regex = r#"^the summary shortlog lists "(.*)"$"#)]
fn summary_shortlog_lists(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = summary_view(world)
        .shortlog()
        .rows()
        .iter()
        .map(|row: &ShortlogRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then("the summary shortlog is empty")]
fn summary_shortlog_is_empty(world: &mut UsecaseWorld) {
    assert!(summary_view(world).shortlog().rows().is_empty());
}

#[then(regex = r#"^the summary lists tags "(.*)"$"#)]
fn summary_lists_tags(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = summary_view(world)
        .tags()
        .shown()
        .iter()
        .map(|row: &TagRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("the summary tags section is not truncated")]
fn summary_tags_not_truncated(world: &mut UsecaseWorld) {
    assert!(!summary_view(world).tags().is_truncated());
}

#[then(regex = r#"^the summary lists heads "(.*)"$"#)]
fn summary_lists_heads(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = summary_view(world)
        .heads()
        .shown()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("the summary heads section is not truncated")]
fn summary_heads_not_truncated(world: &mut UsecaseWorld) {
    assert!(!summary_view(world).heads().is_truncated());
}

// --- remotes: accessors ------------------------------------------------------

/// The assembled remotes view, or a panic if the scenario produced an error.
fn remotes_view(world: &UsecaseWorld) -> &RemotesView {
    world
        .remotes_result
        .as_ref()
        .expect("assemble the remotes first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The remotes failure, or a panic if the scenario produced a success.
fn remotes_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .remotes_result
        .as_ref()
        .expect("assemble the remotes first")
    {
        Ok(_) => panic!("expected assembling the remotes to fail"),
        Err(failure) => failure,
    }
}

/// The assembled block for remote `name`, or a panic if it is absent.
fn remote_block<'a>(world: &'a UsecaseWorld, name: &str) -> &'a RemoteBlock {
    remotes_view(world)
        .blocks()
        .iter()
        .find(|block: &&RemoteBlock| block.name() == name)
        .unwrap_or_else(|| panic!("no remote block for {name}"))
}

/// The tracking-branch row named `branch` in remote `remote`, or a panic.
fn tracking_row<'a>(world: &'a UsecaseWorld, remote: &str, branch: &str) -> &'a HeadRow {
    remote_block(world, remote)
        .heads()
        .iter()
        .find(|row: &&HeadRow| row.name() == branch)
        .unwrap_or_else(|| panic!("no tracking branch {branch} on remote {remote}"))
}

/// Serializes one URL line to its `role value` form (`missing` for the
/// placeholder), so a scenario asserts a block's whole URL set with one string.
fn url_line_string(line: &RemoteUrl) -> String {
    match line {
        RemoteUrl::Combined(url) => format!("combined {url}"),
        RemoteUrl::Fetch(url) => format!("fetch {url}"),
        RemoteUrl::Push(url) => format!("push {url}"),
        RemoteUrl::Missing => "missing".to_owned(),
    }
}

/// The settings the remotes use case reads: the `remote_heads` feature on or off,
/// matching gitweb's `gitweb_check_feature('remote_heads')` gate.
fn remotes_settings(world: &UsecaseWorld) -> Settings {
    if !world.remote_heads_enabled {
        return Settings::builtin();
    }
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::RemoteHeads,
        FeatureLayer {
            default: Some(vec!["1".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

// --- remotes: Givens ---------------------------------------------------------

#[given("the remote_heads feature is enabled")]
fn remote_heads_enabled(world: &mut UsecaseWorld) {
    world.remote_heads_enabled = true;
}

#[given("the remote_heads feature is disabled")]
fn remote_heads_disabled(world: &mut UsecaseWorld) {
    world.remote_heads_enabled = false;
}

// --- search_help -------------------------------------------------------------

/// A feature layer that turns a feature off (gitweb's `gitweb.<name> = 0`).
fn feature_off() -> FeatureLayer {
    FeatureLayer {
        default: Some(vec!["0".to_owned()]),
        overridable: None,
    }
}

/// The settings the search-help use case reads: the `grep` and `pickaxe` gates,
/// each left at its enabled-by-default builtin unless the scenario disabled it.
fn search_help_settings(world: &UsecaseWorld) -> Settings {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    if world.search_help_grep_disabled {
        features.insert(FeatureName::Grep, feature_off());
    }
    if world.search_help_pickaxe_disabled {
        features.insert(FeatureName::Pickaxe, feature_off());
    }
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

#[given("the grep feature is disabled")]
fn search_help_grep_disabled(world: &mut UsecaseWorld) {
    world.search_help_grep_disabled = true;
}

#[given("the pickaxe feature is disabled")]
fn search_help_pickaxe_disabled(world: &mut UsecaseWorld) {
    world.search_help_pickaxe_disabled = true;
}

#[when("I assemble the search help")]
fn assemble_search_help_when(world: &mut UsecaseWorld) {
    let settings: Settings = search_help_settings(world);
    world.search_help_view = Some(assemble_search_help(&settings));
}

#[then(regex = r#"^the documented topics are "(.*)"$"#)]
fn documented_topics_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = world
        .search_help_view
        .as_ref()
        .expect("assemble the search help first")
        .topics()
        .iter()
        .map(|topic: &SearchHelpTopic| topic.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[given(regex = r#"^a remote "([^"]*)" fetching from "([^"]*)" pushing to "([^"]*)"$"#)]
fn given_usecase_remote_fetch_push(
    world: &mut UsecaseWorld,
    name: String,
    fetch: String,
    push: String,
) {
    world
        .remotes
        .push(Remote::new(name, Some(fetch), Some(push)));
}

#[given(regex = r#"^a remote "([^"]*)" fetching from "([^"]*)"$"#)]
fn given_usecase_remote_fetch(world: &mut UsecaseWorld, name: String, fetch: String) {
    world.remotes.push(Remote::new(name, Some(fetch), None));
}

#[given(regex = r#"^the remote "([^"]*)" tracks branch "([^"]*)" committed at (\d+)$"#)]
fn remote_tracks_branch(world: &mut UsecaseWorld, remote: String, branch: String, epoch: i64) {
    let tip: ObjectId = fake_oid(&format!("{remote}/{branch}"));
    world.remote_branches.push(FakeRemoteBranch {
        remote,
        name: branch,
        tip,
        epoch,
    });
}

#[given(regex = r#"^the remote "([^"]*)" tracks branch "([^"]*)" at commit "([^"]*)"$"#)]
fn remote_tracks_branch_at_commit(
    world: &mut UsecaseWorld,
    remote: String,
    branch: String,
    commit: String,
) {
    let tip: ObjectId = fake_oid(&commit);
    world.remote_branches.push(FakeRemoteBranch {
        remote,
        name: branch,
        tip,
        epoch: 500_000,
    });
}

// --- remotes: Whens ----------------------------------------------------------

#[when("I assemble the remotes")]
fn assemble_the_remotes(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    let settings: Settings = remotes_settings(world);
    world.remotes_result = Some(assemble_remotes(&repo, &settings, None, world.now));
}

#[when(regex = r#"^I assemble the remote "([^"]*)"$"#)]
fn assemble_one_remote(world: &mut UsecaseWorld, name: String) {
    let repo: FakeRepository = fake_repo(world);
    let settings: Settings = remotes_settings(world);
    world.remotes_result = Some(assemble_remotes(&repo, &settings, Some(&name), world.now));
}

// --- remotes: Thens ----------------------------------------------------------

#[then(regex = r#"^the shown remotes are "(.*)"$"#)]
fn shown_remotes_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = remotes_view(world)
        .blocks()
        .iter()
        .map(|block: &RemoteBlock| block.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the remote "([^"]*)" URL lines are "(.*)"$"#)]
fn remote_url_lines_are(world: &mut UsecaseWorld, name: String, expected: String) {
    let rendered: String = remote_block(world, &name)
        .urls()
        .iter()
        .map(url_line_string)
        .collect::<Vec<String>>()
        .join(", ");
    assert_eq!(rendered, expected);
}

#[then(regex = r#"^the remote "([^"]*)" tracks no branches$"#)]
fn remote_tracks_no_branches(world: &mut UsecaseWorld, name: String) {
    assert!(remote_block(world, &name).heads().is_empty());
}

#[then(regex = r#"^the remote "([^"]*)" tracks "(.*)"$"#)]
fn remote_tracks(world: &mut UsecaseWorld, name: String, expected: String) {
    let names: Vec<&str> = remote_block(world, &name)
        .heads()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the remote "([^"]*)" tracking branch "([^"]*)" shows the age "([^"]*)"$"#)]
fn remote_tracking_age(world: &mut UsecaseWorld, remote: String, branch: String, expected: String) {
    let humanized: String = tracking_row(world, &remote, &branch)
        .age()
        .expect("the tracking branch has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the remote "([^"]*)" tracking branch "([^"]*)" is current$"#)]
fn remote_tracking_current(world: &mut UsecaseWorld, remote: String, branch: String) {
    assert!(tracking_row(world, &remote, &branch).current());
}

#[then("the remotes view is the single-remote view")]
fn remotes_view_is_single(world: &mut UsecaseWorld) {
    assert!(remotes_view(world).is_single());
}

#[then("assembling the remotes fails as forbidden")]
fn remotes_fails_forbidden(world: &mut UsecaseWorld) {
    assert!(matches!(remotes_error(world), DomainError::Forbidden(_)));
}

#[then("assembling the remotes fails as not found")]
fn remotes_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(remotes_error(world), DomainError::NotFound(_)));
}

// --- history: accessors ------------------------------------------------------

/// The assembled history view, or a panic if the scenario produced an error.
fn history_view(world: &UsecaseWorld) -> &HistoryView {
    world
        .history_result
        .as_ref()
        .expect("assemble the history first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The history failure, or a panic if the scenario produced a success.
fn history_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .history_result
        .as_ref()
        .expect("assemble the history first")
    {
        Ok(_) => panic!("expected assembling the history to fail"),
        Err(failure) => failure,
    }
}

/// The history row for the commit declared as `name`, or a panic if it is absent.
fn history_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a HistoryRow {
    let id: ObjectId = fake_oid(name);
    history_view(world)
        .rows()
        .iter()
        .find(|row: &&HistoryRow| row.id() == id.as_str())
        .unwrap_or_else(|| panic!("no history row for {name}"))
}

/// The fake commit declared as `name`, mutable so a later step can record the
/// paths it changed.
fn commit_named_mut<'a>(world: &'a mut UsecaseWorld, name: &str) -> &'a mut FakeCommit {
    let id: ObjectId = fake_oid(name);
    world
        .commits
        .iter_mut()
        .find(|commit: &&mut FakeCommit| commit.id == id)
        .unwrap_or_else(|| panic!("no commit named {name}"))
}

// --- history: Givens ---------------------------------------------------------

#[given(regex = r#"^commit "([^"]*)" changes file "([^"]*)" to blob "([^"]*)"$"#)]
fn commit_changes_file(world: &mut UsecaseWorld, name: String, path: String, blob: String) {
    let oid: ObjectId = fake_oid(&blob);
    let commit: &mut FakeCommit = commit_named_mut(world, &name);
    commit.touches.push(path.clone());
    commit.present.push(FakePathEntry {
        path,
        oid,
        kind: ObjectKind::Blob,
    });
}

#[given(regex = r#"^commit "([^"]*)" changes directory "([^"]*)" to tree "([^"]*)"$"#)]
fn commit_changes_directory(world: &mut UsecaseWorld, name: String, path: String, tree: String) {
    let oid: ObjectId = fake_oid(&tree);
    let commit: &mut FakeCommit = commit_named_mut(world, &name);
    commit.touches.push(path.clone());
    commit.present.push(FakePathEntry {
        path,
        oid,
        kind: ObjectKind::Tree,
    });
}

#[given(regex = r#"^commit "([^"]*)" deletes "([^"]*)"$"#)]
fn commit_deletes(world: &mut UsecaseWorld, name: String, path: String) {
    commit_named_mut(world, &name).touches.push(path);
}

// --- history: Whens ----------------------------------------------------------

#[when(
    regex = r#"^I assemble the history of "([^"]*)" from the default branch with page size (\d+)$"#
)]
fn assemble_default_history(world: &mut UsecaseWorld, path: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.history_result = Some(assemble_history(
        &repo,
        None,
        &path,
        world.now,
        Page::new(0, size),
    ));
}

#[when(regex = r#"^I assemble the history of "([^"]*)" from "([^"]*)" with page size (\d+)$"#)]
fn assemble_rev_history(world: &mut UsecaseWorld, path: String, rev: String, size: usize) {
    let repo: FakeRepository = fake_repo(world);
    world.history_result = Some(assemble_history(
        &repo,
        Some(&rev),
        &path,
        world.now,
        Page::new(0, size),
    ));
}

// --- history: Thens ----------------------------------------------------------

#[then("assembling the history fails")]
fn history_fails(world: &mut UsecaseWorld) {
    assert!(matches!(history_error(world), DomainError::Backend(_)));
}

#[then(regex = r#"^the history lists "(.*)"$"#)]
fn history_lists(world: &mut UsecaseWorld, expected: String) {
    let actual: Vec<String> = history_view(world)
        .rows()
        .iter()
        .map(|row: &HistoryRow| row.id().to_owned())
        .collect();
    let wanted: Vec<String> = expected
        .split(", ")
        .map(|name: &str| fake_oid(name).as_str().to_owned())
        .collect();
    assert_eq!(actual, wanted);
}

#[then(regex = r#"^the history file name is "([^"]*)"$"#)]
fn history_file_name_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(history_view(world).file_name(), expected);
}

#[then(regex = r#"^the history file type is "([^"]*)"$"#)]
fn history_file_type_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(history_view(world).file_type().as_str(), expected);
}

#[then(regex = r#"^the history current blob is "([^"]*)"$"#)]
fn history_current_blob_is(world: &mut UsecaseWorld, blob: String) {
    assert_eq!(history_view(world).file_hash(), fake_oid(&blob).as_str());
}

#[then("the history has a further page")]
fn history_has_further_page(world: &mut UsecaseWorld) {
    assert!(history_view(world).has_more());
}

#[then("the history has no further page")]
fn history_has_no_further_page(world: &mut UsecaseWorld) {
    assert!(!history_view(world).has_more());
}

#[then(regex = r#"^the history row "([^"]*)" is by "([^"]*)"$"#)]
fn history_row_is_by(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).author(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" shows the subject "(.*)"$"#)]
fn history_row_shows_subject(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).title(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" date cell shows "(.*)"$"#)]
fn history_row_date_cell_shows(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).date().displayed(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" author shortens to "(.*)"$"#)]
fn history_row_author_shortens_to(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(history_row(world, &name).author_short(), expected);
}

#[then(regex = r#"^the history row "([^"]*)" offers a diff to current "([^"]*)"$"#)]
fn history_row_offers_diff(world: &mut UsecaseWorld, name: String, blob: String) {
    assert_eq!(
        history_row(world, &name).diff_to_current(),
        Some(fake_oid(&blob).as_str())
    );
}

#[then(regex = r#"^the history row "([^"]*)" offers no diff to current$"#)]
fn history_row_offers_no_diff(world: &mut UsecaseWorld, name: String) {
    assert_eq!(history_row(world, &name).diff_to_current(), None);
}

#[then(regex = r#"^the history row "([^"]*)" carries the marker "(.*)"$"#)]
fn history_row_carries_marker(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(
        format_markers(history_row(world, &name).markers()),
        expected
    );
}

// --- tree: accessors ---------------------------------------------------------

/// The assembled tree view, or a panic if the scenario produced an error.
fn tree_view(world: &UsecaseWorld) -> &TreeView {
    world
        .tree_result
        .as_ref()
        .expect("assemble the tree first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The tree failure, or a panic if the scenario produced a success.
fn tree_error(world: &UsecaseWorld) -> &DomainError {
    match world.tree_result.as_ref().expect("assemble the tree first") {
        Ok(_) => panic!("expected assembling the tree to fail"),
        Err(failure) => failure,
    }
}

/// The listed tree row for `name`, or a panic if it is absent.
fn tree_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a TreeRow {
    tree_view(world)
        .rows()
        .iter()
        .find(|row: &&TreeRow| row.name() == name)
        .unwrap_or_else(|| panic!("no tree row for {name}"))
}

/// The mutable root tree node, or a panic if the base was not declared yet.
fn tree_root_mut(world: &mut UsecaseWorld) -> &mut FakeTreeNode {
    world
        .tree_nodes
        .iter_mut()
        .find(|node: &&mut FakeTreeNode| node.label == "root")
        .expect("declare the tree base first")
}

// --- tree: Givens ------------------------------------------------------------

#[given(regex = r#"^the tree base is commit "(.*)"$"#)]
fn tree_base_is_commit(world: &mut UsecaseWorld, title: String) {
    world.tree_commit_title = Some(title);
    world.tree_show_sizes = true;
    world.tree_nodes.push(FakeTreeNode {
        label: "root".to_owned(),
        path: None,
        entries: Vec::new(),
    });
}

#[given(regex = r#"^the tree has file "([^"]*)" of (\d+) bytes$"#)]
fn tree_has_file(world: &mut UsecaseWorld, name: String, size: u64) {
    tree_root_mut(world).entries.push(FakeTreeEntry {
        mode: "100644".to_owned(),
        name,
        size,
        target: None,
    });
}

#[given(regex = r#"^the tree has executable "([^"]*)" of (\d+) bytes$"#)]
fn tree_has_executable(world: &mut UsecaseWorld, name: String, size: u64) {
    tree_root_mut(world).entries.push(FakeTreeEntry {
        mode: "100755".to_owned(),
        name,
        size,
        target: None,
    });
}

#[given(regex = r#"^the tree has symlink "([^"]*)" pointing to "([^"]*)"$"#)]
fn tree_has_symlink(world: &mut UsecaseWorld, name: String, target: String) {
    tree_root_mut(world).entries.push(FakeTreeEntry {
        mode: "120000".to_owned(),
        name,
        size: target.len() as u64,
        target: Some(target),
    });
}

#[given(regex = r#"^the tree has directory "([^"]*)"$"#)]
fn tree_has_directory(world: &mut UsecaseWorld, name: String) {
    tree_root_mut(world).entries.push(FakeTreeEntry {
        mode: "040000".to_owned(),
        name,
        size: 0,
        target: None,
    });
}

#[given(regex = r#"^the tree has submodule "([^"]*)"$"#)]
fn tree_has_submodule(world: &mut UsecaseWorld, name: String) {
    tree_root_mut(world).entries.push(FakeTreeEntry {
        mode: "160000".to_owned(),
        name,
        size: 0,
        target: None,
    });
}

#[given(regex = r#"^the directory "([^"]*)" lists file "([^"]*)" of (\d+) bytes$"#)]
fn directory_lists_file(world: &mut UsecaseWorld, path: String, name: String, size: u64) {
    world.tree_nodes.push(FakeTreeNode {
        label: path.clone(),
        path: Some(path),
        entries: vec![FakeTreeEntry {
            mode: "100644".to_owned(),
            name,
            size,
            target: None,
        }],
    });
}

#[given("the show-sizes feature is off")]
fn tree_show_sizes_off(world: &mut UsecaseWorld) {
    world.tree_show_sizes = false;
}

// --- tree: Whens -------------------------------------------------------------

#[when("I assemble the tree")]
fn assemble_the_tree(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.tree_result = Some(assemble_tree(&repo, None, None, world.tree_show_sizes));
}

#[when(regex = r#"^I assemble the tree of "([^"]*)"$"#)]
fn assemble_the_tree_of(world: &mut UsecaseWorld, path: String) {
    let repo: FakeRepository = fake_repo(world);
    world.tree_result = Some(assemble_tree(
        &repo,
        None,
        Some(&path),
        world.tree_show_sizes,
    ));
}

// --- tree: Thens -------------------------------------------------------------

#[then("no tree entries are listed")]
fn no_tree_entries(world: &mut UsecaseWorld) {
    assert!(tree_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed tree entries are "(.*)"$"#)]
fn listed_tree_entries_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = tree_view(world)
        .rows()
        .iter()
        .map(|row: &TreeRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the tree entry "([^"]*)" is a "([^"]*)"$"#)]
fn tree_entry_is_a(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(tree_row(world, &name).object_kind().as_str(), expected);
}

#[then(regex = r#"^the tree entry "([^"]*)" has size (\d+)$"#)]
fn tree_entry_has_size(world: &mut UsecaseWorld, name: String, expected: u64) {
    assert_eq!(tree_row(world, &name).size(), Some(expected));
}

#[then(regex = r#"^the tree entry "([^"]*)" has no size$"#)]
fn tree_entry_has_no_size(world: &mut UsecaseWorld, name: String) {
    assert_eq!(tree_row(world, &name).size(), None);
}

#[then(regex = r#"^the tree entry "([^"]*)" permission string is "([^"]*)"$"#)]
fn tree_entry_permission_string(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(tree_row(world, &name).mode().permission_string(), expected);
}

#[then(regex = r#"^the tree entry "([^"]*)" points to "([^"]*)"$"#)]
fn tree_entry_points_to(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(
        tree_row(world, &name).symlink_target(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the tree shows the commit subject "(.*)"$"#)]
fn tree_shows_commit_subject(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(tree_view(world).commit_title(), Some(expected.as_str()));
}

#[then("the tree size column is hidden")]
fn tree_size_column_hidden(world: &mut UsecaseWorld) {
    assert!(!tree_view(world).show_sizes());
}

#[then("the tree offers a parent row to the root")]
fn tree_parent_to_root(world: &mut UsecaseWorld) {
    let parent: &gitweb_domain::usecase::tree::TreeParent =
        tree_view(world).parent().expect("a parent row");
    assert_eq!(parent.path(), None);
}

#[then(regex = r#"^the tree offers a parent row to "([^"]*)"$"#)]
fn tree_parent_to_path(world: &mut UsecaseWorld, expected: String) {
    let parent: &gitweb_domain::usecase::tree::TreeParent =
        tree_view(world).parent().expect("a parent row");
    assert_eq!(parent.path(), Some(expected.as_str()));
}

#[then("the tree offers no parent row")]
fn tree_offers_no_parent(world: &mut UsecaseWorld) {
    assert!(tree_view(world).parent().is_none());
}

#[then("assembling the tree fails as not found")]
fn tree_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(tree_error(world), DomainError::NotFound(_)));
}

// --- blob: accessors ---------------------------------------------------------

/// The assembled blob view, or a panic if the scenario produced an error.
fn blob_view(world: &UsecaseWorld) -> &BlobView {
    world
        .blob_result
        .as_ref()
        .expect("assemble the blob first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The blob failure, or a panic if the scenario produced a success.
fn blob_error(world: &UsecaseWorld) -> &DomainError {
    match world.blob_result.as_ref().expect("assemble the blob first") {
        Ok(_) => panic!("expected assembling the blob to fail"),
        Err(failure) => failure,
    }
}

// --- blob: Givens ------------------------------------------------------------

#[given(regex = r#"^the blob base is commit "(.*)"$"#)]
fn given_blob_base(world: &mut UsecaseWorld, subject: String) {
    world.blob_base_title = Some(subject);
}

#[given(regex = r#"^the blob "([^"]*)" at "([^"]*)" contains text "(.*)"$"#)]
fn given_blob_text(world: &mut UsecaseWorld, label: String, path: String, text: String) {
    // The feature writes a multi-line file as the two characters `\n`; expand
    // them to real newlines so the use case sees the lines it must split.
    let content: String = text.replace("\\n", "\n");
    world.blob_files.push(FakeBlobFile {
        label,
        path: Some(path),
        content: content.into_bytes(),
    });
}

#[given(regex = r#"^the blob "([^"]*)" at "([^"]*)" contains bytes "([^"]*)"$"#)]
fn given_blob_bytes(world: &mut UsecaseWorld, label: String, path: String, hex: String) {
    world.blob_files.push(FakeBlobFile {
        label,
        path: Some(path),
        content: parse_hex_bytes(&hex),
    });
}

// --- blob: Whens -------------------------------------------------------------

#[when(regex = r#"^I assemble the blob at path "([^"]*)"$"#)]
fn assemble_blob_at_path(world: &mut UsecaseWorld, path: String) {
    let repo: FakeRepository = fake_repo(world);
    world.blob_result = Some(assemble_blob(
        &repo,
        Some("HEAD"),
        None,
        Some(&path),
        FallbackEncoding::Latin1,
    ));
}

#[when(regex = r#"^I assemble the blob of id "([^"]*)"$"#)]
fn assemble_blob_of_id(world: &mut UsecaseWorld, id: String) {
    let repo: FakeRepository = fake_repo(world);
    world.blob_result = Some(assemble_blob(
        &repo,
        None,
        Some(&id),
        None,
        FallbackEncoding::Latin1,
    ));
}

#[when("I assemble the blob with neither id nor path")]
fn assemble_blob_neither(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.blob_result = Some(assemble_blob(
        &repo,
        Some("HEAD"),
        None,
        None,
        FallbackEncoding::Latin1,
    ));
}

// --- blob: Thens -------------------------------------------------------------

#[then("the blob displays as text")]
fn blob_displays_text(world: &mut UsecaseWorld) {
    assert_eq!(blob_view(world).display(), BlobDisplay::Text);
}

#[then("the blob displays as an image")]
fn blob_displays_image(world: &mut UsecaseWorld) {
    assert_eq!(blob_view(world).display(), BlobDisplay::Image);
}

#[then("the blob displays as binary")]
fn blob_displays_binary(world: &mut UsecaseWorld) {
    assert_eq!(blob_view(world).display(), BlobDisplay::Binary);
}

#[then(regex = r#"^the blob has (\d+) lines$"#)]
fn blob_has_lines(world: &mut UsecaseWorld, count: usize) {
    assert_eq!(blob_view(world).lines().len(), count);
}

#[then("the blob has no lines")]
fn blob_has_no_lines(world: &mut UsecaseWorld) {
    assert!(blob_view(world).lines().is_empty());
}

#[then(regex = r#"^blob line (\d+) is "(.*)"$"#)]
fn blob_line_is(world: &mut UsecaseWorld, number: usize, expected: String) {
    assert_eq!(blob_view(world).lines()[number - 1], expected);
}

#[then(regex = r#"^the blob shows the commit subject "(.*)"$"#)]
fn blob_shows_subject(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(blob_view(world).commit_title(), Some(expected.as_str()));
}

#[then("the blob shows no commit subject")]
fn blob_shows_no_subject(world: &mut UsecaseWorld) {
    assert_eq!(blob_view(world).commit_title(), None);
}

#[then("assembling the blob fails as invalid")]
fn blob_fails_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(blob_error(world), DomainError::Invalid(_)));
}

#[then("assembling the blob fails as not found")]
fn blob_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(blob_error(world), DomainError::NotFound(_)));
}

// --- blob_plain --------------------------------------------------------------

/// The assembled raw-blob view, or a panic if the scenario produced an error.
fn raw_blob_view(world: &UsecaseWorld) -> &BlobPlainView {
    world
        .blob_plain_result
        .as_ref()
        .expect("serve the blob raw first")
        .as_ref()
        .expect("serving succeeded")
}

/// The raw-blob failure, or a panic if the scenario produced a success.
fn raw_blob_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .blob_plain_result
        .as_ref()
        .expect("serve the blob raw first")
    {
        Ok(_) => panic!("expected serving the blob raw to fail"),
        Err(failure) => failure,
    }
}

#[when(regex = r#"^I serve the blob raw at path "([^"]*)"$"#)]
fn serve_raw_at_path(world: &mut UsecaseWorld, path: String) {
    let repo: FakeRepository = fake_repo(world);
    world.blob_plain_result = Some(assemble_blob_plain(
        &repo,
        Some("HEAD"),
        None,
        Some(&path),
        None,
        false,
    ));
}

#[when(regex = r#"^I serve the blob raw of id "([^"]*)"$"#)]
fn serve_raw_of_id(world: &mut UsecaseWorld, id: String) {
    let repo: FakeRepository = fake_repo(world);
    world.blob_plain_result = Some(assemble_blob_plain(
        &repo,
        None,
        Some(&id),
        None,
        None,
        false,
    ));
}

#[when("I serve the blob raw with neither id nor path")]
fn serve_raw_neither(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.blob_plain_result = Some(assemble_blob_plain(
        &repo,
        Some("HEAD"),
        None,
        None,
        None,
        false,
    ));
}

#[then(regex = r#"^the raw blob is served as "([^"]*)"$"#)]
fn raw_blob_served_as(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(raw_blob_view(world).content_type(), expected);
}

#[then(regex = r#"^the raw blob is offered inline as "([^"]*)"$"#)]
fn raw_blob_offered_inline(world: &mut UsecaseWorld, file_name: String) {
    assert_eq!(
        raw_blob_view(world).content_disposition(),
        format!(r#"inline; filename="{file_name}""#)
    );
}

#[then(regex = r#"^the raw blob body is "(.*)"$"#)]
fn raw_blob_body_is(world: &mut UsecaseWorld, text: String) {
    let expected: Vec<u8> = text.replace("\\n", "\n").into_bytes();
    assert_eq!(raw_blob_view(world).bytes(), expected.as_slice());
}

#[then(regex = r#"^the raw blob body has bytes "([^"]*)"$"#)]
fn raw_blob_body_has_bytes(world: &mut UsecaseWorld, hex: String) {
    assert_eq!(
        raw_blob_view(world).bytes(),
        parse_hex_bytes(&hex).as_slice()
    );
}

#[then("serving the raw blob fails as invalid")]
fn raw_blob_fails_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(raw_blob_error(world), DomainError::Invalid(_)));
}

#[then("serving the raw blob fails as not found")]
fn raw_blob_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(raw_blob_error(world), DomainError::NotFound(_)));
}

// --- object: the generic dispatch redirect (git_object) ----------------------

/// The resolved redirect target, or a panic if the scenario produced an error.
fn object_redirect(world: &UsecaseWorld) -> &ObjectRedirect {
    world
        .object_result
        .as_ref()
        .expect("assemble the object redirect first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The redirect failure, or a panic if the scenario produced a success.
fn object_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .object_result
        .as_ref()
        .expect("assemble the object redirect first")
    {
        Ok(_) => panic!("expected assembling the object to fail"),
        Err(failure) => failure,
    }
}

#[when(regex = r#"^I assemble the object redirect for hash "([^"]*)"$"#)]
fn assemble_object_for_hash(world: &mut UsecaseWorld, hash: String) {
    let repo: FakeRepository = fake_repo(world);
    world.object_result = Some(assemble_object_redirect(&repo, Some(&hash), None, None));
}

#[when(regex = r#"^I assemble the object redirect for base "([^"]*)" and file "([^"]*)"$"#)]
fn assemble_object_for_base_path(world: &mut UsecaseWorld, base: String, file: String) {
    let repo: FakeRepository = fake_repo(world);
    world.object_result = Some(assemble_object_redirect(
        &repo,
        None,
        Some(&base),
        Some(&file),
    ));
}

#[when("I assemble the object redirect with neither a hash nor a base")]
fn assemble_object_with_neither(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.object_result = Some(assemble_object_redirect(&repo, None, None, None));
}

#[then(regex = r#"^the redirect action is "([^"]*)"$"#)]
fn redirect_action_is(world: &mut UsecaseWorld, expected: String) {
    let want: Action = Action::parse(&expected).expect("a valid action name");
    assert_eq!(object_redirect(world).action, want);
}

#[then(regex = r#"^the redirect hash is "([^"]*)"$"#)]
fn redirect_hash_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(
        object_redirect(world).hash.as_deref(),
        Some(expected.as_str())
    );
}

#[then("the redirect has a hash")]
fn redirect_has_a_hash(world: &mut UsecaseWorld) {
    assert!(object_redirect(world).hash.is_some());
}

#[then("the redirect has no hash")]
fn redirect_has_no_hash(world: &mut UsecaseWorld) {
    assert_eq!(object_redirect(world).hash, None);
}

#[then(regex = r#"^the redirect base is "([^"]*)"$"#)]
fn redirect_base_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(
        object_redirect(world).hash_base.as_deref(),
        Some(expected.as_str())
    );
}

#[then("the redirect has no base")]
fn redirect_has_no_base(world: &mut UsecaseWorld) {
    assert_eq!(object_redirect(world).hash_base, None);
}

#[then(regex = r#"^the redirect file is "([^"]*)"$"#)]
fn redirect_file_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(
        object_redirect(world).file_name.as_deref(),
        Some(expected.as_str())
    );
}

#[then("the redirect has no file")]
fn redirect_has_no_file(world: &mut UsecaseWorld) {
    assert_eq!(object_redirect(world).file_name, None);
}

#[then(regex = r#"^assembling the object fails with "([^"]*)"$"#)]
fn assembling_object_fails_with(world: &mut UsecaseWorld, message: String) {
    assert_eq!(object_error(world).message(), message);
}

// --- object: the no-action default inline dispatch (dispatch git_get_type) ----

/// The resolved dispatch failure, or a panic if the scenario produced a success.
fn dispatch_action_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .dispatch_action_result
        .as_ref()
        .expect("resolve the dispatch action first")
    {
        Ok(_) => panic!("expected resolving the dispatch action to fail"),
        Err(failure) => failure,
    }
}

#[when(regex = r#"^I resolve the dispatch action for hash "([^"]*)"$"#)]
fn resolve_dispatch_for_hash(world: &mut UsecaseWorld, hash: String) {
    let repo: FakeRepository = fake_repo(world);
    world.dispatch_action_result = Some(resolve_dispatch_action(&repo, Some(&hash), None, None));
}

#[when(regex = r#"^I resolve the dispatch action for base "([^"]*)" and file "([^"]*)"$"#)]
fn resolve_dispatch_for_base_path(world: &mut UsecaseWorld, base: String, file: String) {
    let repo: FakeRepository = fake_repo(world);
    world.dispatch_action_result = Some(resolve_dispatch_action(
        &repo,
        None,
        Some(&base),
        Some(&file),
    ));
}

#[then(regex = r#"^the dispatch action is "([^"]*)"$"#)]
fn dispatch_action_is(world: &mut UsecaseWorld, expected: String) {
    let want: Action = Action::parse(&expected).expect("a valid action name");
    let got: Action = *world
        .dispatch_action_result
        .as_ref()
        .expect("resolve the dispatch action first")
        .as_ref()
        .expect("resolution succeeded");
    assert_eq!(got, want);
}

#[then(regex = r#"^resolving the dispatch action fails with "([^"]*)"$"#)]
fn resolving_dispatch_fails_with(world: &mut UsecaseWorld, message: String) {
    assert_eq!(dispatch_action_error(world).message(), message);
}

// --- feed: Whens -------------------------------------------------------------

#[when("I assemble the feed of the default branch")]
fn assemble_default_feed(world: &mut UsecaseWorld) {
    let repo: FakeRepository = fake_repo(world);
    world.feed_result = Some(assemble_feed(&repo, None, None, world.now));
}

#[when(regex = r#"^I assemble the feed of the default branch narrowed to "([^"]*)"$"#)]
fn assemble_narrowed_feed(world: &mut UsecaseWorld, path: String) {
    let repo: FakeRepository = fake_repo(world);
    world.feed_result = Some(assemble_feed(&repo, None, Some(&path), world.now));
}

// --- feed: Thens -------------------------------------------------------------

/// The assembled feed, or a panic if assembly failed.
fn feed_view(world: &UsecaseWorld) -> &Feed {
    world
        .feed_result
        .as_ref()
        .expect("assemble the feed first")
        .as_ref()
        .expect("feed assembly succeeded")
}

/// The feed entry for the commit named `name`, by its derived id.
fn feed_entry<'a>(world: &'a UsecaseWorld, name: &str) -> &'a FeedEntry {
    let id: ObjectId = fake_oid(name);
    feed_view(world)
        .entries()
        .iter()
        .find(|entry: &&FeedEntry| entry.id() == id.as_str())
        .unwrap_or_else(|| panic!("no feed entry for {name}"))
}

#[then(regex = r#"^the feed has (\d+) entries$"#)]
fn feed_has_entries(world: &mut UsecaseWorld, expected: usize) {
    assert_eq!(feed_view(world).entries().len(), expected);
}

#[then("the feed has a latest timestamp")]
fn feed_has_latest(world: &mut UsecaseWorld) {
    assert!(feed_view(world).latest().is_some());
}

#[then("the feed has no latest timestamp")]
fn feed_has_no_latest(world: &mut UsecaseWorld) {
    assert!(feed_view(world).latest().is_none());
}

#[then(regex = r#"^the feed entry "([^"]*)" is titled "(.*)"$"#)]
fn feed_entry_titled(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(feed_entry(world, &name).title(), expected);
}

#[then(regex = r#"^the feed entry "([^"]*)" lists files "([^"]*)"$"#)]
fn feed_entry_lists_files(world: &mut UsecaseWorld, name: String, expected: String) {
    let listed: String = feed_entry(world, &name)
        .files()
        .iter()
        .map(FeedFile::to_path)
        .collect::<Vec<&str>>()
        .join(",");
    assert_eq!(listed, expected);
}

// --- commit: fixtures, Whens, Thens -----------------------------------------

/// The object kind a scenario names for the commit fixture's resolved object.
fn object_kind_of(word: &str) -> ObjectKind {
    match word {
        "blob" => ObjectKind::Blob,
        "tree" => ObjectKind::Tree,
        "tag" => ObjectKind::Tag,
        _ => ObjectKind::Commit,
    }
}

/// A `100644` regular-file mode for the commit fixture's diff entries.
fn regular_file_mode() -> FileMode {
    FileMode::from_octal("100644").expect("100644 is a valid mode")
}

/// A representative tree-entry mode for `kind`, so the per-path history walk's
/// `path_entry` row carries a mode consistent with the object it resolved (a
/// tree for a directory, a gitlink for a submodule commit, a regular file
/// otherwise). Only the id is observed for these rows, so the exact file mode is
/// a faithful-but-don't-care choice.
fn mode_for_kind(kind: ObjectKind) -> FileMode {
    let octal: &str = match kind {
        ObjectKind::Tree => "040000",
        ObjectKind::Commit => "160000",
        ObjectKind::Blob | ObjectKind::Tag => "100644",
    };
    FileMode::from_octal(octal).expect("a valid octal mode")
}

/// The absent (`000000`) mode for a created or deleted side.
fn absent_mode() -> FileMode {
    FileMode::from_octal("000000").expect("000000 is a valid mode")
}

/// One ordinary diff entry for a changed path, shaped to its status: a creation
/// comes from the empty side, a deletion goes to it, anything else is a
/// same-path modification.
fn fixture_diff_entry(path: &str, token: &str) -> DiffEntry {
    let status: ChangeStatus = ChangeStatus::parse(token).expect("a valid status token");
    match status.kind() {
        ChangeKind::Added => DiffEntry::new(
            status,
            absent_mode(),
            regular_file_mode(),
            fake_oid(&format!("from-{path}")).null_like(),
            fake_oid(&format!("to-{path}")),
            path.to_owned(),
            path.to_owned(),
        ),
        ChangeKind::Deleted => DiffEntry::new(
            status,
            regular_file_mode(),
            absent_mode(),
            fake_oid(&format!("from-{path}")),
            fake_oid(&format!("to-{path}")).null_like(),
            path.to_owned(),
            path.to_owned(),
        ),
        _ => DiffEntry::new(
            status,
            regular_file_mode(),
            regular_file_mode(),
            fake_oid(&format!("from-{path}")),
            fake_oid(&format!("to-{path}")),
            path.to_owned(),
            path.to_owned(),
        ),
    }
}

/// One combined diff entry for a merge changed path: every parent reports a plain
/// modification of the path, resolving to a single merge result.
fn fixture_combined_entry(path: &str, nparents: usize) -> CombinedDiffEntry {
    let parents: Vec<CombinedParent> = (0..nparents)
        .map(|index: usize| {
            CombinedParent::new(
                ChangeStatus::from_modification(regular_file_mode(), regular_file_mode()),
                regular_file_mode(),
                fake_oid(&format!("from-{path}-{index}")),
            )
        })
        .collect();
    CombinedDiffEntry::new(
        parents,
        regular_file_mode(),
        fake_oid(&format!("to-{path}")),
        path.to_owned(),
    )
}

/// The commit fixture under construction, expecting a `Given a commit …` first.
fn commit_fixture_mut(world: &mut UsecaseWorld) -> &mut FakeCommitFixture {
    world
        .commit_fixture
        .as_mut()
        .expect("declare a commit fixture first")
}

#[given(regex = r#"^a commit "([^"]*)" with author "([^"]*)"$"#)]
fn given_commit(world: &mut UsecaseWorld, id: String, ident: String) {
    let author: Signature = Signature::parse(&ident).expect("a valid author ident");
    world.commit_fixture = Some(FakeCommitFixture {
        rev: "HEAD".to_owned(),
        id: fake_oid(&format!("commit-{id}")),
        tree: fake_oid(&format!("tree-{id}")),
        parents: Vec::new(),
        author: author.clone(),
        committer: author,
        message: String::new(),
        kind: ObjectKind::Commit,
        diff: Vec::new(),
        combined: Vec::new(),
        patch: Patch::new(Vec::new()),
        binary_sizes: Vec::new(),
        rev_name_tag: None,
    });
}

#[given(regex = r#"^the commit is tag-named "(.*)"$"#)]
fn given_commit_tag_named(world: &mut UsecaseWorld, name: String) {
    commit_fixture_mut(world).rev_name_tag = Some(name);
}

#[given(regex = r#"^the commit committer is "([^"]*)"$"#)]
fn given_commit_committer(world: &mut UsecaseWorld, ident: String) {
    commit_fixture_mut(world).committer =
        Signature::parse(&ident).expect("a valid committer ident");
}

#[given(regex = r#"^the commit message is "(.*)"$"#)]
fn given_commit_message(world: &mut UsecaseWorld, message: String) {
    commit_fixture_mut(world).message = message;
}

#[given(regex = r#"^the commit has parent "([^"]*)"$"#)]
fn given_commit_parent(world: &mut UsecaseWorld, label: String) {
    commit_fixture_mut(world).parents.push(fake_oid(&label));
}

#[given(regex = r#"^the commit object kind is "([^"]*)"$"#)]
fn given_commit_object_kind(world: &mut UsecaseWorld, word: String) {
    commit_fixture_mut(world).kind = object_kind_of(&word);
}

#[given(regex = r#"^the commit changes "([^"]*)" with status "([^"]*)"$"#)]
fn given_commit_changes(world: &mut UsecaseWorld, path: String, token: String) {
    let entry: DiffEntry = fixture_diff_entry(&path, &token);
    commit_fixture_mut(world).diff.push(entry);
}

#[given(regex = r#"^the merge changes "([^"]*)"$"#)]
fn given_merge_changes(world: &mut UsecaseWorld, path: String) {
    let nparents: usize = commit_fixture_mut(world).parents.len();
    let entry: CombinedDiffEntry = fixture_combined_entry(&path, nparents);
    commit_fixture_mut(world).combined.push(entry);
}

#[when(regex = r#"^I assemble the commit view for "([^"]*)"$"#)]
fn assemble_commit_view(world: &mut UsecaseWorld, rev: String) {
    let repo: FakeRepository = fake_repo(world);
    world.commit_result = Some(assemble_commit(&repo, Some(&rev), None));
}

#[when(regex = r#"^I assemble the commit view for "([^"]*)" against parent "([^"]*)"$"#)]
fn assemble_commit_view_against(world: &mut UsecaseWorld, rev: String, parent: String) {
    let repo: FakeRepository = fake_repo(world);
    // The fixture's parents are `fake_oid(label)`; resolve passes a full oid
    // through to itself, the way `git rev-parse <oid>` does.
    let explicit: ObjectId = fake_oid(&parent);
    world.commit_result = Some(assemble_commit(&repo, Some(&rev), Some(explicit.as_str())));
}

/// A single-file create patch for `path`: the `FilePatch` a `--root` (or
/// new-file) diff carries, an absent from-side and a `100644` to-side. The
/// content is empty text — the diff header (`diff --git …`) renders regardless,
/// which is all the structural patch/patches scenarios assert.
fn created_file(path: &str) -> FilePatch {
    let to_oid: ObjectId = fake_oid(&format!("to-{path}"));
    FilePatch::new(
        ChangeStatus::added(),
        FileMode::from_octal("000000").expect("a valid absent mode"),
        FileMode::from_octal("100644").expect("a valid file mode"),
        to_oid.null_like(),
        to_oid,
        path.to_owned(),
        path.to_owned(),
        FileContent::Text(Vec::new()),
    )
}

#[given(regex = r#"^the commit diff creates "([^"]*)"$"#)]
fn given_commitdiff_creates(world: &mut UsecaseWorld, path: String) {
    commit_fixture_mut(world).patch = Patch::new(vec![created_file(&path)]);
}

/// A single-file binary modify patch for `path`: a `FilePatch` whose content git
/// treats as binary, with distinct from/to blob ids whose `old`/`new` byte sizes
/// the diffstat reads back through [`Repository::object_size`].
fn modified_binary_file(path: &str) -> FilePatch {
    FilePatch::new(
        ChangeStatus::parse("M").expect("M is a valid status"),
        regular_file_mode(),
        regular_file_mode(),
        fake_oid(&format!("binfrom-{path}")),
        fake_oid(&format!("binto-{path}")),
        path.to_owned(),
        path.to_owned(),
        FileContent::Binary,
    )
}

#[given(regex = r#"^the commit diff modifies binary "([^"]*)" from (\d+) to (\d+) bytes$"#)]
fn given_commitdiff_modifies_binary(world: &mut UsecaseWorld, path: String, old: u64, new: u64) {
    let file: FilePatch = modified_binary_file(&path);
    let from_oid: ObjectId = file.from_oid().clone();
    let to_oid: ObjectId = file.to_oid().clone();
    let fixture: &mut FakeCommitFixture = commit_fixture_mut(world);
    fixture.patch = Patch::new(vec![file]);
    fixture.binary_sizes.push((from_oid, old));
    fixture.binary_sizes.push((to_oid, new));
}

#[when(regex = r#"^I assemble the commitdiff text for "([^"]*)"$"#)]
fn assemble_commitdiff_text(world: &mut UsecaseWorld, rev: String) {
    let repo: FakeRepository = fake_repo(world);
    world.commitdiff_result = Some(assemble_commit_diff(&repo, Some(&rev), None, None));
}

#[then(regex = r#"^the commitdiff text contains "(.*)"$"#)]
fn then_commitdiff_text_contains(world: &mut UsecaseWorld, expected: String) {
    let text: &str = match world
        .commitdiff_result
        .as_ref()
        .expect("assemble the commitdiff text first")
    {
        Ok(text) => text,
        Err(error) => panic!("expected commitdiff text, got error: {error:?}"),
    };
    assert!(
        text.contains(&expected),
        "expected commitdiff text to contain {expected:?}, got:\n{text}"
    );
}

#[then(regex = r#"^assembling the commitdiff text fails with "(.*)"$"#)]
fn then_commitdiff_text_fails(world: &mut UsecaseWorld, expected: String) {
    match world
        .commitdiff_result
        .as_ref()
        .expect("assemble the commitdiff text first")
    {
        Ok(text) => panic!("expected a failure, got commitdiff text:\n{text}"),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

/// The self link the commitdiff_plain scenarios render against; its exact bytes
/// do not matter to these structural assertions, only that the `X-Git-Url` line
/// carries it.
const PLAIN_SELF_URL: &str = "http://localhost?p=r.git;a=commitdiff_plain;h=HEAD";

#[when(regex = r#"^I assemble the commitdiff_plain for "([^"]*)"$"#)]
fn assemble_commitdiff_plain_step(world: &mut UsecaseWorld, rev: String) {
    let repo: FakeRepository = fake_repo(world);
    world.commitdiff_plain_result = Some(assemble_commitdiff_plain(&repo, Some(&rev), None));
}

/// Renders the assembled plain body (asserting the use case succeeded).
fn commitdiff_plain_body(world: &UsecaseWorld) -> String {
    match world
        .commitdiff_plain_result
        .as_ref()
        .expect("assemble the commitdiff_plain first")
    {
        Ok(plain) => plain.render(PLAIN_SELF_URL),
        Err(error) => panic!("expected a commitdiff_plain, got error: {error:?}"),
    }
}

#[then(regex = r#"^the commitdiff_plain body contains "(.*)"$"#)]
fn then_commitdiff_plain_contains(world: &mut UsecaseWorld, expected: String) {
    let body: String = commitdiff_plain_body(world);
    assert!(
        body.contains(&expected),
        "expected commitdiff_plain body to contain {expected:?}, got:\n{body}"
    );
}

#[then(regex = r#"^the commitdiff_plain body does not contain "(.*)"$"#)]
fn then_commitdiff_plain_not_contains(world: &mut UsecaseWorld, unexpected: String) {
    let body: String = commitdiff_plain_body(world);
    assert!(
        !body.contains(&unexpected),
        "expected commitdiff_plain body not to contain {unexpected:?}, got:\n{body}"
    );
}

#[then(regex = r#"^the commitdiff_plain body has a line "(.*)"$"#)]
fn then_commitdiff_plain_has_line(world: &mut UsecaseWorld, expected: String) {
    let body: String = commitdiff_plain_body(world);
    assert!(
        body.lines().any(|line: &str| line == expected),
        "expected commitdiff_plain body to have the exact line {expected:?}, got:\n{body}"
    );
}

#[then(regex = r#"^assembling the commitdiff_plain fails with "(.*)"$"#)]
fn then_commitdiff_plain_fails(world: &mut UsecaseWorld, expected: String) {
    match world
        .commitdiff_plain_result
        .as_ref()
        .expect("assemble the commitdiff_plain first")
    {
        Ok(plain) => panic!(
            "expected a failure, got body:\n{}",
            plain.render(PLAIN_SELF_URL)
        ),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

/// The fixture commit's id, the bare line `git diff-tree` prints for a root.
fn fixture_commit_id(world: &UsecaseWorld) -> String {
    world
        .commit_fixture
        .as_ref()
        .expect("declare a commit fixture first")
        .id
        .as_str()
        .to_owned()
}

#[then("the commitdiff_plain body carries the commit id on its own line")]
fn then_commitdiff_plain_has_id_line(world: &mut UsecaseWorld) {
    let id: String = fixture_commit_id(world);
    let body: String = commitdiff_plain_body(world);
    assert!(
        body.lines().any(|line: &str| line == id),
        "expected commitdiff_plain body to carry the commit id {id:?} on its own line, got:\n{body}"
    );
}

#[then("the commitdiff_plain body has no bare commit-id line")]
fn then_commitdiff_plain_no_id_line(world: &mut UsecaseWorld) {
    let id: String = fixture_commit_id(world);
    let body: String = commitdiff_plain_body(world);
    assert!(
        !body.lines().any(|line: &str| line == id),
        "expected commitdiff_plain body to have no bare commit-id line {id:?}, got:\n{body}"
    );
}

// --- patch (format-patch single) ---------------------------------------------

#[given(regex = r#"^the commit has subject "([^"]*)" and body "([^"]*)"$"#)]
fn given_commit_subject_and_body(world: &mut UsecaseWorld, subject: String, body: String) {
    commit_fixture_mut(world).message = format!("{subject}\n\n{body}\n");
}

#[when(regex = r#"^I assemble the patch for "([^"]*)" with limit (\d+) and version "([^"]*)"$"#)]
fn assemble_patch_step(world: &mut UsecaseWorld, rev: String, limit: usize, version: String) {
    let repo: FakeRepository = fake_repo(world);
    world.patch_result = Some(assemble_patch(&repo, Some(&rev), limit, &version));
}

/// Renders the assembled patch stream (asserting the use case succeeded).
fn patch_stream(world: &UsecaseWorld) -> String {
    match world
        .patch_result
        .as_ref()
        .expect("assemble the patch first")
    {
        Ok(stream) => stream.render(),
        Err(error) => panic!("expected a patch stream, got error: {error:?}"),
    }
}

#[then(regex = r#"^the patch stream contains "(.*)"$"#)]
fn then_patch_contains(world: &mut UsecaseWorld, expected: String) {
    let stream: String = patch_stream(world);
    assert!(
        stream.contains(&expected),
        "expected patch stream to contain {expected:?}, got:\n{stream}"
    );
}

#[then(regex = r#"^the patch stream does not contain "(.*)"$"#)]
fn then_patch_not_contains(world: &mut UsecaseWorld, unexpected: String) {
    let stream: String = patch_stream(world);
    assert!(
        !stream.contains(&unexpected),
        "expected patch stream not to contain {unexpected:?}, got:\n{stream}"
    );
}

#[then(regex = r#"^the patch stream has a line "(.*)"$"#)]
fn then_patch_has_line(world: &mut UsecaseWorld, expected: String) {
    let stream: String = patch_stream(world);
    assert!(
        stream.lines().any(|line: &str| line == expected),
        "expected patch stream to have the exact line {expected:?}, got:\n{stream}"
    );
}

#[then(regex = r#"^assembling the patch fails with "(.*)"$"#)]
fn then_patch_fails(world: &mut UsecaseWorld, expected: String) {
    match world
        .patch_result
        .as_ref()
        .expect("assemble the patch first")
    {
        Ok(stream) => panic!("expected a failure, got stream:\n{}", stream.render()),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

// --- patches (format-patch range) --------------------------------------------

/// The patch series fixture, asserting one was declared.
fn patch_series_mut(world: &mut UsecaseWorld) -> &mut FakePatchSeries {
    world
        .patch_series
        .as_mut()
        .expect("declare a patch series first")
}

#[given(regex = r#"^a patch series authored by "([^"]*)"$"#)]
fn given_patch_series(world: &mut UsecaseWorld, ident: String) {
    let author: Signature = Signature::parse(&ident).expect("a valid author ident");
    world.patch_series = Some(FakePatchSeries {
        author,
        commits: Vec::new(),
        tip_is_commit: true,
    });
}

#[given(regex = r#"^a patch commit "([^"]*)" with subject "([^"]*)" creating "([^"]*)"$"#)]
fn given_patch_commit(world: &mut UsecaseWorld, label: String, subject: String, path: String) {
    let series: &mut FakePatchSeries = patch_series_mut(world);
    // The parent (diff base) is the previous, older commit — the series is
    // declared oldest-first, so it is whatever was pushed last.
    let parent: Option<ObjectId> = series
        .commits
        .last()
        .map(|c: &FakePatchCommit| c.id.clone());
    series.commits.push(FakePatchCommit {
        id: fake_oid(&format!("patches-{label}")),
        parent,
        subject,
        patch: Patch::new(vec![created_file(&path)]),
    });
}

#[given("the patch series tip is not a commit")]
fn given_patch_series_non_commit(world: &mut UsecaseWorld) {
    patch_series_mut(world).tip_is_commit = false;
}

#[when(regex = r#"^I assemble the patches for "([^"]*)" with limit (\d+) and version "([^"]*)"$"#)]
fn assemble_patches_step(world: &mut UsecaseWorld, rev: String, limit: usize, version: String) {
    let repo: FakeRepository = fake_repo(world);
    world.patches_result = Some(assemble_patches(&repo, Some(&rev), limit, &version));
}

/// Renders the assembled patches stream (asserting the use case succeeded).
fn patches_stream(world: &UsecaseWorld) -> String {
    match world
        .patches_result
        .as_ref()
        .expect("assemble the patches first")
    {
        Ok(stream) => stream.render(),
        Err(error) => panic!("expected a patches stream, got error: {error:?}"),
    }
}

#[then(regex = r#"^the patches stream has a line "(.*)"$"#)]
fn then_patches_has_line(world: &mut UsecaseWorld, expected: String) {
    let stream: String = patches_stream(world);
    assert!(
        stream.lines().any(|line: &str| line == expected),
        "expected patches stream to have the exact line {expected:?}, got:\n{stream}"
    );
}

#[then(regex = r#"^the patches stream does not contain "(.*)"$"#)]
fn then_patches_not_contains(world: &mut UsecaseWorld, unexpected: String) {
    let stream: String = patches_stream(world);
    assert!(
        !stream.contains(&unexpected),
        "expected patches stream not to contain {unexpected:?}, got:\n{stream}"
    );
}

#[then(regex = r#"^"(.*)" comes before "(.*)" in the patches stream$"#)]
fn then_patches_order(world: &mut UsecaseWorld, first: String, second: String) {
    let stream: String = patches_stream(world);
    let at_first: Option<usize> = stream.find(&first);
    let at_second: Option<usize> = stream.find(&second);
    assert!(
        matches!((at_first, at_second), (Some(a), Some(b)) if a < b),
        "expected {first:?} before {second:?} in patches stream, got:\n{stream}"
    );
}

#[then(regex = r#"^assembling the patches fails with "(.*)"$"#)]
fn then_patches_fails(world: &mut UsecaseWorld, expected: String) {
    match world
        .patches_result
        .as_ref()
        .expect("assemble the patches first")
    {
        Ok(stream) => panic!("expected a failure, got stream:\n{}", stream.render()),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

// --- blobdiff_plain ----------------------------------------------------------

/// The self link the blobdiff_plain scenarios render against; its exact bytes do
/// not matter to these structural assertions, only that the body carries it.
const BLOBDIFF_SELF_URL: &str = "http://localhost?p=r.git;a=blobdiff_plain;hb=HEAD;hpb=base;f=x";

/// A modified file patch for `path` with no hunks — enough to carry the
/// `diff --git` and `index` headers the selection asserts on, without a diff
/// algorithm.
fn modified_file_patch(path: &str) -> FilePatch {
    let from_oid: ObjectId = fake_oid(&format!("from-{path}"));
    let to_oid: ObjectId = fake_oid(&format!("to-{path}"));
    let mode: FileMode = FileMode::from_octal("100644").expect("a valid file mode");
    FilePatch::new(
        ChangeStatus::from_modification(mode, mode),
        mode,
        mode,
        from_oid,
        to_oid,
        path.to_owned(),
        path.to_owned(),
        FileContent::Text(Vec::new()),
    )
}

#[given(regex = r#"^the diff modifies "([^"]*)"$"#)]
fn given_diff_modifies(world: &mut UsecaseWorld, path: String) {
    world.blobdiff_files.push(modified_file_patch(&path));
}

#[when(
    regex = r#"^I assemble the blobdiff_plain of "([^"]*)" with base "([^"]*)" and parent base "([^"]*)"$"#
)]
fn assemble_blobdiff_plain_step(world: &mut UsecaseWorld, file: String, hb: String, hpb: String) {
    let files: Vec<FilePatch> = world.blobdiff_files.clone();
    commit_fixture_mut(world).patch = Patch::new(files);
    let repo: FakeRepository = fake_repo(world);
    world.blobdiff_plain_result = Some(assemble_blobdiff_plain(&repo, &hb, &hpb, &file));
}

/// Renders the assembled blobdiff_plain body (asserting the use case succeeded).
fn blobdiff_plain_body(world: &UsecaseWorld) -> String {
    match world
        .blobdiff_plain_result
        .as_ref()
        .expect("assemble the blobdiff_plain first")
    {
        Ok(plain) => plain.render(BLOBDIFF_SELF_URL),
        Err(error) => panic!("expected a blobdiff_plain, got error: {error:?}"),
    }
}

#[then(regex = r#"^the blobdiff_plain body contains "(.*)"$"#)]
fn then_blobdiff_plain_contains(world: &mut UsecaseWorld, expected: String) {
    let body: String = blobdiff_plain_body(world);
    assert!(
        body.contains(&expected),
        "expected blobdiff_plain body to contain {expected:?}, got:\n{body}"
    );
}

#[then(regex = r#"^the blobdiff_plain body does not contain "(.*)"$"#)]
fn then_blobdiff_plain_not_contains(world: &mut UsecaseWorld, unexpected: String) {
    let body: String = blobdiff_plain_body(world);
    assert!(
        !body.contains(&unexpected),
        "expected blobdiff_plain body not to contain {unexpected:?}, got:\n{body}"
    );
}

#[then(regex = r#"^assembling the blobdiff_plain fails with "(.*)"$"#)]
fn then_blobdiff_plain_fails(world: &mut UsecaseWorld, expected: String) {
    match world
        .blobdiff_plain_result
        .as_ref()
        .expect("assemble the blobdiff_plain first")
    {
        Ok(plain) => panic!(
            "expected a failure, got body:\n{}",
            plain.render(BLOBDIFF_SELF_URL)
        ),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

// --- blobdiff (html resolution) ----------------------------------------------

/// The fixed new-side id two "twin" added files share, so a by-id resolution is
/// ambiguous; both [`given_diff_twin_adds`] and the shared-id When use it.
fn twin_shared_oid() -> ObjectId {
    fake_oid("twin-shared")
}

/// A renamed file patch from `from` to `to` with no hunks — enough to carry the
/// from/to paths the resolution reports, without a diff algorithm.
fn renamed_file_patch(from: &str, to: &str) -> FilePatch {
    let mode: FileMode = FileMode::from_octal("100644").expect("a valid file mode");
    FilePatch::new(
        ChangeStatus::renamed(100),
        mode,
        mode,
        fake_oid(&format!("from-{from}")),
        fake_oid(&format!("to-{to}")),
        from.to_owned(),
        to.to_owned(),
        FileContent::Text(Vec::new()),
    )
}

/// An added file patch for `path` carrying `to_oid` as its new-side id.
fn added_file_patch_with_oid(path: &str, to_oid: ObjectId) -> FilePatch {
    let mode: FileMode = FileMode::from_octal("100644").expect("a valid file mode");
    let absent: FileMode = FileMode::from_octal("000000").expect("a valid absent mode");
    FilePatch::new(
        ChangeStatus::added(),
        absent,
        mode,
        to_oid.null_like(),
        to_oid,
        path.to_owned(),
        path.to_owned(),
        FileContent::Text(Vec::new()),
    )
}

#[given(regex = r#"^the diff renames "([^"]*)" to "([^"]*)"$"#)]
fn given_diff_renames(world: &mut UsecaseWorld, from: String, to: String) {
    world.blobdiff_files.push(renamed_file_patch(&from, &to));
}

#[given(regex = r#"^the diff adds "([^"]*)" and "([^"]*)" sharing new-side content$"#)]
fn given_diff_twin_adds(world: &mut UsecaseWorld, first: String, second: String) {
    world
        .blobdiff_files
        .push(added_file_patch_with_oid(&first, twin_shared_oid()));
    world
        .blobdiff_files
        .push(added_file_patch_with_oid(&second, twin_shared_oid()));
}

/// Folds the accumulated file patches into the fixture's whole-tree patch and
/// runs the blobdiff resolution for the given selector.
fn run_blobdiff(
    world: &mut UsecaseWorld,
    hb: &str,
    hpb: &str,
    file_name: Option<&str>,
    hash: Option<&str>,
) {
    let files: Vec<FilePatch> = world.blobdiff_files.clone();
    commit_fixture_mut(world).patch = Patch::new(files);
    let repo: FakeRepository = fake_repo(world);
    world.blobdiff_result = Some(assemble_blobdiff(&repo, hb, hpb, file_name, hash));
}

/// The new-side id of the accumulated file patch whose new-side path is `path`.
fn blobdiff_to_oid(world: &UsecaseWorld, path: &str) -> String {
    world
        .blobdiff_files
        .iter()
        .find(|file: &&FilePatch| file.to_path() == path)
        .expect("the diff touches that path")
        .to_oid()
        .as_str()
        .to_owned()
}

#[when(
    regex = r#"^I assemble the blobdiff of "([^"]*)" with base "([^"]*)" and parent base "([^"]*)"$"#
)]
fn assemble_blobdiff_by_path(world: &mut UsecaseWorld, file: String, hb: String, hpb: String) {
    run_blobdiff(world, &hb, &hpb, Some(&file), None);
}

#[when(
    regex = r#"^I assemble the blobdiff by the new-side id of "([^"]*)" with base "([^"]*)" and parent base "([^"]*)"$"#
)]
fn assemble_blobdiff_by_id(world: &mut UsecaseWorld, file: String, hb: String, hpb: String) {
    let id: String = blobdiff_to_oid(world, &file);
    run_blobdiff(world, &hb, &hpb, None, Some(&id));
}

#[when(
    regex = r#"^I assemble the blobdiff with neither file nor hash, base "([^"]*)" and parent base "([^"]*)"$"#
)]
fn assemble_blobdiff_no_selector(world: &mut UsecaseWorld, hb: String, hpb: String) {
    run_blobdiff(world, &hb, &hpb, None, None);
}

#[when(
    regex = r#"^I assemble the blobdiff by the shared new-side id with base "([^"]*)" and parent base "([^"]*)"$"#
)]
fn assemble_blobdiff_shared_id(world: &mut UsecaseWorld, hb: String, hpb: String) {
    let id: String = twin_shared_oid().as_str().to_owned();
    run_blobdiff(world, &hb, &hpb, None, Some(&id));
}

/// The assembled blobdiff view (asserting the resolution succeeded).
fn blobdiff_view(world: &UsecaseWorld) -> &BlobdiffView {
    match world
        .blobdiff_result
        .as_ref()
        .expect("assemble the blobdiff first")
    {
        Ok(view) => view,
        Err(error) => panic!("expected a blobdiff view, got error: {error:?}"),
    }
}

#[then(regex = r#"^the blobdiff file name is "([^"]*)"$"#)]
fn then_blobdiff_file_name(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(blobdiff_view(world).file_name(), expected);
}

#[then("the blobdiff has no file parent")]
fn then_blobdiff_no_file_parent(world: &mut UsecaseWorld) {
    assert_eq!(blobdiff_view(world).file_parent(), None);
}

#[then(regex = r#"^the blobdiff file parent is "([^"]*)"$"#)]
fn then_blobdiff_file_parent(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(blobdiff_view(world).file_parent(), Some(expected.as_str()));
}

#[then(regex = r#"^the blobdiff title is "(.*)"$"#)]
fn then_blobdiff_title(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(blobdiff_view(world).title(), expected);
}

/// gitweb's degenerate `git_blobdiff` title for a non-commit base:
/// `esc_html("$hash vs $hash_parent")`, the resolved new-side then old-side blob
/// ids. Built here from the same patch the resolution selected, so it pins the
/// format and the new-before-old order without restating production's bytes.
#[then(regex = r#"^the blobdiff title pairs the new-side and old-side ids of "([^"]*)"$"#)]
fn then_blobdiff_degenerate_title(world: &mut UsecaseWorld, path: String) {
    let file: &FilePatch = world
        .blobdiff_files
        .iter()
        .find(|file: &&FilePatch| file.to_path() == path)
        .expect("the diff touches that path");
    let expected: String = format!("{} vs {}", file.to_oid().as_str(), file.from_oid().as_str());
    assert_eq!(blobdiff_view(world).title(), expected);
}

#[then(regex = r#"^assembling the blobdiff fails with "(.*)"$"#)]
fn then_blobdiff_fails(world: &mut UsecaseWorld, expected: String) {
    match world
        .blobdiff_result
        .as_ref()
        .expect("assemble the blobdiff first")
    {
        Ok(view) => panic!("expected a failure, got view: {view:?}"),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

/// The assembled commit view (asserting the use case succeeded).
fn commit_view(world: &UsecaseWorld) -> &CommitView {
    match world
        .commit_result
        .as_ref()
        .expect("assemble the commit first")
    {
        Ok(view) => view,
        Err(error) => panic!("expected a commit view, got error: {error:?}"),
    }
}

/// The ordinary changed-files rows (asserting the diff is an ordinary one).
fn ordinary_changes(world: &UsecaseWorld) -> &[gitweb_domain::usecase::commit::OrdinaryChange] {
    match commit_view(world).changes() {
        ChangedFiles::Ordinary { changes, .. } => changes,
        ChangedFiles::Combined(_) => panic!("expected an ordinary diff, got a combined one"),
    }
}

/// The base the ordinary changed-files were diffed against (asserting the diff is
/// an ordinary one).
fn ordinary_base(world: &UsecaseWorld) -> Option<&ObjectId> {
    match commit_view(world).changes() {
        ChangedFiles::Ordinary { base, .. } => base.as_ref(),
        ChangedFiles::Combined(_) => panic!("expected an ordinary diff, got a combined one"),
    }
}

/// The combined changed-files rows (asserting the diff is a combined one).
fn combined_changes(world: &UsecaseWorld) -> &[gitweb_domain::usecase::commit::CombinedChange] {
    match commit_view(world).changes() {
        ChangedFiles::Combined(rows) => rows,
        ChangedFiles::Ordinary { .. } => panic!("expected a combined diff, got an ordinary one"),
    }
}

#[then(regex = r#"^the commit author name is "(.*)"$"#)]
fn commit_author_name_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(commit_view(world).author().name(), expected);
}

#[then(regex = r#"^the commit author email is "(.*)"$"#)]
fn commit_author_email_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(
        commit_view(world).author().email().expect("an email"),
        expected
    );
}

#[then(regex = r#"^the commit committer name is "(.*)"$"#)]
fn commit_committer_name_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(commit_view(world).committer().name(), expected);
}

#[then(regex = r#"^the commit title is "(.*)"$"#)]
fn commit_title_is(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(commit_view(world).title(), expected);
}

#[then(regex = r"^the commit has (\d+) parents?$")]
fn commit_has_parents(world: &mut UsecaseWorld, expected: usize) {
    assert_eq!(commit_view(world).parents().len(), expected);
}

#[then("the commit is a merge")]
fn commit_is_a_merge(world: &mut UsecaseWorld) {
    assert!(commit_view(world).is_merge());
}

#[then("the commit is not a merge")]
fn commit_is_not_a_merge(world: &mut UsecaseWorld) {
    assert!(!commit_view(world).is_merge());
}

#[then("the changed files are ordinary")]
fn changed_files_are_ordinary(world: &mut UsecaseWorld) {
    assert!(matches!(
        commit_view(world).changes(),
        ChangedFiles::Ordinary { .. }
    ));
}

#[then(regex = r#"^the ordinary base is "([^"]*)"$"#)]
fn the_ordinary_base_is(world: &mut UsecaseWorld, label: String) {
    assert_eq!(ordinary_base(world), Some(&fake_oid(&label)));
}

#[then("the changed files are combined")]
fn changed_files_are_combined(world: &mut UsecaseWorld) {
    assert!(matches!(
        commit_view(world).changes(),
        ChangedFiles::Combined(_)
    ));
}

#[then(regex = r"^there are (\d+) changed files$")]
fn there_are_changed_files(world: &mut UsecaseWorld, expected: usize) {
    assert_eq!(commit_view(world).changes().len(), expected);
}

#[then(regex = r#"^ordinary change (\d+) is "(.*)"$"#)]
fn ordinary_change_is(world: &mut UsecaseWorld, index: usize, expected: String) {
    assert_eq!(ordinary_changes(world)[index - 1].to_path(), expected);
}

#[then(regex = r"^combined change (\d+) has (\d+) parent sides$")]
fn combined_change_parent_sides(world: &mut UsecaseWorld, index: usize, expected: usize) {
    assert_eq!(combined_changes(world)[index - 1].parents().len(), expected);
}

#[then(regex = r#"^assembling the commit fails with "(.*)"$"#)]
fn assembling_commit_fails_with(world: &mut UsecaseWorld, expected: String) {
    match world
        .commit_result
        .as_ref()
        .expect("assemble the commit first")
    {
        Ok(view) => panic!("expected a failure, got a commit view: {view:?}"),
        Err(error) => assert_eq!(error.message(), expected),
    }
}

// --- snapshot: assembling the archive over the port --------------------------

/// Builds the committer identity a snapshot commit is dated by.
fn snapshot_committer(epoch: i64, tz: &str) -> Signature {
    Signature::parse(&format!("Ada <ada@example.com> {epoch} {tz}")).expect("a valid identity line")
}

#[given(regex = r#"^a snapshot commit "([0-9a-f]+)" dated (\d+) ([+-]\d{4})$"#)]
fn given_snapshot_commit(world: &mut UsecaseWorld, hex: String, epoch: i64, tz: String) {
    world.snapshot = Some(FakeSnapshot {
        id: ObjectId::parse(&hex).expect("a valid object id"),
        rev: hex,
        kind: ObjectKind::Commit,
        committer: Some(snapshot_committer(epoch, &tz)),
    });
}

#[given(regex = r#"^a snapshot tree "([0-9a-f]+)"$"#)]
fn given_snapshot_tree(world: &mut UsecaseWorld, hex: String) {
    world.snapshot = Some(FakeSnapshot {
        id: ObjectId::parse(&hex).expect("a valid object id"),
        rev: hex,
        kind: ObjectKind::Tree,
        committer: None,
    });
}

#[given(regex = r#"^a snapshot blob "([0-9a-f]+)"$"#)]
fn given_snapshot_blob(world: &mut UsecaseWorld, hex: String) {
    world.snapshot = Some(FakeSnapshot {
        id: ObjectId::parse(&hex).expect("a valid object id"),
        rev: hex,
        kind: ObjectKind::Blob,
        committer: None,
    });
}

#[given(regex = r#"^the snapshot project is "([^"]*)"$"#)]
fn given_snapshot_project(world: &mut UsecaseWorld, project: String) {
    world.snapshot_project = project;
}

#[given(regex = r#"^the site enables snapshot formats "([^"]*)"$"#)]
fn given_site_snapshot_formats(world: &mut UsecaseWorld, list: String) {
    world.snapshot_configured = if list.trim().is_empty() {
        Vec::new()
    } else {
        list.split(',')
            .map(|token: &str| token.trim().to_owned())
            .collect()
    };
}

/// Drives the snapshot use case with the world's project, configured formats, and
/// the given hash / requested format.
fn assemble_world_snapshot(world: &mut UsecaseWorld, hash: Option<&str>, requested: Option<&str>) {
    let repo: FakeRepository = fake_repo(world);
    world.snapshot_result = Some(assemble_snapshot(
        &repo,
        &world.snapshot_project,
        &world.snapshot_configured,
        requested,
        hash,
        &["heads"],
    ));
}

#[when(regex = r#"^I assemble the snapshot of "([^"]*)" requesting "([^"]*)"$"#)]
fn when_assemble_snapshot_requesting(world: &mut UsecaseWorld, hash: String, requested: String) {
    assemble_world_snapshot(world, Some(&hash), Some(&requested));
}

#[when(regex = r#"^I assemble the snapshot of "([^"]*)" with no requested format$"#)]
fn when_assemble_snapshot_default(world: &mut UsecaseWorld, hash: String) {
    assemble_world_snapshot(world, Some(&hash), None);
}

#[when("I assemble the snapshot with no hash")]
fn when_assemble_snapshot_no_hash(world: &mut UsecaseWorld) {
    assemble_world_snapshot(world, None, Some("tgz"));
}

/// The assembled snapshot view, or a panic if the scenario produced an error.
fn snapshot_view(world: &UsecaseWorld) -> &SnapshotView {
    world
        .snapshot_result
        .as_ref()
        .expect("assemble the snapshot first")
        .as_ref()
        .expect("snapshot assembly succeeded")
}

/// The snapshot failure, or a panic if the scenario produced a success.
fn snapshot_error(world: &UsecaseWorld) -> &DomainError {
    match world
        .snapshot_result
        .as_ref()
        .expect("assemble the snapshot first")
    {
        Ok(_) => panic!("expected snapshot assembly to fail"),
        Err(error) => error,
    }
}

#[then(regex = r#"^the snapshot content type is "([^"]*)"$"#)]
fn then_snapshot_content_type(world: &mut UsecaseWorld, expected: String) {
    assert_eq!(snapshot_view(world).content_type(), expected);
}

#[then(regex = r#"^the snapshot is offered inline as "([^"]*)"$"#)]
fn then_snapshot_inline_as(world: &mut UsecaseWorld, file_name: String) {
    assert_eq!(
        snapshot_view(world).content_disposition(),
        format!(r#"inline; filename="{file_name}""#)
    );
}

#[then("the snapshot is dated")]
fn then_snapshot_dated(world: &mut UsecaseWorld) {
    assert!(snapshot_view(world).last_modified().is_some());
}

#[then("the snapshot has no date")]
fn then_snapshot_undated(world: &mut UsecaseWorld) {
    assert!(snapshot_view(world).last_modified().is_none());
}

#[then(regex = r#"^the snapshot archive is "([^"]*)"$"#)]
fn then_snapshot_archive_is(world: &mut UsecaseWorld, expected: String) {
    let bytes: &[u8] = snapshot_view(world).bytes();
    assert_eq!(String::from_utf8_lossy(bytes), expected);
}

#[then("assembling the snapshot fails as invalid")]
fn then_snapshot_fails_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(snapshot_error(world), DomainError::Invalid(_)));
}

#[then("assembling the snapshot fails as forbidden")]
fn then_snapshot_fails_forbidden(world: &mut UsecaseWorld) {
    assert!(matches!(snapshot_error(world), DomainError::Forbidden(_)));
}

#[then("assembling the snapshot fails as not found")]
fn then_snapshot_fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(snapshot_error(world), DomainError::NotFound(_)));
}

#[tokio::main]
async fn main() {
    UsecaseWorld::run("features/usecase").await;
}
