//! The remotes page (modernized `git_remotes` / `git_remote_block`).
//!
//! gitweb's remotes view stacks, per configured remote, a small URL table
//! (`format_repo_url` rows: "URL" / "Fetch URL" / "Push URL", or "No remote URL")
//! over the remote's tracking branches rendered by the very same `git_heads_body`
//! the heads page uses. The all-remotes view groups each remote under a header
//! that links to its single-remote view; the single-remote view shows just one
//! block under a title naming the remote, with no per-block header.
//!
//! So this view reuses [`heads_table`] wholesale for the tracking branches and
//! only adds the page's own furniture: the title header, the per-remote section
//! header, and the URL table. URLs are gitweb's `href()`, the web boundary's job,
//! so every link and the URL-line labels arrive finished — this layer only lays
//! them out and escapes them.

use crate::chrome::{Crumb, NavItem, PageFooter, footer, page_header, page_nav};
use crate::heads::{HeadsTable, heads_table};
use crate::markup::{Markup, html};

/// One URL line of a remote's block: the label gitweb's `format_repo_url` shows
/// (absent for the "No remote URL" placeholder) and the value (a URL, or the
/// placeholder text). The web boundary maps the domain URL-line rule onto these.
#[derive(Debug, Clone)]
pub struct RemoteUrlLine {
    /// The row label (`URL` / `Fetch URL` / `Push URL`), or `None` for the
    /// unlabelled placeholder row.
    pub label: Option<String>,
    /// The URL, or the `No remote URL` placeholder text.
    pub value: String,
}

/// One remote's block: its name, an optional link to its single-remote view
/// (present in the all-remotes view, absent in the single-remote view), its URL
/// lines, and its tracking branches as a heads table.
#[derive(Debug, Clone)]
pub struct RemoteBlockView {
    /// The remote's name.
    pub name: String,
    /// The single-remote view URL the name links to, or `None` when this *is* the
    /// single-remote view (the title already names the remote).
    pub href: Option<String>,
    /// The URL lines (gitweb's `git_remote_block` URL table).
    pub urls: Vec<RemoteUrlLine>,
    /// The remote's tracking branches, rendered as the heads table.
    pub heads: HeadsTable,
}

/// The whole remotes page body: the breadcrumb header, the refs sub-navigation
/// (tags | heads | remotes), a title naming the page, and one block per remote.
#[derive(Debug, Clone)]
pub struct RemotesPage {
    /// Breadcrumb trail (home / project / remotes).
    pub crumbs: Vec<Crumb>,
    /// The refs sub-navigation; the current view's own entry has no link.
    pub ref_views: Vec<NavItem>,
    /// The page title: `<project> remotes` (all) or `<remote> remote for
    /// <project>` (single).
    pub title: String,
    /// The remote blocks, in name order (one block for the single-remote view).
    pub blocks: Vec<RemoteBlockView>,
}

/// Renders the remotes page body: the breadcrumb header, the refs sub-navigation,
/// the title, and one block per remote inside a `main` element, then the footer.
/// The caller wraps this in the document (`gitweb_render::chrome::document`).
#[must_use]
pub fn remotes_body(page: &RemotesPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.ref_views, None))
        main class="content" {
            div class="header" { span class="title" { (page.title) } }
            @for block in &page.blocks {
                (remote_block(block))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// One remote's block: its name header (linking to the single-remote view, only
/// in the all-remotes view), the URL table, and its tracking-branch heads table.
fn remote_block(block: &RemoteBlockView) -> Markup {
    html! {
        section class="remote" {
            @if let Some(href) = &block.href {
                div class="header" { a class="title" href=(href) { (block.name) } }
            }
            (url_table(&block.urls))
            (heads_table(&block.heads))
        }
    }
}

/// The URL table: one row per URL line, the label in the first cell (blank for
/// the placeholder) and the value in the second (gitweb's `format_repo_url`).
fn url_table(urls: &[RemoteUrlLine]) -> Markup {
    html! {
        table class="remote-urls" {
            tbody {
                @for line in urls {
                    tr class="metadata_url" {
                        td class="label" { @if let Some(label) = &line.label { (label) } }
                        td { (line.value) }
                    }
                }
            }
        }
    }
}
