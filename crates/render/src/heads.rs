//! The heads table (modernized `git_heads` / `git_heads_body`).
//!
//! gitweb renders the branches as a table with one row per branch: the
//! last-change age, the branch name linking to its shortlog, and per-branch
//! shortlog / log / tree links, with the branch HEAD points at marked
//! `current_head`. URLs are gitweb's `href()`, the web boundary's job, so this
//! view takes finished hrefs and only decides layout, escaping, the recency CSS
//! hook, and the current marker. The age is a domain [`Age`]; an absent age is
//! gitweb's "unknown".

use gitweb_domain::model::age::Age;

use crate::age::age_class_name;
use crate::chrome::{Crumb, MoreLink, NavItem, PageFooter, footer, page_header, page_nav};
use crate::markup::{Markup, html};

/// One branch row: its name (linking to the branch shortlog), the per-branch
/// quick links, the last-change age, and whether it is the current branch.
#[derive(Debug, Clone)]
pub struct HeadEntryView {
    /// Short branch name, shown and linked to its shortlog.
    pub name: String,
    /// `shortlog` URL for this branch; the name links here too.
    pub shortlog: String,
    /// `log` URL for this branch.
    pub log: String,
    /// `tree` URL for this branch.
    pub tree: String,
    /// Last-change age, or `None` for a tip with no commit time ("unknown").
    pub age: Option<Age>,
    /// Whether this branch's tip is HEAD's commit.
    pub current: bool,
}

/// The heads table: the branch rows in display order, with an optional trailing
/// "more" row (gitweb's `$extra` — the summary page's "..." link to the full
/// heads action; absent on the standalone heads page, which lists every branch).
#[derive(Debug, Clone)]
pub struct HeadsTable {
    /// The branch rows, already ordered by the use case.
    pub rows: Vec<HeadEntryView>,
    /// The "more" affordance, present only when the list was capped (the summary
    /// page past 16 branches).
    pub more: Option<MoreLink>,
}

/// The whole heads page body: the breadcrumb trail, the refs sub-navigation
/// (tags | heads | …), and the table.
#[derive(Debug, Clone)]
pub struct HeadsPage {
    /// Breadcrumb trail (home / project / heads).
    pub crumbs: Vec<Crumb>,
    /// The refs sub-navigation; the current view has no link.
    pub ref_views: Vec<NavItem>,
    /// The branch table.
    pub table: HeadsTable,
}

/// Renders the heads page body: the breadcrumb header, the refs sub-navigation,
/// the table inside a `main` element (or a note when there are no branches), and
/// the footer. The caller wraps this in the document
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn heads_body(page: &HeadsPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.ref_views, None))
        main class="content" {
            h2 class="section-title" { "Heads" }
            @if page.table.rows.is_empty() {
                p class="note" { "No branches." }
            } @else {
                (heads_table(&page.table))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// Renders the heads table: a row per branch, then the optional "more" row.
#[must_use]
pub fn heads_table(table: &HeadsTable) -> Markup {
    html! {
        table class="heads" {
            tbody {
                @for row in &table.rows {
                    (head_row(row))
                }
                @if let Some(more) = &table.more {
                    tr class="more" {
                        td colspan="3" { a href=(more.href) { (more.label) } }
                    }
                }
            }
        }
    }
}

/// One branch row: age, name linking to shortlog, and the quick links. The
/// current branch's row carries the `current` class (gitweb's `current_head`).
fn head_row(row: &HeadEntryView) -> Markup {
    html! {
        tr class=[row.current.then_some("current")] {
            (age_cell(row.age))
            td class="name" { a class="list name" href=(row.shortlog) { (row.name) } }
            td class="links" {
                a href=(row.shortlog) { "shortlog" }
                " | "
                a href=(row.log) { "log" }
                " | "
                a href=(row.tree) { "tree" }
            }
        }
    }
}

/// The age cell: gitweb's relative age coloured by recency, or "unknown" for a
/// tip with no commit time (gitweb's `$ref_item{'age'} = "unknown"`).
fn age_cell(age: Option<Age>) -> Markup {
    html! {
        @match age {
            Some(age) => td class={ "age " (age_class_name(age.class())) } { (age.humanized()) },
            None => td class="age age-unknown" { "unknown" },
        }
    }
}
