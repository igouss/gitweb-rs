//! The commit view (modernized `git_commit`).
//!
//! gitweb's `git_commit` renders one commit: a page navigation carrying the
//! parent/merge context, an object-header table (the author and committer
//! authorship rows, the commit id, the tree link, and one row per parent), the
//! message body, and the changed-files table (`git_difftree_body`). We modernize
//! the markup but keep the information: the same authorship, the same parent and
//! tree links, the same per-file change notes and action links.
//!
//! Every URL is gitweb's `href()`, built at the web boundary, so this layer takes
//! finished hrefs and decides only layout, escaping, and the markup of the
//! absolute authorship dates ([`timestamp_badge`]) and the message body
//! ([`crate::message`]). The changed-files table is shared with the commitdiff
//! view, so it is built from view-model rows the boundary fills for either action.

use gitweb_domain::model::message_body::LogLine;
use gitweb_domain::model::timestamp::Timestamp;

use crate::chrome::{Crumb, FormatLink, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};
use crate::message::log_lines;
use crate::timestamp::timestamp_badge;

/// One authorship row (author or committer): the identity's name, optional
/// email, and absolute date — gitweb's `git_print_authorship_rows`.
#[derive(Debug, Clone)]
pub struct AuthorRow {
    /// The identity's display name.
    pub name: String,
    /// The identity's email, rendered in angle brackets when present.
    pub email: Option<String>,
    /// The absolute date, rendered as the shared timestamp badge.
    pub timestamp: Timestamp,
}

/// An object id shown as a link: the id is the link text, `href` its target —
/// the commit view's tree row.
#[derive(Debug, Clone)]
pub struct LinkedId {
    /// The object id (the link text).
    pub id: String,
    /// The object view's URL.
    pub href: String,
}

/// One parent row: the parent's id linking to its commit view, plus the
/// `commit | diff` action links (gitweb's parent rows).
#[derive(Debug, Clone)]
pub struct ParentRow {
    /// The parent commit's id (the link text).
    pub id: String,
    /// The parent's `commit` view URL.
    pub commit_href: String,
    /// The `commitdiff` URL of this commit against this parent.
    pub diff_href: String,
}

/// The parent/merge context shown in the page navigation (gitweb's
/// `$formats_nav` for `commit`): `(initial)` for a root commit, `(parent: …)`
/// for a single parent, `(merge: … …)` for a merge.
#[derive(Debug, Clone)]
pub enum ParentNav {
    /// A root commit — gitweb's `(initial)`.
    Initial,
    /// A single-parent commit, linking to its one parent's commit view.
    Single(ParentNavLink),
    /// A merge, linking to each parent's commit view.
    Merge(Vec<ParentNavLink>),
}

/// One linked parent in the parent navigation: the short id and its commit URL.
#[derive(Debug, Clone)]
pub struct ParentNavLink {
    /// The abbreviated parent id (gitweb's `substr($parent, 0, 7)`).
    pub short: String,
    /// The parent's `commit` view URL.
    pub href: String,
}

/// The status note shown for a changed-files row — gitweb's `git_difftree_body`
/// `file_status` span. New/deleted/modified notes are link-free text; a
/// rename/copy note embeds a link to the old path's blob.
#[derive(Debug, Clone)]
pub enum ChangeNoteView {
    /// No note (a pure content modification).
    None,
    /// A new / deleted / modified note: a status category (CSS class) and its
    /// link-free text (the domain's [`FileChangeNote::text`]).
    ///
    /// [`FileChangeNote::text`]: gitweb_domain::model::file_change::FileChangeNote::text
    Text {
        /// The status category (`new` / `deleted` / `changed`).
        category: String,
        /// The link-free span text.
        text: String,
    },
    /// A rename / copy note: the category, a link to the old path, the similarity,
    /// and an optional new mode.
    Rename {
        /// The status category (`moved` / `copied`).
        category: String,
        /// The old path (the link text).
        from_path: String,
        /// The old path's `blob` URL.
        from_href: String,
        /// The rename/copy similarity percentage.
        similarity: u8,
        /// The new permission bits, present only when the mode changed.
        mode: Option<String>,
    },
}

/// One row of the changed-files table: the path (optionally linked), its status
/// note, and the per-file action links the boundary built.
#[derive(Debug, Clone)]
pub struct ChangedRow {
    /// The path shown in the row (the to-side path).
    pub path: String,
    /// The path's own link (to its blob), or `None` when it is not linkable
    /// (a path deleted in a merge).
    pub path_href: Option<String>,
    /// The status note span.
    pub note: ChangeNoteView,
    /// The per-file action links (`blob` / `diff` / `history` / …), in order.
    pub links: Vec<FormatLink>,
}

/// The whole commit page: the breadcrumb header, the action navigation with the
/// parent/merge context, the object-header table, the message body, and the
/// changed-files table.
#[derive(Debug, Clone)]
pub struct CommitPage {
    /// Breadcrumb trail (home / project / commit).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The parent/merge context shown on the right of the navigation.
    pub parent_nav: ParentNav,
    /// The commit title, shown as the page heading.
    pub title: String,
    /// The commit's own object id.
    pub commit_id: String,
    /// The tree row's linked id (links to the tree view).
    pub tree: LinkedId,
    /// The author authorship row.
    pub author: AuthorRow,
    /// The committer authorship row.
    pub committer: AuthorRow,
    /// One row per parent commit.
    pub parents: Vec<ParentRow>,
    /// The processed message body lines.
    pub comment: Vec<LogLine>,
    /// The changed-files rows (ordinary or combined, both flattened to rows).
    pub changed: Vec<ChangedRow>,
}

/// Renders the commit page body: the breadcrumb header, the navigation with the
/// parent context, the object-header table, the message body, and the
/// changed-files table — then the footer. The caller wraps this in the document
/// chrome (`gitweb_render::chrome::document`).
#[must_use]
pub fn commit_body(page: &CommitPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, Some(parent_nav(&page.parent_nav))))
        main class="content commit-view" {
            h2 class="section-title" { (page.title) }
            table class="object-header" {
                tbody {
                    (author_row("author", &page.author))
                    (author_row("committer", &page.committer))
                    tr {
                        td class="field" { "commit" }
                        td class="value sha1" { (page.commit_id) }
                        td class="link" {}
                    }
                    (tree_row(&page.tree))
                    @for parent in &page.parents {
                        (parent_row(parent))
                    }
                }
            }
            div class="page_body log" { (log_lines(&page.comment)) }
            (changed_files(&page.changed))
        }
    }
}

/// The parent/merge context for the navigation extra.
fn parent_nav(nav: &ParentNav) -> Markup {
    html! {
        span class="parent-nav" {
            @match nav {
                ParentNav::Initial => { "(initial)" }
                ParentNav::Single(parent) => {
                    "(parent: " a href=(parent.href) { (parent.short) } ")"
                }
                ParentNav::Merge(parents) => {
                    "(merge: "
                    @for (index, parent) in parents.iter().enumerate() {
                        @if index > 0 { " " }
                        a href=(parent.href) { (parent.short) }
                    }
                    ")"
                }
            }
        }
    }
}

/// One authorship row: the role label, the name with optional email, and the
/// absolute date (gitweb's `git_print_authorship_rows`). Shared with the
/// commitdiff host page ([`crate::commitdiff`]), which shows the same rows.
pub(crate) fn author_row(role: &str, who: &AuthorRow) -> Markup {
    html! {
        tr {
            td class="field" { (role) }
            td class="value" {
                span class="author-name" { (who.name) }
                @if let Some(email) = &who.email {
                    " " span class="author-email" { "<" (email) ">" }
                }
                " " (timestamp_badge(&who.timestamp))
            }
            td class="link" {}
        }
    }
}

/// The tree row: the tree id linking to the tree view, and a `tree` action link.
fn tree_row(tree: &LinkedId) -> Markup {
    html! {
        tr {
            td class="field" { "tree" }
            td class="value sha1" { a class="list" href=(tree.href) { (tree.id) } }
            td class="link" { a href=(tree.href) { "tree" } }
        }
    }
}

/// One parent row: the parent id linking to its commit, and `commit | diff`.
fn parent_row(parent: &ParentRow) -> Markup {
    html! {
        tr {
            td class="field" { "parent" }
            td class="value sha1" { a class="list" href=(parent.commit_href) { (parent.id) } }
            td class="link" {
                a href=(parent.commit_href) { "commit" }
                " | "
                a href=(parent.diff_href) { "diff" }
            }
        }
    }
}

/// The changed-files table: a count heading and one row per changed path, or a
/// note when the commit introduced no change. Shared with the commitdiff host
/// page ([`crate::commitdiff`]), whose rows carry the patch-anchor links.
pub(crate) fn changed_files(rows: &[ChangedRow]) -> Markup {
    html! {
        section class="changed-files" {
            @if rows.is_empty() {
                p class="note" { "No changed files." }
            } @else {
                h3 class="files-changed" { (rows.len()) " files changed" }
                table class="diff_tree" {
                    tbody {
                        @for row in rows {
                            (changed_row(row))
                        }
                    }
                }
            }
        }
    }
}

/// One changed-files row: the path (linked when linkable), the status note span,
/// and the per-file action links separated by `|`.
fn changed_row(row: &ChangedRow) -> Markup {
    html! {
        tr {
            td class="path" {
                @match &row.path_href {
                    Some(href) => a class="list" href=(href) { (row.path) },
                    None => (row.path),
                }
            }
            td class="status" { (change_note(&row.note)) }
            td class="link" {
                @for (index, link) in row.links.iter().enumerate() {
                    @if index > 0 { " | " }
                    a href=(link.href) { (link.label) }
                }
            }
        }
    }
}

/// The status note span: a link-free text note, a rename/copy note with its old
/// path linked, or nothing for a pure content modification.
fn change_note(note: &ChangeNoteView) -> Markup {
    html! {
        @match note {
            ChangeNoteView::None => {}
            ChangeNoteView::Text { category, text } => {
                span class={ "file_status " (category) } { "[" (text) "]" }
            }
            ChangeNoteView::Rename { category, from_path, from_href, similarity, mode } => {
                span class={ "file_status " (category) } {
                    "[" (category) " from "
                    a class="list" href=(from_href) { (from_path) }
                    " with " (similarity) "% similarity"
                    @if let Some(mode) = mode { ", mode: " (mode) }
                    "]"
                }
            }
        }
    }
}
