//! The `heads` use case: gitweb's `git_heads` / `git_get_heads_list`, minus the
//! HTML.
//!
//! gitweb lists the branches with `for-each-ref --sort=-HEAD
//! --sort=-committerdate`, so the rows come out newest-commit-first, with the
//! branch HEAD points at floated above any branch tied on commit time, then by
//! ref name (git's implicit final key). Each row carries its tip commit's age
//! relative to the request time, and the branch (or branches) whose tip *is*
//! HEAD's commit is marked current — gitweb compares commit ids
//! (`$ref{'id'} eq $head_at`), which is not always the one symbolic-HEAD branch.
//!
//! This is the orchestration over the [`Repository`] port, producing a
//! framework-free [`HeadsView`] the render adapter turns into a table. The clock
//! is injected (`now`), so ages are computed here, once, and the view-model
//! stays free of any time dependency. An unborn repository (no HEAD) lists its
//! branches with nothing marked current, and a repository with no branches lists
//! nothing — neither is an error.

use std::cmp::Ordering;

use crate::error::DomainError;
use crate::model::age::Age;
use crate::model::commit::Commit;
use crate::model::object_id::ObjectId;
use crate::model::reference::Reference;
use crate::port::repository::Repository;

/// One branch as it appears on the heads listing.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadRow {
    name: String,
    full_name: String,
    age: Option<Age>,
    current: bool,
}

impl HeadRow {
    /// The display short branch name (e.g. `main`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The fully-qualified ref name (`refs/heads/<name>`), for link building.
    #[must_use]
    pub fn full_name(&self) -> &str {
        &self.full_name
    }

    /// The age of the branch tip relative to the request time, or `None` when
    /// the tip has no commit time (gitweb's "unknown").
    #[must_use]
    pub fn age(&self) -> Option<Age> {
        self.age
    }

    /// Whether this branch's tip is HEAD's commit (gitweb's `current_head`).
    #[must_use]
    pub fn current(&self) -> bool {
        self.current
    }
}

/// The assembled heads page: the branches in display order.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct HeadsView {
    rows: Vec<HeadRow>,
}

impl HeadsView {
    /// The branches, in display order.
    #[must_use]
    pub fn rows(&self) -> &[HeadRow] {
        &self.rows
    }
}

/// A branch enriched with the bits that decide its order, before it becomes a
/// display row: its tip's committer epoch (the primary sort key), whether it is
/// the symbolic-HEAD branch (the tie-break), and the display fields.
struct HeadEntry {
    name: String,
    full_name: String,
    epoch: i64,
    is_head_branch: bool,
    current: bool,
}

/// Assembles the heads page over the [`Repository`] port. `now` is the
/// request-time epoch the relative ages are measured against. `branch_refs` is
/// gitweb's `get_branch_refs` — the directories under `refs/` that count as
/// branches (`heads`, plus the validated `extra-branch-refs` entries) — so the
/// listing covers every branch directory, not just `refs/heads/`.
///
/// # Errors
///
/// Propagates the repository's error if ref listing or a tip-commit read fails.
/// An unborn repository whose HEAD names no commit is not an error — it simply
/// floats no branch and marks none current.
pub fn assemble_heads(
    repo: &dyn Repository,
    now: i64,
    branch_refs: &[&str],
) -> Result<HeadsView, DomainError> {
    let (head_branch, head_commit): (Option<String>, Option<ObjectId>) = read_head(repo)?;
    let references: Vec<Reference> = branch_references(repo, branch_refs)?;
    let rows: Vec<HeadRow> = assemble_head_rows(
        repo,
        &references,
        head_branch.as_deref(),
        head_commit.as_ref(),
        now,
        |reference: &Reference| reference.short().into_owned(),
    )?;
    Ok(HeadsView { rows })
}

/// Lists the refs under every branch directory `branch_refs` names — gitweb's
/// `map { "refs/$_" } get_branch_refs()` fed to a single `for-each-ref`. The
/// per-directory listings are concatenated; the global
/// `--sort=-HEAD --sort=-committerdate` order is restored downstream by
/// [`assemble_head_rows`], so it does not matter that the port reports each
/// directory separately.
///
/// # Errors
///
/// Propagates the repository's error if any directory's ref listing fails.
fn branch_references(
    repo: &dyn Repository,
    branch_refs: &[&str],
) -> Result<Vec<Reference>, DomainError> {
    let mut references: Vec<Reference> = Vec::new();
    for dir in branch_refs {
        references.extend(repo.references(&format!("refs/{dir}/"))?);
    }
    Ok(references)
}

/// Enriches a set of branch-like refs into ordered head rows: each ref's tip age
/// relative to `now`, the current-branch marker (its tip is HEAD's commit), and a
/// display name from `display`. The rows come out in gitweb's
/// `--sort=-HEAD --sort=-committerdate` order.
///
/// Shared by the heads listing (refs under `refs/heads/`, shown under their short
/// name) and the remotes view (a remote's tracking branches under
/// `refs/remotes/<name>/`, shown without the `<name>/` prefix); both are the same
/// `git_heads_body` table over a different ref set and naming, so the enrichment
/// lives here once.
///
/// # Errors
///
/// Propagates the repository's error if a tip-commit read fails.
pub(crate) fn assemble_head_rows(
    repo: &dyn Repository,
    references: &[Reference],
    head_branch: Option<&str>,
    head_commit: Option<&ObjectId>,
    now: i64,
    display: impl Fn(&Reference) -> String,
) -> Result<Vec<HeadRow>, DomainError> {
    let mut entries: Vec<HeadEntry> = references
        .iter()
        .map(|reference: &Reference| {
            entry_for(
                repo,
                reference,
                head_branch,
                head_commit,
                display(reference),
            )
        })
        .collect::<Result<Vec<HeadEntry>, DomainError>>()?;
    entries.sort_by(order_heads);
    Ok(entries
        .into_iter()
        .map(|entry: HeadEntry| HeadRow {
            name: entry.name,
            full_name: entry.full_name,
            age: age_of(entry.epoch, now),
            current: entry.current,
        })
        .collect())
}

/// Reads HEAD: the fully-qualified branch it points at and the commit it
/// resolves to. An unborn HEAD (gitweb's empty `git_get_head_hash`) yields
/// `(None, None)`; any other failure propagates. Shared with the remotes use
/// case, which needs HEAD's commit to mark a tracking branch current.
pub(crate) fn read_head(
    repo: &dyn Repository,
) -> Result<(Option<String>, Option<ObjectId>), DomainError> {
    match repo.head() {
        Ok(reference) => Ok((
            Some(reference.name().full().to_owned()),
            Some(reference.target().clone()),
        )),
        Err(DomainError::NotFound(_)) => Ok((None, None)),
        Err(other) => Err(other),
    }
}

/// Enriches one branch ref with its tip's committer epoch and its HEAD relation,
/// under the already-resolved display `name`.
///
/// A tip that cannot be read as a commit (gitweb's for-each-ref
/// `%(committer)` returning empty) degrades to epoch 0 — gitweb renders this
/// as age "unknown" and still lists the branch. Only unexpected errors
/// propagate.
fn entry_for(
    repo: &dyn Repository,
    reference: &Reference,
    head_branch: Option<&str>,
    head_commit: Option<&ObjectId>,
    name: String,
) -> Result<HeadEntry, DomainError> {
    let epoch: i64 = match repo.find_commit(reference.target()) {
        Ok(commit) => commit.committer().epoch(),
        Err(DomainError::NotFound(_)) => 0,
        Err(other) => return Err(other),
    };
    let full_name: String = reference.name().full().to_owned();
    Ok(HeadEntry {
        name,
        is_head_branch: head_branch == Some(full_name.as_str()),
        current: head_commit == Some(reference.target()),
        full_name,
        epoch,
    })
}

/// gitweb's `--sort=-HEAD --sort=-committerdate` order: newest commit first,
/// the symbolic-HEAD branch floated above any branch tied on commit time, then
/// ref name ascending (git's implicit final key).
fn order_heads(a: &HeadEntry, b: &HeadEntry) -> Ordering {
    b.epoch
        .cmp(&a.epoch)
        .then_with(|| b.is_head_branch.cmp(&a.is_head_branch))
        .then_with(|| a.full_name.cmp(&b.full_name))
}

/// The branch tip's age relative to `now`, or `None` when the tip carries no
/// commit time (gitweb renders this as "unknown").
fn age_of(epoch: i64, now: i64) -> Option<Age> {
    if epoch == 0 {
        None
    } else {
        Some(Age::from_seconds(now - epoch))
    }
}
