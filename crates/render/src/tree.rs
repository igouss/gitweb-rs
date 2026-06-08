//! The directory listing table (modernized `git_tree` / `git_print_tree_entry`).
//!
//! gitweb renders a tree as a table of `mode [size] name links` rows. Each entry
//! renders by its git object type: a blob links its name to the blob view and
//! offers `blob | history | raw` (a symlink also shows ` -> target`); a sub-tree
//! links its name to the nested tree view and offers `tree | history`; anything
//! else — a gitlink/submodule — is named in plain text with at most a history
//! link. Browsing inside a directory prepends a `..` row linking to the parent.
//!
//! The size column is shown only when gitweb's `show-sizes` feature is on. All
//! URLs are built by the web boundary, so this layer takes finished hrefs and
//! only places them; every value is escaped by maud.

use crate::chrome::{Crumb, NavItem, PageFooter, footer, page_header, page_nav};
use crate::markup::{Markup, html};

/// One action link in an entry's links cell: a label and the href it points at.
#[derive(Debug, Clone)]
pub struct TreeLink {
    /// The link text (e.g. `blob`, `history`, `raw`, `tree`).
    pub label: String,
    /// The finished href the boundary built.
    pub href: String,
}

/// One tree-entry row: its permission string, the optional size cell text, the
/// name (linked unless it is a gitlink, which is named in plain text), the
/// symlink target shown after the name, and the action links.
#[derive(Debug, Clone)]
pub struct TreeRowView {
    /// The symbolic permission string (gitweb's `mode_str`).
    pub mode: String,
    /// The size cell text — a byte count for a file, `-` for a directory or
    /// gitlink. Consulted only when the table shows sizes.
    pub size: Option<String>,
    /// The entry's leaf name.
    pub name: String,
    /// The href the name links to (blob or tree view); `None` names the entry in
    /// plain text, as gitweb does for a gitlink/submodule.
    pub name_href: Option<String>,
    /// For a symlink, the path it points to, shown as ` -> target`.
    pub symlink_target: Option<String>,
    /// The entry's action links, rendered in order separated by `|`.
    pub links: Vec<TreeLink>,
}

/// The `..` parent-directory row: the href it links up to.
#[derive(Debug, Clone)]
pub struct TreeParentRow {
    /// The href of the directory one level up.
    pub href: String,
}

/// The tree table: whether the size column is shown, the optional `..` row, and
/// the entry rows in git's order.
#[derive(Debug, Clone)]
pub struct TreeTable {
    /// Whether the size column is shown (gitweb's `show-sizes`).
    pub show_sizes: bool,
    /// The `..` parent affordance, present only when browsing inside a directory.
    pub parent: Option<TreeParentRow>,
    /// The entry rows.
    pub rows: Vec<TreeRowView>,
}

/// The whole tree page body: the breadcrumb header, the action navigation, the
/// title (the commit subject or the tree hash), the browsed path, and the table.
#[derive(Debug, Clone)]
pub struct TreePage {
    /// Breadcrumb trail (home / project / tree).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The header title: the commit subject, or the tree hash when the base is a
    /// raw tree.
    pub title: String,
    /// The browsed directory path, shown when inside a subdirectory.
    pub path: Option<String>,
    /// The entry table.
    pub table: TreeTable,
}

/// Renders the tree page body: the breadcrumb header, the action navigation, the
/// title and browsed path, the table inside a `main` element (or a note when the
/// directory is empty), and the footer. The caller wraps this in the document
/// chrome.
#[must_use]
pub fn tree_body(page: &TreePage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            h2 class="section-title" { (page.title) }
            @if let Some(path) = &page.path {
                p class="path" { (path) }
            }
            @if page.table.rows.is_empty() && page.table.parent.is_none() {
                p class="note" { "Empty directory." }
            } @else {
                (tree_table(&page.table))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// Renders the tree table: the optional `..` row, then one row per entry.
#[must_use]
pub fn tree_table(table: &TreeTable) -> Markup {
    html! {
        table class="tree" {
            tbody {
                @if let Some(parent) = &table.parent {
                    (parent_row(parent, table.show_sizes))
                }
                @for row in &table.rows {
                    (entry_row(row, table.show_sizes))
                }
            }
        }
    }
}

/// The `..` row: the directory permission string, a blank size cell when sizes
/// are shown, and the `..` link.
fn parent_row(parent: &TreeParentRow, show_sizes: bool) -> Markup {
    html! {
        tr class="parent" {
            td class="mode" { "drwxr-xr-x" }
            @if show_sizes {
                td class="size" {}
            }
            td class="name" { a href=(parent.href) { ".." } }
            td class="links" {}
        }
    }
}

/// One entry row: mode, the optional size cell, the name (linked or plain, with
/// a symlink target appended), and the action links separated by `|`.
fn entry_row(row: &TreeRowView, show_sizes: bool) -> Markup {
    html! {
        tr {
            td class="mode" { (row.mode) }
            @if show_sizes {
                td class="size" { (row.size.as_deref().unwrap_or("-")) }
            }
            td class="name" {
                @if let Some(href) = &row.name_href {
                    a href=(href) { (row.name) }
                } @else {
                    (row.name)
                }
                @if let Some(target) = &row.symlink_target {
                    " -> " (target)
                }
            }
            td class="links" { (links_cell(&row.links)) }
        }
    }
}

/// The action links cell: each link in order, separated by ` | `.
fn links_cell(links: &[TreeLink]) -> Markup {
    html! {
        @for (index, link) in links.iter().enumerate() {
            @if index > 0 { " | " }
            a href=(link.href) { (link.label) }
        }
    }
}
