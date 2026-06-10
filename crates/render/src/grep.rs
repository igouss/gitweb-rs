//! The grep-results table (modernized `git_search_files`).
//!
//! gitweb lists the grep matches grouped per file: a file row links to the blob,
//! then one row per matching line — the 1-based line number linking to `blob#lN`
//! and the untabified line text with its matched span in a `<span class="match">`
//! — or a single "Binary file" row for a binary match. An empty result shows
//! "No matches found"; an overflowing one appends "Too many matches, listing
//! trimmed".
//!
//! Everything arrives finished from the use case and the boundary: the untabify
//! and the lead/match/trail split are the domain's calls, the URLs are gitweb's
//! `href()` (the web layer's job), so this view only places them. The match span
//! is the one deliberate highlight; the surrounding text is auto-escaped by the
//! template engine like every other interpolation, and the line cell is
//! white-space-preserving so the untabified columns line up.

use crate::chrome::{Crumb, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};

/// One matching line's display text: split around the highlighted span, or shown
/// whole when the display regexp did not match it (gitweb's else branch).
#[derive(Debug, Clone)]
pub enum GrepLineView {
    /// The line split into lead / matched span / trail.
    Highlighted {
        /// Text before the match (auto-escaped).
        lead: String,
        /// The matched fragment, wrapped in `<span class="match">`.
        matched: String,
        /// Text after the match (auto-escaped).
        trail: String,
    },
    /// The whole line, with no span marked.
    Plain(String),
}

/// One row under a file: a matching line or the binary marker.
#[derive(Debug, Clone)]
pub enum GrepRowView {
    /// A binary file whose bytes matched: shown as "Binary file".
    Binary,
    /// A matching text line.
    Line {
        /// The 1-based line number shown in the line-number cell.
        number: usize,
        /// The `blob#lN` URL the line number links to.
        line_href: String,
        /// The line's display text.
        content: GrepLineView,
    },
}

/// One file's block: its path, the blob URL it links to, and its matching rows.
#[derive(Debug, Clone)]
pub struct GrepFileBlock {
    /// The file path, relative to the searched tree.
    pub path: String,
    /// The `blob` URL the path links to.
    pub blob_href: String,
    /// The matching rows under this file, in line order.
    pub rows: Vec<GrepRowView>,
}

/// The whole grep page body: the breadcrumb header, the action navigation, the
/// results header, and either the per-file table or the "No matches found" note.
#[derive(Debug, Clone)]
pub struct GrepPage {
    /// Breadcrumb trail (home / project / search).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The base commit's subject, shown as the results header.
    pub header_title: String,
    /// The matching files, grouped in tree order.
    pub files: Vec<GrepFileBlock>,
    /// Whether there were no matches at all (gitweb's "No matches found").
    pub no_match: bool,
    /// Whether the cap trimmed the listing (gitweb's "listing trimmed" notice).
    pub trimmed: bool,
}

/// Renders the grep page body. The caller wraps this in the document chrome
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn grep_body(page: &GrepPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            h2 class="section-title" { (page.header_title) }
            @if page.no_match {
                p class="note" { "No matches found" }
            } @else {
                table class="grep" {
                    @for file in &page.files {
                        (file_block(file))
                    }
                }
                @if page.trimmed {
                    p class="note" { "Too many matches, listing trimmed" }
                }
            }
        }
    }
}

/// One file's block: a header row linking to the blob, then its matching rows.
fn file_block(file: &GrepFileBlock) -> Markup {
    html! {
        tbody class="file" {
            tr class="path" {
                td class="list" colspan="2" {
                    a class="list" href=(file.blob_href) { (file.path) }
                }
            }
            @for row in &file.rows {
                (grep_row(row))
            }
        }
    }
}

/// One row under a file: the binary marker, or a numbered line with its text.
fn grep_row(row: &GrepRowView) -> Markup {
    html! {
        @match row {
            GrepRowView::Binary => tr class="binary" {
                td class="binary" colspan="2" { "Binary file" }
            },
            GrepRowView::Line { number, line_href, content } => tr class="match" {
                td class="linenr" {
                    a class="linenr" href=(line_href) { (number) }
                }
                td class="pre" { (line_text(content)) }
            },
        }
    }
}

/// A matched line's text: the lead, the match in its span, and the trail; or the
/// whole line plain when the display regexp did not mark it.
fn line_text(content: &GrepLineView) -> Markup {
    html! {
        @match content {
            GrepLineView::Highlighted { lead, matched, trail } => {
                (lead)
                span class="match" { (matched) }
                (trail)
            },
            GrepLineView::Plain(whole) => (whole),
        }
    }
}
