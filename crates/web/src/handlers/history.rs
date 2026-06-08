//! The `history` handler: gitweb's `git_history` page (per-path log).
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_history`] use case over it for the requested path and base
//! revision (HEAD by default) and page, maps the resulting view-model to the
//! render layer's table — building every link with [`href`], since URLs are the
//! boundary's job — wraps it in the document chrome, and returns the page as a
//! [`View`]. The clock lives here, at the boundary: the request time is read once
//! and handed to the use case so the date cells are computed against a real `now`
//! while the domain stays clock-free.
//!
//! Unlike the shortlog, the history needs a file: gitweb only reaches
//! `git_history` from a tracked path, so a request without one is
//! `die_error(400)`.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::{Page, Repository};
use gitweb_domain::usecase::history::{HistoryRow, HistoryView, assemble_history};
use gitweb_render::chrome::{Crumb, DocumentHead, MoreLink, NavItem, document};
use gitweb_render::history::{HistoryEntryView, HistoryPage, HistoryTable, history_body};
use gitweb_render::markup::Markup;

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href;

/// gitweb's `git_log_generic` page size (`parse_commits($base, 101, 100*$page)`).
const PAGE_SIZE: usize = 100;

/// Serves the history page over a wired project store and the resolved settings.
pub struct HistoryHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl HistoryHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for HistoryHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let path: &str = request
            .file_name
            .as_ref()
            .map(SafePath::as_str)
            .ok_or_else(|| DomainError::Invalid("File name needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let rev: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        let page_num: usize = request.page.unwrap_or(0) as usize;
        let page: Page = Page::from_page(page_num, PAGE_SIZE);
        let view: HistoryView =
            assemble_history(repository.as_ref(), rev, path, now_epoch(), page)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            rev,
            path,
            page_num,
            &view,
        )))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(
    settings: &Settings,
    project: &str,
    rev: Option<&str>,
    path: &str,
    page_num: usize,
    view: &HistoryView,
) -> Markup {
    let ftype: &str = view.file_type().as_str();
    let page: HistoryPage = HistoryPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, rev),
        file_name: path.to_owned(),
        table: HistoryTable {
            object_label: ftype.to_owned(),
            rows: view
                .rows()
                .iter()
                .map(|row: &HistoryRow| history_entry(project, path, rev, view, row))
                .collect(),
            more: view
                .has_more()
                .then(|| more_link(project, rev, path, page_num)),
        },
    };
    let head: DocumentHead = DocumentHead {
        title: format!("{project} / history / {path}"),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    document(&head, history_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current history view.
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
            label: "history".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav`): summary, the
/// shortlog and the verbose log, and the tree, all scoped to the same revision
/// when one was named.
fn nav(project: &str, rev: Option<&str>) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(scoped_href(project, "shortlog", "h", rev)),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(scoped_href(project, "log", "h", rev)),
        },
        NavItem {
            label: "tree".to_owned(),
            href: Some(scoped_href(project, "tree", "hb", rev)),
        },
    ]
}

/// An action link scoped to the named revision through `param`, or unscoped (the
/// adapter defaults that to HEAD) when no revision was named.
fn scoped_href(project: &str, action: &str, param: &str, rev: Option<&str>) -> String {
    match rev {
        Some(revision) => href(&[("p", project), ("a", action), (param, revision)]),
        None => href(&[("p", project), ("a", action)]),
    }
}

/// Maps a use-case history row to a render row, building the per-commit links.
/// The ftype/raw/commitdiff links are scoped to this commit; the "diff to
/// current" link (present only when the use case offered one) diffs this commit's
/// blob against the view's current one, mirroring gitweb's `blobdiff` params.
fn history_entry(
    project: &str,
    path: &str,
    rev: Option<&str>,
    view: &HistoryView,
    row: &HistoryRow,
) -> HistoryEntryView {
    let id: &str = row.id();
    let is_blob: bool = view.file_type() == ObjectKind::Blob;
    let ftype: &str = view.file_type().as_str();
    HistoryEntryView {
        date_displayed: row.date().displayed().to_owned(),
        date_tooltip: row.date().tooltip().to_owned(),
        author: row.author().to_owned(),
        author_short: row.author_short().to_owned(),
        title: row.title().to_owned(),
        title_short: row.title_short().to_owned(),
        commit: href(&[("p", project), ("a", "commit"), ("h", id)]),
        object: href(&[("p", project), ("a", ftype), ("hb", id), ("f", path)]),
        commitdiff: href(&[("p", project), ("a", "commitdiff"), ("h", id)]),
        raw: is_blob.then(|| href(&[("p", project), ("a", "blob_plain"), ("hb", id), ("f", path)])),
        diff_to_current: row
            .diff_to_current()
            .map(|parent: &str| blobdiff_href(project, path, rev, view.file_hash(), parent, id)),
    }
}

/// The "diff to current" link (gitweb's `blobdiff`): from this commit's blob
/// (`hash_parent` / `hash_parent_base` = the row's commit) to the view's current
/// blob (`hash`) on the base revision (`hash_base`, omitted to default to HEAD).
fn blobdiff_href(
    project: &str,
    path: &str,
    rev: Option<&str>,
    current: &str,
    parent: &str,
    commit: &str,
) -> String {
    match rev {
        Some(base) => href(&[
            ("p", project),
            ("a", "blobdiff"),
            ("h", current),
            ("hp", parent),
            ("hb", base),
            ("hpb", commit),
            ("f", path),
        ]),
        None => href(&[
            ("p", project),
            ("a", "blobdiff"),
            ("h", current),
            ("hp", parent),
            ("hpb", commit),
            ("f", path),
        ]),
    }
}

/// The "next page" affordance: a link to the following page of the same view.
fn more_link(project: &str, rev: Option<&str>, path: &str, page_num: usize) -> MoreLink {
    let next: String = (page_num + 1).to_string();
    let href: String = match rev {
        Some(revision) => href(&[
            ("p", project),
            ("a", "history"),
            ("hb", revision),
            ("f", path),
            ("pg", &next),
        ]),
        None => href(&[("p", project), ("a", "history"), ("f", path), ("pg", &next)]),
    };
    MoreLink {
        href,
        label: "next".to_owned(),
    }
}
