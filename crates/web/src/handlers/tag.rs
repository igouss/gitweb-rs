//! The `tag` handler: gitweb's `git_tag` page.
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`show_tag`] use case over it for the request hash, maps the
//! resulting view-model to the render layer's tag page — building the tagged
//! object's link with [`href`], since URLs are the boundary's job — wraps it in
//! the document chrome, and returns the page as a [`View`]. gitweb's `git_tag`
//! shows the tagger's own absolute date, so no request clock is involved.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::tag::{TagAuthor, TagView, show_tag};
use gitweb_render::chrome::{Crumb, DocumentHead, document};
use gitweb_render::markup::Markup;
use gitweb_render::tag::{TagAuthorView, TagPage, TaggedObjectView, tag_body};

use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::response::View;
use crate::url::href;

/// Serves the single annotated-tag page over a wired project store and the
/// resolved settings.
pub struct TagHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl TagHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for TagHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let view: TagView = show_tag(repository.as_ref(), requested_hash(request))?;
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            &view,
            chrome,
        )))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and the object link.
fn render_page(settings: &Settings, project: &str, view: &TagView, chrome: PageChrome) -> Markup {
    let page: TagPage = TagPage {
        crumbs: crumbs(settings.site_name(), project, view.name()),
        name: view.name().to_owned(),
        object: object_view(project, view),
        tagger: view.tagger().map(author_view),
        message_lines: view.message().lines().map(str::to_owned).collect(),
    };
    let head: DocumentHead =
        document_head(format!("{project} / tag / {}", view.name()), chrome.feeds);
    document(&head, &chrome.foot, tag_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current tag.
fn crumbs(site_name: &str, project: &str, name: &str) -> Vec<Crumb> {
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
            label: name.to_owned(),
            href: None,
        },
    ]
}

/// The tagged-object row's view: the object id and kind label, both linking to
/// the object's own view (gitweb's `href(action=>type, hash=>object)`).
fn object_view(project: &str, view: &TagView) -> TaggedObjectView {
    let kind: &str = view.object_kind().as_str();
    TaggedObjectView {
        id: view.object().as_str().to_owned(),
        kind_label: kind.to_owned(),
        href: href(&[("p", project), ("a", kind), ("h", view.object().as_str())]),
    }
}

/// Maps the use-case tagger to the render authorship view.
fn author_view(author: &TagAuthor) -> TagAuthorView {
    TagAuthorView {
        name: author.name().to_owned(),
        email: author.email().map(str::to_owned),
        timestamp: author.timestamp().clone(),
    }
}

/// The request hash (gitweb's `$hash`), or the empty string when none was given
/// — which the use case resolves to nothing, gitweb's `die_error(404)`.
fn requested_hash(request: &Request) -> &str {
    request.hash.as_ref().map_or("", SafeRef::as_str)
}
