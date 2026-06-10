//! The pickaxe-results table (modernized `git_search_changes`).
//!
//! gitweb lists the pickaxe matches one commit per row: the date, the author, and
//! the subject (linking to the commit), and — under the subject — each file whose
//! occurrence count the commit changed, its path in a `<span class="match">`
//! linking to the file's blob, then a `commit | tree` link cell. A deleted file is
//! already dropped upstream (it has no blob to link), and gitweb prints no "no
//! matches" note for pickaxe — an empty result is simply an empty table.
//!
//! Everything arrives finished from the use case and the boundary: the date's
//! two-week swap, the author and subject chops are the domain's calls, the URLs are
//! gitweb's `href()` (the web layer's job), so this view only places them. The
//! match span is the one deliberate highlight; every other interpolation is
//! auto-escaped by the template engine.

use crate::chrome::{Crumb, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};

/// One changed file under a matching commit: its path and the blob URL the path
/// links to.
#[derive(Debug, Clone)]
pub struct PickaxeFileLink {
    /// The file path, relative to the repository root.
    pub path: String,
    /// The `blob` URL the path links to.
    pub blob_href: String,
}

/// One matching commit's row: its date cell, author, subject (linking to the
/// commit), the changed files, and the commit/tree link URLs.
#[derive(Debug, Clone)]
pub struct PickaxeEntryView {
    /// The date as displayed (the two-week swap already resolved).
    pub date_displayed: String,
    /// The other-of-the-pair date, shown as the cell's tooltip.
    pub date_tooltip: String,
    /// The full author name, shown as the author cell's tooltip.
    pub author: String,
    /// The author name chopped for the cell.
    pub author_short: String,
    /// The full commit subject, shown as the subject link's tooltip.
    pub title: String,
    /// The short commit subject shown in the link.
    pub title_short: String,
    /// The `commit` URL the subject and the link cell point at.
    pub commit_href: String,
    /// The `tree` URL the link cell points at.
    pub tree_href: String,
    /// The changed files under this commit (deletions already dropped), in order.
    pub files: Vec<PickaxeFileLink>,
}

/// The whole pickaxe page body: the breadcrumb header, the action navigation, the
/// results header, and the per-commit table (empty when nothing matched — gitweb
/// prints no note for pickaxe).
#[derive(Debug, Clone)]
pub struct PickaxePage {
    /// Breadcrumb trail (home / project / search).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The base commit's subject, shown as the results header.
    pub header_title: String,
    /// The matching commit rows, newest first.
    pub rows: Vec<PickaxeEntryView>,
}

/// Renders the pickaxe page body. The caller wraps this in the document chrome
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn pickaxe_body(page: &PickaxePage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            h2 class="section-title" { (page.header_title) }
            table class="pickaxe" {
                @for row in &page.rows {
                    (entry_row(row))
                }
            }
        }
    }
}

/// One commit's row: the date, the author, the subject with its changed files, and
/// the commit/tree links.
fn entry_row(row: &PickaxeEntryView) -> Markup {
    html! {
        tr {
            td class="age" title=(row.date_tooltip) { (row.date_displayed) }
            td class="author" title=(row.author) { (row.author_short) }
            td class="subject" {
                a class="list subject" href=(row.commit_href) title=(row.title) {
                    (row.title_short)
                }
                @if !row.files.is_empty() {
                    ul class="match-files" {
                        @for file in &row.files {
                            li {
                                a class="list" href=(file.blob_href) {
                                    span class="match" { (file.path) }
                                }
                            }
                        }
                    }
                }
            }
            td class="link" {
                a href=(row.commit_href) { "commit" }
                " | "
                a href=(row.tree_href) { "tree" }
            }
        }
    }
}
