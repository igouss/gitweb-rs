//! Mapping a domain ref marker to its render badge — the link-building half of
//! gitweb's `format_ref_marker`.
//!
//! The domain decides which refs badge a commit and where each badge points
//! (gitweb's `$dest_action` / `$dest`, [`gitweb_domain::model::ref_marker`]); the
//! badge's URL is the boundary's job (gitweb's `href`). This helper is that one
//! step, shared by every commit-list handler — shortlog, log, history — so the
//! badge link is built identically wherever it appears.

use gitweb_domain::model::ref_marker::RefMarker;
use gitweb_render::refs::RefMarkerView;

use crate::url::href;

/// Maps a domain ref marker to the render badge, building its link from the
/// marker's destination action and ref (gitweb's `href(action=>..., hash=>...)`).
/// The CSS class is the ref kind, plus `indirect` for an annotated tag.
pub(crate) fn marker_view(project: &str, marker: &RefMarker) -> RefMarkerView {
    let class: String = if marker.indirect() {
        format!("{} indirect", marker.kind().class_token())
    } else {
        marker.kind().class_token().to_owned()
    };
    RefMarkerView {
        name: marker.name().to_owned(),
        class,
        title: marker.title().to_owned(),
        href: href(&[
            ("p", project),
            ("a", marker.action().as_str()),
            ("h", marker.dest()),
        ]),
    }
}
