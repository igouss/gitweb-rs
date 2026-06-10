//! The `forks` handler: gitweb's `git_forks` page.
//!
//! Glue only. It reads the project and the requested sort order, runs the
//! [`assemble_forks`] use case over the wired [`ProjectStore`] (which scopes the
//! listing to the project's forks subdirectory and 404s `No forks found` when it
//! has none), and maps the resulting view-model to the render layer — reusing the
//! project-list handler's [`project_table`] mapper, so a fork links exactly as a
//! project on the landing page does, and wrapping it in the project-context
//! chrome (breadcrumbs, the per-project action bar, the `<project> forks` title).
//! The clock lives here, at the boundary: the request time is read once and
//! handed to the use case so the forks' ages are computed against a real `now`.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::usecase::forks::assemble_forks;
use gitweb_domain::usecase::project_list::ProjectListView;
use gitweb_render::chrome::{Crumb, DocumentHead, NavItem, document};
use gitweb_render::forks::{ForksPage, forks_body};
use gitweb_render::markup::Markup;

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::handlers::project_list::project_table;
use crate::response::View;
use crate::url::href;

/// Serves the forks page over a wired project store and the resolved settings.
pub struct ForksHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl ForksHandler {
    /// Wires the handler with the store it lists forks from and the settings it
    /// reads the site name, chrome, and the forks feature gate from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for ForksHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let view: ProjectListView = assemble_forks(
            self.store.as_ref(),
            &self.settings,
            project,
            request.order.as_deref(),
            now_epoch(),
        )?;
        Ok(View::html(render_page(&self.settings, project, &view)))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(settings: &Settings, project: &str, view: &ProjectListView) -> Markup {
    let page: ForksPage = ForksPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project),
        title: format!("{project} forks"),
        // The forks page's sort headers replay the forks action.
        list: project_table(view, &[("p", project), ("a", "forks")]),
    };
    let head: DocumentHead = DocumentHead {
        title: format!("{project} / forks"),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    document(&head, forks_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current forks view.
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
            label: "forks".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('', …)`): no tab is
/// current on the forks page, so summary, shortlog, log, and tree all link.
fn nav(project: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(href(&[("p", project), ("a", "shortlog")])),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(href(&[("p", project), ("a", "log")])),
        },
        NavItem {
            label: "tree".to_owned(),
            href: Some(href(&[("p", project), ("a", "tree")])),
        },
    ]
}
