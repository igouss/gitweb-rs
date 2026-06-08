//! The diff-viewer host page (modernized `git_commitdiff` HTML view).
//!
//! gitweb's `git_commitdiff` reads `git diff-tree --patch-with-raw` and colours
//! the diff itself, line by line, with `format_diff_line` /
//! `git_patchset_body`. We modernize that: the diff body is no longer rendered
//! on the server at all. Instead this page ships a `#diff-root` container that
//! carries the URL of a *clean* unified diff (git's `diff --git` patch text, the
//! [`crate::markup`]-free output of the domain's `Patch::render`, served by the
//! diff-text endpoint), and boots a small client module that fetches that URL
//! and renders it with the vendored `@pierre/diffs` viewer.
//!
//! What stays the server's job is everything around the diff: the breadcrumb
//! header, the action navigation, the alternate-format affordances (`raw` /
//! `patch`), the commit subject as the page title, and — for the no-JavaScript
//! case — a fallback link straight to the raw diff. The rendered and empty-diff
//! states are produced entirely on the client (see `static/diff-viewer.js`) and
//! are covered by documented manual checks, consistent with the client-JS bead.
//!
//! Every URL is built by the web boundary, so this layer takes finished hrefs:
//! the `diff_url` the viewer fetches and the `viewer_module_src` of the boot
//! module. Every value is escaped by maud.

use crate::chrome::{
    Crumb, FormatLink, NavItem, PageFooter, footer, formats_nav, page_header, page_nav,
};
use crate::markup::{Markup, html};

/// The whole diff-viewer host page: the breadcrumb header, the action
/// navigation with the alternate-format affordances, the commit subject as the
/// title, an optional file path (for a single-file diff), and the diff-root
/// container the client viewer fills.
#[derive(Debug, Clone)]
pub struct DiffHostPage {
    /// Breadcrumb trail (home / project / commitdiff).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The alternate-format affordances shown on the right (raw / patch).
    pub formats: Vec<FormatLink>,
    /// The page title: the commit subject.
    pub title: String,
    /// The file path, shown when the diff is scoped to a single file (blobdiff).
    pub path: Option<String>,
    /// The URL of the clean unified diff the client viewer fetches, and that the
    /// no-JavaScript fallback links to.
    pub diff_url: String,
    /// The `src` of the boot module that renders the diff (a static asset).
    pub viewer_module_src: String,
}

/// Renders the diff host page body: the breadcrumb header, the action navigation
/// with the alternate-format affordances, the title and optional path, and the
/// `#diff-root` container — a loading placeholder plus a no-JavaScript fallback —
/// followed by the boot module. The caller wraps this in the document chrome.
#[must_use]
pub fn diff_host_body(page: &DiffHostPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, formats_nav(&page.formats)))
        main class="content diff-view" {
            h2 class="section-title" { (page.title) }
            @if let Some(path) = &page.path {
                p class="path" { (path) }
            }
            section id="diff-root" class="diff-root" data-diff-url=(page.diff_url) {
                p class="status" { "Loading diff…" }
                noscript {
                    p {
                        "JavaScript is required to render the diff inline. "
                        a href=(page.diff_url) { "View the raw unified diff" }
                        "."
                    }
                }
            }
            script type="module" src=(page.viewer_module_src) {}
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}
