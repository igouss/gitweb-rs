//! The HTML5 document skeleton and `<head>` (modernized `git_header_html`).
//!
//! gitweb emits an XHTML 1.0 document with an XML prolog; we emit modern
//! `<!DOCTYPE html>` HTML5. The *behaviour* preserved is what the head carries:
//! the page title, the stylesheet, an optional favicon, optional syndication
//! `<link rel="alternate">` feed metadata, and gitweb's `robots: index,
//! nofollow` policy. Title/URL assembly is the web boundary's job; this takes
//! finished strings.

use super::footer::{PageFooter, footer};
use crate::markup::{DOCTYPE, Markup, html};

/// One `<link rel="alternate">` syndication feed advertised in the head
/// (gitweb's `print_feed_meta`).
#[derive(Debug, Clone)]
pub struct FeedLink {
    /// Human-readable feed title (e.g. `"project - log - RSS feed"`).
    pub title: String,
    /// URL of the feed.
    pub href: String,
    /// MIME type, e.g. `application/rss+xml` or `application/atom+xml`.
    pub mime: String,
}

/// Everything the document `<head>` needs. Strings are already-decoded text;
/// escaping happens at render time.
#[derive(Debug, Clone)]
pub struct DocumentHead {
    /// `<title>` text.
    pub title: String,
    /// URL of the stylesheet to link.
    pub stylesheet_href: String,
    /// Optional favicon URL.
    pub favicon_href: Option<String>,
    /// Optional syndication feeds advertised via `<link rel="alternate">`.
    pub feeds: Vec<FeedLink>,
}

/// Wraps a rendered page `body` in the full HTML5 document: `head`'s title,
/// stylesheet, optional favicon, and optional feed metadata in the `<head>`,
/// and `foot`'s footer chrome (gitweb's `git_footer_html`) closing the
/// `<body>` below the content — the head and the footer bracket the page the
/// same way.
#[must_use]
pub fn document(head: &DocumentHead, foot: &PageFooter, body: Markup) -> Markup {
    html! {
        (DOCTYPE)
        html lang="en" {
            head {
                meta charset="utf-8";
                meta name="viewport" content="width=device-width, initial-scale=1";
                meta name="generator" content="gitweb-rs";
                meta name="robots" content="index, nofollow";
                title { (head.title) }
                link rel="stylesheet" href=(head.stylesheet_href);
                @if let Some(favicon) = &head.favicon_href {
                    link rel="icon" href=(favicon);
                }
                @for feed in &head.feeds {
                    link rel="alternate" type=(feed.mime) title=(feed.title) href=(feed.href);
                }
            }
            body { (body) (footer(foot)) }
        }
    }
}
