//! The `commit` use case: gitweb's `git_commit`, minus the HTML.
//!
//! `git_commit` resolves a commit-ish (defaulting `$hash ||= $hash_base ||
//! "HEAD"`), reads the commit, and renders its metadata (authorship, the tree
//! and parent links), its message, and the changed-files table
//! (`git_difftree_body`). The diff it shows is the commit against the base the
//! [`changed_files_base`] rule picks: the chosen explicit parent (gitweb's
//! `$hash_parent`, only the `commitdiff` view passes one), else the combined
//! `-c` diff for a merge, else an ordinary single-parent diff — or the empty-tree
//! diff for a root commit.
//!
//! This is the orchestration over the [`Repository`] port: resolve, classify (a
//! non-commit is gitweb's `die_error(404, "Unknown commit object")`), read the
//! commit, resolve any explicit parent, then build the changed-files set per the
//! base rule — [`Repository::combined_diff`] for a merge with no explicit parent,
//! else [`Repository::diff`] against the chosen base. Each ordinary row carries
//! its [`FileChangeNote`]; the per-file links and the parent/tree URLs are the
//! boundary's job. Authorship dates are the commit's own absolute timestamps
//! ([`Timestamp`]), so this use case needs no clock.

use crate::error::DomainError;
use crate::model::change::ChangeStatus;
use crate::model::commit::Commit;
use crate::model::commitdiff::{ChangedFilesBase, changed_files_base};
use crate::model::diff::{CombinedDiff, CombinedDiffEntry, Diff, DiffEntry};
use crate::model::file_change::FileChangeNote;
use crate::model::file_mode::FileMode;
use crate::model::message_body::{LogLine, log_lines};
use crate::model::object_id::ObjectId;
use crate::model::object_kind::ObjectKind;
use crate::model::signature::Signature;
use crate::model::timestamp::Timestamp;
use crate::port::repository::{RenameDetection, Repository};

/// One authorship line on the commit view (author or committer): who, their
/// email when the ident carries one, and the absolute date — gitweb's
/// `git_print_authorship_rows` / `format_timestamp_html`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorLine {
    name: String,
    email: Option<String>,
    timestamp: Timestamp,
}

impl AuthorLine {
    /// The identity's display name.
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The identity's email, if the ident carries one.
    #[must_use]
    pub fn email(&self) -> Option<&str> {
        self.email.as_deref()
    }

    /// The absolute date this identity acted at.
    #[must_use]
    pub fn timestamp(&self) -> &Timestamp {
        &self.timestamp
    }
}

/// One changed path in an ordinary (single-parent or root) diff, with the facts
/// the changed-files row needs: its status, the from/to ids, modes and paths,
/// and the derived [`FileChangeNote`] (absent for a pure content modification).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OrdinaryChange {
    status: ChangeStatus,
    from_oid: ObjectId,
    to_oid: ObjectId,
    from_path: String,
    to_path: String,
    from_mode: FileMode,
    to_mode: FileMode,
    note: Option<FileChangeNote>,
}

impl OrdinaryChange {
    /// The change status (gitweb's diff-tree status letter + similarity).
    #[must_use]
    pub fn status(&self) -> ChangeStatus {
        self.status
    }

    /// The from-side object id (all-zero when the path is being created).
    #[must_use]
    pub fn from_oid(&self) -> &ObjectId {
        &self.from_oid
    }

    /// The to-side object id (all-zero when the path is being deleted).
    #[must_use]
    pub fn to_oid(&self) -> &ObjectId {
        &self.to_oid
    }

    /// The from-side path (differs from the to-side only on rename/copy).
    #[must_use]
    pub fn from_path(&self) -> &str {
        &self.from_path
    }

    /// The to-side path.
    #[must_use]
    pub fn to_path(&self) -> &str {
        &self.to_path
    }

    /// The from-side mode.
    #[must_use]
    pub fn from_mode(&self) -> FileMode {
        self.from_mode
    }

    /// The to-side mode.
    #[must_use]
    pub fn to_mode(&self) -> FileMode {
        self.to_mode
    }

    /// The status note shown in the row, absent for a pure content modification.
    #[must_use]
    pub fn note(&self) -> Option<&FileChangeNote> {
        self.note.as_ref()
    }
}

/// One parent's side of a combined (merge) changed-files row: how the path
/// differs from that parent and the parent-side id, mirroring gitweb's per-parent
/// columns. The per-parent path is the result path (a combined `-c` diff does not
/// track per-parent renames).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedParentChange {
    status: ChangeStatus,
    from_oid: ObjectId,
}

impl CombinedParentChange {
    /// The status of the path relative to this parent.
    #[must_use]
    pub fn status(&self) -> ChangeStatus {
        self.status
    }

    /// This parent's from-side id (all-zero when the path is new to it).
    #[must_use]
    pub fn from_oid(&self) -> &ObjectId {
        &self.from_oid
    }
}

/// One changed path in a combined (merge) diff: the per-parent sides and the
/// single merge result, plus gitweb's `is_deleted` / `has_history` /
/// `not_deleted` row predicates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CombinedChange {
    parents: Vec<CombinedParentChange>,
    to_oid: ObjectId,
    to_mode: FileMode,
    to_path: String,
    is_deleted: bool,
    has_history: bool,
    not_deleted: bool,
}

impl CombinedChange {
    /// The per-parent sides, one per parent of the merge.
    #[must_use]
    pub fn parents(&self) -> &[CombinedParentChange] {
        &self.parents
    }

    /// The merge-result object id (all-zero when deleted in the merge).
    #[must_use]
    pub fn to_oid(&self) -> &ObjectId {
        &self.to_oid
    }

    /// The merge-result mode.
    #[must_use]
    pub fn to_mode(&self) -> FileMode {
        self.to_mode
    }

    /// The path in the merge result.
    #[must_use]
    pub fn to_path(&self) -> &str {
        &self.to_path
    }

    /// Whether the path was deleted in the merge (gitweb's `is_deleted`).
    #[must_use]
    pub fn is_deleted(&self) -> bool {
        self.is_deleted
    }

    /// Whether the path has a prior version in some parent (gitweb's
    /// `has_history`): the `blob` and `history` links are shown when it does.
    #[must_use]
    pub fn has_history(&self) -> bool {
        self.has_history
    }

    /// Whether the path still exists against some parent (gitweb's `not_deleted`):
    /// the result `blob` link is shown when it does.
    #[must_use]
    pub fn not_deleted(&self) -> bool {
        self.not_deleted
    }
}

/// The changed-files set of a commit: an ordinary two-tree diff against a single
/// base, or a combined diff against every parent of a merge.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ChangedFiles {
    /// A two-tree diff against a single `base` — the chosen explicit parent, the
    /// first parent, or `None` for the empty tree of a root commit. `base` is the
    /// revision the per-row `blob` / `blame` / `history` links are taken at
    /// (gitweb's `$parent = $parents[0]` in `git_difftree_body`), so the boundary
    /// reads it from here rather than re-deriving the commit's first parent.
    Ordinary {
        /// The base the diff and its per-row links are taken against, or `None`
        /// for the empty tree of a root commit.
        base: Option<ObjectId>,
        /// The changed paths.
        changes: Vec<OrdinaryChange>,
    },
    /// A merge's combined diff against all its parents at once.
    Combined(Vec<CombinedChange>),
}

impl ChangedFiles {
    /// The number of changed paths, regardless of diff shape.
    #[must_use]
    pub fn len(&self) -> usize {
        match self {
            Self::Ordinary { changes, .. } => changes.len(),
            Self::Combined(rows) => rows.len(),
        }
    }

    /// Whether nothing changed (a commit that introduces no diff).
    #[must_use]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

/// One commit resolved and ready to render: gitweb's `%co` for `git_commit`,
/// framework-free, plus its changed-files set.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CommitView {
    id: ObjectId,
    tree: ObjectId,
    parents: Vec<ObjectId>,
    author: AuthorLine,
    committer: AuthorLine,
    title: String,
    comment: Vec<LogLine>,
    changes: ChangedFiles,
}

impl CommitView {
    /// The commit's own object id (gitweb's `$co{'id'}`).
    #[must_use]
    pub fn id(&self) -> &ObjectId {
        &self.id
    }

    /// The root tree this commit snapshots (gitweb's `$co{'tree'}`).
    #[must_use]
    pub fn tree(&self) -> &ObjectId {
        &self.tree
    }

    /// The parent commits, in order; empty for a root commit.
    #[must_use]
    pub fn parents(&self) -> &[ObjectId] {
        &self.parents
    }

    /// Whether this is a merge (more than one parent) — gitweb shows the combined
    /// diff and the `(merge: …)` parent navigation for it.
    #[must_use]
    pub fn is_merge(&self) -> bool {
        self.parents.len() > 1
    }

    /// The authoring identity and date.
    #[must_use]
    pub fn author(&self) -> &AuthorLine {
        &self.author
    }

    /// The committing identity and date.
    #[must_use]
    pub fn committer(&self) -> &AuthorLine {
        &self.committer
    }

    /// The commit title (gitweb's `$co{'title'}`): the first message line,
    /// chopped to 80, shown in the page header.
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The processed message body lines (gitweb's `git_print_log`).
    #[must_use]
    pub fn comment(&self) -> &[LogLine] {
        &self.comment
    }

    /// The changed-files set (ordinary or combined).
    #[must_use]
    pub fn changes(&self) -> &ChangedFiles {
        &self.changes
    }
}

/// gitweb's site-default rename detection (`@diff_opts = ('-M')`): renames only.
/// Threading the configured level is a later refinement; the commit view uses the
/// default the way an out-of-the-box gitweb does.
const DETECTION: RenameDetection = RenameDetection::RenamesOnly;

/// Resolves `revision` (defaulting to `HEAD`) to a commit and assembles its view.
///
/// `explicit_parent` is gitweb's `$hash_parent`: when given, the changed-files
/// table is the ordinary two-tree diff against that parent (the `commit` view
/// never passes one; the `commitdiff` view passes the chosen parent). With no
/// explicit parent a merge's table is the combined diff and a non-merge's is the
/// diff against its first parent — [`changed_files_base`].
///
/// # Errors
///
/// Returns [`DomainError::NotFound`] with gitweb's `Unknown commit object`
/// message when the revision names something that is not a commit, and propagates
/// the repository's not-found when the revision (or `explicit_parent`) resolves to
/// nothing — both gitweb's `die_error(404)`.
pub fn assemble_commit(
    repo: &dyn Repository,
    revision: Option<&str>,
    explicit_parent: Option<&str>,
) -> Result<CommitView, DomainError> {
    let oid: ObjectId = repo.resolve(revision.unwrap_or("HEAD"))?;
    if repo.object_kind(&oid)? != ObjectKind::Commit {
        return Err(DomainError::NotFound("Unknown commit object".to_owned()));
    }
    let commit: Commit = repo.find_commit(&oid)?;
    let explicit: Option<ObjectId> = match explicit_parent {
        Some(rev) => Some(repo.resolve(rev)?),
        None => None,
    };
    let changes: ChangedFiles = changed_files(repo, &commit, explicit.as_ref())?;
    Ok(CommitView {
        id: commit.id().clone(),
        tree: commit.tree().clone(),
        parents: commit.parents().to_vec(),
        author: author_line(commit.author()),
        committer: author_line(commit.committer()),
        title: commit.title(),
        comment: log_lines(commit.message()),
        changes,
    })
}

/// Reads the commit's changed-files set for the chosen base ([`changed_files_base`]):
/// a combined diff for a merge with no explicit parent (gitweb's `-c`), otherwise
/// an ordinary two-tree diff against the explicit parent, the first parent, or the
/// empty tree of a root commit (gitweb's `--root`).
fn changed_files(
    repo: &dyn Repository,
    commit: &Commit,
    explicit: Option<&ObjectId>,
) -> Result<ChangedFiles, DomainError> {
    match changed_files_base(commit.parents(), explicit) {
        ChangedFilesBase::Combined => {
            let combined: CombinedDiff = repo.combined_diff(commit.id())?;
            Ok(ChangedFiles::Combined(
                combined.entries().iter().map(combined_change).collect(),
            ))
        }
        ChangedFilesBase::Ordinary { base } => {
            let diff: Diff = repo.diff(base.as_ref(), commit.id(), DETECTION)?;
            Ok(ChangedFiles::Ordinary {
                base,
                changes: diff.entries().iter().map(ordinary_change).collect(),
            })
        }
    }
}

/// Maps a parsed identity to a view authorship line carrying its own absolute
/// timestamp (gitweb's `parse_date($co{author_epoch}, …_tz)`).
fn author_line(who: &Signature) -> AuthorLine {
    AuthorLine {
        name: who.name().to_owned(),
        email: who.email().map(str::to_owned),
        timestamp: Timestamp::from_signature(who),
    }
}

/// Maps one ordinary diff entry to a changed-files row, deriving its status note.
fn ordinary_change(entry: &DiffEntry) -> OrdinaryChange {
    OrdinaryChange {
        status: entry.status(),
        from_oid: entry.from_oid().clone(),
        to_oid: entry.to_oid().clone(),
        from_path: entry.from_path().to_owned(),
        to_path: entry.to_path().to_owned(),
        from_mode: entry.from_mode(),
        to_mode: entry.to_mode(),
        note: FileChangeNote::derive(entry.status(), entry.from_mode(), entry.to_mode()),
    }
}

/// Maps one combined diff entry to a merge changed-files row, carrying its
/// per-parent sides and the row predicates gitweb computes inline.
fn combined_change(entry: &CombinedDiffEntry) -> CombinedChange {
    let parents: Vec<CombinedParentChange> = entry
        .parents()
        .iter()
        .map(|parent| CombinedParentChange {
            status: parent.status(),
            from_oid: parent.from_oid().clone(),
        })
        .collect();
    CombinedChange {
        parents,
        to_oid: entry.to_oid().clone(),
        to_mode: entry.to_mode(),
        to_path: entry.to_path().to_owned(),
        is_deleted: entry.is_deleted(),
        has_history: entry.has_history(),
        not_deleted: entry.not_deleted(),
    }
}
