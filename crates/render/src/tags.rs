//! The tags table (modernized `git_tags` / `git_tags_body`).
//!
//! gitweb renders the tags as a table with one row per tag: the creation age,
//! the tag name linking to the tagged object via that object's kind, the
//! annotation subject (chopped, linking to the single `tag` view) for annotated
//! tags, a `tag` selflink for annotated tags, and reftype-dependent links —
//! `shortlog` / `log` for a commit, `raw` for a blob, nothing more for a tree.
//! URLs are gitweb's `href()`, the web boundary's job, so this view takes
//! finished hrefs and only decides layout, escaping, the subject chop, and the
//! recency CSS hook. The age is a domain [`Age`]; an absent age is "unknown".

use gitweb_domain::model::age::Age;
use gitweb_domain::model::chop::{ChopMode, chop_str};

use crate::age::age_class_name;
use crate::chrome::{Crumb, MoreLink, NavItem, PageFooter, footer, page_header, page_nav};
use crate::markup::{Markup, html};

/// gitweb chops a tag's subject to 30 characters with 5 of slack (`chop_str
/// $comment, 30, 5`) in `git_tags_body`.
const SUBJECT_LEN: usize = 30;
const SUBJECT_SLACK: usize = 5;

/// The kind of object a tag names, carrying the reftype-dependent quick links:
/// `shortlog` / `log` for a commit, `raw` for a blob, none for a tree.
#[derive(Debug, Clone)]
pub enum TagReftype {
    /// The tag names a commit; offers `shortlog` and `log` of the tag.
    Commit {
        /// `shortlog` URL for the tag.
        shortlog: String,
        /// `log` URL for the tag.
        log: String,
    },
    /// The tag names a blob; offers a `raw` link.
    Blob {
        /// `blob_plain` URL for the tagged blob.
        raw: String,
    },
    /// The tag names a tree; no extra link.
    Tree,
}

impl TagReftype {
    /// The reftype label gitweb prints as the link text (`commit` / `blob` /
    /// `tree`).
    #[must_use]
    pub fn label(&self) -> &'static str {
        match self {
            TagReftype::Commit { .. } => "commit",
            TagReftype::Blob { .. } => "blob",
            TagReftype::Tree => "tree",
        }
    }
}

/// One tag row: its name (linking to the tagged object), the annotation subject
/// and `tag` selflink for annotated tags, the reftype-dependent links, and the
/// creation age.
#[derive(Debug, Clone)]
pub struct TagEntryView {
    /// Creation age, or `None` for a tag with no creation time ("unknown").
    pub age: Option<Age>,
    /// Short tag name, shown and linked to the tagged object.
    pub name: String,
    /// URL of the tagged object (gitweb's `href(action=>reftype, hash=>refid)`);
    /// the name and the reftype label both link here.
    pub object_href: String,
    /// Annotation subject, present only for annotated tags.
    pub subject: Option<String>,
    /// URL of the single `tag` view (`href(action=>tag, hash=>id)`); the subject
    /// and the selflink point here.
    pub tag_view_href: String,
    /// Whether this is an annotated tag (gitweb's `type eq "tag"`); only then is
    /// the `tag` selflink shown.
    pub annotated: bool,
    /// The tagged object's kind and its reftype-dependent links.
    pub reftype: TagReftype,
}

/// The tags table: the tag rows in display order, with an optional trailing
/// "more" row (gitweb's `$extra` — the summary page's "..." link to the full
/// tags action; absent on the standalone tags page, which lists every tag).
#[derive(Debug, Clone)]
pub struct TagsTable {
    /// The tag rows, already ordered by the use case.
    pub rows: Vec<TagEntryView>,
    /// The "more" affordance, present only when the list was capped (the summary
    /// page past 16 tags).
    pub more: Option<MoreLink>,
}

/// The whole tags page body: the breadcrumb trail, the refs sub-navigation
/// (tags | heads | …), and the table.
#[derive(Debug, Clone)]
pub struct TagsPage {
    /// Breadcrumb trail (home / project / tags).
    pub crumbs: Vec<Crumb>,
    /// The refs sub-navigation; the current view has no link.
    pub ref_views: Vec<NavItem>,
    /// The tag table.
    pub table: TagsTable,
}

/// Renders the tags page body: the breadcrumb header, the refs sub-navigation,
/// the table inside a `main` element (or a note when there are no tags), and the
/// footer. The caller wraps this in the document (`gitweb_render::chrome::document`).
#[must_use]
pub fn tags_body(page: &TagsPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.ref_views, None))
        main class="content" {
            h2 class="section-title" { "Tags" }
            @if page.table.rows.is_empty() {
                p class="note" { "No tags." }
            } @else {
                (tags_table(&page.table))
            }
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// Renders the tags table: a row per tag, then the optional "more" row.
#[must_use]
pub fn tags_table(table: &TagsTable) -> Markup {
    html! {
        table class="tags" {
            tbody {
                @for row in &table.rows {
                    (tag_row(row))
                }
                @if let Some(more) = &table.more {
                    tr class="more" {
                        td colspan="5" { a href=(more.href) { (more.label) } }
                    }
                }
            }
        }
    }
}

/// One tag row: age, name linking to the tagged object, the subject and `tag`
/// selflink for annotated tags, and the reftype-dependent links.
fn tag_row(row: &TagEntryView) -> Markup {
    html! {
        tr {
            (age_cell(row.age))
            td class="name" { a class="list name" href=(row.object_href) { (row.name) } }
            (subject_cell(row.subject.as_deref(), &row.tag_view_href))
            (selflink_cell(row.annotated, &row.tag_view_href))
            (link_cell(row))
        }
    }
}

/// The age cell: gitweb's relative creation age coloured by recency, or
/// "unknown" for a tag with no creation time (gitweb's `$tag{'age'}` absent).
fn age_cell(age: Option<Age>) -> Markup {
    html! {
        @match age {
            Some(age) => td class={ "age " (age_class_name(age.class())) } { (age.humanized()) },
            None => td class="age age-unknown" { "unknown" },
        }
    }
}

/// The subject cell: gitweb's `format_subject_html`, present only for annotated
/// tags. The subject is chopped to [`SUBJECT_LEN`] (with [`SUBJECT_SLACK`]) and
/// links to the single `tag` view; when chopping shortens it, the full text is
/// kept as a `title`.
fn subject_cell(subject: Option<&str>, tag_view_href: &str) -> Markup {
    html! {
        td class="subject" {
            @if let Some(full) = subject {
                (subject_link(full, tag_view_href))
            }
        }
    }
}

/// The subject link (gitweb's `format_subject_html`): the chopped subject, with
/// the full text as a `title` only when chopping actually shortened it.
fn subject_link(full: &str, href: &str) -> Markup {
    let short: String = chop_str(full, SUBJECT_LEN, SUBJECT_SLACK, ChopMode::Right);
    html! {
        @if short == full {
            a class="list subject" href=(href) { (full) }
        } @else {
            a class="list subject" href=(href) title=(full) { (short) }
        }
    }
}

/// The selflink cell: gitweb prints a `tag` link to the single tag view only
/// when the ref is a tag object (annotated); a lightweight tag's cell is empty.
fn selflink_cell(annotated: bool, tag_view_href: &str) -> Markup {
    html! {
        td class="selflink" {
            @if annotated {
                a href=(tag_view_href) { "tag" }
            }
        }
    }
}

/// The reftype link cell: the reftype label linking to the tagged object,
/// followed by the reftype-dependent links — `shortlog` / `log` for a commit,
/// `raw` for a blob, none for a tree.
fn link_cell(row: &TagEntryView) -> Markup {
    html! {
        td class="links" {
            a href=(row.object_href) { (row.reftype.label()) }
            (reftype_links(&row.reftype))
        }
    }
}

/// The reftype-dependent links gitweb appends after the reftype label.
fn reftype_links(reftype: &TagReftype) -> Markup {
    html! {
        @match reftype {
            TagReftype::Commit { shortlog, log } => {
                " | " a href=(shortlog) { "shortlog" }
                " | " a href=(log) { "log" }
            }
            TagReftype::Blob { raw } => {
                " | " a href=(raw) { "raw" }
            }
            TagReftype::Tree => {}
        }
    }
}
