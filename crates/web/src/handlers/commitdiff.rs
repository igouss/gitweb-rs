//! The `commitdiff` handler: gitweb's `git_commitdiff` HTML page, modernized.
//!
//! Glue only. It opens the requested project, runs the [`assemble_commit`] use
//! case (the same view the `commit` page reads: authorship, message, and the
//! changed-files set against the commit's base), and maps it to the diff-viewer
//! host page — the authorship rows, the message body, the changed-files table in
//! `commitdiff` context (patch-anchor links, shared via
//! [`changed_files`](crate::handlers::changed_files)), and the `#diff-root`
//! container whose `data-diff-url` points at the clean-diff endpoint
//! ([`crate::diff_text`]). The navigation carries the `raw` / `patch` format
//! affordances and the parent/merge context gitweb builds.
//!
//! gitweb diffs a merge with no chosen parent as a combined `--cc` diff; our
//! client viewer renders two-tree unified diffs, so the diff URL targets the
//! merge's first parent (the [`diff_base`](gitweb_domain::model::commitdiff)
//! rule), while the changed-files table still shows the combined diff. The
//! `patch` affordance is gated the way gitweb's git_commitdiff gates it: the
//! `patches` feature on, at most one parent, and the `email-privacy` feature off.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::commit::{AuthorLine, CommitView, assemble_commit};
use gitweb_render::chrome::{Crumb, DocumentHead, FormatLink, NavItem, document};
use gitweb_render::commit::{AuthorRow, ParentNavLink};
use gitweb_render::commitdiff::{CommitdiffNav, CommitdiffPage, commitdiff_body};
use gitweb_render::markup::Markup;

use crate::assets::DIFF_VIEWER_PATH;
use crate::diff_text::diff_text_href;
use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::handlers::changed_files::{self, Context};
use crate::response::View;
use crate::url::href;

/// Serves the commitdiff host page over a wired project store and settings.
pub struct CommitdiffHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl CommitdiffHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name, chrome, and feature gates from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for CommitdiffHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        // gitweb's `$hash ||= $hash_base || "HEAD"`: the commit, then the base, then HEAD.
        let revision: Option<&str> = request
            .hash
            .as_ref()
            .or(request.hash_base.as_ref())
            .map(SafeRef::as_str);
        // gitweb's `$hash_parent`: a chosen parent makes the changed-files table
        // (and the viewer URL and parentage) an ordinary two-tree diff against it,
        // rather than the merge's combined diff or a non-merge's first-parent diff.
        let explicit: Option<&str> = request.hash_parent.as_ref().map(SafeRef::as_str);
        let view: CommitView = assemble_commit(repository.as_ref(), revision, explicit)?;
        let blame_on: bool = self.settings.feature(FeatureName::Blame).enabled();
        // gitweb's `$hash =~ /^$oid_regex$/` (after `$hash ||= $hash_base || "HEAD"`):
        // a commitdiff addressed by a literal oid is immutable and cacheable for a day.
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            &view,
            explicit,
            blame_on,
            chrome,
        ))
        .with_expiry(Expiry::for_hash(revision)))
    }
}

/// Maps the use-case view to the commitdiff host page and wraps it in the chrome.
fn render_page(
    settings: &Settings,
    project: &str,
    view: &CommitView,
    explicit: Option<&str>,
    blame_on: bool,
    chrome: PageChrome,
) -> Markup {
    let hash: &str = view.id().as_str();
    let page: CommitdiffPage = CommitdiffPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, hash, view.tree().as_str()),
        formats: formats(settings, project, hash, view.parents().len()),
        parentage: parentage(project, view.parents(), explicit),
        title: view.title().to_owned(),
        author: author_row(view.author()),
        committer: author_row(view.committer()),
        comment: view.comment().to_vec(),
        changed: changed_files::rows(Context::Commitdiff, project, view, blame_on),
        diff_url: diff_url(project, hash, explicit),
        viewer_module_src: DIFF_VIEWER_PATH.to_owned(),
    };
    let head: DocumentHead = document_head(
        format!("{project} / commitdiff / {}", view.title()),
        chrome.feeds,
        chrome.scripts,
    );
    document(&head, &chrome.foot, commitdiff_body(&page))
}

/// The breadcrumb trail: home, the project (summary), then commitdiff.
fn crumbs(site_name: &str, project: &str) -> Vec<Crumb> {
    vec![
        Crumb {
            label: site_name.to_owned(),
            href: Some(href(&[])),
        },
        Crumb {
            label: project.to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        Crumb {
            label: "commitdiff".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('commitdiff', …)`):
/// summary, the shortlog and verbose log scoped to this commit, commit,
/// commitdiff (the current view), and the tree at this commit.
fn nav(project: &str, hash: &str, tree: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(href(&[("p", project), ("a", "shortlog"), ("h", hash)])),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(href(&[("p", project), ("a", "log"), ("h", hash)])),
        },
        NavItem {
            label: "commit".to_owned(),
            href: Some(href(&[("p", project), ("a", "commit"), ("h", hash)])),
        },
        NavItem {
            label: "commitdiff".to_owned(),
            href: None,
        },
        NavItem {
            label: "tree".to_owned(),
            href: Some(href(&[
                ("p", project),
                ("a", "tree"),
                ("h", tree),
                ("hb", hash),
            ])),
        },
    ]
}

/// The alternate-format affordances: `raw` (always, to `commitdiff_plain`) and,
/// gated the way gitweb gates it (`patches` on, at most one parent, no
/// `email-privacy`), `patch`.
fn formats(settings: &Settings, project: &str, hash: &str, parents: usize) -> Vec<FormatLink> {
    let mut formats: Vec<FormatLink> = vec![FormatLink {
        label: "raw".to_owned(),
        href: href(&[("p", project), ("a", "commitdiff_plain"), ("h", hash)]),
    }];
    let patches_on: bool = settings.feature(FeatureName::Patches).enabled();
    let email_privacy: bool = settings.feature(FeatureName::EmailPrivacy).enabled();
    if patches_on && parents <= 1 && !email_privacy {
        formats.push(FormatLink {
            label: "patch".to_owned(),
            href: href(&[("p", project), ("a", "patch"), ("h", hash)]),
        });
    }
    formats
}

/// The parent/merge context: gitweb's commitdiff `$formats_nav` branch — an
/// explicit parent (`from[ parent N]`), then `(initial)` / `(parent: …)` /
/// `(merge: …)` by parent count.
fn parentage(project: &str, parents: &[ObjectId], explicit: Option<&str>) -> CommitdiffNav {
    if let Some(parent) = explicit {
        let number: Option<usize> = parents
            .iter()
            .position(|candidate: &ObjectId| candidate.as_str() == parent)
            .map(|index: usize| index + 1);
        return CommitdiffNav::FromParent {
            number,
            link: nav_link(project, parent),
        };
    }
    match parents {
        [] => CommitdiffNav::Initial,
        [parent] => CommitdiffNav::SingleParent(nav_link(project, parent.as_str())),
        many => CommitdiffNav::Merge(
            many.iter()
                .map(|parent: &ObjectId| nav_link(project, parent.as_str()))
                .collect(),
        ),
    }
}

/// One linked parent in the navigation: the abbreviated id and its commit URL.
fn nav_link(project: &str, parent: &str) -> ParentNavLink {
    ParentNavLink {
        short: parent.chars().take(7).collect(),
        href: href(&[("p", project), ("a", "commit"), ("h", parent)]),
    }
}

/// The clean-diff endpoint URL the viewer fetches: the commit, and the explicit
/// parent when one is given. With no explicit parent the endpoint picks the base
/// itself (first parent, or the empty tree for a root commit).
fn diff_url(project: &str, hash: &str, explicit: Option<&str>) -> String {
    match explicit {
        Some(parent) => diff_text_href(&[("p", project), ("h", hash), ("hp", parent)]),
        None => diff_text_href(&[("p", project), ("h", hash)]),
    }
}

/// An authorship row from a use-case authorship line.
fn author_row(line: &AuthorLine) -> AuthorRow {
    AuthorRow {
        name: line.name().to_owned(),
        email: line.email().map(str::to_owned),
        timestamp: line.timestamp().clone(),
    }
}
