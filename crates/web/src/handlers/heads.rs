//! The `heads` handler: gitweb's `git_heads` page.
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_heads`] use case over it, maps the resulting view-model to
//! the render layer's table — building each link with [`href`], since URLs are
//! the boundary's job — wraps it in the document chrome, and returns the page as
//! a [`View`]. The clock lives here, at the boundary: the request time is read
//! once and handed to the use case so ages are computed against a real `now`
//! while the domain stays clock-free.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::heads::{HeadRow, HeadsView, assemble_heads};
use gitweb_render::chrome::{Crumb, DocumentHead, NavItem, document};
use gitweb_render::heads::{HeadEntryView, HeadsPage, HeadsTable, heads_body};
use gitweb_render::markup::Markup;

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href;

/// Serves the heads page over a wired project store and the resolved settings.
pub struct HeadsHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl HeadsHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for HeadsHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let view: HeadsView = assemble_heads(repository.as_ref(), now_epoch())?;
        Ok(View::html(render_page(&self.settings, project, &view)))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(settings: &Settings, project: &str, view: &HeadsView) -> Markup {
    let page: HeadsPage = HeadsPage {
        crumbs: crumbs(settings.site_name(), project),
        ref_views: ref_views(project),
        table: HeadsTable {
            rows: view
                .rows()
                .iter()
                .map(|row| head_entry(project, row))
                .collect(),
            // The standalone heads page lists every branch, so it never caps.
            more: None,
        },
    };
    let head: DocumentHead = DocumentHead {
        title: format!("{project} / heads"),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    document(&head, heads_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current heads view.
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
            label: "heads".to_owned(),
            href: None,
        },
    ]
}

/// The refs sub-navigation (gitweb's `format_ref_views`): tags links away, the
/// current heads view is plain text. The remotes view is added by its own
/// capability when the `remote_heads` feature is enabled.
fn ref_views(project: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "tags".to_owned(),
            href: Some(href(&[("p", project), ("a", "tags")])),
        },
        NavItem {
            label: "heads".to_owned(),
            href: None,
        },
    ]
}

/// Maps a use-case head row to a render row, building the per-branch links.
/// gitweb links the name and shortlog to the branch shortlog, log to its log,
/// and tree to its tree (with the branch as both hash and hash-base). Shared
/// with the summary page's heads section.
pub(crate) fn head_entry(project: &str, row: &HeadRow) -> HeadEntryView {
    let full: &str = row.full_name();
    HeadEntryView {
        name: row.name().to_owned(),
        shortlog: href(&[("p", project), ("a", "shortlog"), ("h", full)]),
        log: href(&[("p", project), ("a", "log"), ("h", full)]),
        tree: href(&[("p", project), ("a", "tree"), ("h", full), ("hb", full)]),
        age: row.age(),
        current: row.current(),
    }
}
