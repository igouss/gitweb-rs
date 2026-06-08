//! The `commit` handler: gitweb's `git_commit` page (one commit's metadata,
//! message, and changed files).
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`],
//! runs the [`assemble_commit`] use case for the requested revision, then maps
//! the resulting view-model to the render layer's page — building every link with
//! [`href`], since URLs are the boundary's job. The changed-files rows are the
//! faithful link logic of gitweb's `git_difftree_body`, shared with the
//! commitdiff view through [`changed_files`](crate::handlers::changed_files);
//! the `commit` context links a modified file to its blobdiff. The `blame`
//! affordance is gated on the `blame` feature, read here at the boundary, the
//! way gitweb's `$have_blame` does.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::commit::{AuthorLine, CommitView, assemble_commit};
use gitweb_render::chrome::{Crumb, DocumentHead, NavItem, document};
use gitweb_render::commit::{
    AuthorRow, CommitPage, LinkedId, ParentNav, ParentNavLink, ParentRow, commit_body,
};
use gitweb_render::markup::Markup;

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::dispatch::Handler;
use crate::handlers::changed_files::{self, Context};
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
        changed: changed_files::rows(Context::Commit, project, view, blame_on),
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

/// A `tree` URL at this commit (gitweb's `href(action=>tree, hash=>tree, hash_base=>hash)`).
fn tree_href(project: &str, tree: &str, hash: &str) -> String {
    href(&[("p", project), ("a", "tree"), ("h", tree), ("hb", hash)])
}
