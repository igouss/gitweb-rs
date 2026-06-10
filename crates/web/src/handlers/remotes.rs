//! The `remotes` handler: gitweb's `git_remotes` page.
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! reads the `hash` parameter as the single-remote selector, runs the
//! [`assemble_remotes`] use case over the repository (which enforces the
//! `remote_heads` feature gate, so a disabled feature surfaces as the use case's
//! [`DomainError::Forbidden`] that the web boundary maps to gitweb's 403), and
//! maps the resulting view-model to the render layer — building every link with
//! [`href`], since URLs are the boundary's job. The tracking-branch rows reuse the
//! heads handler's [`head_entry`] mapper, so a remote's branches link exactly as
//! the heads page's do. The clock lives here, at the boundary: the request time
//! is read once and handed to the use case for the tracking-branch ages.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::remote::RemoteUrl;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::remotes::{RemoteBlock, RemotesView, assemble_remotes};
use gitweb_render::chrome::{Crumb, DocumentHead, FeedLink, document};
use gitweb_render::heads::HeadsTable;
use gitweb_render::markup::Markup;
use gitweb_render::remotes::{RemoteBlockView, RemoteUrlLine, RemotesPage, remotes_body};

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::feed_meta::{document_head, page_feeds};
use crate::handlers::heads::head_entry;
use crate::handlers::refs::{CurrentRef, ref_views};
use crate::response::View;
use crate::url::href;

/// Serves the remotes page over a wired project store and the resolved settings.
pub struct RemotesHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl RemotesHandler {
    /// Wires the handler with the store it opens projects from and the settings it
    /// reads the site name, chrome, and the `remote_heads` feature gate from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for RemotesHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let selected: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let view: RemotesView =
            assemble_remotes(repository.as_ref(), &self.settings, selected, now_epoch())?;
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
    view: &RemotesView,
    feeds: Vec<FeedLink>,
) -> Markup {
    // The all-remotes view marks `remotes` current in the refs bar; the
    // single-remote view marks nothing (gitweb's `format_ref_views('')`), so its
    // `remotes` entry links back to the all-remotes view.
    let current: CurrentRef = if view.is_single() {
        CurrentRef::None
    } else {
        CurrentRef::Remotes
    };
    let page: RemotesPage = RemotesPage {
        crumbs: crumbs(settings.site_name(), project),
        ref_views: ref_views(project, settings, current),
        title: title(project, view),
        blocks: view
            .blocks()
            .iter()
            .map(|block: &RemoteBlock| block_view(project, block, view.is_single()))
            .collect(),
    };
    let head: DocumentHead = document_head(format!("{project} / remotes"), feeds);
    document(&head, remotes_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current remotes view.
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
            label: "remotes".to_owned(),
            href: None,
        },
    ]
}

/// The page title: gitweb's `"$remote remote for $project"` for the single-remote
/// view, `"$project remotes"` for the all-remotes view.
fn title(project: &str, view: &RemotesView) -> String {
    if view.is_single() {
        let name: &str = view.blocks().first().map_or("", RemoteBlock::name);
        format!("{name} remote for {project}")
    } else {
        format!("{project} remotes")
    }
}

/// One remote's render block: its URL lines and tracking branches, and — only in
/// the all-remotes view — a link from its name to its single-remote view.
fn block_view(project: &str, block: &RemoteBlock, single: bool) -> RemoteBlockView {
    RemoteBlockView {
        name: block.name().to_owned(),
        href: (!single).then(|| href(&[("p", project), ("a", "remotes"), ("h", block.name())])),
        urls: block.urls().iter().map(url_line).collect(),
        heads: HeadsTable {
            // gitweb passes no limit to the remotes' heads body, so every tracking
            // branch is shown and there is never a "more" row.
            rows: block
                .heads()
                .iter()
                .map(|row| head_entry(project, row))
                .collect(),
            more: None,
        },
    }
}

/// Maps a domain URL line to its render row: gitweb's `format_repo_url` label for
/// each kind, and the `No remote URL` placeholder text for a remote with none.
fn url_line(url: &RemoteUrl) -> RemoteUrlLine {
    match url {
        RemoteUrl::Combined(value) => labelled("URL", value),
        RemoteUrl::Fetch(value) => labelled("Fetch URL", value),
        RemoteUrl::Push(value) => labelled("Push URL", value),
        RemoteUrl::Missing => RemoteUrlLine {
            label: None,
            value: "No remote URL".to_owned(),
        },
    }
}

/// A labelled URL line.
fn labelled(label: &str, value: &str) -> RemoteUrlLine {
    RemoteUrlLine {
        label: Some(label.to_owned()),
        value: value.to_owned(),
    }
}
