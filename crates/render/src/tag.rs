//! The single annotated-tag view (modernized `git_tag`).
//!
//! gitweb's `git_tag` renders one tag object: a header naming the tag, a
//! one-row "object header" table linking the *tagged* object via its kind, the
//! tagger authorship rows when the tag carries a tagger, and the message body.
//! URLs are gitweb's `href()`, built at the web boundary, so this view takes the
//! finished object link and only decides layout, escaping, and the recency CSS
//! hook on the tagger's age. The message is pre-split into display lines by the
//! boundary (gitweb's `$tag{'comment'}` list).

use gitweb_domain::model::age::Age;

use crate::age::age_class_name;
use crate::chrome::{Crumb, PageFooter, footer, page_header};
use crate::markup::{Markup, html};

/// The tagged object as the object row presents it: the object's id (the link
/// text), the kind label gitweb prints (`commit` / `blob` / `tree` / `tag`), and
/// the finished URL of the object's own view, which both the id and the label
/// link to.
#[derive(Debug, Clone)]
pub struct TaggedObjectView {
    /// The tagged object's id, shown as the link text.
    pub id: String,
    /// The tagged object's kind label (gitweb's `$tag{'type'}`).
    pub kind_label: String,
    /// URL of the tagged object's view (gitweb's `href(action=>type, hash=>object)`).
    pub href: String,
}

/// The tagger authorship row, present only when the tag object carries a tagger:
/// the tagger's name, their email when the ident has one, and how long ago the
/// tag was made.
#[derive(Debug, Clone)]
pub struct TagAuthorView {
    /// The tagger's display name.
    pub name: String,
    /// The tagger's email, if any (rendered inside angle brackets).
    pub email: Option<String>,
    /// How long ago the tag was made, coloured by recency.
    pub age: Age,
}

/// The whole single-tag page body: the breadcrumb trail, the tag name heading,
/// the object-header table (object row + optional tagger row), and the message
/// body.
#[derive(Debug, Clone)]
pub struct TagPage {
    /// Breadcrumb trail (home / project / tag name).
    pub crumbs: Vec<Crumb>,
    /// The tag's name, shown as the page heading.
    pub name: String,
    /// The tagged object the object row links to.
    pub object: TaggedObjectView,
    /// The tagger authorship, present only when the tag carries a tagger.
    pub tagger: Option<TagAuthorView>,
    /// The message body, pre-split into display lines.
    pub message_lines: Vec<String>,
}

/// Renders the single-tag page body: the breadcrumb header, the tag-name
/// heading, the object-header table inside a `main` element, and the message
/// body, then the footer. The caller wraps this in the document
/// (`gitweb_render::chrome::document`).
#[must_use]
pub fn tag_body(page: &TagPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        main class="content" {
            h2 class="section-title" { (page.name) }
            table class="object-header" {
                tbody {
                    (object_row(&page.object))
                    (tagger_row(page.tagger.as_ref()))
                }
            }
            (message_body(&page.message_lines))
        }
        (footer(&PageFooter { description: None, links: Vec::new() }))
    }
}

/// The object row: the label `object`, the tagged object's id linking to its
/// view, and the kind label linking to the same view (gitweb's two `href`s to
/// `action=>type`).
fn object_row(object: &TaggedObjectView) -> Markup {
    html! {
        tr {
            td class="field" { "object" }
            td class="value" { a class="list" href=(object.href) { (object.id) } }
            td class="link" { a href=(object.href) { (object.kind_label) } }
        }
    }
}

/// The tagger row, emitted only when the tag carries a tagger (gitweb prints the
/// authorship rows `if defined($tag{'author'})`): the tagger's name, their email
/// in angle brackets when present, and the recency-coloured age.
fn tagger_row(tagger: Option<&TagAuthorView>) -> Markup {
    html! {
        @if let Some(tagger) = tagger {
            tr {
                td class="field" { "tagger" }
                td class="value" {
                    span class="tagger-name" { (tagger.name) }
                    @if let Some(email) = &tagger.email {
                        " " span class="tagger-email" { "<" (email) ">" }
                    }
                    " " (age_badge(tagger.age))
                }
                td class="link" {}
            }
        }
    }
}

/// The tagger's age, coloured by recency the same way the ref listings colour
/// theirs.
fn age_badge(age: Age) -> Markup {
    html! {
        span class={ "age " (age_class_name(age.class())) } { (age.humanized()) }
    }
}

/// The message body: each line of the tag message on its own, escaped, separated
/// by line breaks (gitweb prints each `$tag{'comment'}` line followed by
/// `<br/>`). An empty message renders an empty body.
fn message_body(lines: &[String]) -> Markup {
    html! {
        div class="tag-message" {
            @for line in lines {
                (line) br;
            }
        }
    }
}
