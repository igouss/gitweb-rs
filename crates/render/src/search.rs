//! The search-results table (modernized `git_search_message` /
//! `git_search_grep_body`).
//!
//! gitweb lists each matching commit as a row: a date cell (the shown form with
//! the alternate as a tooltip), the author (chopped, with the full name as a
//! tooltip when chopped), the subject linking to the commit, the highlighted
//! matching fragments of the message — each a lead, a `<span class="match">`, and
//! a trail — and the per-commit `commit | commitdiff | tree` links. The page
//! navigation carries gitweb's `first · prev · next` paging affordance, and when
//! the first page has no matches at all gitweb prints a plain "No match."
//!
//! Everything arrives finished from the boundary and the use case: the date swap
//! and the snippet trimming are the domain's calls, the URLs are gitweb's
//! `href()` (the web layer's job), so this view only places them. The match
//! fragment is the one deliberate highlight; the surrounding text is
//! auto-escaped by the template engine like every other interpolation.

use crate::chrome::{Crumb, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};

/// One highlighted message-line fragment of a matching commit: the trimmed text
/// before the match, the match itself, and the trimmed text after it.
#[derive(Debug, Clone)]
pub struct SnippetView {
    /// Text before the match (auto-escaped).
    pub lead: String,
    /// The matched fragment, wrapped in `<span class="match">`.
    pub matched: String,
    /// Text after the match (auto-escaped).
    pub trail: String,
}

/// One search-result row.
#[derive(Debug, Clone)]
pub struct SearchEntryView {
    /// The text shown in the date cell (gitweb's `age_string_date`).
    pub date_displayed: String,
    /// The date cell's hover tooltip (gitweb's `age_string_age`).
    pub date_tooltip: String,
    /// The full author name (the row's `title` when the name is chopped).
    pub author: String,
    /// The author name chopped to gitweb's 15 characters for the row.
    pub author_short: String,
    /// The full commit subject (the link `title` when the subject is chopped).
    pub title: String,
    /// The short commit subject shown when the subject was chopped.
    pub title_short: String,
    /// `commit` URL for this commit; the subject links here too.
    pub commit: String,
    /// `commitdiff` URL for this commit.
    pub commitdiff: String,
    /// `tree` URL for this commit.
    pub tree: String,
    /// The highlighted message-line snippets, in message order.
    pub snippets: Vec<SnippetView>,
}

/// The `first · prev · next` paging navigation: each link is present only when
/// that move is possible (gitweb shows the rest as disabled text).
#[derive(Debug, Clone, Default)]
pub struct SearchPaging {
    /// Link back to page 0, present only when not already there.
    pub first: Option<String>,
    /// Link to the previous page, present only when one exists.
    pub prev: Option<String>,
    /// Link to the next page, present only when a further page exists.
    pub next: Option<String>,
}

/// The whole search page body: the breadcrumb header, the action navigation with
/// the paging affordance, and either the results table or the "No match." note.
#[derive(Debug, Clone)]
pub struct SearchPage {
    /// Breadcrumb trail (home / project / search).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The `first · prev · next` paging affordance.
    pub paging: SearchPaging,
    /// The base commit's subject, shown as the results header.
    pub header_title: String,
    /// The matching commit rows for this page.
    pub rows: Vec<SearchEntryView>,
    /// Whether this is the first page with no matches at all (gitweb's "No
    /// match."); a later empty page shows an empty table instead.
    pub no_match: bool,
}

/// Renders the search page body. The caller wraps this in the document chrome
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn search_body(page: &SearchPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, Some(paging_nav(&page.paging))))
        main class="content" {
            h2 class="section-title" { (page.header_title) }
            @if page.no_match {
                p class="note" { "No match." }
            } @else {
                (results_table(&page.rows))
            }
        }
    }
}

/// The `first · prev · next` affordance for the action nav's extra slot.
fn paging_nav(paging: &SearchPaging) -> Markup {
    html! {
        span class="paging" {
            (paging_link("first", paging.first.as_deref()))
            " · "
            (paging_link("prev", paging.prev.as_deref()))
            " · "
            (paging_link("next", paging.next.as_deref()))
        }
    }
}

/// One paging move: a link when the move is possible, otherwise disabled text.
fn paging_link(label: &str, href: Option<&str>) -> Markup {
    html! {
        @match href {
            Some(target) => a href=(target) { (label) },
            None => span class="nav-disabled" { (label) },
        }
    }
}

/// The results table: a row per matching commit.
fn results_table(rows: &[SearchEntryView]) -> Markup {
    html! {
        table class="search commit-search" {
            tbody {
                @for row in rows {
                    (result_row(row))
                }
            }
        }
    }
}

/// One result row: the date cell, the author cell, the subject with its
/// highlighted snippets, and the per-commit links.
fn result_row(row: &SearchEntryView) -> Markup {
    html! {
        tr {
            td class="date" title=(row.date_tooltip) { i { (row.date_displayed) } }
            (author_cell(row))
            td class="subject" {
                (subject_link(row))
                @for snippet in &row.snippets {
                    br;
                    (snippet_view(snippet))
                }
            }
            td class="links" {
                a href=(row.commit) { "commit" }
                " | "
                a href=(row.commitdiff) { "commitdiff" }
                " | "
                a href=(row.tree) { "tree" }
            }
        }
    }
}

/// The author cell: the chopped name, carrying the full name as a tooltip only
/// when the chop actually shortened it (gitweb's `chop_and_escape_str`).
fn author_cell(row: &SearchEntryView) -> Markup {
    html! {
        td class="author" {
            @if row.author_short == row.author {
                (row.author)
            } @else {
                span title=(row.author) { (row.author_short) }
            }
        }
    }
}

/// The subject link: the short subject with the full one as a tooltip when it was
/// chopped, otherwise the full subject plain (gitweb's `format_subject_html`).
fn subject_link(row: &SearchEntryView) -> Markup {
    html! {
        @if row.title_short.chars().count() < row.title.chars().count() {
            a class="list subject" title=(row.title) href=(row.commit) { (row.title_short) }
        } @else {
            a class="list subject" href=(row.commit) { (row.title) }
        }
    }
}

/// One highlighted snippet: the lead, the match in its span, and the trail.
fn snippet_view(snippet: &SnippetView) -> Markup {
    html! {
        span class="snippet" {
            (snippet.lead)
            span class="match" { (snippet.matched) }
            (snippet.trail)
        }
    }
}
