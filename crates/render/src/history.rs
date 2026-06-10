//! The per-path history table (modernized `git_history` / `git_history_body`).
//!
//! gitweb renders a file's history as one row per commit that touched it: a date
//! cell (`<td title="age_string_age"><i>age_string_date</i></td>`), the author
//! (chopped for the row, full name as a tooltip when chopped), the subject
//! linking to the commit (short subject with the full one as a tooltip when
//! chopped), and a links cell that differs from the shortlog's: the file's
//! blob/tree at that commit (labelled with the file's type), `commitdiff`, and —
//! for a file — `raw` plus, when this commit's content differs from the current
//! version, `diff to current`. A trailing row carries gitweb's `$extra` "next"
//! affordance.
//!
//! The file's type is constant across the history (gitweb's `$ftype`), so it is
//! the table's, labelling every ftype link the same way. The date swap, the
//! author chop, and the diff-to-current decision are all the domain's calls; URLs
//! are the web boundary's. This view only places finished strings. The ref badges
//! after the subject are gitweb's `format_ref_marker` (the domain rule + the
//! shared [`crate::refs`] view).

use crate::chrome::{Crumb, MoreLink, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};
use crate::refs::{RefMarkerView, ref_markers};

/// One history row: the date cell (the swap already resolved into the shown text
/// and its tooltip), the author (full and chopped), the subject (full and short),
/// and the finished link hrefs the boundary built for this commit.
#[derive(Debug, Clone)]
pub struct HistoryEntryView {
    /// The text shown in the date cell (gitweb's `age_string_date`).
    pub date_displayed: String,
    /// The date cell's hover tooltip (gitweb's `age_string_age`).
    pub date_tooltip: String,
    /// The full author name (the row's `title` when the name is chopped).
    pub author: String,
    /// The author name chopped to gitweb's 15/3 bound for the row.
    pub author_short: String,
    /// The full commit subject (the link `title` when the subject is chopped).
    pub title: String,
    /// The short commit subject shown when the subject was chopped.
    pub title_short: String,
    /// `commit` URL for this commit; the subject links here.
    pub commit: String,
    /// The file's blob/tree at this commit (gitweb's `$ftype` action link).
    pub object: String,
    /// `commitdiff` URL for this commit.
    pub commitdiff: String,
    /// `blob_plain` (raw) URL, present for a file and absent for a directory.
    pub raw: Option<String>,
    /// `blobdiff` URL, present only when this commit's content differs from the
    /// view's current version (gitweb's "diff to current").
    pub diff_to_current: Option<String>,
    /// The ref badges whose tip is this commit (gitweb's `format_ref_marker`),
    /// rendered after the subject; empty when no ref points here.
    pub refs: Vec<RefMarkerView>,
}

/// The history table: the file-type label every ftype link carries, the commit
/// rows, and an optional trailing "more" row.
#[derive(Debug, Clone)]
pub struct HistoryTable {
    /// The file's type name (gitweb's `$ftype`: `"blob"` or `"tree"`), the text
    /// of every ftype link.
    pub object_label: String,
    /// The commit rows, already ordered and resolved by the use case.
    pub rows: Vec<HistoryEntryView>,
    /// The "more" affordance, present only when a further page exists.
    pub more: Option<MoreLink>,
}

/// The whole history page body: the breadcrumb header, the action navigation, the
/// tracked path, and the table.
#[derive(Debug, Clone)]
pub struct HistoryPage {
    /// Breadcrumb trail (home / project / history).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The tracked file or directory, shown in the header.
    pub file_name: String,
    /// The commit table.
    pub table: HistoryTable,
}

/// Renders the history page body: the breadcrumb header, the action navigation,
/// the tracked path, the table inside a `main` element (or a note when there are
/// no commits), and the footer. The caller wraps this in the document chrome.
#[must_use]
pub fn history_body(page: &HistoryPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            h2 class="section-title" { "History" }
            p class="path" { (page.file_name) }
            @if page.table.rows.is_empty() {
                p class="note" { "No commits." }
            } @else {
                (history_table(&page.table))
            }
        }
    }
}

/// Renders the history table: a row per commit, then the optional "more" row.
#[must_use]
pub fn history_table(table: &HistoryTable) -> Markup {
    html! {
        table class="history" {
            tbody {
                @for row in &table.rows {
                    (history_row(row, &table.object_label))
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
/// per-commit links — the ftype link (labelled `object_label`), `commitdiff`,
/// then `raw` and `diff to current` when present.
fn history_row(row: &HistoryEntryView, object_label: &str) -> Markup {
    html! {
        tr {
            td class="date" title=(row.date_tooltip) { i { (row.date_displayed) } }
            (author_cell(row))
            td class="subject" { (subject_link(row)) (ref_markers(&row.refs)) }
            td class="links" {
                a href=(row.object) { (object_label) }
                " | "
                a href=(row.commitdiff) { "commitdiff" }
                @if let Some(raw) = &row.raw {
                    " | "
                    a href=(raw) { "raw" }
                }
                @if let Some(diff) = &row.diff_to_current {
                    " | "
                    a href=(diff) { "diff to current" }
                }
            }
        }
    }
}

/// The author cell: the chopped name, carrying the full name as a tooltip only
/// when the chop actually shortened it (gitweb's `chop_and_escape_str`).
fn author_cell(row: &HistoryEntryView) -> Markup {
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
/// Both forms link to the commit.
fn subject_link(row: &HistoryEntryView) -> Markup {
    html! {
        @if row.title_short.chars().count() < row.title.chars().count() {
            a class="list subject" title=(row.title) href=(row.commit) { (row.title_short) }
        } @else {
            a class="list subject" href=(row.commit) { (row.title) }
        }
    }
}
