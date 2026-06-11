//! The blame handlers: gitweb's `git_blame` / `git_blame_incremental` /
//! `git_blame_data` (`git_blame_common`).
//!
//! Glue only. The three actions share one use case ([`assemble_blame`]) and one
//! gate — gitweb's `gitweb_check_feature('blame')` (`die_error(403, "Blame view
//! not allowed")`), read here at the boundary from the resolved [`Settings`]. A
//! request that names no file is gitweb's `die_error(400, "No file name given")`;
//! the base revision defaults to HEAD (gitweb's `$hash_base ||=
//! git_get_head_hash`).
//!
//! [`BlameHandler`] serves the server-rendered table (porcelain), building each
//! row's commit and line links with [`href`]. [`BlameIncrementalHandler`] serves
//! the empty-row shell that boots the shipped client module over the blame_data
//! URL and the project URL. [`BlameDataHandler`] serves the `git blame
//! --incremental` text the module streams — a `text/plain` [`View`], no render
//! layer beyond the stream builder. The line-number links point at the blamed
//! commit itself (gitweb's fallback when a line has no recorded `previous`).

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::blame::{BlameGroup, BlameRow, BlameView, assemble_blame};
use gitweb_render::blame::{
    BlameDataGroup, BlameIncrementalPage, BlamePage, BlameShellLine, BlameTableGroup,
    BlameTableLine, blame_body, blame_data, blame_incremental_body,
};
use gitweb_render::chrome::{Crumb, DocumentHead, FormatLink, NavItem, document};
use gitweb_render::markup::Markup;

use crate::assets::BLAME_INCREMENTAL_PATH;
use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::response::View;
use crate::url::href;

/// `text/plain; charset=utf-8` — gitweb's blame_data header.
const BLAME_DATA_MIME: &str = "text/plain; charset=utf-8";

/// The inputs every blame action shares, lifted from the request once: the
/// project, the tracked file, and the optional base revision.
struct BlameInputs<'a> {
    project: &'a str,
    file: &'a str,
    base: Option<&'a str>,
}

impl<'a> BlameInputs<'a> {
    /// Lifts the shared inputs from the request, enforcing gitweb's
    /// `die_error(400)` cases: a project and a file name are both required.
    fn from_request(request: &'a Request) -> Result<Self, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let file: &str = request
            .file_name
            .as_ref()
            .map(SafePath::as_str)
            .ok_or_else(|| DomainError::Invalid("No file name given".to_owned()))?;
        let base: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        Ok(Self {
            project,
            file,
            base,
        })
    }
}

/// gitweb's `gitweb_check_feature('blame') or die_error(403, ...)` — the gate the
/// three actions share, read from the resolved settings.
fn require_blame_enabled(settings: &Settings) -> Result<(), DomainError> {
    if settings.feature(FeatureName::Blame).enabled() {
        Ok(())
    } else {
        Err(DomainError::Forbidden("Blame view not allowed".to_owned()))
    }
}

/// Assembles the blame over a freshly-opened repository for the shared inputs.
fn assemble(
    store: &(dyn ProjectStore + Send + Sync),
    inputs: &BlameInputs<'_>,
) -> Result<BlameView, DomainError> {
    let repository: Box<dyn Repository> = store.open(inputs.project)?;
    assemble_blame(repository.as_ref(), inputs.base, inputs.file)
}

/// Serves the server-rendered blame table (gitweb's `git_blame`, porcelain mode).
pub struct BlameHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl BlameHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the `blame` gate and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for BlameHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        require_blame_enabled(&self.settings)?;
        let inputs: BlameInputs<'_> = BlameInputs::from_request(request)?;
        let view: BlameView = assemble(self.store.as_ref(), &inputs)?;
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_table(
            &self.settings,
            &inputs,
            &view,
            chrome,
        )))
    }
}

/// Serves the incremental blame shell (gitweb's `git_blame_incremental`).
pub struct BlameIncrementalHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl BlameIncrementalHandler {
    /// Wires the handler with the store and the settings it reads the gate and
    /// chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for BlameIncrementalHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        require_blame_enabled(&self.settings)?;
        let inputs: BlameInputs<'_> = BlameInputs::from_request(request)?;
        let view: BlameView = assemble(self.store.as_ref(), &inputs)?;
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_shell(
            &self.settings,
            &inputs,
            &view,
            chrome,
        )))
    }
}

/// Serves the blame_data stream (gitweb's `git_blame_data`).
pub struct BlameDataHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl BlameDataHandler {
    /// Wires the handler with the store and the settings it reads the gate from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for BlameDataHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        require_blame_enabled(&self.settings)?;
        let inputs: BlameInputs<'_> = BlameInputs::from_request(request)?;
        let view: BlameView = assemble(self.store.as_ref(), &inputs)?;
        let groups: Vec<BlameDataGroup> = view
            .groups()
            .iter()
            .map(|group: &BlameGroup| data_group(group, inputs.file))
            .collect();
        Ok(View::text(BLAME_DATA_MIME, blame_data(&groups)))
    }
}

/// Maps the use-case view to the server-rendered table page and wraps it in the
/// document chrome.
fn render_table(
    settings: &Settings,
    inputs: &BlameInputs<'_>,
    view: &BlameView,
    chrome: PageChrome,
) -> Markup {
    let groups: Vec<BlameTableGroup> = view
        .groups()
        .iter()
        .map(|group: &BlameGroup| table_group(inputs, group, view.rows()))
        .collect();
    let page: BlamePage = BlamePage {
        crumbs: crumbs(settings.site_name(), inputs.project),
        nav: nav(inputs),
        formats: formats(inputs, "blame_incremental", "blame (incremental)"),
        title: view.commit_title().to_owned(),
        path: inputs.file.to_owned(),
        groups,
    };
    let head: DocumentHead = document_head(
        title(inputs.project, inputs.file),
        chrome.feeds,
        chrome.scripts,
    );
    document(&head, &chrome.foot, blame_body(&page))
}

/// Maps the use-case view to the incremental shell page and wraps it in the
/// document chrome.
fn render_shell(
    settings: &Settings,
    inputs: &BlameInputs<'_>,
    view: &BlameView,
    chrome: PageChrome,
) -> Markup {
    let lines: Vec<BlameShellLine> = view
        .rows()
        .iter()
        .map(|row: &BlameRow| BlameShellLine {
            final_lineno: row.final_lineno(),
            text: row.content().to_owned(),
        })
        .collect();
    let page: BlameIncrementalPage = BlameIncrementalPage {
        crumbs: crumbs(settings.site_name(), inputs.project),
        nav: nav(inputs),
        formats: formats(inputs, "blame", "blame (non-incremental)"),
        title: view.commit_title().to_owned(),
        path: inputs.file.to_owned(),
        lines,
        data_url: data_href(inputs),
        project_url: href(&[("p", inputs.project)]),
        module_src: BLAME_INCREMENTAL_PATH.to_owned(),
    };
    let head: DocumentHead = document_head(
        title(inputs.project, inputs.file),
        chrome.feeds,
        chrome.scripts,
    );
    document(&head, &chrome.foot, blame_incremental_body(&page))
}

/// Maps one use-case group to the server-rendered table group, building the
/// commit link and each line's line-number link. The line-number link points at
/// the blame of the group's own commit and the queried file (gitweb's fallback
/// when a line has no recorded `previous`), anchored at the line's original
/// number.
fn table_group(inputs: &BlameInputs<'_>, group: &BlameGroup, rows: &[BlameRow]) -> BlameTableGroup {
    let lines: Vec<BlameTableLine> = rows
        .iter()
        .filter(|row: &&BlameRow| row.commit() == group.commit())
        .filter(|row: &&BlameRow| row_in_group(row, group))
        .map(|row: &BlameRow| BlameTableLine {
            final_lineno: row.final_lineno(),
            orig_lineno: row.orig_lineno(),
            linenr_href: linenr_href(inputs, group.commit(), row.orig_lineno()),
            text: row.content().to_owned(),
        })
        .collect();
    BlameTableGroup {
        short_commit: group.short_commit().to_owned(),
        commit_href: href(&[
            ("p", inputs.project),
            ("a", "commit"),
            ("h", group.commit()),
        ]),
        tooltip: format!("{}, {}", group.author(), group.date()),
        lines,
    }
}

/// Whether a row's final line number falls within `group`'s span (the group owns
/// the consecutive run `[final_lineno, final_lineno + numlines)`).
fn row_in_group(row: &BlameRow, group: &BlameGroup) -> bool {
    let start: usize = group.final_lineno();
    row.final_lineno() >= start && row.final_lineno() < start + group.numlines()
}

/// Maps one use-case group to a blame_data record: the queried file is its
/// `filename` (gitweb's blame falls back to the queried name when a line has no
/// renamed source).
fn data_group(group: &BlameGroup, file: &str) -> BlameDataGroup {
    BlameDataGroup {
        commit: group.commit().to_owned(),
        orig_lineno: group.orig_lineno(),
        final_lineno: group.final_lineno(),
        numlines: group.numlines(),
        author: group.author().to_owned(),
        author_time: group.author_time(),
        author_tz: group.author_tz().to_owned(),
        filename: file.to_owned(),
    }
}

/// The line-number link: the blame of `commit` and the queried file, anchored at
/// the line's original number (gitweb's `$blamed#l$orig_lineno`).
fn linenr_href(inputs: &BlameInputs<'_>, commit: &str, orig_lineno: usize) -> String {
    format!(
        "{}#l{orig_lineno}",
        href(&[
            ("p", inputs.project),
            ("a", "blame"),
            ("hb", commit),
            ("f", inputs.file),
        ])
    )
}

/// The blame_data URL the incremental shell streams — the same base/file, the
/// `blame_data` action (gitweb's `href(action=>"blame_data", -replay=>1)`).
fn data_href(inputs: &BlameInputs<'_>) -> String {
    let mut params: Vec<(&str, &str)> = vec![("p", inputs.project), ("a", "blame_data")];
    if let Some(base) = inputs.base {
        params.push(("hb", base));
    }
    params.push(("f", inputs.file));
    href(&params)
}

/// The document `<title>`: `project / blame / file`.
fn title(project: &str, file: &str) -> String {
    format!("{project} / blame / {file}")
}

/// The breadcrumb trail: home, the project (its summary), then blame.
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
            label: "blame".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar: summary and the file's history, scoped to the base
/// revision when one was named.
fn nav(inputs: &BlameInputs<'_>) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", inputs.project), ("a", "summary")])),
        },
        NavItem {
            label: "history".to_owned(),
            href: Some(file_action_href(inputs, "history")),
        },
    ]
}

/// The file affordances (gitweb's `formats_nav`): the blob, the *other* blame
/// rendering (`alt_action`/`alt_label`), the history, and the raw file at HEAD.
fn formats(inputs: &BlameInputs<'_>, alt_action: &str, alt_label: &str) -> Vec<FormatLink> {
    vec![
        FormatLink {
            label: "blob".to_owned(),
            href: file_action_href(inputs, "blob"),
        },
        FormatLink {
            label: alt_label.to_owned(),
            href: file_action_href(inputs, alt_action),
        },
        FormatLink {
            label: "history".to_owned(),
            href: file_action_href(inputs, "history"),
        },
        FormatLink {
            label: "HEAD".to_owned(),
            href: href(&[
                ("p", inputs.project),
                ("a", "blame"),
                ("hb", "HEAD"),
                ("f", inputs.file),
            ]),
        },
    ]
}

/// A per-file action link: the action, the base revision when named, and the
/// file path.
fn file_action_href(inputs: &BlameInputs<'_>, action: &str) -> String {
    let mut params: Vec<(&str, &str)> = vec![("p", inputs.project), ("a", action)];
    if let Some(base) = inputs.base {
        params.push(("hb", base));
    }
    params.push(("f", inputs.file));
    href(&params)
}
