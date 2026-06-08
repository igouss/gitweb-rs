//! The `remotes` use case: gitweb's `git_remotes`, minus the HTML.
//!
//! gitweb's remotes view lists the configured remotes, each as a block of its
//! fetch/push URLs (the [`Remote::url_lines`] rule) and its remote-tracking
//! branches under `refs/remotes/<name>/`, enriched exactly as the heads listing
//! enriches `refs/heads/` (tip age, current marker) — only the displayed branch
//! name drops the `<name>/` prefix. Two shapes, as gitweb's `git_remotes` splits
//! them: the all-remotes view (`$remote` unset) shows every configured remote;
//! the single-remote view (`hash=<name>`) shows just that one, 404 when it is not
//! configured.
//!
//! The whole view is gated behind the `remote_heads` feature: gitweb's
//! `gitweb_check_feature('remote_heads') or die_error(403, ...)`. The gate is
//! checked here, so a disabled feature is a [`DomainError::Forbidden`] the web
//! boundary maps to gitweb's 403 before any repository read happens.
//!
//! This is the orchestration over the [`Repository`] port plus the resolved
//! [`Settings`] (for the feature gate). The clock is injected (`now`) for the
//! tracking-branch ages, the same discipline as the heads use case it reuses.

use std::borrow::Cow;

use crate::error::DomainError;
use crate::model::object_id::ObjectId;
use crate::model::reference::Reference;
use crate::model::remote::{Remote, RemoteUrl};
use crate::model::settings::{FeatureName, Settings};
use crate::port::repository::Repository;
use crate::usecase::heads::{HeadRow, assemble_head_rows, read_head};

/// One remote's block on the remotes page: its name, its URL lines, and its
/// remote-tracking branches (the same rows the heads table shows, named without
/// the `<remote>/` prefix).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemoteBlock {
    name: String,
    urls: Vec<RemoteUrl>,
    heads: Vec<HeadRow>,
}

impl RemoteBlock {
    /// The remote's name (`origin`, `upstream`, …).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The remote's URL lines (gitweb's `git_remote_block` URL table).
    #[must_use]
    pub fn urls(&self) -> &[RemoteUrl] {
        &self.urls
    }

    /// The remote's tracking branches, in heads-table order (newest commit first).
    #[must_use]
    pub fn heads(&self) -> &[HeadRow] {
        &self.heads
    }
}

/// The assembled remotes page: one block per shown remote, and whether this is
/// the single-remote view (gitweb renders the two shapes differently).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RemotesView {
    blocks: Vec<RemoteBlock>,
    single: bool,
}

impl RemotesView {
    /// The remote blocks, in name order (one block for the single-remote view).
    #[must_use]
    pub fn blocks(&self) -> &[RemoteBlock] {
        &self.blocks
    }

    /// Whether this is the single-remote view (`hash=<name>`), as opposed to the
    /// all-remotes view.
    #[must_use]
    pub fn is_single(&self) -> bool {
        self.single
    }
}

/// Assembles the remotes page over the [`Repository`] port. `selected` is the
/// `hash` parameter — `Some(name)` for the single-remote view, `None` for the
/// all-remotes view; `now` is the request-time epoch the tracking-branch ages are
/// measured against; `settings` carries the `remote_heads` feature gate.
///
/// # Errors
///
/// [`DomainError::Forbidden`] when the `remote_heads` feature is disabled
/// (gitweb's 403). [`DomainError::NotFound`] when no remotes are configured at
/// all (the all-remotes view) or the named remote is not configured (the
/// single-remote view) — gitweb's 404s. Otherwise propagates the repository's
/// error from reading remotes, refs, or a tip commit.
pub fn assemble_remotes(
    repo: &dyn Repository,
    settings: &Settings,
    selected: Option<&str>,
    now: i64,
) -> Result<RemotesView, DomainError> {
    if !settings.feature(FeatureName::RemoteHeads).enabled() {
        return Err(DomainError::Forbidden(
            "Remote heads view is disabled".to_owned(),
        ));
    }

    let remotes: Vec<Remote> = repo.remotes()?;
    let shown: Vec<Remote> = select(remotes, selected)?;

    let (head_branch, head_commit): (Option<String>, Option<ObjectId>) = read_head(repo)?;
    let blocks: Vec<RemoteBlock> = shown
        .iter()
        .map(|remote: &Remote| {
            remote_block(
                repo,
                remote,
                head_branch.as_deref(),
                head_commit.as_ref(),
                now,
            )
        })
        .collect::<Result<Vec<RemoteBlock>, DomainError>>()?;

    Ok(RemotesView {
        blocks,
        single: selected.is_some(),
    })
}

/// Narrows the configured remotes to the ones the view shows: the single named
/// remote for the single-remote view (404 when it is not configured), or all of
/// them for the all-remotes view (404 when none are configured) — gitweb's two
/// `die_error(404, ...)` messages.
fn select(remotes: Vec<Remote>, selected: Option<&str>) -> Result<Vec<Remote>, DomainError> {
    match selected {
        Some(name) => remotes
            .into_iter()
            .find(|remote: &Remote| remote.name() == name)
            .map(|remote: Remote| vec![remote])
            .ok_or_else(|| DomainError::NotFound(format!("Remote {name} not found"))),
        None if remotes.is_empty() => Err(DomainError::NotFound("No remotes found".to_owned())),
        None => Ok(remotes),
    }
}

/// Builds one remote's block: its URL lines and its tracking branches under
/// `refs/remotes/<name>/`, enriched with the shared heads rule and named without
/// the `<name>/` prefix gitweb strips in `fill_remote_heads`.
fn remote_block(
    repo: &dyn Repository,
    remote: &Remote,
    head_branch: Option<&str>,
    head_commit: Option<&ObjectId>,
    now: i64,
) -> Result<RemoteBlock, DomainError> {
    let prefix: String = format!("refs/remotes/{}/", remote.name());
    let references: Vec<Reference> = repo.references(&prefix)?;
    let strip: String = format!("{}/", remote.name());
    let heads: Vec<HeadRow> = assemble_head_rows(
        repo,
        &references,
        head_branch,
        head_commit,
        now,
        |reference: &Reference| tracking_name(reference, &strip),
    )?;
    Ok(RemoteBlock {
        name: remote.name().to_owned(),
        urls: remote.url_lines(),
        heads,
    })
}

/// A tracking branch's display name: its short ref name with the leading
/// `<remote>/` stripped (gitweb's `$_->{'name'} =~ s!^$remote/!!`), so
/// `refs/remotes/origin/main` shows as `main`.
fn tracking_name(reference: &Reference, strip: &str) -> String {
    let short: Cow<'_, str> = reference.short();
    if let Some(rest) = short.strip_prefix(strip) {
        return rest.to_owned();
    }
    short.into_owned()
}
