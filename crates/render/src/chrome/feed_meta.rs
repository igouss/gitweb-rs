//! Laying out a page's feed auto-discovery `<link rel="alternate">` set —
//! gitweb's `print_feed_meta`.
//!
//! A project page advertises four feeds: RSS and Atom, each in a plain and a
//! `--no-merges` variant, titled `"<project> - <feed title> - <FORMAT> feed"`
//! (the no-merges variant adds `" (no merges)"`). The projects list instead
//! advertises its plain-text project index and its OPML feed, titled with the
//! site name. The descriptive feed title (gitweb's `get_feed_info`) and the
//! URLs are decided upstream — the domain rule and the web boundary; this
//! module owns only the link wording, ordering, and media types. Rendering the
//! `<link>` tags themselves is [`super::document`]'s job.

use super::document::FeedLink;

/// `application/rss+xml` — gitweb's RSS feed media type.
const RSS_MIME: &str = "application/rss+xml";
/// `application/atom+xml` — gitweb's Atom feed media type.
const ATOM_MIME: &str = "application/atom+xml";

/// The four feed URLs a project page advertises. The web boundary builds these
/// via gitweb's `href()` (`a=rss`/`a=atom`, the no-merges variants adding
/// `opt=--no-merges`); render never builds URLs, so it takes the finished
/// strings.
#[derive(Debug, Clone)]
pub struct FeedHrefs {
    /// `a=rss`.
    pub rss: String,
    /// `a=rss;opt=--no-merges`.
    pub rss_no_merges: String,
    /// `a=atom`.
    pub atom: String,
    /// `a=atom;opt=--no-merges`.
    pub atom_no_merges: String,
}

/// The four `<link rel="alternate">` feeds a project page advertises (gitweb's
/// `print_feed_meta` project branch): RSS then Atom, each in a plain and a
/// `(no merges)` variant, in that order. Each title is
/// `"<project> - <title> - <FORMAT> feed"`, with `" (no merges)"` appended to
/// the merge-filtered variant.
#[must_use]
pub fn project_feed_links(project: &str, title: &str, hrefs: &FeedHrefs) -> Vec<FeedLink> {
    vec![
        feed_link(project, title, "RSS", "", &hrefs.rss, RSS_MIME),
        feed_link(
            project,
            title,
            "RSS",
            " (no merges)",
            &hrefs.rss_no_merges,
            RSS_MIME,
        ),
        feed_link(project, title, "Atom", "", &hrefs.atom, ATOM_MIME),
        feed_link(
            project,
            title,
            "Atom",
            " (no merges)",
            &hrefs.atom_no_merges,
            ATOM_MIME,
        ),
    ]
}

/// One project feed link: `"<project> - <title> - <format> feed<suffix>"`.
fn feed_link(
    project: &str,
    title: &str,
    format: &str,
    suffix: &str,
    href: &str,
    mime: &str,
) -> FeedLink {
    FeedLink {
        title: format!("{project} - {title} - {format} feed{suffix}"),
        href: href.to_owned(),
        mime: mime.to_owned(),
    }
}

/// The two `<link rel="alternate">` feeds the projects list advertises (gitweb's
/// `print_feed_meta` projects-list branch): the plain-text project index, then
/// the OPML feed, both titled with the site name.
#[must_use]
pub fn project_list_feed_links(
    site_name: &str,
    project_index_href: &str,
    opml_href: &str,
) -> Vec<FeedLink> {
    vec![
        FeedLink {
            title: format!("{site_name} projects list"),
            href: project_index_href.to_owned(),
            mime: "text/plain; charset=utf-8".to_owned(),
        },
        FeedLink {
            title: format!("{site_name} projects feeds"),
            href: opml_href.to_owned(),
            mime: "text/x-opml".to_owned(),
        },
    ]
}
