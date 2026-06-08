//! The per-page action navigation (modernized `git_print_page_nav`).
//!
//! gitweb's action bar — `summary | shortlog | log | commit | commitdiff |
//! tree` — with the current view shown as plain text rather than a link, and an
//! optional trailing extra (the pager, or alternate formats). Which actions
//! appear, and their URLs, are the web boundary's job; this renders the finished
//! item list and the optional pre-rendered extra.

use crate::markup::{Markup, html};

/// One entry in the action bar. `href` absent marks the current view, rendered
/// as plain text instead of a link.
#[derive(Debug, Clone)]
pub struct NavItem {
    /// Visible label (the action name).
    pub label: String,
    /// Target URL, or `None` for the current view.
    pub href: Option<String>,
}

/// A "show more" / pager affordance — gitweb's `$extra` next-page link. The data
/// is shared (a target URL and a label), but each view places it differently: the
/// shortlog as a trailing table row, the verbose log as a trailing nav link.
#[derive(Debug, Clone)]
pub struct MoreLink {
    /// Target URL of the further page.
    pub href: String,
    /// Visible label (`...` or `next`).
    pub label: String,
}

/// Renders the action bar: each item is a link, except the current view (no
/// `href`), which is plain text. A `|` separator sits between items, and any
/// pre-rendered `extra` (pager, formats) is appended.
#[must_use]
pub fn page_nav(items: &[NavItem], extra: Option<Markup>) -> Markup {
    html! {
        nav class="page-nav" {
            @for (index, item) in items.iter().enumerate() {
                @if index > 0 { span class="sep" { "|" } }
                @match &item.href {
                    Some(href) => a href=(href) { (item.label) },
                    None => span class="current" { (item.label) },
                }
            }
            @if let Some(extra) = extra {
                div class="nav-extra" { (extra) }
            }
        }
    }
}
