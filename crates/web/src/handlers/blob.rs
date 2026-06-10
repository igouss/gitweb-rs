//! The `blob` handler: gitweb's `git_blob` page (one file at a revision).
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_blob`] use case over it for the requested hash / base /
//! file name, then maps the resulting view-model to the render layer's page —
//! building every link with [`href`], since URLs are the boundary's job — wraps
//! it in the document chrome, and returns the page as a [`View`]. The `blame`
//! affordance is gated on the `blame` feature (read here, at the boundary) and a
//! text blob, the way gitweb's `$have_blame &&= text mimetype` does.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blob::BlobDisplay;
use gitweb_domain::model::encoding::FallbackEncoding;
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::blob::{BlobView, assemble_blob};
use gitweb_render::blob::{BlobContent, BlobLine, BlobPage, blob_body};
use gitweb_render::chrome::{Crumb, DocumentHead, FeedLink, FormatLink, NavItem, document};
use gitweb_render::markup::Markup;

use crate::dispatch::Handler;
use crate::feed_meta::{document_head, page_feeds};
use crate::response::View;
use crate::url::href;

/// Serves the blob page over a wired project store and the resolved settings.
pub struct BlobHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl BlobHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name, chrome, and `blame` feature from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for BlobHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let base: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        let hash: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let file: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        let view: BlobView = assemble_blob(
            repository.as_ref(),
            base,
            hash,
            file,
            FallbackEncoding::Latin1,
        )?;
        // gitweb's `$hash =~ /^$oid_regex$/`: a blob addressed by a literal oid is
        // immutable and cacheable for a day; one resolved through a file name is
        // not (`$hash` is then undefined when the expires check runs).
        let feeds: Vec<FeedLink> = page_feeds(&self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            base,
            file,
            &view,
            feeds,
        ))
        .with_expiry(Expiry::for_hash(hash)))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(
    settings: &Settings,
    project: &str,
    base: Option<&str>,
    file: Option<&str>,
    view: &BlobView,
    feeds: Vec<FeedLink>,
) -> Markup {
    let page: BlobPage = BlobPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, base),
        formats: formats(settings, project, base, file, view),
        title: view
            .commit_title()
            .unwrap_or_else(|| view.blob_id())
            .to_owned(),
        path: file.map(str::to_owned),
        content: content(project, base, file, view),
    };
    let head: DocumentHead = document_head(blob_title(project, file), feeds);
    document(&head, blob_body(&page))
}

/// The document `<title>`: `project / blob / file`, or `project / blob` for a
/// loose blob with no file name.
fn blob_title(project: &str, file: Option<&str>) -> String {
    match file {
        Some(file) => format!("{project} / blob / {file}"),
        None => format!("{project} / blob"),
    }
}

/// The breadcrumb trail: home, the project (linking to its summary), then blob.
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
            label: "blob".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar: summary, the shortlog and verbose log, all scoped
/// to the same revision when one was named.
fn nav(project: &str, base: Option<&str>) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(scoped(project, "shortlog", base)),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(scoped(project, "log", base)),
        },
    ]
}

/// An action link scoped to the named revision through `h`, or unscoped (the
/// adapter defaults that to HEAD) when no revision was named.
fn scoped(project: &str, action: &str, base: Option<&str>) -> String {
    match base {
        Some(base) => href(&[("p", project), ("a", action), ("h", base)]),
        None => href(&[("p", project), ("a", action)]),
    }
}

/// The file content view-model: a text blob's numbered lines, an image's inline
/// `<img>` source, or a binary's raw download link.
fn content(project: &str, base: Option<&str>, file: Option<&str>, view: &BlobView) -> BlobContent {
    match view.display() {
        BlobDisplay::Text => BlobContent::Text {
            lines: numbered(view.lines()),
        },
        BlobDisplay::Image => BlobContent::Image {
            src: raw_href(project, view.blob_id(), base, file),
            alt: file.unwrap_or_else(|| view.blob_id()).to_owned(),
        },
        BlobDisplay::Binary => BlobContent::Binary {
            raw_href: raw_href(project, view.blob_id(), base, file),
        },
    }
}

/// Numbers the decoded lines from 1, as gitweb's `$nr` counter does.
fn numbered(lines: &[String]) -> Vec<BlobLine> {
    lines
        .iter()
        .enumerate()
        .map(|(index, text): (usize, &String)| BlobLine {
            number: index + 1,
            text: text.clone(),
        })
        .collect()
}

/// The file affordances (gitweb's `formats_nav`). With a file name: blame (when
/// the feature is on and the blob is text) | history | raw | HEAD. For a loose
/// blob (no file name) only the raw download, gitweb's `else` branch.
fn formats(
    settings: &Settings,
    project: &str,
    base: Option<&str>,
    file: Option<&str>,
    view: &BlobView,
) -> Vec<FormatLink> {
    let Some(file) = file else {
        return vec![FormatLink {
            label: "raw".to_owned(),
            href: raw_href(project, view.blob_id(), base, None),
        }];
    };
    let mut formats: Vec<FormatLink> = Vec::new();
    let blame_on: bool = settings.feature(FeatureName::Blame).enabled();
    if blame_on && view.display() == BlobDisplay::Text {
        formats.push(FormatLink {
            label: "blame".to_owned(),
            href: file_action_href(project, "blame", base, file),
        });
    }
    formats.push(FormatLink {
        label: "history".to_owned(),
        href: file_action_href(project, "history", base, file),
    });
    formats.push(FormatLink {
        label: "raw".to_owned(),
        href: raw_href(project, view.blob_id(), base, Some(file)),
    });
    formats.push(FormatLink {
        label: "HEAD".to_owned(),
        href: href(&[("p", project), ("a", "blob"), ("hb", "HEAD"), ("f", file)]),
    });
    formats
}

/// The raw blob view (gitweb's `blob_plain` / `<img>` source): the blob id, plus
/// the base revision and file name when known.
fn raw_href(project: &str, blob_id: &str, base: Option<&str>, file: Option<&str>) -> String {
    let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", "blob_plain"), ("h", blob_id)];
    if let Some(base) = base {
        params.push(("hb", base));
    }
    if let Some(file) = file {
        params.push(("f", file));
    }
    href(&params)
}

/// A per-file action link (history, blame): the action, the base revision when
/// known, and the file path.
fn file_action_href(project: &str, action: &str, base: Option<&str>, file: &str) -> String {
    let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", action)];
    if let Some(base) = base {
        params.push(("hb", base));
    }
    params.push(("f", file));
    href(&params)
}
