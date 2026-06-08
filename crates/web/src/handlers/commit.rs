//! The `commit` handler: gitweb's `git_commit` page (one commit's metadata,
//! message, and changed files).
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_commit`] use case for the requested revision, then maps
//! the resulting view-model to the render layer's page — building every link with
//! [`href`], since URLs are the boundary's job. The changed-files rows are the
//! faithful link logic of gitweb's `git_difftree_body`: which `blob` / `blobdiff`
//! / `history` / `blame` links a row carries depends on its status and on whether
//! the diff is ordinary or combined. The `blame` affordance is gated on the
//! `blame` feature, read here at the boundary, the way gitweb's `$have_blame`
//! does.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::change::ChangeKind;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::commit::{
    AuthorLine, ChangedFiles, CombinedChange, CommitView, OrdinaryChange, assemble_commit,
};
use gitweb_render::chrome::{Crumb, DocumentHead, FormatLink, NavItem, document};
use gitweb_render::commit::{
    AuthorRow, ChangeNoteView, ChangedRow, CommitPage, LinkedId, ParentNav, ParentNavLink,
    ParentRow, commit_body,
};
use gitweb_render::markup::Markup;

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::dispatch::Handler;
use crate::response::View;
use crate::url::href;

/// Serves the commit page over a wired project store and the resolved settings.
pub struct CommitHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl CommitHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name, chrome, and `blame` feature from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for CommitHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        // gitweb's `$hash ||= $hash_base || "HEAD"`: the commit, then the base, then HEAD.
        let revision: Option<&str> = request
            .hash
            .as_ref()
            .or(request.hash_base.as_ref())
            .map(SafeRef::as_str);
        let view: CommitView = assemble_commit(repository.as_ref(), revision)?;
        let blame_on: bool = self.settings.feature(FeatureName::Blame).enabled();
        Ok(View::html(render_page(
            &self.settings,
            project,
            &view,
            blame_on,
        )))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(settings: &Settings, project: &str, view: &CommitView, blame_on: bool) -> Markup {
    let hash: &str = view.id().as_str();
    let tree: &str = view.tree().as_str();
    let page: CommitPage = CommitPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, hash, tree),
        parent_nav: parent_nav(project, view.parents()),
        title: view.title().to_owned(),
        commit_id: hash.to_owned(),
        tree: LinkedId {
            id: tree.to_owned(),
            href: tree_href(project, tree, hash),
        },
        author: author_row(view.author()),
        committer: author_row(view.committer()),
        parents: parent_rows(project, hash, view.parents()),
        comment: view.comment().to_vec(),
        changed: changed_rows(project, view, blame_on),
    };
    let head: DocumentHead = DocumentHead {
        title: format!("{project} / commit / {}", view.title()),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    document(&head, commit_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then commit.
fn crumbs(site_name: &str, project: &str) -> Vec<Crumb> {
    vec![
        Crumb {
            label: site_name.to_owned(),
            href: Some(href(&[])),
        },
        Crumb {
            label: project.to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        Crumb {
            label: "commit".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('commit', …)`):
/// summary, the shortlog and verbose log scoped to this commit, commit (the
/// current view), commitdiff, and the tree at this commit.
fn nav(project: &str, hash: &str, tree: &str) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(href(&[("p", project), ("a", "shortlog"), ("h", hash)])),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(href(&[("p", project), ("a", "log"), ("h", hash)])),
        },
        NavItem {
            label: "commit".to_owned(),
            href: None,
        },
        NavItem {
            label: "commitdiff".to_owned(),
            href: Some(href(&[("p", project), ("a", "commitdiff"), ("h", hash)])),
        },
        NavItem {
            label: "tree".to_owned(),
            href: Some(tree_href(project, tree, hash)),
        },
    ]
}

/// The parent/merge context for the navigation: `(initial)` for a root commit,
/// `(parent: …)` for one parent, `(merge: … …)` for a merge.
fn parent_nav(project: &str, parents: &[ObjectId]) -> ParentNav {
    match parents {
        [] => ParentNav::Initial,
        [parent] => ParentNav::Single(parent_nav_link(project, parent)),
        many => ParentNav::Merge(
            many.iter()
                .map(|parent: &ObjectId| parent_nav_link(project, parent))
                .collect(),
        ),
    }
}

/// One linked parent in the navigation: the abbreviated id and its commit URL.
fn parent_nav_link(project: &str, parent: &ObjectId) -> ParentNavLink {
    ParentNavLink {
        short: short_id(parent),
        href: href(&[("p", project), ("a", "commit"), ("h", parent.as_str())]),
    }
}

/// One parent row of the object header: the parent id linking to its commit, and
/// `commit | diff` (the commitdiff of this commit against that parent).
fn parent_rows(project: &str, hash: &str, parents: &[ObjectId]) -> Vec<ParentRow> {
    parents
        .iter()
        .map(|parent: &ObjectId| ParentRow {
            id: parent.as_str().to_owned(),
            commit_href: href(&[("p", project), ("a", "commit"), ("h", parent.as_str())]),
            diff_href: href(&[
                ("p", project),
                ("a", "commitdiff"),
                ("h", hash),
                ("hp", parent.as_str()),
            ]),
        })
        .collect()
}

/// An authorship row from a use-case authorship line.
fn author_row(line: &AuthorLine) -> AuthorRow {
    AuthorRow {
        name: line.name().to_owned(),
        email: line.email().map(str::to_owned),
        timestamp: line.timestamp().clone(),
    }
}

/// gitweb's `substr($parent, 0, 7)`: the seven-character abbreviated id.
fn short_id(oid: &ObjectId) -> String {
    oid.as_str().chars().take(7).collect()
}

/// The changed-files rows, mapped from the use case's ordinary or combined diff.
fn changed_rows(project: &str, view: &CommitView, blame_on: bool) -> Vec<ChangedRow> {
    let hash: &str = view.id().as_str();
    match view.changes() {
        ChangedFiles::Ordinary(rows) => {
            let parent: Option<&str> = view.parents().first().map(ObjectId::as_str);
            rows.iter()
                .map(|change: &OrdinaryChange| {
                    ordinary_row(project, hash, parent, blame_on, change)
                })
                .collect()
        }
        ChangedFiles::Combined(rows) => rows
            .iter()
            .map(|change: &CombinedChange| combined_row(project, hash, view.parents(), change))
            .collect(),
    }
}

/// One ordinary changed-files row, with the status note and per-file links
/// gitweb's `git_difftree_body` builds for the `commit` action.
fn ordinary_row(
    project: &str,
    hash: &str,
    parent: Option<&str>,
    blame_on: bool,
    change: &OrdinaryChange,
) -> ChangedRow {
    let to_id: &str = change.to_oid().as_str();
    let from_id: &str = change.from_oid().as_str();
    match change.status().kind() {
        ChangeKind::Added => {
            let blob: String = blob_href(project, to_id, hash, change.to_path());
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(blob.clone()),
                note: text_note(change),
                links: vec![format_link("blob", blob)],
            }
        }
        ChangeKind::Deleted => {
            let base: &str = parent.unwrap_or(hash);
            let blob: String = blob_href(project, from_id, base, change.to_path());
            let mut links: Vec<FormatLink> = vec![format_link("blob", blob.clone())];
            push_blame(&mut links, blame_on, project, base, change.to_path());
            links.push(history_link(project, base, change.to_path()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(blob),
                note: text_note(change),
                links,
            }
        }
        ChangeKind::Modified | ChangeKind::TypeChanged => {
            let base: &str = parent.unwrap_or(hash);
            let blob: String = blob_href(project, to_id, hash, change.to_path());
            let mut links: Vec<FormatLink> = Vec::new();
            if to_id != from_id {
                links.push(format_link(
                    "diff",
                    blobdiff_href(project, to_id, from_id, hash, base, change.to_path(), None),
                ));
            }
            links.push(format_link("blob", blob.clone()));
            push_blame(&mut links, blame_on, project, hash, change.to_path());
            links.push(history_link(project, hash, change.to_path()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(blob),
                note: text_note(change),
                links,
            }
        }
        ChangeKind::Renamed | ChangeKind::Copied => {
            let base: &str = parent.unwrap_or(hash);
            let path_link: String = blob_href(project, to_id, hash, change.to_path());
            let note: ChangeNoteView = rename_note(project, base, change);
            let mut links: Vec<FormatLink> = Vec::new();
            if to_id != from_id {
                links.push(format_link(
                    "diff",
                    blobdiff_href(
                        project,
                        to_id,
                        from_id,
                        hash,
                        base,
                        change.to_path(),
                        Some(change.from_path()),
                    ),
                ));
            }
            links.push(format_link(
                "blob",
                blob_href(project, to_id, base, change.to_path()),
            ));
            push_blame(&mut links, blame_on, project, hash, change.to_path());
            links.push(history_link(project, hash, change.to_path()));
            ChangedRow {
                path: change.to_path().to_owned(),
                path_href: Some(path_link),
                note,
                links,
            }
        }
    }
}

/// One combined (merge) changed-files row: the per-parent `diffN` / `blobN`
/// links, then the result `blob` and `history`, mirroring gitweb's combined
/// branch of `git_difftree_body`.
fn combined_row(
    project: &str,
    hash: &str,
    parents: &[ObjectId],
    change: &CombinedChange,
) -> ChangedRow {
    let to_id: &str = change.to_oid().as_str();
    let to_path: &str = change.to_path();
    let mut links: Vec<FormatLink> = Vec::new();
    for (index, side) in change.parents().iter().enumerate() {
        let parent: &str = parents.get(index).map(ObjectId::as_str).unwrap_or(hash);
        match side.status().kind() {
            ChangeKind::Added => {}
            ChangeKind::Deleted => links.push(format_link(
                format!("blob{}", index + 1),
                blob_href(project, side.from_oid().as_str(), hash, to_path),
            )),
            _ => links.push(format_link(
                format!("diff{}", index + 1),
                blobdiff_href(
                    project,
                    to_id,
                    side.from_oid().as_str(),
                    hash,
                    parent,
                    to_path,
                    Some(to_path),
                ),
            )),
        }
    }
    if change.not_deleted() {
        links.push(format_link(
            "blob",
            blob_href(project, to_id, hash, to_path),
        ));
    }
    if change.has_history() {
        links.push(history_link(project, hash, to_path));
    }
    ChangedRow {
        path: to_path.to_owned(),
        path_href: (!change.is_deleted()).then(|| blob_href(project, to_id, hash, to_path)),
        note: ChangeNoteView::None,
        links,
    }
}

/// The status note view for a new / deleted / modified row: the domain's
/// link-free note text, or no note for a pure content modification.
fn text_note(change: &OrdinaryChange) -> ChangeNoteView {
    match change.note() {
        None => ChangeNoteView::None,
        Some(note) => ChangeNoteView::Text {
            category: note.category().to_owned(),
            text: note.text().unwrap_or_default(),
        },
    }
}

/// The rename/copy note view: the category, a link to the old path's blob (at the
/// parent), the similarity, and any mode change.
fn rename_note(project: &str, base: &str, change: &OrdinaryChange) -> ChangeNoteView {
    let note = change
        .note()
        .expect("a rename or copy carries a status note");
    ChangeNoteView::Rename {
        category: note.category().to_owned(),
        from_path: change.from_path().to_owned(),
        from_href: blob_href(
            project,
            change.from_oid().as_str(),
            base,
            change.from_path(),
        ),
        similarity: note.similarity().unwrap_or_default(),
        mode: note.mode().map(str::to_owned),
    }
}

/// Appends a `blame` link for `path` at `base` when the `blame` feature is on.
fn push_blame(links: &mut Vec<FormatLink>, blame_on: bool, project: &str, base: &str, path: &str) {
    if blame_on {
        links.push(format_link(
            "blame",
            href(&[("p", project), ("a", "blame"), ("hb", base), ("f", path)]),
        ));
    }
}

/// A `blob` URL: the object id, the base revision it is viewed at, and the path.
fn blob_href(project: &str, oid: &str, base: &str, path: &str) -> String {
    href(&[
        ("p", project),
        ("a", "blob"),
        ("h", oid),
        ("hb", base),
        ("f", path),
    ])
}

/// A `blobdiff` URL between two blobs of a path, optionally a rename's old path.
fn blobdiff_href(
    project: &str,
    to_id: &str,
    from_id: &str,
    hash: &str,
    parent: &str,
    path: &str,
    file_parent: Option<&str>,
) -> String {
    let mut params: Vec<(&str, &str)> = vec![
        ("p", project),
        ("a", "blobdiff"),
        ("h", to_id),
        ("hp", from_id),
        ("hb", hash),
        ("hpb", parent),
        ("f", path),
    ];
    if let Some(from_path) = file_parent {
        params.push(("fp", from_path));
    }
    href(&params)
}

/// A `tree` URL at this commit (gitweb's `href(action=>tree, hash=>tree, hash_base=>hash)`).
fn tree_href(project: &str, tree: &str, hash: &str) -> String {
    href(&[("p", project), ("a", "tree"), ("h", tree), ("hb", hash)])
}

/// A `history` action link for `path` at `base`.
fn history_link(project: &str, base: &str, path: &str) -> FormatLink {
    format_link(
        "history",
        href(&[("p", project), ("a", "history"), ("hb", base), ("f", path)]),
    )
}

/// A labelled link.
fn format_link(label: impl Into<String>, href: String) -> FormatLink {
    FormatLink {
        label: label.into(),
        href,
    }
}
