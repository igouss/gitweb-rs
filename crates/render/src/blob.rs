//! The blob body (modernized `git_blob`).
//!
//! gitweb shows one file three ways. A text blob is rendered line by line, each
//! line numbered and anchorable (gitweb's `<a id="lN" .. class="linenr">`); an
//! image is shown inline as an `<img>`; any other binary is not inlined and is
//! offered only as a raw download. An empty text file shows a short note rather
//! than an empty table.
//!
//! Every URL is built by the web boundary, so this layer takes finished hrefs
//! and only places them; every value is escaped by maud. Whitespace in source
//! lines (including tabs) is preserved by the stylesheet, not by rewriting the
//! text here.

use crate::chrome::{Crumb, FormatLink, NavItem, formats_nav, page_header, page_nav};
use crate::markup::{Markup, html};

/// One source line: its 1-based number (the `lN` anchor) and its text.
#[derive(Debug, Clone)]
pub struct BlobLine {
    /// The 1-based line number, used as the `lN` id and `#lN` link.
    pub number: usize,
    /// The decoded line text.
    pub text: String,
}

/// The body of a blob view, by how the file is to be shown.
#[derive(Debug, Clone)]
pub enum BlobContent {
    /// A text file: its lines, numbered and anchorable. Empty means an empty
    /// file, shown as a note.
    Text {
        /// The file's lines in order.
        lines: Vec<BlobLine>,
    },
    /// An image, shown inline: the raw `src` and the `alt`/title text.
    Image {
        /// The raw-blob URL the `<img>` loads.
        src: String,
        /// The alt and title text (the file name).
        alt: String,
    },
    /// A non-image binary, offered only as a raw download at `raw_href`.
    Binary {
        /// The raw-blob URL the download link points at.
        raw_href: String,
    },
}

/// The whole blob page body: the breadcrumb header, the action navigation with
/// the file affordances, the title (commit subject or blob hash), the file path,
/// and the content.
#[derive(Debug, Clone)]
pub struct BlobPage {
    /// Breadcrumb trail (home / project / blob).
    pub crumbs: Vec<Crumb>,
    /// The per-project action navigation; the current view has no link.
    pub nav: Vec<NavItem>,
    /// The file affordances shown on the right (blame / history / raw / HEAD).
    pub formats: Vec<FormatLink>,
    /// The header title: the commit subject, or the blob hash for a loose blob.
    pub title: String,
    /// The file path, shown when known.
    pub path: Option<String>,
    /// The file content.
    pub content: BlobContent,
}

/// Renders the blob page body: the breadcrumb header, the action navigation with
/// the file affordances, the title and path, the content inside a `main`, and the
/// footer. The caller wraps this in the document chrome.
#[must_use]
pub fn blob_body(page: &BlobPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, formats_nav(&page.formats)))
        main class="content" {
            h2 class="section-title" { (page.title) }
            @if let Some(path) = &page.path {
                p class="path" { (path) }
            }
            (blob_content(&page.content))
        }
    }
}

/// Renders just the file content (the part that varies by display kind): the
/// numbered line table for text (a note when empty), an inline `<img>` for an
/// image, or a download link for any other binary.
#[must_use]
pub fn blob_content(content: &BlobContent) -> Markup {
    match content {
        BlobContent::Text { lines } if lines.is_empty() => html! {
            p class="note" { "Empty file." }
        },
        BlobContent::Text { lines } => html! {
            table class="blob" {
                tbody {
                    @for line in lines {
                        tr id=(format!("l{}", line.number)) {
                            td class="linenr" {
                                a href=(format!("#l{}", line.number)) { (line.number) }
                            }
                            td class="line" { (line.text) }
                        }
                    }
                }
            }
        },
        BlobContent::Image { src, alt } => html! {
            img class="blob-image" src=(src) alt=(alt) title=(alt);
        },
        BlobContent::Binary { raw_href } => html! {
            p class="note" {
                "This is a binary file. "
                a href=(raw_href) { "Download raw" }
            }
        },
    }
}
