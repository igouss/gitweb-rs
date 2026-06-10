//! The `shortlog` handler: gitweb's `git_shortlog` page.
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_shortlog`] use case over it from the requested revision
//! (HEAD by default) and page, maps the resulting view-model to the render
//! layer's table — building every link with [`href`], since URLs are the
//! boundary's job — wraps it in the document chrome, and returns the page as a
//! [`View`]. The clock lives here, at the boundary: the request time is read once
//! and handed to the use case so the date cells are computed against a real `now`
//! while the domain stays clock-free.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::ref_marker::RefMarker;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::{Page, Repository};
use gitweb_domain::usecase::shortlog::{ShortlogRow, ShortlogView, assemble_shortlog};
use gitweb_render::chrome::{Crumb, DocumentHead, FeedLink, MoreLink, NavItem, document};
use gitweb_render::markup::Markup;
use gitweb_render::shortlog::{ShortlogEntryView, ShortlogPage, ShortlogTable, shortlog_body};

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::feed_meta::{document_head, page_feeds};
use crate::handlers::ref_markers::marker_view;
use crate::response::View;
use crate::url::href;

/// gitweb's `git_log_generic` page size (`parse_commits($base, 101, 100*$page)`).
const PAGE_SIZE: usize = 100;

/// Serves the shortlog page over a wired project store and the resolved settings.
pub struct ShortlogHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl ShortlogHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for ShortlogHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let rev: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let page_num: usize = request.page.unwrap_or(0) as usize;
        let page: Page = Page::from_page(page_num, PAGE_SIZE);
        let view: ShortlogView = assemble_shortlog(repository.as_ref(), rev, now_epoch(), page)?;
        let feeds: Vec<FeedLink> = page_feeds(&self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            rev,
            page_num,
            &view,
            feeds,
        )))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(
    settings: &Settings,
    project: &str,
    rev: Option<&str>,
    page_num: usize,
    view: &ShortlogView,
    feeds: Vec<FeedLink>,
) -> Markup {
    let page: ShortlogPage = ShortlogPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, rev),
        table: ShortlogTable {
            rows: view
                .rows()
                .iter()
                .map(|row: &ShortlogRow| shortlog_entry(project, row))
                .collect(),
            more: view.has_more().then(|| more_link(project, rev, page_num)),
        },
    };
    let head: DocumentHead = document_head(format!("{project} / shortlog"), feeds);
    document(&head, shortlog_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current shortlog view.
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
            label: "shortlog".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav`): summary, the
/// current shortlog view as plain text, then the verbose log and the tree, all
/// scoped to the same revision when one was named.
fn nav(project: &str, rev: Option<&str>) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: None,
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

/// Maps a use-case shortlog row to a render row, building the per-commit links.
/// The date strings are already resolved by the domain; only the URLs are built
/// here, keyed on the commit id. Shared with the summary page's shortlog section.
pub(crate) fn shortlog_entry(project: &str, row: &ShortlogRow) -> ShortlogEntryView {
    let id: &str = row.id();
    ShortlogEntryView {
        date_displayed: row.date().displayed().to_owned(),
        date_tooltip: row.date().tooltip().to_owned(),
        author: row.author().to_owned(),
        author_short: row.author_short().to_owned(),
        title: row.title().to_owned(),
        title_short: row.title_short().to_owned(),
        commit: href(&[("p", project), ("a", "commit"), ("h", id)]),
        commitdiff: href(&[("p", project), ("a", "commitdiff"), ("h", id)]),
        tree: href(&[("p", project), ("a", "tree"), ("h", id), ("hb", id)]),
        refs: row
            .markers()
            .iter()
            .map(|marker: &RefMarker| marker_view(project, marker))
            .collect(),
    }
}

/// The "next page" affordance: a link to the following page of the same view.
fn more_link(project: &str, rev: Option<&str>, page_num: usize) -> MoreLink {
    let next: String = (page_num + 1).to_string();
    let href: String = match rev {
        Some(revision) => href(&[
            ("p", project),
            ("a", "shortlog"),
            ("h", revision),
            ("pg", &next),
        ]),
        None => href(&[("p", project), ("a", "shortlog"), ("pg", &next)]),
    };
    MoreLink {
        href,
        label: "next".to_owned(),
    }
}
