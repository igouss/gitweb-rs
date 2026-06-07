//! The `project_list` handler: gitweb's `git_project_list` landing page.
//!
//! Glue only. It reads the requested sort order, runs the
//! [`assemble_project_list`] use case over the wired [`ProjectStore`], maps the
//! resulting view-model to the render layer's table — building each link with
//! [`href`], since URLs are the boundary's job — wraps it in the document
//! chrome, and returns the page as a [`View`]. The clock lives here, at the
//! boundary: the request time is read once and handed to the use case so the
//! ages are computed against a real `now` while the domain stays clock-free.

use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::usecase::project_list::{
    ProjectListRow, ProjectListView, assemble_project_list,
};
use gitweb_render::chrome::{DocumentHead, document};
use gitweb_render::markup::Markup;
use gitweb_render::project_list::{
    ProjectLinks, ProjectList, ProjectListPage, ProjectRow, SortHeader, project_list_body,
};

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href;

/// Serves the projects-list landing page over a wired project store and the
/// resolved global settings.
pub struct ProjectListHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl ProjectListHandler {
    /// Wires the handler with the store it lists and the settings it reads the
    /// default order, site name, and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for ProjectListHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let view: ProjectListView = assemble_project_list(
            self.store.as_ref(),
            &self.settings,
            request.order.as_deref(),
            now_epoch(),
        )?;
        Ok(View::html(render_page(&self.settings, &view)))
    }
}

/// The request-time epoch the relative ages are measured against. A clock before
/// the Unix epoch is clamped to 0 rather than panicking.
fn now_epoch() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|elapsed| i64::try_from(elapsed.as_secs()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs that go in the head.
fn render_page(settings: &Settings, view: &ProjectListView) -> Markup {
    let active: &str = view.order().as_str();
    let page: ProjectListPage = ProjectListPage {
        site_name: settings.site_name().to_owned(),
        list: ProjectList {
            project_header: sort_header("Project", "project", active),
            description_header: sort_header("Description", "descr", active),
            owner_header: sort_header("Owner", "owner", active),
            age_header: sort_header("Last Change", "age", active),
            rows: view.rows().iter().map(render_row).collect(),
        },
    };
    let head: DocumentHead = DocumentHead {
        title: settings.site_name().to_owned(),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    document(&head, project_list_body(&page))
}

/// One column header: a re-sort link, or plain text for the active column.
fn sort_header(label: &str, key: &str, active: &str) -> SortHeader {
    SortHeader {
        label: label.to_owned(),
        href: if key == active {
            None
        } else {
            Some(href(&[("o", key)]))
        },
    }
}

/// Maps a use-case row to a render row, building the summary and quick links.
fn render_row(row: &ProjectListRow) -> ProjectRow {
    let name: &str = row.name();
    let summary: String = href(&[("p", name), ("a", "summary")]);
    ProjectRow {
        name: name.to_owned(),
        href: summary.clone(),
        description: row.description().map(str::to_owned),
        owner: row.owner().map(str::to_owned),
        age: row.age(),
        links: ProjectLinks {
            summary,
            shortlog: href(&[("p", name), ("a", "shortlog")]),
            log: href(&[("p", name), ("a", "log")]),
            tree: href(&[("p", name), ("a", "tree")]),
        },
    }
}
