//! The `search` handler: gitweb's `git_search` dispatch to `git_search_message`
//! (the `commit` / `author` / `committer` facets).
//!
//! Glue only. It opens the requested project through the wired [`ProjectStore`]
//! (404 on a miss), reads the search feature gate from [`Settings`], and requires
//! a search term (gitweb's 400 "Text field is empty" when `s` is absent). It maps
//! the search-type parameter to a [`SearchKind`] — defaulting to `commit`, the
//! way gitweb does (`$searchtype ||= 'commit'`) — and runs [`assemble_search`]
//! from the requested revision (HEAD by default) and page. The resulting rows map
//! to the render layer's table, every URL built with [`href`]; the paging
//! navigation replays the search parameters across pages. The clock lives here,
//! at the boundary, so the date cells are computed against a real `now`.
//!
//! The `grep` and `pickaxe` search types are each routed — as in gitweb's
//! `git_search` — to their own path before the message facets, because each
//! returns a different result shape: `grep` (`git_search_files`,
//! [`SearchHandler::handle_grep`]) is a per-file line table, and `pickaxe`
//! (`git_search_changes`, [`SearchHandler::handle_pickaxe`]) is a commit list with
//! each commit's count-changing files. Any other search type is gitweb's 400
//! "Unknown search type" else branch.

use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::SafeRef;
use gitweb_domain::model::search_snippet::MatchSnippet;
use gitweb_domain::model::settings::{FeatureName, Settings};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::{Page, Repository, SearchKind};
use gitweb_domain::usecase::grep::{GrepFileView, GrepLine, GrepRow, GrepView, assemble_grep};
use gitweb_domain::usecase::pickaxe::{PickaxeFile, PickaxeRow, PickaxeView, assemble_pickaxe};
use gitweb_domain::usecase::search::{SearchCriteria, SearchRow, SearchView, assemble_search};
use gitweb_render::chrome::{Crumb, DocumentHead, NavItem, document};
use gitweb_render::grep::{GrepFileBlock, GrepLineView, GrepPage, GrepRowView, grep_body};
use gitweb_render::markup::Markup;
use gitweb_render::pickaxe::{PickaxeEntryView, PickaxeFileLink, PickaxePage, pickaxe_body};
use gitweb_render::search::{SearchEntryView, SearchPage, SearchPaging, SnippetView, search_body};

use crate::clock::now_epoch;
use crate::dispatch::Handler;
use crate::feed_meta::{PageChrome, document_head, page_chrome};
use crate::response::View;
use crate::url::href;

/// gitweb's `git_search_message` page size (`parse_commits($hash, 101, 100*$page)`).
const PAGE_SIZE: usize = 100;

/// gitweb's default search type (`$searchtype ||= 'commit'`).
const DEFAULT_SEARCHTYPE: &str = "commit";

/// The grep facet's search type (`$searchtype eq 'grep'`).
const GREP_SEARCHTYPE: &str = "grep";

/// The pickaxe facet's search type (`$searchtype eq 'pickaxe'`).
const PICKAXE_SEARCHTYPE: &str = "pickaxe";

/// Serves the commit/author/committer search page over a wired project store and
/// the resolved settings (which carry the `search` feature gate).
pub struct SearchHandler {
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
}

impl SearchHandler {
    /// Wires the handler with the store it opens projects from and the settings
    /// it reads the search gate and chrome from.
    #[must_use]
    pub fn new(store: Arc<dyn ProjectStore + Send + Sync>, settings: Arc<Settings>) -> Self {
        Self { store, settings }
    }

    /// Serves the grep facet (gitweb's `git_search_files`): gate search then grep,
    /// grep the base revision, and render the per-file table. The base id roots
    /// every blob link (gitweb's `hash_base => $co{'id'}`).
    fn handle_grep(
        &self,
        project: &str,
        repository: &dyn Repository,
        rev: Option<&str>,
        text: &str,
        use_regexp: bool,
        chrome: PageChrome,
    ) -> Result<View, DomainError> {
        let search_enabled: bool = self.settings.feature(FeatureName::Search).enabled();
        let grep_enabled: bool = self.settings.feature(FeatureName::Grep).enabled();
        let view: GrepView = assemble_grep(
            repository,
            search_enabled,
            grep_enabled,
            rev,
            text,
            use_regexp,
        )?;
        Ok(View::html(render_grep_page(
            &self.settings,
            project,
            rev,
            &view,
            chrome,
        )))
    }

    /// Serves the pickaxe facet (gitweb's `git_search_changes`): gate search then
    /// pickaxe, run `git log -S` from the base revision, and render the commit
    /// table with each match's count-changing files. Each row roots its own blob
    /// and tree links at its commit id (gitweb's `hash_base => $co{'id'}`), so the
    /// base id is needed only for the page navigation.
    fn handle_pickaxe(
        &self,
        project: &str,
        repository: &dyn Repository,
        rev: Option<&str>,
        text: &str,
        use_regexp: bool,
        chrome: PageChrome,
    ) -> Result<View, DomainError> {
        let search_enabled: bool = self.settings.feature(FeatureName::Search).enabled();
        let pickaxe_enabled: bool = self.settings.feature(FeatureName::Pickaxe).enabled();
        let view: PickaxeView = assemble_pickaxe(
            repository,
            search_enabled,
            pickaxe_enabled,
            rev,
            text,
            use_regexp,
            now_epoch(),
        )?;
        Ok(View::html(render_pickaxe_page(
            &self.settings,
            project,
            rev,
            &view,
            chrome,
        )))
    }
}

impl Handler for SearchHandler {
    fn handle(&self, request: &Request) -> Result<View, DomainError> {
        let project: &str = request
            .project
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
        let repository: Box<dyn Repository> = self.store.open(project)?;
        let text: &str = request
            .search_text
            .as_deref()
            .ok_or_else(|| DomainError::Invalid("Text field is empty".to_owned()))?;
        let searchtype: &str = request.search_type.as_deref().unwrap_or(DEFAULT_SEARCHTYPE);
        let use_regexp: bool = request.search_use_regexp;
        let rev: Option<&str> = request.hash.as_ref().map(SafeRef::as_str);
        let chrome: PageChrome = page_chrome(self.store.as_ref(), &self.settings, request)?;

        // gitweb's git_search dispatches `grep` and `pickaxe` to their own subs —
        // each a different result shape (a per-file line table; a commit list with
        // changed files), not a commit-message SearchKind — so each routes before
        // the message facets.
        if searchtype == GREP_SEARCHTYPE {
            return self.handle_grep(project, repository.as_ref(), rev, text, use_regexp, chrome);
        }
        if searchtype == PICKAXE_SEARCHTYPE {
            return self.handle_pickaxe(
                project,
                repository.as_ref(),
                rev,
                text,
                use_regexp,
                chrome,
            );
        }

        let kind: SearchKind = search_kind(searchtype)?;
        let page_num: usize = request.page.unwrap_or(0) as usize;
        let enabled: bool = self.settings.feature(FeatureName::Search).enabled();

        let criteria: SearchCriteria = SearchCriteria {
            kind,
            text: text.to_owned(),
            use_regexp,
        };
        let view: SearchView = assemble_search(
            repository.as_ref(),
            enabled,
            rev,
            &criteria,
            now_epoch(),
            Page::from_page(page_num, PAGE_SIZE),
        )?;

        let params: SearchParams<'_> = SearchParams {
            project,
            rev,
            searchtype,
            text,
            use_regexp,
        };
        Ok(View::html(render_page(
            &self.settings,
            &params,
            page_num,
            &view,
            chrome,
        )))
    }
}

/// Maps the search-type parameter to a [`SearchKind`], defaulting `commit`.
/// `grep` is dispatched before this (its own path), so an unhandled type here
/// (incl. the not-yet-served `pickaxe`) is gitweb's 400 "Unknown search type"
/// else branch.
fn search_kind(searchtype: &str) -> Result<SearchKind, DomainError> {
    match searchtype {
        "commit" => Ok(SearchKind::Commit),
        "author" => Ok(SearchKind::Author),
        "committer" => Ok(SearchKind::Committer),
        _ => Err(DomainError::Invalid("Unknown search type".to_owned())),
    }
}

/// The search request parameters the page and its paging links replay.
struct SearchParams<'a> {
    project: &'a str,
    rev: Option<&'a str>,
    searchtype: &'a str,
    text: &'a str,
    use_regexp: bool,
}

/// Maps the use-case view to the render view-model and wraps the body in the
/// document chrome — the boundary owns the asset URLs and every link.
fn render_page(
    settings: &Settings,
    params: &SearchParams<'_>,
    page_num: usize,
    view: &SearchView,
    chrome: PageChrome,
) -> Markup {
    let project: &str = params.project;
    let page: SearchPage = SearchPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, params.rev),
        paging: paging(params, page_num, view.has_more()),
        header_title: view.title().to_owned(),
        rows: view
            .rows()
            .iter()
            .map(|row: &SearchRow| entry(project, row))
            .collect(),
        no_match: page_num == 0 && view.rows().is_empty(),
    };
    let head: DocumentHead =
        document_head(format!("{project} / search"), chrome.feeds, chrome.scripts);
    document(&head, &chrome.foot, search_body(&page))
}

/// The breadcrumb trail: home, the project (linking to its summary), then search.
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
            label: "search".to_owned(),
            href: None,
        },
    ]
}

/// The per-project action bar (gitweb's `git_print_page_nav('','',…)` with
/// nothing current): summary, shortlog, log, tree — scoped to the search base.
fn nav(project: &str, rev: Option<&str>) -> Vec<NavItem> {
    vec![
        NavItem {
            label: "summary".to_owned(),
            href: Some(href(&[("p", project), ("a", "summary")])),
        },
        NavItem {
            label: "shortlog".to_owned(),
            href: Some(scoped_href(project, "shortlog", "h", rev)),
        },
        NavItem {
            label: "log".to_owned(),
            href: Some(scoped_href(project, "log", "h", rev)),
        },
        NavItem {
            label: "tree".to_owned(),
            href: Some(scoped_href(project, "tree", "hb", rev)),
        },
    ]
}

/// An action link scoped to the named revision through `param`, or unscoped (the
/// adapter defaults that to HEAD) when no revision was named.
fn scoped_href(project: &str, action: &str, param: &str, rev: Option<&str>) -> String {
    match rev {
        Some(revision) => href(&[("p", project), ("a", action), (param, revision)]),
        None => href(&[("p", project), ("a", action)]),
    }
}

/// The `first · prev · next` paging affordance: each link is present only when
/// that move is possible, replaying the search parameters (gitweb's
/// `href(-replay=>1, page=>…)`).
fn paging(params: &SearchParams<'_>, page_num: usize, has_more: bool) -> SearchPaging {
    SearchPaging {
        first: (page_num > 0).then(|| search_href(params, 0)),
        prev: (page_num > 0).then(|| search_href(params, page_num - 1)),
        next: has_more.then(|| search_href(params, page_num + 1)),
    }
}

/// A search-results URL for `page` (page 0 omits `pg`, gitweb's default), with
/// the project, base revision, search type (omitted when the default), term, and
/// the regexp flag replayed.
fn search_href(params: &SearchParams<'_>, page: usize) -> String {
    let page_value: String = page.to_string();
    let mut query: Vec<(&str, &str)> =
        vec![("p", params.project), ("a", "search"), ("s", params.text)];
    if params.searchtype != DEFAULT_SEARCHTYPE {
        query.push(("st", params.searchtype));
    }
    if params.use_regexp {
        query.push(("sr", "1"));
    }
    if let Some(rev) = params.rev {
        query.push(("h", rev));
    }
    if page > 0 {
        query.push(("pg", &page_value));
    }
    href(&query)
}

/// Maps a use-case search row to a render row, building the per-commit links and
/// carrying the date, author, subject, and highlighted snippets the use case
/// already resolved.
fn entry(project: &str, row: &SearchRow) -> SearchEntryView {
    let id: &str = row.id();
    SearchEntryView {
        date_displayed: row.date().displayed().to_owned(),
        date_tooltip: row.date().tooltip().to_owned(),
        author: row.author().to_owned(),
        author_short: row.author_short().to_owned(),
        title: row.title().to_owned(),
        title_short: row.title_short().to_owned(),
        commit: href(&[("p", project), ("a", "commit"), ("h", id)]),
        commitdiff: href(&[("p", project), ("a", "commitdiff"), ("h", id)]),
        tree: href(&[("p", project), ("a", "tree"), ("h", id), ("hb", id)]),
        snippets: row
            .snippets()
            .iter()
            .map(|snippet: &MatchSnippet| SnippetView {
                lead: snippet.lead().to_owned(),
                matched: snippet.matched().to_owned(),
                trail: snippet.trail().to_owned(),
            })
            .collect(),
    }
}

/// Maps the grep use-case view to the render view-model and wraps the body in the
/// document chrome. The base id is the `hash_base` of every blob link; an empty
/// file list is gitweb's "No matches found".
fn render_grep_page(
    settings: &Settings,
    project: &str,
    rev: Option<&str>,
    view: &GrepView,
    chrome: PageChrome,
) -> Markup {
    let page: GrepPage = GrepPage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, rev),
        header_title: view.title().to_owned(),
        files: view
            .files()
            .iter()
            .map(|file: &GrepFileView| grep_file(project, view.base_id(), file))
            .collect(),
        no_match: view.files().is_empty(),
        trimmed: view.trimmed(),
    };
    let head: DocumentHead =
        document_head(format!("{project} / search"), chrome.feeds, chrome.scripts);
    document(&head, &chrome.foot, grep_body(&page))
}

/// Maps one use-case file group to a render block: the blob link rooted at the
/// base id (gitweb's `href(action=>"blob", hash_base=>$co{'id'}, file_name=>…)`),
/// and each line linking to its `blob#lN` anchor.
fn grep_file(project: &str, base_id: &str, file: &GrepFileView) -> GrepFileBlock {
    let blob_href: String = href(&[
        ("p", project),
        ("a", "blob"),
        ("hb", base_id),
        ("f", file.path()),
    ]);
    GrepFileBlock {
        path: file.path().to_owned(),
        rows: file
            .rows()
            .iter()
            .map(|row: &GrepRow| grep_row(&blob_href, row))
            .collect(),
        blob_href,
    }
}

/// Maps one use-case row to a render row: the binary marker, or a numbered line
/// whose number anchors at `blob#lN`.
fn grep_row(blob_href: &str, row: &GrepRow) -> GrepRowView {
    match row {
        GrepRow::Binary => GrepRowView::Binary,
        GrepRow::Line { line_no, line } => GrepRowView::Line {
            number: *line_no,
            line_href: format!("{blob_href}#l{line_no}"),
            content: grep_line(line),
        },
    }
}

/// Maps a use-case line's text to the render shape: the highlighted lead/match/
/// trail, or the whole untabified line when the display regexp did not mark it.
fn grep_line(line: &GrepLine) -> GrepLineView {
    match line {
        GrepLine::Highlighted(snippet) => GrepLineView::Highlighted {
            lead: snippet.lead().to_owned(),
            matched: snippet.matched().to_owned(),
            trail: snippet.trail().to_owned(),
        },
        GrepLine::Plain(whole) => GrepLineView::Plain(whole.clone()),
    }
}

/// Maps the pickaxe use-case view to the render view-model and wraps the body in
/// the document chrome. An empty result is gitweb's empty pickaxe table — no note.
fn render_pickaxe_page(
    settings: &Settings,
    project: &str,
    rev: Option<&str>,
    view: &PickaxeView,
    chrome: PageChrome,
) -> Markup {
    let page: PickaxePage = PickaxePage {
        crumbs: crumbs(settings.site_name(), project),
        nav: nav(project, rev),
        header_title: view.title().to_owned(),
        rows: view
            .rows()
            .iter()
            .map(|row: &PickaxeRow| pickaxe_entry(project, row))
            .collect(),
    };
    let head: DocumentHead =
        document_head(format!("{project} / search"), chrome.feeds, chrome.scripts);
    document(&head, &chrome.foot, pickaxe_body(&page))
}

/// Maps one use-case match to a render row: the commit and tree links rooted at
/// the commit (gitweb's `hash=>$co{'tree'}, hash_base=>$co{'id'}` for the tree),
/// and each changed file linking to its blob at that commit.
fn pickaxe_entry(project: &str, row: &PickaxeRow) -> PickaxeEntryView {
    let id: &str = row.id();
    PickaxeEntryView {
        date_displayed: row.date().displayed().to_owned(),
        date_tooltip: row.date().tooltip().to_owned(),
        author: row.author().to_owned(),
        author_short: row.author_short().to_owned(),
        title: row.title().to_owned(),
        title_short: row.title_short().to_owned(),
        commit_href: href(&[("p", project), ("a", "commit"), ("h", id)]),
        tree_href: href(&[
            ("p", project),
            ("a", "tree"),
            ("h", row.tree_id()),
            ("hb", id),
        ]),
        files: row
            .files()
            .iter()
            .map(|file: &PickaxeFile| pickaxe_file(project, id, file))
            .collect(),
    }
}

/// Maps one changed file to its render link: the blob at the commit (gitweb's
/// `href(action=>"blob", hash_base=>$co{'id'}, hash=>$set{'to_id'}, file_name=>…)`).
fn pickaxe_file(project: &str, commit_id: &str, file: &PickaxeFile) -> PickaxeFileLink {
    PickaxeFileLink {
        path: file.path().to_owned(),
        blob_href: href(&[
            ("p", project),
            ("a", "blob"),
            ("hb", commit_id),
            ("h", file.blob()),
            ("f", file.path()),
        ]),
    }
}
