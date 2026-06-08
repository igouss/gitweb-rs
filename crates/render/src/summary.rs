//! The summary page (modernized `git_summary`).
//!
//! gitweb's landing page stacks a metadata table (description, owner, last
//! change, clone URLs), an optional raw `README.html`, then the recent shortlog,
//! tags, and heads — each under a header that links to its full action, and each
//! ending in a "..." link when it overran gitweb's cap of 16. The section tables
//! are exactly the ones the standalone heads / tags / shortlog pages render, so
//! this view reuses [`heads_table`], [`tags_table`], and [`shortlog_table`]
//! wholesale and only adds the page's own furniture: the metadata block, the
//! README sink, and the per-section headers.
//!
//! URLs are gitweb's `href()`, the web boundary's job, so every link arrives
//! finished. The README is the one place this view emits raw HTML: gitweb's
//! `insert_file` injects the file's bytes verbatim, so it passes through
//! [`raw`] — the caller (the summary use case) has already applied the
//! `prevent_xss` gate that decides whether a README reaches here at all.

use gitweb_domain::model::timestamp::Timestamp;

use crate::chrome::{Crumb, NavItem, PageFooter, footer, page_header, page_nav};
use crate::heads::{HeadsTable, heads_table};
use crate::markup::{Markup, html, raw};
use crate::shortlog::{ShortlogTable, shortlog_table};
use crate::tags::{TagsTable, tags_table};
use crate::timestamp::timestamp_badge;

/// The metadata block at the top of the summary: the always-shown description,
/// the owner and last-change time when present, and the clone URLs.
#[derive(Debug, Clone)]
pub struct SummaryMetadata {
    /// The project description (gitweb's `none` placeholder is already resolved
    /// by the use case).
    pub description: String,
    /// The owner, or `None` when absent or suppressed by `omit_owner`.
    pub owner: Option<String>,
    /// HEAD's committer time, or `None` for an unborn repository.
    pub last_change: Option<Timestamp>,
    /// The clone URLs to advertise, in order; empty when there are none.
    pub clone_urls: Vec<String>,
}

/// One summary section: the table to show and the URL of its full action, which
/// the section header links to.
#[derive(Debug, Clone)]
pub struct ShortlogSection {
    /// URL of the full `shortlog` action.
    pub href: String,
    /// The recent-history table (its own `more` row carries the "..." link).
    pub table: ShortlogTable,
}

/// The tags section: the tags table and the URL of the full `tags` action.
#[derive(Debug, Clone)]
pub struct TagsSection {
    /// URL of the full `tags` action.
    pub href: String,
    /// The tags table (its own `more` row carries the "..." link).
    pub table: TagsTable,
}

/// The heads section: the heads table and the URL of the full `heads` action.
#[derive(Debug, Clone)]
pub struct HeadsSection {
    /// URL of the full `heads` action.
    pub href: String,
    /// The heads table (its own `more` row carries the "..." link).
    pub table: HeadsTable,
}

/// The whole summary page body: the breadcrumb header, the action navigation,
/// the metadata block, the optional README, and the three sections — each shown
/// only when it has rows (gitweb prints a section only `if @list`).
#[derive(Debug, Clone)]
pub struct SummaryPage {
    /// Breadcrumb trail (home / project).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current `summary` view has no link.
    pub nav: Vec<NavItem>,
    /// The metadata block.
    pub metadata: SummaryMetadata,
    /// The raw `README.html` to inject verbatim, or `None`.
    pub readme: Option<String>,
    /// The shortlog section, present only when there is history.
    pub shortlog: Option<ShortlogSection>,
    /// The tags section, present only when there are tags.
    pub tags: Option<TagsSection>,
    /// The heads section, present only when there are branches.
    pub heads: Option<HeadsSection>,
}

/// Renders the summary page body: the breadcrumb header, the action navigation,
/// the metadata block, the optional README, and the shortlog / tags / heads
/// sections, all inside a `main` element, then the footer. The caller wraps this
/// in the document (`gitweb_render::chrome::document`).
#[must_use]
pub fn summary_body(page: &SummaryPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            (metadata_table(&page.metadata))
            @if let Some(readme) = &page.readme {
                (readme_block(readme))
            }
            @if let Some(section) = &page.shortlog {
                (section_header("shortlog", &section.href))
                (shortlog_table(&section.table))
            }
            @if let Some(section) = &page.tags {
                (section_header("tags", &section.href))
                (tags_table(&section.table))
            }
            @if let Some(section) = &page.heads {
                (section_header("heads", &section.href))
                (heads_table(&section.table))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// The metadata table: description, then owner and last change when present,
/// then a row per clone URL (the first labelled "URL", the rest unlabelled, as
/// gitweb's `format_repo_url`).
fn metadata_table(metadata: &SummaryMetadata) -> Markup {
    html! {
        table class="metadata" {
            tbody {
                tr id="metadata_desc" {
                    td class="label" { "description" }
                    td { (metadata.description) }
                }
                @if let Some(owner) = &metadata.owner {
                    tr id="metadata_owner" {
                        td class="label" { "owner" }
                        td { (owner) }
                    }
                }
                @if let Some(last_change) = &metadata.last_change {
                    tr id="metadata_lchange" {
                        td class="label" { "last change" }
                        td { (timestamp_badge(last_change)) }
                    }
                }
                (clone_url_rows(&metadata.clone_urls))
            }
        }
    }
}

/// One row per clone URL: gitweb labels the first "URL" and leaves the rest
/// blank (`format_repo_url`'s `$url_tag` reset).
fn clone_url_rows(clone_urls: &[String]) -> Markup {
    html! {
        @for (index, url) in clone_urls.iter().enumerate() {
            tr class="metadata_url" {
                td class="label" { @if index == 0 { "URL" } }
                td { (url) }
            }
        }
    }
}

/// The raw README block: gitweb's `insert_file` output, emitted verbatim through
/// the one raw-HTML sink. The use case already gated this on `prevent_xss`.
fn readme_block(readme: &str) -> Markup {
    html! {
        section class="readme" {
            h2 class="header" { "readme" }
            div class="readme-body" { (raw(readme)) }
        }
    }
}

/// A section header linking to the full action (modernized
/// `git_print_header_div`): the action name as a titled link.
fn section_header(title: &str, href: &str) -> Markup {
    html! {
        div class="header" {
            a class="title" href=(href) { (title) }
        }
    }
}
