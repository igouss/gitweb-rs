//! The `opml` handler: gitweb's `git_opml`.
//!
//! Glue only. It runs the [`assemble_opml`] use case over the wired
//! [`ProjectStore`], maps the surviving projects into the render layer's
//! view-model — building each absolute `rss`/`summary` link with [`href_full`],
//! since URLs are the boundary's job, and chopping the displayed path the way
//! gitweb does (`chop_str($path, 25, 5)`) — and serves the serialized
//! `opml.xml`. gitweb sets `text/xml; charset=utf-8` and offers it inline under
//! that fixed filename. The action takes no project, so the request is ignored.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::chop::{ChopMode, chop_str};
use gitweb_domain::model::request::Request;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::usecase::opml::{Opml, OpmlProject, assemble_opml};
use gitweb_render::opml::{OpmlRowView, OpmlView, opml};

use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href_full;

/// gitweb's `text/xml; charset=utf-8` for the `opml.xml` body. Public so the
/// golden conformance test can assert it against gitweb's captured header.
pub const OPML_CONTENT_TYPE: &str = "text/xml; charset=utf-8";
/// gitweb's `-content_disposition` for `git_opml`. Public so the golden
/// conformance test can assert it against gitweb's captured header.
pub const OPML_DISPOSITION: &str = r#"inline; filename="opml.xml""#;

/// gitweb's `chop_str($proj{'path'}, 25, 5)` — the displayed-path bound.
const PATH_LEN: usize = 25;
/// gitweb's `chop_str` slack for the OPML path (`add_len`).
const PATH_SLACK: usize = 5;

/// Maps the domain outline into the render layer's view-model: the site name for
/// the `<title>`, and for each project its chopped path and the absolute
/// `rss`/`summary` URLs. `base` is the site URL (scheme and host, no trailing
/// slash). Shared by the handler and the golden conformance test so both produce
/// identical bytes.
#[must_use]
pub fn assemble_opml_view(outline: &Opml, settings: &Settings, base: &str) -> OpmlView {
    OpmlView {
        site_name: settings.site_name().to_owned(),
        rows: outline
            .projects()
            .iter()
            .map(|project: &OpmlProject| row_view(project, base))
            .collect(),
    }
}

/// Maps one outline project to its render row.
fn row_view(project: &OpmlProject, base: &str) -> OpmlRowView {
    let path: &str = project.path();
    OpmlRowView {
        text: chop_str(path, PATH_LEN, PATH_SLACK, ChopMode::Right),
        xml_url: href_full(base, &[("p", path), ("a", "rss")]),
        html_url: href_full(base, &[("p", path), ("a", "summary")]),
    }
}

/// Serves the OPML project outline over a wired project store and resolved
/// settings, with links absolute against the configured site `base` URL.
pub struct OpmlHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
    base: String,
}

impl OpmlHandler {
    /// Wires the handler with the store it lists, the settings it reads the site
    /// name from, and the site `base` URL the feed/summary links are absolute
    /// against.
    #[must_use]
    pub fn new(
        store: Arc<dyn ProjectStore + Send + Sync>,
        settings: Arc<Settings>,
        base: String,
    ) -> Self {
        Self {
            store,
            settings,
            base,
        }
    }
}

impl Handler for OpmlHandler {
    fn handle(&self, _request: &Request) -> Result<View, DomainError> {
        let outline: Opml = assemble_opml(self.store.as_ref())?;
        let view: OpmlView = assemble_opml_view(&outline, &self.settings, &self.base);
        Ok(View::text_attachment(
            OPML_CONTENT_TYPE,
            OPML_DISPOSITION,
            opml(&view),
        ))
    }
}
