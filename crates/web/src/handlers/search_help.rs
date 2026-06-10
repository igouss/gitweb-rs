//! The `search_help` handler: gitweb's `git_search_help` page.
//!
//! Glue only. gitweb's search-help page is always available — it has no `search`
//! feature gate, only the per-section `grep`/`pickaxe` gates the use case applies
//! — but it is a per-project action, so a request for a project that does not
//! exist is gitweb's 404 "No such project". That existence check is the one thing
//! that touches the repository here: the handler opens the project through the
//! wired [`ProjectStore`] (404 on a miss) and then ignores it, because the page
//! reads no repository object. It runs the [`assemble_search_help`] use case over
//! the resolved [`Settings`] for the topic list, builds the chrome (breadcrumbs
//! and the per-project action nav, scoped to the requested revision), and wraps
//! the rendered body in the document — every URL built with [`href`], the
//! boundary's job.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::search_help::{SearchHelpView, assemble_search_help};
use gitweb_render::chrome::{Crumb, DocumentHead, NavItem, document};
use gitweb_render::markup::Markup;
use gitweb_render::search_help::{SearchHelpPage, search_help_body};

use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::response::View;
use crate::url::href;

/// Serves the search-help page over a wired project store and the resolved
/// settings (which carry the `grep`/`pickaxe` gates the use case reads).
pub struct SearchHelpHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl SearchHelpHandler {
    /// Wires the handler with the store it opens projects from (for the
    /// existence check) and the settings it reads the chrome and feature gates
    /// from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for SearchHelpHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        // gitweb's per-project actions 404 a project that does not exist; the page
        // itself reads nothing further from the repository.
        let _repository: Box<dyn Repository> = self.store.open(project)?;
        let view: SearchHelpView = assemble_search_help(&self.settings);
        let rev: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            rev,
            &view,
            chrome,
        )))
    }
}

/// Maps the use-case view to the render view-model and wraps the body in the
/// document chrome — the boundary owns the asset URLs and every nav link.
fn render_page(
    settings: &Settings,
    project: &str,
    rev: Option<&str>,
    view: &SearchHelpView,
    chrome: PageChrome,
) -> Markup {
    let page: SearchHelpPage = SearchHelpPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, rev),
        topics: view.topics().to_vec(),
    };
    let head: DocumentHead = document_head(format!("{project} / search help"), chrome.feeds);
    document(&head, &chrome.foot, search_help_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// search-help page.
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
            label: "search help".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('','',...)` with
/// nothing marked current): summary, shortlog, log, tree — every item a link,
/// scoped to the requested revision when one was named.
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
