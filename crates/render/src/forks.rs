//! The forks page (modernized `git_forks`).
//!
//! gitweb's forks view lists a project's forks with the very same
//! `git_project_list_body` the landing page uses, under a `$project forks`
//! header and the per-project page navigation. So this view reuses
//! [`project_list`] wholesale for the table and only adds the page's own
//! furniture: the breadcrumb header, the project action bar, and the title.
//! URLs arrive finished from the web boundary, so this layer only lays them out.

use crate::chrome::{Crumb, NavItem, page_header, page_nav};
use crate::markup::{Markup, html};
use crate::project_list::{ProjectList, project_list};

/// The whole forks page body: the breadcrumb trail (home / project / forks), the
/// per-project action bar, a title naming the project, and the forks list table.
#[derive(Debug, Clone)]
pub struct ForksPage {
    /// Breadcrumb trail (home / project / forks).
    pub crumbs: Vec<Crumb>,
    /// The per-project action bar (summary, shortlog, log, tree).
    pub nav: Vec<NavItem>,
    /// The page title: `<project> forks`.
    pub title: String,
    /// The forks list, rendered as the project-list table (with its fork column).
    pub list: ProjectList,
}

/// Renders the forks page body: the breadcrumb header, the project action bar,
/// the title, and the forks table inside a `main` element, then the footer. The
/// caller wraps this in the document (`gitweb_render::chrome::document`).
#[must_use]
pub fn forks_body(page: &ForksPage) -> Markup {
    html! {
        (page_header(None, &page.crumbs))
        (page_nav(&page.nav, None))
        main class="content" {
            div class="header" { span class="title" { (page.title) } }
            (project_list(&page.list))
        }
    }
}
