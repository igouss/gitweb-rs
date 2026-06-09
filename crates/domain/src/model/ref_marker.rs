//! Ref badges on commit rows: gitweb's `format_ref_marker`.
//!
//! gitweb decorates a commit in the shortlog, log and history bodies with a small
//! badge per ref whose tip is that commit (`git_get_references` reads every ref
//! once with `show-ref --dereference`; `format_ref_marker` renders the markers for
//! a given commit id). An annotated tag points at a tag object that peels to the
//! commit, so it arrives [`indirect`](crate::model::reference::DereferencedRef::indirect)
//! and links to the `tag` view rather than the commit log; a lightweight tag or a
//! branch head points straight at the commit and links to the current list view.
//!
//! This is the pure grouping rule: given the dereferenced refs and a commit, it
//! yields the ordered badges for that commit. The link *URLs* are the web
//! boundary's job (gitweb's `href`), so a marker carries the ingredients — the
//! destination [`Action`] and the fully-qualified ref to put in its `hash` — not a
//! finished link. The badge markup is the render layer's.

use crate::model::action::Action;
use crate::model::object_id::ObjectId;
use crate::model::reference::DereferencedRef;

/// The semantic category of a ref badge — gitweb's `$type`, derived from the ref
/// namespace and used as the badge's CSS class.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RefKind {
    /// A tag (`refs/tags/...`).
    Tag,
    /// A branch head (`refs/heads/...`).
    Head,
    /// A remote-tracking branch (`refs/remotes/...`).
    Remote,
    /// Any other namespace, carrying gitweb's `$type` token verbatim.
    Other(String),
}

impl RefKind {
    /// gitweb's `$type` token — the badge's CSS class.
    #[must_use]
    pub fn class_token(&self) -> &str {
        match self {
            Self::Tag => "tag",
            Self::Head => "head",
            Self::Remote => "remote",
            Self::Other(token) => token,
        }
    }

    /// Classifies gitweb's `$type` (the namespace with its trailing plural `s`
    /// already removed, e.g. `tag`, `head`, `remote`).
    fn from_type(token: &str) -> Self {
        match token {
            "tag" => Self::Tag,
            "head" => Self::Head,
            "remote" => Self::Remote,
            other => Self::Other(other.to_owned()),
        }
    }
}

/// Which commit-list body the markers decorate. gitweb keys `format_ref_marker`'s
/// `$dest_action` off the current `$action`; this is the slice of that relevant to
/// the badge link.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum MarkerView {
    /// The shortlog (and the summary, which reuses it).
    Shortlog,
    /// The verbose log.
    Log,
    /// A file's history.
    History,
    /// A single annotated tag's page.
    Tag,
    /// Any other page that shows commit rows.
    Other,
}

/// One ref badge decorating a commit row.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RefMarker {
    kind: RefKind,
    indirect: bool,
    name: String,
    title: String,
    action: Action,
    dest: String,
}

impl RefMarker {
    /// The badge category (its CSS class).
    #[must_use]
    pub fn kind(&self) -> &RefKind {
        &self.kind
    }

    /// Whether the ref is an annotated tag (gitweb's `indirect` class modifier).
    #[must_use]
    pub fn indirect(&self) -> bool {
        self.indirect
    }

    /// The text shown in the badge — gitweb's `$name`, the ref name without its
    /// namespace prefix (e.g. `v1.0`, `next`, `origin/next`).
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }

    /// The badge's `title` tooltip — gitweb's `$ref`, the ref name without the
    /// `refs/` prefix (e.g. `tags/v1.0`, `heads/next`).
    #[must_use]
    pub fn title(&self) -> &str {
        &self.title
    }

    /// The view the badge links to — gitweb's `$dest_action`.
    #[must_use]
    pub fn action(&self) -> Action {
        self.action
    }

    /// The fully-qualified ref the link targets in its `hash` — gitweb's `$dest`.
    #[must_use]
    pub fn dest(&self) -> &str {
        &self.dest
    }
}

/// The badges for `commit`, in the order the refs are given, drawn from the refs
/// whose [`target`](DereferencedRef::target) is `commit`. Mirrors
/// `format_ref_marker($refs, $id)` for one commit id.
#[must_use]
pub fn markers_for(
    refs: &[DereferencedRef],
    commit: &ObjectId,
    view: MarkerView,
) -> Vec<RefMarker> {
    refs.iter()
        .filter(|reference: &&DereferencedRef| reference.target() == commit)
        .map(|reference: &DereferencedRef| marker_of(reference, view))
        .collect()
}

/// Builds one badge from a ref tip that lands on the commit row.
fn marker_of(reference: &DereferencedRef, view: MarkerView) -> RefMarker {
    // gitweb's `$ref`: the name without the `refs/` prefix (and without the
    // `^{}` peel, which never reaches us — the indirect flag carries it). It is
    // both the badge's title and the source of the type/name split.
    let full: &str = reference.name().full();
    let title: &str = full.strip_prefix("refs/").unwrap_or(full);
    let (type_token, name): (&str, &str) = split_type(title);
    let indirect: bool = reference.indirect();
    RefMarker {
        kind: RefKind::from_type(type_token),
        indirect,
        name: name.to_owned(),
        title: title.to_owned(),
        action: dest_action(indirect, view),
        // gitweb's `$dest`: prefix `refs/` back on when the name lacks it.
        dest: format!("refs/{title}"),
    }
}

/// gitweb's `m!^(.*?)s?/(.*)$!`: split on the first `/`, dropping one trailing
/// plural `s` from the namespace to get the type (`tags` -> `tag`). A name with
/// no `/` is typed `ref` and kept whole.
fn split_type(stripped: &str) -> (&str, &str) {
    match stripped.split_once('/') {
        Some((namespace, name)) => (namespace.strip_suffix('s').unwrap_or(namespace), name),
        None => ("ref", stripped),
    }
}

/// gitweb's `$dest_action`: an annotated tag links to the `tag` view (unless the
/// reader is already on a tag page, where it falls back to the default shortlog);
/// a direct ref follows the current list view (log/history/shortlog) and
/// otherwise defaults to the shortlog.
fn dest_action(indirect: bool, view: MarkerView) -> Action {
    if indirect {
        match view {
            MarkerView::Tag => Action::Shortlog,
            _ => Action::Tag,
        }
    } else {
        match view {
            MarkerView::Log => Action::Log,
            MarkerView::History => Action::History,
            _ => Action::Shortlog,
        }
    }
}
