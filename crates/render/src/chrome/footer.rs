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

/// The two project-page feed URLs a footer links (gitweb's `href(action =>
/// rss/atom, …)`): RSS and Atom, scoped to the branch and file
/// `get_feed_info` found. The web boundary builds these; render takes the
/// finished strings.
#[derive(Debug, Clone)]
pub struct FooterFeedHrefs {
    /// `a=rss`.
    pub rss: String,
    /// `a=atom`.
    pub atom: String,
}

/// The RSS and Atom feed anchors a project page's footer shows (gitweb's
/// `git_footer_html` project branch): RSS then Atom, each titled
/// `"<feed title> <FORMAT> feed"` — the feed title is `get_feed_info`'s,
/// defaulting to `log`.
#[must_use]
pub fn project_footer_links(feed_title: &str, hrefs: &FooterFeedHrefs) -> Vec<FooterLink> {
    vec![
        FooterLink {
            label: "RSS".to_owned(),
            href: hrefs.rss.clone(),
            title: format!("{feed_title} RSS feed"),
        },
        FooterLink {
            label: "Atom".to_owned(),
            href: hrefs.atom.clone(),
            title: format!("{feed_title} Atom feed"),
        },
    ]
}

/// The OPML and plain-text-index anchors the projects-list footer shows
/// (gitweb's `git_footer_html` list branch): OPML then TXT, in that order.
#[must_use]
pub fn project_list_footer_links(opml_href: &str, project_index_href: &str) -> Vec<FooterLink> {
    vec![
        FooterLink {
            label: "OPML".to_owned(),
            href: opml_href.to_owned(),
            title: "OPML".to_owned(),
        },
        FooterLink {
            label: "TXT".to_owned(),
            href: project_index_href.to_owned(),
            title: "TXT".to_owned(),
        },
    ]
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
