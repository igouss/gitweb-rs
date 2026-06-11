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

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project_filter::ProjectFilter;
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

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
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
        let filter: Option<ProjectFilter> = request.filter();
        let view: ProjectListView = assemble_project_list(
            self.store.as_ref(),
            &self.settings,
            request.order.as_deref(),
            filter.as_ref(),
            now_epoch(),
        )?;
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_page(&self.settings, &view, chrome)))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs that go in the head.
fn render_page(settings: &Settings, view: &ProjectListView, chrome: PageChrome) -> Markup {
    let page: ProjectListPage = ProjectListPage {
        site_name: settings.site_name().to_owned(),
        // The landing page's sort headers replay the bare request (no action), so
        // their re-sort base is empty: `?o=<key>`.
        list: project_table(view, &[]),
    };
    let head: DocumentHead = document_head(
        settings.site_name().to_owned(),
        chrome.feeds,
        chrome.scripts,
    );
    document(&head, &chrome.foot, project_list_body(&page))
}

/// Maps a use-case view to the render table, shared by the landing page and the
/// forks view. `replay` is the parameter base the sortable column headers re-sort
/// against — empty for the landing page (`?o=<key>`), `p=<project>;a=forks` for
/// the forks view — so each page's headers replay its own action.
pub(crate) fn project_table(view: &ProjectListView, replay: &[(&str, &str)]) -> ProjectList {
    let active: &str = view.order().as_str();
    ProjectList {
        project_header: sort_header("Project", "project", active, replay),
        description_header: sort_header("Description", "descr", active, replay),
        owner_header: sort_header("Owner", "owner", active, replay),
        age_header: sort_header("Last Change", "age", active, replay),
        forks_enabled: view.forks_enabled(),
        rows: view.rows().iter().map(render_row).collect(),
    }
}

/// One column header: a re-sort link (the replay base plus `o=<key>`), or plain
/// text for the active column.
fn sort_header(label: &str, key: &str, active: &str, replay: &[(&str, &str)]) -> SortHeader {
    let href: Option<String> = (key != active).then(|| {
        let mut params: Vec<(&str, &str)> = replay.to_vec();
        params.push(("o", key));
        href(&params)
    });
    SortHeader {
        label: label.to_owned(),
        href,
    }
}

/// Maps a use-case row to a render row, building the summary, quick links, and —
/// for a fork-capable project — the forks-view URL the leading `+` and `forks`
/// link point to.
fn render_row(row: &ProjectListRow) -> ProjectRow {
    let name: &str = row.name();
    let summary: String = href(&[("p", name), ("a", "summary")]);
    ProjectRow {
        name: name.to_owned(),
        href: summary.clone(),
        description: row.description().map(str::to_owned),
        owner: row.owner().map(str::to_owned),
        age: row.age(),
        fork_state: row.fork_state(),
        forks_href: href(&[("p", name), ("a", "forks")]),
        links: ProjectLinks {
            summary,
            shortlog: href(&[("p", name), ("a", "shortlog")]),
            log: href(&[("p", name), ("a", "log")]),
            tree: href(&[("p", name), ("a", "tree")]),
        },
    }
}
