//! Building a page's feed auto-discovery `<head>` links — the boundary half of
//! gitweb's `print_feed_meta`.
//!
//! gitweb prints these `<link rel="alternate">` tags on every `200 OK` HTML
//! page (`print_header_links`). This module is the glue: it reads the page's
//! request and the deployment settings, classifies the feed with the domain
//! rule ([`feed_info`]), builds the feed URLs with [`href`] (URL assembly is the
//! boundary's job), and hands the parts to the render layer
//! ([`project_feed_links`] / [`project_list_feed_links`]) to lay out.
//!
//! Divergence from gitweb: gitweb's `href()` joins parameters with `;` in its
//! canonical order; our modernized HTML pages use [`href`], which joins with `&`
//! in call order — the same dialect every other link on the page uses. The
//! format-stable feed *bodies* keep gitweb's byte-faithful form elsewhere.

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::branch_refs::get_branch_refs;
use gitweb_domain::model::feed_meta::{FeedInfo, feed_info};
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_render::chrome::{
    DocumentHead, FeedHrefs, FeedLink, FooterFeedHrefs, PageFooter, project_feed_links,
    project_footer_links, project_list_feed_links, project_list_footer_links,
};

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};
use crate::url::href;

/// A page's document chrome built from the request: the head's syndication
/// feeds and the footer. The head and the footer bracket the page the same way,
/// so they travel together — every HTML handler builds this once with
/// [`page_chrome`] and threads it to its render helper, which turns `feeds`
/// into the [`DocumentHead`] (with the page's title) and hands `foot` to
/// [`document`](gitweb_render::chrome::document).
#[derive(Debug, Clone)]
pub struct PageChrome {
    /// The head's feed auto-discovery links (gitweb's `print_feed_meta`).
    pub feeds: Vec<FeedLink>,
    /// The footer (gitweb's `git_footer_html`).
    pub foot: PageFooter,
}

/// Builds a page's whole document chrome — the head's feeds and the footer —
/// from the request, the boundary half of gitweb's `print_feed_meta` plus
/// `git_footer_html`. One seam so the head and the footer are built and
/// threaded identically.
///
/// # Errors
/// [`DomainError::Backend`] when a malformed `extra-branch-refs` entry fails
/// validation (gitweb's `die_error(500, ...)`), surfaced as it is at the other
/// [`get_branch_refs`] consumers.
///
/// The footer reads the project description from `store` (gitweb's
/// `git_get_project_description`); the head feeds need no store, so only the
/// footer half consults it.
pub fn page_chrome(
    store: &dyn ProjectStore,
    settings: &Settings,
    request: &Request,
) -> Result<PageChrome, DomainError> {
    Ok(PageChrome {
        feeds: page_feeds(settings, request)?,
        foot: page_footer(store, settings, request)?,
    })
}

/// Builds a page's document `<head>` view-model: its title, the shared
/// stylesheet and favicon, and the feed auto-discovery links. Every HTML
/// handler funnels through here so the chrome boilerplate lives in one place.
#[must_use]
pub fn document_head(title: String, feeds: Vec<FeedLink>) -> DocumentHead {
    DocumentHead {
        title,
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds,
    }
}

/// Builds a page's footer view-model — the boundary half of gitweb's
/// `git_footer_html`. The head and the footer bracket the page the same way:
/// every HTML handler builds this from the request and hands it to
/// [`document`](gitweb_render::chrome::document) alongside the head.
///
/// In project context it shows the project description (gitweb's
/// `git_get_project_description`, read from `store`, only when defined) above
/// the page's RSS and Atom feeds (the same feed the head advertises, classified
/// by [`project_feed_info`], but as visible anchors titled `"<feed title>
/// <FORMAT> feed"` and without the head's `--no-merges` variants); on the
/// projects list it links the OPML feed and the plain-text index and shows no
/// description (gitweb has no project to describe there).
///
/// # Errors
/// [`DomainError::Backend`] when a malformed `extra-branch-refs` entry fails
/// validation (gitweb's `die_error(500, ...)`), the same failure [`page_feeds`]
/// surfaces; or any error [`ProjectStore::description`] surfaces.
pub fn page_footer(
    store: &dyn ProjectStore,
    settings: &Settings,
    request: &Request,
) -> Result<PageFooter, DomainError> {
    let Some(project) = request.project.as_deref() else {
        // gitweb's projects-list branch: the OPML feed and the plain-text index,
        // scoped to the active project_filter when one is set (see
        // [`list_feed_href`]). No project, no description.
        return Ok(PageFooter {
            description: None,
            links: project_list_footer_links(
                &list_feed_href("opml", request),
                &list_feed_href("project_index", request),
            ),
        });
    };
    let info: FeedInfo = project_feed_info(settings, request)?;
    let hrefs: FooterFeedHrefs = FooterFeedHrefs {
        rss: feed_href(project, "rss", &info, false),
        atom: feed_href(project, "atom", &info, false),
    };
    Ok(PageFooter {
        description: store.description(project)?,
        links: project_footer_links(info.title(), &hrefs),
    })
}

/// The `<link rel="alternate">` feeds a page advertises, mirroring gitweb's
/// `print_feed_meta`: four RSS/Atom links on a project page, or the project
/// index and OPML links on the projects list.
///
/// # Errors
/// [`DomainError::Backend`] when a malformed `extra-branch-refs` entry fails
/// validation (gitweb's `die_error(500, ...)`), surfaced here as it is at the
/// other [`get_branch_refs`] consumers.
pub fn page_feeds(settings: &Settings, request: &Request) -> Result<Vec<FeedLink>, DomainError> {
    let Some(project) = request.project.as_deref() else {
        // gitweb's projects-list branch: the plain-text index and the OPML feed,
        // scoped to the active project_filter when one is set (see
        // [`list_feed_href`]) — kept in step with the footer.
        return Ok(project_list_feed_links(
            settings.site_name(),
            &list_feed_href("project_index", request),
            &list_feed_href("opml", request),
        ));
    };
    let info: FeedInfo = project_feed_info(settings, request)?;
    Ok(project_feed_links(
        project,
        info.title(),
        &feed_hrefs(project, &info),
    ))
}

/// Classifies a project page's feed (gitweb's `get_feed_info`): the branch and
/// file the feeds are scoped to and the descriptive title. The head's `<link>`
/// feeds and the footer's anchors share this — they advertise the same feed,
/// only with different wording and URLs.
///
/// # Errors
/// [`DomainError::Backend`] when a malformed `extra-branch-refs` entry fails
/// validation (gitweb's `die_error(500, ...)`), surfaced as it is at the other
/// [`get_branch_refs`] consumers.
fn project_feed_info(settings: &Settings, request: &Request) -> Result<FeedInfo, DomainError> {
    // The branch directories get_feed_info classifies refs against (heads plus
    // the validated extra-branch-refs), exactly as the heads/summary consumers
    // resolve them — a malformed entry is gitweb's die_error(500).
    let extra: &[String] = settings
        .feature(FeatureName::ExtraBranchRefs)
        .default_options();
    let branch_refs: Vec<String> = get_branch_refs(extra)?;
    let branch_refs: Vec<&str> = branch_refs.iter().map(String::as_str).collect();
    let action: &str = request.action.map_or("", Action::as_str);
    Ok(feed_info(
        action,
        request.hash_base.as_ref().map(SafeRef::as_str),
        request.hash.as_ref().map(SafeRef::as_str),
        request.file_name.as_ref().map(SafePath::as_str),
        &branch_refs,
    ))
}

/// One projects-list feed URL (`project_index` or `opml`), scoped to the active
/// project filter (gitweb's `project_filter => $project_filter`) when one is set.
///
/// gitweb is internally inconsistent here: its footer (`git_footer_html`) scopes
/// these links to the filter but its head (`print_feed_meta`) does not. We scope
/// both — the head's `<link>` feeds and the footer's anchors stay in step (the
/// user directive: header and footer behave the same way), so a `pf`-filtered
/// projects list keeps its syndication links inside the filtered view.
fn list_feed_href(action: &str, request: &Request) -> String {
    let mut params: Vec<(&str, &str)> = vec![("a", action)];
    if let Some(filter) = request.project_filter.as_deref() {
        params.push(("pf", filter));
    }
    href(&params)
}

/// The four feed URLs for a project page (gitweb's `href(action => rss/atom,
/// …)`): the plain and `--no-merges` variants of each of RSS and Atom.
fn feed_hrefs(project: &str, info: &FeedInfo) -> FeedHrefs {
    FeedHrefs {
        rss: feed_href(project, "rss", info, false),
        rss_no_merges: feed_href(project, "rss", info, true),
        atom: feed_href(project, "atom", info, false),
        atom_no_merges: feed_href(project, "atom", info, true),
    }
}

/// One feed URL: the project, the feed action (`rss`/`atom`), the file and
/// branch the feed is scoped to (when [`feed_info`] found them), and — for the
/// merge-filtered variant — gitweb's `opt => --no-merges`. The parameter order
/// follows gitweb's `@cgi_param_mapping` (`p`, `a`, `f`, `h`, `opt`).
fn feed_href(project: &str, action: &str, info: &FeedInfo, no_merges: bool) -> String {
    let mut params: Vec<(&str, &str)> = vec![("p", project), ("a", action)];
    if let Some(file) = info.file_name() {
        params.push(("f", file));
    }
    if let Some(hash) = info.hash() {
        params.push(("h", hash));
    }
    if no_merges {
        params.push(("opt", "--no-merges"));
    }
    href(&params)
}
