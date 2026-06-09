//! The `blobdiff` handler: gitweb's `git_blobdiff` HTML page, modernized.
//!
//! Glue only. gitweb's `git_blobdiff` (html) colourises one file's diff between
//! two base commits in the page itself. We modernize that the way the
//! [`commitdiff`](crate::handlers::commitdiff) view does — the page is a host
//! shell whose `#diff-root` the vendored `@pierre/diffs` viewer fills from a
//! clean unified diff it fetches from the diff-text endpoint
//! ([`crate::diff_text`]) — only scoped to the one file: the host carries the
//! file path, and the diff URL carries `f`.
//!
//! It opens the project, runs the [`assemble_blobdiff`] use case to resolve
//! *which* file (by its new-side path `f`, or the legacy by-hash `h`) and read
//! the base commit's subject, then maps that to the
//! [`DiffHostPage`](gitweb_render::diff_host::DiffHostPage). The `raw` affordance
//! points at `blobdiff_plain` (gitweb's `git_blobdiff('plain')`); the
//! inline/side-by-side toggle is the client viewer's concern, so it is not
//! emitted here.
//!
//! gitweb serves the new-style URI: `hb` (base) + `hpb` (parent base) + `f`
//! (file), or `h` (a blob's new-side id) in place of `f`. A missing base is
//! gitweb's 404; the use case maps a missing file (404), an ambiguous spec
//! (400), and the no-selector case (400).

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::blobdiff::{BlobdiffView, assemble_blobdiff};
use gitweb_render::chrome::{Crumb, DocumentHead, FormatLink, NavItem, document};
use gitweb_render::diff_host::{DiffHostPage, diff_host_body};
use gitweb_render::markup::Markup;

use crate::assets::{DIFF_VIEWER_PATH, FAVICON_PATH, STYLESHEET_PATH};
use crate::diff_text::diff_text_href;
use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href;

/// Serves the blobdiff host page over a wired project store and settings.
pub struct BlobdiffHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl BlobdiffHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for BlobdiffHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;

        // gitweb's git_blobdiff: a missing base falls through to the 404 "Missing
        // one of the blob diff parameters"; with the bases present, the use case
        // decides the file selector (a missing one is its 400).
        let hash_base: &str = request
            .hash_base
            .as_ref()
            .map(SafeRef::as_str)
            .ok_or_else(|| {
                DomainError::NotFound("Missing one of the blob diff parameters".to_owned())
            })?;
        let hash_parent_base: &str = request
            .hash_parent_base
            .as_ref()
            .map(SafeRef::as_str)
            .ok_or_else(|| {
                DomainError::NotFound("Missing one of the blob diff parameters".to_owned())
            })?;
        let file_name: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        let hash: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);

        let repository: Box<dyn Repository> = self.store.open(project)?;
        let view: BlobdiffView = assemble_blobdiff(
            repository.as_ref(),
            hash_base,
            hash_parent_base,
            file_name,
            hash,
        )?;
        // gitweb's git_blobdiff dual-oid gate: a one-day window only when both
        // the base and the parent base are literal object ids.
        Ok(View::html(render_page(
            &self.settings,
            project,
            hash_base,
            hash_parent_base,
            &view,
        ))
        .with_expiry(Expiry::for_hashes(&[
            Some(hash_base),
            Some(hash_parent_base),
        ])))
    }
}

/// Maps the resolved view to the diff host page and wraps it in the chrome.
fn render_page(
    settings: &Settings,
    project: &str,
    hash_base: &str,
    hash_parent_base: &str,
    view: &BlobdiffView,
) -> Markup {
    let file_name: &str = view.file_name();
    let page: DiffHostPage = DiffHostPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, hash_base),
        formats: formats(
            project,
            hash_base,
            hash_parent_base,
            file_name,
            view.file_parent(),
        ),
        title: view.title().to_owned(),
        path: Some(file_name.to_owned()),
        diff_url: diff_url(project, hash_base, hash_parent_base, file_name),
        viewer_module_src: DIFF_VIEWER_PATH.to_owned(),
    };
    let head: DocumentHead = DocumentHead {
        title: format!("{project} / blobdiff / {file_name}"),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    document(&head, diff_host_body(&page))
}

/// The breadcrumb trail: home, the project (summary), then blobdiff.
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
            label: "blobdiff".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('', …, $hash_base, …)`):
/// summary, the shortlog and verbose log, commit, commitdiff, and the tree —
/// every item scoped to the base commit, none marked current (blobdiff is not a
/// nav action).
fn nav(project: &str, hash_base: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(href(&[("p", project), ("a", "shortlog"), ("h", hash_base)])),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(href(&[("p", project), ("a", "log"), ("h", hash_base)])),
        },
        NavItem {
            label: "commit".to_owned(),
            href: Some(href(&[("p", project), ("a", "commit"), ("h", hash_base)])),
        },
        NavItem {
            label: "commitdiff".to_owned(),
            href: Some(href(&[
                ("p", project),
                ("a", "commitdiff"),
                ("h", hash_base),
            ])),
        },
        NavItem {
            label: "tree".to_owned(),
            href: Some(href(&[("p", project), ("a", "tree"), ("hb", hash_base)])),
        },
    ]
}

/// The alternate-format affordance: `raw`, pointing at `blobdiff_plain` for the
/// same single file (carrying the rename's `fp` old path when there is one).
fn formats(
    project: &str,
    hash_base: &str,
    hash_parent_base: &str,
    file_name: &str,
    file_parent: Option<&str>,
) -> Vec<FormatLink> {
    let mut params: Vec<(&str, &str)> = vec![
        ("p", project),
        ("a", "blobdiff_plain"),
        ("hb", hash_base),
        ("hpb", hash_parent_base),
        ("f", file_name),
    ];
    if let Some(from_path) = file_parent {
        params.push(("fp", from_path));
    }
    vec![FormatLink {
        label: "raw".to_owned(),
        href: href(&params),
    }]
}

/// The clean-diff endpoint URL the viewer fetches, scoped to the one file: the
/// two bases as the commit (`h`) and its parent (`hp`), and the file's new-side
/// path as `f`.
fn diff_url(project: &str, hash_base: &str, hash_parent_base: &str, file_name: &str) -> String {
    diff_text_href(&[
        ("p", project),
        ("h", hash_base),
        ("hp", hash_parent_base),
        ("f", file_name),
    ])
}
