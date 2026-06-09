//! Ref badges on commit rows (modernized `format_ref_marker`).
//!
//! gitweb decorates a commit in the shortlog, log and history with a small badge
//! per ref whose tip is that commit, wrapped in a `<span class="refs">`. This view
//! takes the finished badges — their text, CSS class, tooltip and link the web
//! boundary built — and only lays them out, so every commit-list body can render
//! the same markers. The grouping rule and the link target are the domain's
//! ([`gitweb_domain::model::ref_marker`]); the URL is the web boundary's.

use crate::markup::{Markup, html};

/// One ref badge: its text, the CSS class (the ref kind, plus `indirect` for an
/// annotated tag), the `title` tooltip, and the finished link.
#[derive(Debug, Clone)]
pub struct RefMarkerView {
    /// The badge text — the ref name without its namespace (e.g. `v1.0`).
    pub name: String,
    /// The CSS class beyond the shared `ref`: the kind token and, for an
    /// annotated tag, a trailing `indirect` (e.g. `tag indirect`, `head`).
    pub class: String,
    /// The `title` tooltip — the ref name without `refs/` (e.g. `tags/v1.0`).
    pub title: String,
    /// The finished link the web boundary built for this badge.
    pub href: String,
}

/// Renders the ref badges for one commit row, wrapped in a `<span class="refs">`.
/// Emits nothing when there are no markers, mirroring gitweb returning the empty
/// string so a marker-less row stays clean.
#[must_use]
pub fn ref_markers(markers: &[RefMarkerView]) -> Markup {
    html! {
        @if !markers.is_empty() {
            span class="refs" {
                @for marker in markers {
                    a class=(format!("ref {}", marker.class)) href=(marker.href) title=(marker.title) {
                        (marker.name)
                    }
                }
            }
        }
    }
}
