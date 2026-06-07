//! The page header: optional logo and the breadcrumb trail (modernized
//! `git_header_html`'s `page_header` div + `print_nav_breadcrumbs`).
//!
//! The trail's assembly — home link, project path segments, project, action —
//! needs gitweb's `href()` and so belongs to the web boundary. This layer
//! renders a finished list of crumbs: each is either a link or the current
//! (plain-text) location.

use crate::markup::{Markup, html};

/// The site logo, linked to a home/landing URL.
#[derive(Debug, Clone)]
pub struct Logo {
    /// Where the logo links to.
    pub href: String,
    /// `src` of the logo image.
    pub image_src: String,
    /// Accessible label / link title.
    pub label: String,
}

/// One step in the breadcrumb trail. `href` absent means this is the current
/// location and is rendered as plain text, not a link.
#[derive(Debug, Clone)]
pub struct Crumb {
    /// Visible label.
    pub label: String,
    /// Target URL, or `None` for the current (non-link) crumb.
    pub href: Option<String>,
}

/// Renders the breadcrumb trail: each crumb is a link, except a current crumb
/// (no `href`), which is plain text. A `/` separator sits between crumbs.
#[must_use]
pub fn breadcrumbs(crumbs: &[Crumb]) -> Markup {
    html! {
        nav class="breadcrumbs" {
            @for (index, crumb) in crumbs.iter().enumerate() {
                @if index > 0 { span class="sep" { "/" } }
                @match &crumb.href {
                    Some(href) => a href=(href) { (crumb.label) },
                    None => span class="current" { (crumb.label) },
                }
            }
        }
    }
}

/// Renders the page header: an optional logo followed by the breadcrumb trail.
#[must_use]
pub fn page_header(logo: Option<&Logo>, crumbs: &[Crumb]) -> Markup {
    html! {
        header class="page-header" {
            @if let Some(logo) = logo {
                a class="logo" href=(logo.href) title=(logo.label) {
                    img src=(logo.image_src) alt=(logo.label);
                }
            }
            (breadcrumbs(crumbs))
        }
    }
}
