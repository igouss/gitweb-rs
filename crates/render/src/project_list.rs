//! The projects-list table (modernized `git_project_list_body` /
//! `git_project_list_rows`).
//!
//! gitweb renders the list as a table with sortable column headers — the column
//! the list is sorted by is plain text, the others link to re-sort
//! (`format_sort_th`) — and one row per project: its path and description link
//! to the summary, then the owner, the last-change age (coloured by recency),
//! and the per-project quick links. URLs are gitweb's `href()`, the web
//! boundary's job, so this view takes finished hrefs and only decides layout and
//! escaping. The age is a domain [`Age`]; this names the CSS recency hook and
//! prints gitweb's relative string.

use gitweb_domain::model::age::Age;

use crate::age::age_class_name;
use crate::chrome::{Crumb, PageFooter, footer, page_header};
use crate::markup::{Markup, html};

/// A sortable column header: its label and the link to re-sort by it. An absent
/// `href` marks the column the list is currently sorted by, rendered as plain
/// text (gitweb's `format_sort_th`).
#[derive(Debug, Clone)]
pub struct SortHeader {
    /// Visible column label.
    pub label: String,
    /// Link to sort by this column, or `None` for the active sort column.
    pub href: Option<String>,
}

/// One project's quick links, already built by the web boundary.
#[derive(Debug, Clone)]
pub struct ProjectLinks {
    /// `summary` action URL.
    pub summary: String,
    /// `shortlog` action URL.
    pub shortlog: String,
    /// `log` action URL.
    pub log: String,
    /// `tree` action URL.
    pub tree: String,
}

/// One row of the projects list: a project and the metadata gitweb shows.
#[derive(Debug, Clone)]
pub struct ProjectRow {
    /// Store-relative path, shown as the project name.
    pub name: String,
    /// Link to the project's summary; the name and description link here.
    pub href: String,
    /// Full description, or `None` when the project has none.
    pub description: Option<String>,
    /// Owner, or `None` when unknown.
    pub owner: Option<String>,
    /// Last-change age, or `None` for a project with no commits.
    pub age: Option<Age>,
    /// Per-project quick links.
    pub links: ProjectLinks,
    /// Number of forks folded under this project; `0` when it has none (or the
    /// forks feature is off). A positive count links the leading `+` and adds the
    /// `forks` quick link.
    pub fork_count: usize,
    /// The project's forks view URL, the target of the `+` and `forks` links.
    pub forks_href: String,
}

/// The projects-list table: the four sortable column headers and the rows.
#[derive(Debug, Clone)]
pub struct ProjectList {
    /// The "Project" column header.
    pub project_header: SortHeader,
    /// The "Description" column header.
    pub description_header: SortHeader,
    /// The "Owner" column header.
    pub owner_header: SortHeader,
    /// The "Last Change" column header.
    pub age_header: SortHeader,
    /// Whether the forks feature is on (gitweb's `$check_forks`): when set, the
    /// table carries a leading fork column (header + a cell per row).
    pub forks_enabled: bool,
    /// The rows, already in display order.
    pub rows: Vec<ProjectRow>,
}

/// The whole landing page's content: the site name (the home breadcrumb) and
/// the table. The document `<head>` and wrapper stay with the web boundary,
/// which owns the asset URLs.
#[derive(Debug, Clone)]
pub struct ProjectListPage {
    /// Site name, shown as the current (home) breadcrumb.
    pub site_name: String,
    /// The projects table.
    pub list: ProjectList,
}

/// Renders the landing-page body: the breadcrumb header, the table inside a
/// `main` element, and the footer. The caller wraps this in the document
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn project_list_body(page: &ProjectListPage) -> Markup {
    let crumbs: [Crumb; 1] = [Crumb {
        label: page.site_name.clone(),
        href: None,
    }];
    html! {
        (page_header(None, &crumbs))
        main class="content" { (project_list(&page.list)) }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// Renders the projects-list table.
#[must_use]
pub fn project_list(list: &ProjectList) -> Markup {
    html! {
        table class="project-list" {
            thead {
                tr {
                    @if list.forks_enabled { th class="forks" {} }
                    (sort_th(&list.project_header))
                    (sort_th(&list.description_header))
                    (sort_th(&list.owner_header))
                    (sort_th(&list.age_header))
                    th class="links" {}
                }
            }
            tbody {
                @for row in &list.rows {
                    (project_row(row, list.forks_enabled))
                }
            }
        }
    }
}

/// One column header: a link to re-sort, or plain text for the active column
/// (gitweb's `format_sort_th`).
fn sort_th(header: &SortHeader) -> Markup {
    html! {
        @match &header.href {
            Some(href) => th { a class="sort" href=(href) { (header.label) } },
            None => th class="sorted" { (header.label) },
        }
    }
}

/// One project row: the optional leading fork cell, name and description linking
/// to the summary, owner, age, and the quick links.
fn project_row(row: &ProjectRow, forks_enabled: bool) -> Markup {
    html! {
        tr {
            @if forks_enabled { (fork_cell(row)) }
            td class="project" { a class="list" href=(row.href) { (row.name) } }
            td class="description" {
                @if let Some(description) = &row.description {
                    a class="list" href=(row.href) title=(description) { (description) }
                }
            }
            td class="owner" {
                @if let Some(owner) = &row.owner {
                    em { (owner) }
                }
            }
            (age_cell(row.age))
            td class="links" {
                a href=(row.links.summary) { "summary" }
                " | "
                a href=(row.links.shortlog) { "shortlog" }
                " | "
                a href=(row.links.log) { "log" }
                " | "
                a href=(row.links.tree) { "tree" }
                @if row.fork_count > 0 {
                    " | "
                    a href=(row.forks_href) { "forks" }
                }
            }
        }
    }
}

/// The leading fork cell (gitweb's `$check_forks` `<td>`): a `+` linking to the
/// project's forks view when it has forks (titled with the count), or an empty
/// cell when it has none.
fn fork_cell(row: &ProjectRow) -> Markup {
    html! {
        td class="forks" {
            @if row.fork_count > 0 {
                a href=(row.forks_href) title=(format!("{} forks", row.fork_count)) { "+" }
            }
        }
    }
}

/// The "Last Change" cell: gitweb's relative age coloured by recency, or
/// "No commits" for a project with none.
fn age_cell(age: Option<Age>) -> Markup {
    html! {
        @match age {
            Some(age) => td class={ "age " (age_class_name(age.class())) } { (age.humanized()) },
            None => td class="age age-unknown" { "No commits" },
        }
    }
}
