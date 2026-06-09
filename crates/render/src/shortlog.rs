//! The shortlog table (modernized `git_shortlog` / `git_shortlog_body`).
//!
//! gitweb renders recent history as one row per commit: a date cell (`<td
//! title="age_string_age"><i>age_string_date</i></td>` — the shown form with the
//! alternate as a tooltip), the author (chopped for the row, with the full name
//! as a tooltip when it was chopped, mirroring `chop_and_escape_str`), the
//! subject linking to the commit (the short subject with the full one as a
//! tooltip when chopped, mirroring `format_subject_html`), and the per-commit
//! `commit | commitdiff | tree` links. A trailing row carries gitweb's `$extra`
//! affordance — the `...` link on the summary, the `next` link on the paged
//! action — and appears only when a further page exists.
//!
//! The date swap is the domain's call ([`gitweb_domain::model::commit_date`]),
//! so this view takes the two finished strings and only places them; likewise the
//! author chop is the use case's call, so this view only decides whether the full
//! name still warrants a tooltip. URLs are gitweb's `href()`, the web boundary's
//! job, so every link arrives finished. The ref badges after the subject are
//! gitweb's `format_ref_marker` (the domain rule + the shared [`crate::refs`]
//! view). The avatar and the search-author link are their own features and are
//! not wired here.

use crate::chrome::{Crumb, MoreLink, NavItem, PageFooter, footer, page_header, page_nav};
use crate::markup::{Markup, html};
use crate::refs::{RefMarkerView, ref_markers};

/// One shortlog row: its date cell (the swap already resolved by the domain into
/// the shown text and the tooltip behind it), the author (full and chopped for
/// the row), the subject (full and short), and the per-commit links the boundary
/// built.
#[derive(Debug, Clone)]
pub struct ShortlogEntryView {
    /// The text shown in the date cell (gitweb's `age_string_date`).
    pub date_displayed: String,
    /// The date cell's hover tooltip (gitweb's `age_string_age`).
    pub date_tooltip: String,
    /// The full author name (the row's `title` when the name is chopped).
    pub author: String,
    /// The author name chopped to gitweb's 10 characters for the row.
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
    /// The ref badges whose tip is this commit (gitweb's `format_ref_marker`),
    /// rendered after the subject; empty when no ref points here.
    pub refs: Vec<RefMarkerView>,
}

/// The shortlog table: the commit rows and an optional trailing "more" row.
#[derive(Debug, Clone)]
pub struct ShortlogTable {
    /// The commit rows, already ordered and resolved by the use case.
    pub rows: Vec<ShortlogEntryView>,
    /// The "more" affordance, present only when a further page exists.
    pub more: Option<MoreLink>,
}

/// The whole shortlog page body: the breadcrumb header, the action navigation,
/// and the table.
#[derive(Debug, Clone)]
pub struct ShortlogPage {
    /// Breadcrumb trail (home / project / shortlog).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The commit table.
    pub table: ShortlogTable,
}

/// Renders the shortlog page body: the breadcrumb header, the action navigation,
/// the table inside a `main` element (or a note when there are no commits), and
/// the footer. The caller wraps this in the document
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn shortlog_body(page: &ShortlogPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            h2 class="section-title" { "Shortlog" }
            @if page.table.rows.is_empty() {
                p class="note" { "No commits." }
            } @else {
                (shortlog_table(&page.table))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// Renders the shortlog table: a row per commit, then the optional "more" row.
#[must_use]
pub fn shortlog_table(table: &ShortlogTable) -> Markup {
    html! {
        table class="shortlog" {
            tbody {
                @for row in &table.rows {
                    (shortlog_row(row))
                }
                @if let Some(more) = &table.more {
                    tr class="more" {
                        td colspan="4" { a href=(more.href) { (more.label) } }
                    }
                }
            }
        }
    }
}

/// One commit row: the date cell, the author cell, the subject link, and the
/// per-commit links.
fn shortlog_row(row: &ShortlogEntryView) -> Markup {
    html! {
        tr {
            td class="date" title=(row.date_tooltip) { i { (row.date_displayed) } }
            (author_cell(row))
            td class="subject" { (subject_link(row)) (ref_markers(&row.refs)) }
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
/// when the chop actually shortened it (gitweb's `chop_and_escape_str`, which
/// adds the `title` span only when `$chopped ne $str`).
fn author_cell(row: &ShortlogEntryView) -> Markup {
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
/// chopped, otherwise the full subject plain (gitweb's `format_subject_html`,
/// which compares the two by character length). Both forms link to the commit.
fn subject_link(row: &ShortlogEntryView) -> Markup {
    html! {
        @if row.title_short.chars().count() < row.title.chars().count() {
            a class="list subject" title=(row.title) href=(row.commit) { (row.title_short) }
        } @else {
            a class="list subject" href=(row.commit) { (row.title) }
        }
    }
}
