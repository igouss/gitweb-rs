//! The refs sub-navigation, gitweb's `format_ref_views`.
//!
//! gitweb shows `tags | heads` above the ref-listing views, and appends
//! `remotes` only when the `remote_heads` feature is enabled
//! (`push @ref_views, 'remotes' if gitweb_check_feature('remote_heads')`). The
//! view whose own page this is renders as plain text instead of a link. Shared by
//! the heads, tags, and remotes handlers so the bar is identical across all three
//! and the remotes entry appears (or not) consistently.

use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_render::chrome::NavItem;

use crate::url::href;

/// Which ref view is current — rendered as plain text rather than a link. `None`
/// is gitweb's single-remote view, where every entry links (its `format_ref_views('')`).
#[derive(Debug, Clone, Copy)]
pub(crate) enum CurrentRef {
    /// The tags listing is current.
    Tags,
    /// The heads listing is current.
    Heads,
    /// The all-remotes view is current.
    Remotes,
    /// No entry is current (the single-remote view).
    None,
}

/// The refs sub-navigation: `tags | heads`, plus `remotes` when the
/// `remote_heads` feature is enabled. The `current` view is plain text.
pub(crate) fn ref_views(project: &str, settings: &Settings, current: CurrentRef) -> Vec<NavItem> {
    let mut items: Vec<NavItem> = vec![
        nav_item("tags", project, matches!(current, CurrentRef::Tags)),
        nav_item("heads", project, matches!(current, CurrentRef::Heads)),
    ];
    if settings.feature(FeatureName::RemoteHeads).enabled() {
        items.push(nav_item(
            "remotes",
            project,
            matches!(current, CurrentRef::Remotes),
        ));
    }
    items
}

/// One ref-view entry: a link to its action, or plain text when it is current.
fn nav_item(action: &str, project: &str, current: bool) -> NavItem {
    NavItem {
        label: action.to_owned(),
        href: (!current).then(|| href(&[("p", project), ("a", action)])),
    }
}
