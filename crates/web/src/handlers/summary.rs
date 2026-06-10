//! The `summary` handler: gitweb's `git_summary`, the default landing page.
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! reads its metadata and README from the same store, runs the
//! [`assemble_summary`] use case over the three, then maps the resulting
//! view-model to the render layer — building every link with [`href`], since URLs
//! are the boundary's job. The section row mappers are shared with the standalone
//! heads / tags / shortlog handlers, so the summary's sections link exactly as
//! those pages do. The clock lives here, at the boundary: the request time is
//! read once and handed to the use case for the section ages.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::summary::{SummaryView, assemble_summary};
use gitweb_render::chrome::{Crumb, DocumentHead, FeedLink, MoreLink, NavItem, document};
use gitweb_render::heads::HeadsTable;
use gitweb_render::markup::Markup;
use gitweb_render::shortlog::ShortlogTable;
use gitweb_render::summary::{
    HeadsSection, ShortlogSection, SummaryMetadata, SummaryPage, TagsSection, summary_body,
};
use gitweb_render::tags::TagsTable;

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::feed_meta::{document_head, page_feeds};
use crate::handlers::heads::head_entry;
use crate::handlers::shortlog::shortlog_entry;
use crate::handlers::tags::tag_entry;
use crate::response::View;
use crate::url::href;

/// Serves the summary page over a wired project store and the resolved settings.
pub struct SummaryHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl SummaryHandler {
    /// Wires the handler with the store it opens projects and reads metadata
    /// from, and the settings it reads the site name, the `omit_owner` /
    /// `prevent_xss` gates, and the clone-URL base list from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for SummaryHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let info: ProjectInfo = self.store.info(project)?;
        let readme: Option<String> = self.store.readme_html(project)?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let view: SummaryView = assemble_summary(
            repository.as_ref(),
            &info,
            readme.as_deref(),
            &self.settings,
            now_epoch(),
        )?;
        let feeds: Vec<FeedLink> = page_feeds(&self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
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
    view: &SummaryView,
    feeds: Vec<FeedLink>,
) -> Markup {
    let page: SummaryPage = SummaryPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project),
        metadata: metadata(view),
        readme: view.readme().map(str::to_owned),
        shortlog: shortlog_section(project, view),
        tags: tags_section(project, view),
        heads: heads_section(project, view),
    };
    let head: DocumentHead = document_head(project.to_owned(), feeds);
    document(&head, summary_body(&page))
}

/// The breadcrumb trail: home, then the project itself (the current page).
fn crumbs(site_name: &str, project: &str) -> Vec<Crumb> {
    vec![
        Crumb {
            label: site_name.to_owned(),
            href: Some(href(&[])),
        },
        Crumb {
            label: project.to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('summary', …)`): the
/// current summary view as plain text, then the shortlog, the verbose log, and
/// the tree.
fn nav(project: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: None,
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

/// The metadata block, lifting the already-resolved fields off the view (the
/// description placeholder, the owner gate, and the clone-URL fallback are the
/// use case's calls).
fn metadata(view: &SummaryView) -> SummaryMetadata {
    SummaryMetadata {
        description: view.description().to_owned(),
        owner: view.owner().map(str::to_owned),
        last_change: view.last_change().cloned(),
        clone_urls: view.clone_urls().to_vec(),
    }
}

/// The shortlog section, present only when there is history. Its header and its
/// "..." overflow link both point at the full shortlog action.
fn shortlog_section(project: &str, view: &SummaryView) -> Option<ShortlogSection> {
    if view.shortlog().rows().is_empty() {
        return None;
    }
    let action: String = href(&[("p", project), ("a", "shortlog")]);
    Some(ShortlogSection {
        href: action.clone(),
        table: ShortlogTable {
            rows: view
                .shortlog()
                .rows()
                .iter()
                .map(|row| shortlog_entry(project, row))
                .collect(),
            more: view.shortlog().has_more().then(|| more(action)),
        },
    })
}

/// The tags section, present only when there are tags. Its header and "..." link
/// both point at the full tags action.
fn tags_section(project: &str, view: &SummaryView) -> Option<TagsSection> {
    if view.tags().shown().is_empty() {
        return None;
    }
    let action: String = href(&[("p", project), ("a", "tags")]);
    Some(TagsSection {
        href: action.clone(),
        table: TagsTable {
            rows: view
                .tags()
                .shown()
                .iter()
                .map(|row| tag_entry(project, row))
                .collect(),
            more: view.tags().is_truncated().then(|| more(action)),
        },
    })
}

/// The heads section, present only when there are branches. Its header and "..."
/// link both point at the full heads action.
fn heads_section(project: &str, view: &SummaryView) -> Option<HeadsSection> {
    if view.heads().shown().is_empty() {
        return None;
    }
    let action: String = href(&[("p", project), ("a", "heads")]);
    Some(HeadsSection {
        href: action.clone(),
        table: HeadsTable {
            rows: view
                .heads()
                .shown()
                .iter()
                .map(|row| head_entry(project, row))
                .collect(),
            more: view.heads().is_truncated().then(|| more(action)),
        },
    })
}

/// gitweb's summary "..." overflow link to a section's full action.
fn more(href: String) -> MoreLink {
    MoreLink {
        href,
        label: "...".to_owned(),
    }
}
