//! The commitdiff host page (modernized `git_commitdiff` HTML view).
//!
//! gitweb's `git_commitdiff` renders a commit's authorship and message, the
//! changed-files table (`git_difftree_body`), and then colourises the diff
//! itself with `git_patchset_body`. We keep everything *around* the diff — the
//! navigation with its alternate formats and parent/merge context, the author
//! and committer rows, the message body, and the changed-files table — but
//! replace the server-coloured patchset with a `#diff-root` container the
//! vendored `@pierre/diffs` client viewer fills from a clean unified diff (see
//! [`crate::diff_host`]).
//!
//! The page reuses the commit view's authorship rows and changed-files table
//! ([`crate::commit`]) and the diff host's `#diff-root` container
//! ([`crate::diff_host`]). It differs from the commit view in its object header
//! (gitweb's commitdiff shows only the authorship rows, not the commit/tree/
//! parent rows) and in its navigation extra, which carries the `raw` / `patch`
//! format affordances and the commitdiff parent context.
//!
//! Every URL is built by the web boundary, so this layer takes finished hrefs:
//! the diff URL the viewer fetches, the boot module src, and every link in the
//! navigation and the changed-files rows. Every value is escaped by maud.

use gitweb_domain::model::message_body::LogLine;

use crate::chrome::{Crumb, FormatLink, NavItem, PageFooter, footer, page_header, page_nav};
use crate::commit::{AuthorRow, ChangedRow, ParentNavLink, author_row, changed_files};
use crate::diff_host::diff_root;
use crate::markup::{Markup, html};
use crate::message::log_lines;

/// The commitdiff parent/merge context shown in the navigation — gitweb's
/// `$formats_nav` parent context for `git_commitdiff`.
#[derive(Debug, Clone)]
pub enum CommitdiffNav {
    /// A root commit — gitweb's `(initial)`.
    Initial,
    /// A single-parent commit — gitweb's `(parent: <short>)`.
    SingleParent(ParentNavLink),
    /// commitdiff between two given commits — gitweb's `(from[ parent N]:
    /// <short>)`. `number` is the explicit parent's 1-based position when it is
    /// one of the commit's own parents, absent otherwise.
    FromParent {
        /// The explicit parent's 1-based position among the commit's parents.
        number: Option<usize>,
        /// The explicit parent, linked to its commit view.
        link: ParentNavLink,
    },
    /// A merge shown against all its parents — gitweb's `(merge: <short> …)`.
    Merge(Vec<ParentNavLink>),
}

/// The whole commitdiff host page: the breadcrumb header, the navigation with the
/// format affordances and parent context, the authorship rows, the message body,
/// the changed-files table, and the diff-root container the client viewer fills.
#[derive(Debug, Clone)]
pub struct CommitdiffPage {
    /// Breadcrumb trail (home / project / commitdiff).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The alternate-format affordances shown on the right (`raw`, `patch`).
    pub formats: Vec<FormatLink>,
    /// The parent/merge context shown after the formats.
    pub parentage: CommitdiffNav,
    /// The commit title, shown as the page heading.
    pub title: String,
    /// The author authorship row.
    pub author: AuthorRow,
    /// The committer authorship row.
    pub committer: AuthorRow,
    /// The processed message body lines.
    pub comment: Vec<LogLine>,
    /// The changed-files rows, carrying the commitdiff patch-anchor links.
    pub changed: Vec<ChangedRow>,
    /// The URL of the clean unified diff the client viewer fetches.
    pub diff_url: String,
    /// The `src` of the boot module that renders the diff.
    pub viewer_module_src: String,
}

/// Renders the commitdiff host page body: the breadcrumb header, the navigation
/// with the formats and parent context, the authorship rows, the message body,
/// the changed-files table, and the diff-root container — then the footer. The
/// caller wraps this in the document chrome.
#[must_use]
pub fn commitdiff_body(page: &CommitdiffPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, Some(commitdiff_nav_extra(page))))
        main class="content commitdiff-view" {
            h2 class="section-title" { (page.title) }
            table class="object-header" {
                tbody {
                    (author_row("author", &page.author))
                    (author_row("committer", &page.committer))
                }
            }
            div class="page_body log" { (log_lines(&page.comment)) }
            (changed_files(&page.changed))
            (diff_root(&page.diff_url, &page.viewer_module_src))
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// The navigation extra: the format affordances (`raw` / `patch`), then the
/// parent/merge context — gitweb's `$formats_nav` for `git_commitdiff`.
fn commitdiff_nav_extra(page: &CommitdiffPage) -> Markup {
    html! {
        @for (index, link) in page.formats.iter().enumerate() {
            @if index > 0 { " | " }
            a href=(link.href) { (link.label) }
        }
        " "
        (parentage(&page.parentage))
    }
}

/// The parent/merge context span (gitweb's commitdiff parent navigation).
fn parentage(nav: &CommitdiffNav) -> Markup {
    html! {
        span class="parent-nav" {
            @match nav {
                CommitdiffNav::Initial => { "(initial)" }
                CommitdiffNav::SingleParent(link) => {
                    "(parent: " a href=(link.href) { (link.short) } ")"
                }
                CommitdiffNav::FromParent { number, link } => {
                    "(from"
                    @if let Some(number) = number { " parent " (number) }
                    ": " a href=(link.href) { (link.short) } ")"
                }
                CommitdiffNav::Merge(links) => {
                    "(merge: "
                    @for (index, link) in links.iter().enumerate() {
                        @if index > 0 { " " }
                        a href=(link.href) { (link.short) }
                    }
                    ")"
                }
            }
        }
    }
}
