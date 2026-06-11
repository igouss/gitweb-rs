//! The `tree` handler: gitweb's `git_tree` page (directory listing).
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_tree`] use case over it for the requested base revision
//! (HEAD by default) and optional path, then maps the resulting view-model to
//! the render layer's table — building every link with [`href`], since URLs are
//! the boundary's job — wraps it in the document chrome, and returns the page as
//! a [`View`]. The `show-sizes` feature is read from the resolved settings here,
//! at the boundary, and handed to the use case.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::tree::{TreeRow, TreeView, assemble_tree};
use gitweb_render::chrome::{Crumb, DocumentHead, NavItem, document};
use gitweb_render::markup::Markup;
use gitweb_render::tree::{TreeLink, TreePage, TreeParentRow, TreeRowView, TreeTable, tree_body};

use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::response::View;
use crate::url::href;

/// Serves the tree page over a wired project store and the resolved settings.
pub struct TreeHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl TreeHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the site name, chrome, and `show-sizes` from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }
}

impl Handler for TreeHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let rev: Option<&str> = request.hash_base.as_ref().map(SafeRef::as_str);
        let path: Option<&str> = request.file_name.as_ref().map(SafePath::as_str);
        let show_sizes: bool = self.settings.feature(FeatureName::ShowSizes).enabled();
        let view: TreeView = assemble_tree(repository.as_ref(), rev, path, show_sizes)?;
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;
        Ok(View::html(render_page(
            &self.settings,
            project,
            rev,
            &view,
            chrome,
        )))
    }
}

/// Maps the use-case view to the render view-model and wraps the assembled body
/// in the document chrome — the boundary owns the asset URLs and every link.
fn render_page(
    settings: &Settings,
    project: &str,
    rev: Option<&str>,
    view: &TreeView,
    chrome: PageChrome,
) -> Markup {
    let path: Option<&str> = view.path();
    let page: TreePage = TreePage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, rev),
        title: view
            .commit_title()
            .unwrap_or_else(|| view.base())
            .to_owned(),
        path: path.map(str::to_owned),
        table: TreeTable {
            show_sizes: view.show_sizes(),
            parent: view
                .parent()
                .map(|parent| parent_row(project, rev, parent.path())),
            rows: view
                .rows()
                .iter()
                .map(|row: &TreeRow| entry_row(project, rev, path, row))
                .collect(),
        },
    };
    let head: DocumentHead = document_head(tree_title(project, path), chrome.feeds, chrome.scripts);
    document(&head, &chrome.foot, tree_body(&page))
}

/// The document `<title>`: `project / tree` at the root, `project / tree / path`
/// inside a directory.
fn tree_title(project: &str, path: Option<&str>) -> String {
    match path {
        Some(path) => format!("{project} / tree / {path}"),
        None => format!("{project} / tree"),
    }
}

/// The breadcrumb trail: home, the project (linking to its summary), then tree.
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
            label: "tree".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar: summary, the shortlog and verbose log, all scoped
/// to the same revision when one was named.
fn nav(project: &str, rev: Option<&str>) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(scoped(project, "shortlog", rev)),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(scoped(project, "log", rev)),
        },
    ]
}

/// An action link scoped to the named revision through `h`, or unscoped (the
/// adapter defaults that to HEAD) when no revision was named.
fn scoped(project: &str, action: &str, rev: Option<&str>) -> String {
    match rev {
        Some(revision) => href(&[("p", project), ("a", action), ("h", revision)]),
        None => href(&[("p", project), ("a", action)]),
    }
}

/// The `..` row's link: the parent directory's tree view (gitweb drops the path
/// when the parent is the repository root).
fn parent_row(project: &str, rev: Option<&str>, parent: Option<&str>) -> TreeParentRow {
    TreeParentRow {
        href: tree_href(project, rev, parent),
    }
}

/// Maps one use-case row to a render row, building its links by the entry's git
/// object type, the way gitweb's `git_print_tree_entry` branches.
fn entry_row(project: &str, rev: Option<&str>, dir: Option<&str>, row: &TreeRow) -> TreeRowView {
    let full: String = match dir {
        Some(dir) => format!("{dir}/{}", row.name()),
        None => row.name().to_owned(),
    };
    let size: Option<String> = row.size().map(|bytes: u64| bytes.to_string());
    match row.object_kind() {
        ObjectKind::Tree => directory_row(project, rev, row, &full, size),
        ObjectKind::Commit => submodule_row(project, rev, row, &full, size),
        _ => blob_row(project, rev, row, &full, size),
    }
}

/// A file row: the name links to the blob, the cell offers blob | history | raw.
fn blob_row(
    project: &str,
    rev: Option<&str>,
    row: &TreeRow,
    full: &str,
    size: Option<String>,
) -> TreeRowView {
    let blob: String = blob_href(project, rev, row.oid(), full);
    TreeRowView {
        mode: row.mode().permission_string().to_owned(),
        size,
        name: row.name().to_owned(),
        name_href: Some(blob.clone()),
        symlink_target: row.symlink_target().map(str::to_owned),
        links: vec![
            TreeLink {
                label: "blob".to_owned(),
                href: blob,
            },
            TreeLink {
                label: "history".to_owned(),
                href: history_href(project, rev, full),
            },
            TreeLink {
                label: "raw".to_owned(),
                href: raw_href(project, rev, full),
            },
        ],
    }
}

/// A directory row: the name links to the nested tree, the cell offers tree |
/// history.
fn directory_row(
    project: &str,
    rev: Option<&str>,
    row: &TreeRow,
    full: &str,
    size: Option<String>,
) -> TreeRowView {
    let tree: String = subtree_href(project, rev, row.oid(), full);
    TreeRowView {
        mode: row.mode().permission_string().to_owned(),
        size,
        name: row.name().to_owned(),
        name_href: Some(tree.clone()),
        symlink_target: None,
        links: vec![
            TreeLink {
                label: "tree".to_owned(),
                href: tree,
            },
            TreeLink {
                label: "history".to_owned(),
                href: history_href(project, rev, full),
            },
        ],
    }
}

/// A gitlink/submodule row: the name is plain text, with only a history link.
fn submodule_row(
    project: &str,
    rev: Option<&str>,
    row: &TreeRow,
    full: &str,
    size: Option<String>,
) -> TreeRowView {
    TreeRowView {
        mode: row.mode().permission_string().to_owned(),
        size,
        name: row.name().to_owned(),
        name_href: None,
        symlink_target: None,
        links: vec![TreeLink {
            label: "history".to_owned(),
            href: history_href(project, rev, full),
        }],
    }
}

/// The blob view of a file: `a=blob`, the entry's id, its full path, and the
/// base revision.
fn blob_href(project: &str, rev: Option<&str>, oid: &str, full: &str) -> String {
    match rev {
        Some(base) => href(&[
            ("p", project),
            ("a", "blob"),
            ("h", oid),
            ("hb", base),
            ("f", full),
        ]),
        None => href(&[("p", project), ("a", "blob"), ("h", oid), ("f", full)]),
    }
}

/// The nested tree view of a subdirectory: `a=tree`, the subtree id, its full
/// path, and the base revision.
fn subtree_href(project: &str, rev: Option<&str>, oid: &str, full: &str) -> String {
    match rev {
        Some(base) => href(&[
            ("p", project),
            ("a", "tree"),
            ("h", oid),
            ("hb", base),
            ("f", full),
        ]),
        None => href(&[("p", project), ("a", "tree"), ("h", oid), ("f", full)]),
    }
}

/// The tree view of a directory by path alone (the `..` row): `a=tree`, the base
/// revision, and the path — dropped when the path is the repository root.
fn tree_href(project: &str, rev: Option<&str>, path: Option<&str>) -> String {
    match (rev, path) {
        (Some(base), Some(dir)) => href(&[("p", project), ("a", "tree"), ("hb", base), ("f", dir)]),
        (Some(base), None) => href(&[("p", project), ("a", "tree"), ("hb", base)]),
        (None, Some(dir)) => href(&[("p", project), ("a", "tree"), ("f", dir)]),
        (None, None) => href(&[("p", project), ("a", "tree")]),
    }
}

/// The per-path history view: `a=history`, the base revision, and the full path.
fn history_href(project: &str, rev: Option<&str>, full: &str) -> String {
    match rev {
        Some(base) => href(&[("p", project), ("a", "history"), ("hb", base), ("f", full)]),
        None => href(&[("p", project), ("a", "history"), ("f", full)]),
    }
}

/// The raw blob view: `a=blob_plain`, the base revision, and the full path.
fn raw_href(project: &str, rev: Option<&str>, full: &str) -> String {
    match rev {
        Some(base) => href(&[
            ("p", project),
            ("a", "blob_plain"),
            ("hb", base),
            ("f", full),
        ]),
        None => href(&[("p", project), ("a", "blob_plain"), ("f", full)]),
    }
}
