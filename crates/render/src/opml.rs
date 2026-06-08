//! gitweb's `git_opml` body — the OPML 1.0 project outline.
//!
//! Format-stable (verified by golden conformance, not just behaviourally), so
//! this reproduces gitweb's XML to the byte: the `<opml>` wrapper, a `<title>`
//! of `<site name> OPML Export`, and one `<outline type="rss" …>` per project
//! inside the `git RSS feeds` group. The boundary supplies the title's site name
//! (raw), each project's chopped path (raw) and its absolute `rss`/`summary`
//! links (raw [`href_full`](../../gitweb_web/url/fn.href_full.html) output);
//! here we escape — `esc_html` for the title and the `text`/`title` attributes,
//! `esc_attr` for the URL attributes — and wrap.

use crate::escape::{esc_attr, esc_html};

/// One project in the outline: its chopped path (shown as both `text` and
/// `title`) and the absolute links to its feed and summary page, all raw.
#[derive(Debug, Clone)]
pub struct OpmlRowView {
    /// The chopped project path (raw) — gitweb's `esc_html(chop_str($path,25,5))`
    /// input.
    pub text: String,
    /// The absolute `rss` feed URL (raw `href_full` output) — `xmlUrl`.
    pub xml_url: String,
    /// The absolute `summary` page URL (raw `href_full` output) — `htmlUrl`.
    pub html_url: String,
}

/// The whole outline: the site name behind the `<title>` and the project rows.
#[derive(Debug, Clone)]
pub struct OpmlView {
    /// The site name (raw); this layer escapes it and appends `" OPML Export"`.
    pub site_name: String,
    /// The projects, in discovery order.
    pub rows: Vec<OpmlRowView>,
}

/// Serializes the OPML outline (gitweb's `git_opml` body).
#[must_use]
pub fn opml(view: &OpmlView) -> String {
    let outlines: String = view.rows.iter().map(outline).collect();
    format!(
        "<?xml version=\"1.0\" encoding=\"utf-8\"?>\n\
         <opml version=\"1.0\">\n\
         <head>\n\
         \x20\x20<title>{title} OPML Export</title>\n\
         </head>\n\
         <body>\n\
         <outline text=\"git RSS feeds\">\n\
         {outlines}\
         </outline>\n\
         </body>\n\
         </opml>\n",
        title = esc_html(&view.site_name),
    )
}

/// One `<outline type="rss" …/>` line, the path escaped for HTML and both URLs
/// escaped as attribute values.
fn outline(row: &OpmlRowView) -> String {
    let text: String = esc_html(&row.text);
    format!(
        "<outline type=\"rss\" text=\"{text}\" title=\"{text}\" xmlUrl=\"{}\" htmlUrl=\"{}\"/>\n",
        esc_attr(&row.xml_url),
        esc_attr(&row.html_url),
    )
}
