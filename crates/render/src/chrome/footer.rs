//! The page footer (modernized `git_footer_html`).
//!
//! In project context gitweb shows the project description and RSS/Atom feed
//! links; in project-list context it shows OPML and TXT links instead. Which
//! links appear is decided upstream (the web boundary, which also builds their
//! URLs); this renders the optional description and a finished list of links.
//! The page-timing block and the JS bootstrap are separate concerns handled
//! elsewhere.

use crate::markup::{Markup, html};

/// One footer link (a feed, OPML, or TXT index).
#[derive(Debug, Clone)]
pub struct FooterLink {
    /// Visible label (e.g. `RSS`, `Atom`, `OPML`, `TXT`).
    pub label: String,
    /// Target URL.
    pub href: String,
    /// Link `title` attribute (e.g. `log RSS feed`).
    pub title: String,
}

/// The footer's content.
#[derive(Debug, Clone)]
pub struct PageFooter {
    /// Optional project description shown at the foot of the page.
    pub description: Option<String>,
    /// Footer links (feeds in project context; OPML/TXT in list context).
    pub links: Vec<FooterLink>,
}

/// Renders the page footer: an optional description and the footer links.
#[must_use]
pub fn footer(page_footer: &PageFooter) -> Markup {
    html! {
        footer class="page-footer" {
            @if let Some(description) = &page_footer.description {
                p class="footer-desc" { (description) }
            }
            @for link in &page_footer.links {
                a class="feed" href=(link.href) title=(link.title) { (link.label) }
            }
        }
    }
}
