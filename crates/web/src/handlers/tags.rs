//! The `tags` handler: gitweb's `git_tags` page.
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_tags`] use case over it, maps the resulting view-model to
//! the render layer's table — building each link with [`href`], since URLs are
//! the boundary's job — wraps it in the document chrome, and returns the page as
//! a [`View`]. The clock lives here, at the boundary: the request time is read
//! once and handed to the use case so ages are computed against a real `now`
//! while the domain stays clock-free.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::tags::{TagRow, TagsView, assemble_tags};
use gitweb_render::chrome::{Crumb, DocumentHead, document};
use gitweb_render::markup::Markup;
use gitweb_render::tags::{TagEntryView, TagReftype, TagsPage, TagsTable, tags_body};

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::handlers::refs::{CurrentRef, ref_views};
use crate::response::View;
use crate::url::href;

/// Serves the tags page over a wired project store and the resolved settings.
pub struct TagsHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl TagsHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for TagsHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let view: TagsView = assemble_tags(repository.as_ref(), now_epoch())?;
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
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(settings: &Settings, project: &str, view: &TagsView, chrome: PageChrome) -> Markup {
    let page: TagsPage = TagsPage {
        crumbs: crumbs(settings.site_name(), project),
        ref_views: ref_views(project, settings, CurrentRef::Tags),
        table: TagsTable {
            rows: view
                .rows()
                .iter()
                .map(|row: &TagRow| tag_entry(project, row))
                .collect(),
            // The standalone tags page lists every tag, so it never caps.
            more: None,
        },
    };
    let head: DocumentHead = document_head(format!("{project} / tags"), chrome.feeds);
    document(&head, &chrome.foot, tags_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then the
/// current tags view.
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
            label: "tags".to_owned(),
            href: None,
        },
    ]
}

/// Maps a use-case tag row to a render row, building every link. gitweb links
/// the name and the reftype label to the tagged object (via its kind's action,
/// keyed by `refid`), the subject and the `tag` selflink to the single tag view
/// (keyed by the tag's own id), and the reftype-dependent links off the ref.
/// Shared with the summary page's tags section.
pub(crate) fn tag_entry(project: &str, row: &TagRow) -> TagEntryView {
    TagEntryView {
        age: row.age(),
        name: row.name().to_owned(),
        object_href: object_href(project, row),
        subject: row.subject().map(str::to_owned),
        tag_view_href: href(&[("p", project), ("a", "tag"), ("h", row.tag_id().as_str())]),
        annotated: row.annotated(),
        reftype: reftype(project, row),
    }
}

/// gitweb's `href(action=>reftype, hash=>refid)`: the tagged object's view, by
/// its kind's action.
fn object_href(project: &str, row: &TagRow) -> String {
    href(&[
        ("p", project),
        ("a", row.reftype().as_str()),
        ("h", row.refid().as_str()),
    ])
}

/// Builds the reftype-dependent links. A commit tag links to the tag's shortlog
/// and log (keyed by the ref name); a blob tag links to the raw blob; a tree tag
/// offers nothing more. gitweb fully peels a tag's reftype, so only these three
/// object kinds occur here.
fn reftype(project: &str, row: &TagRow) -> TagReftype {
    match row.reftype() {
        ObjectKind::Commit => TagReftype::Commit {
            shortlog: href(&[("p", project), ("a", "shortlog"), ("h", row.full_name())]),
            log: href(&[("p", project), ("a", "log"), ("h", row.full_name())]),
        },
        ObjectKind::Blob => TagReftype::Blob {
            raw: href(&[
                ("p", project),
                ("a", "blob_plain"),
                ("h", row.refid().as_str()),
            ]),
        },
        ObjectKind::Tree | ObjectKind::Tag => TagReftype::Tree,
    }
}
