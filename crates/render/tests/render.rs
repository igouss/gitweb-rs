//! Gherkin-driven BDD harness for the render layer.
//!
//! Runs every `.feature` under `features/render`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use std::num::NonZeroUsize;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::age::{Age, AgeClass};
use gitweb_domain::model::forks::ForkState;
use gitweb_domain::model::message_body::LogLine;
use gitweb_domain::model::search_help::SearchHelpTopic;
use gitweb_domain::model::tag_age::TagAge;
use gitweb_domain::model::timestamp::Timestamp;
use gitweb_render::age::age_class_name;
use gitweb_render::blame::{
    BlameDataGroup, BlameIncrementalPage, BlameShellLine, BlameTableGroup, BlameTableLine,
    blame_data, blame_incremental_body, blame_table,
};
use gitweb_render::blob::{BlobContent, BlobLine, BlobPage, blob_body, blob_content};
use gitweb_render::chrome::{
    Crumb, DocumentHead, FeedHrefs, FeedLink, FooterFeedHrefs, FooterLink, FormatLink, HiddenField,
    Logo, MoreLink, NavItem, PageFooter, ScriptLink, SearchForm, SearchOption, breadcrumbs,
    document, footer, page_header, page_nav, project_feed_links, project_footer_links,
    project_list_feed_links, project_list_footer_links, search_form,
};
use gitweb_render::commit::{
    AuthorRow, ChangeNoteView, ChangedRow, CommitPage, LinkedId, ParentNav, ParentNavLink,
    ParentRow, commit_body,
};
use gitweb_render::commitdiff::{CommitdiffNav, CommitdiffPage, commitdiff_body};
use gitweb_render::diff_host::{DiffHostPage, diff_host_body};
use gitweb_render::error::{ErrorResponse, HttpStatus, error_page, error_response};
use gitweb_render::escape::{
    esc_attr, esc_html, esc_html_nbsp, esc_index_field, esc_param, esc_path, esc_path_info, esc_url,
};
use gitweb_render::feed::{FeedEntryView, FeedFileView, FeedView, atom, rss};
use gitweb_render::forks::{ForksPage, forks_body};
use gitweb_render::grep::{GrepFileBlock, GrepLineView, GrepPage, GrepRowView, grep_body};
use gitweb_render::heads::{HeadEntryView, HeadsPage, HeadsTable, heads_body, heads_table};
use gitweb_render::history::{
    HistoryEntryView, HistoryPage, HistoryTable, history_body, history_table,
};
use gitweb_render::log::{LogEntryView, LogPage, log_body, log_entries};
use gitweb_render::markup::{Markup, html, raw};
use gitweb_render::opml::{OpmlRowView, OpmlView, opml};
use gitweb_render::pickaxe::{PickaxeEntryView, PickaxeFileLink, PickaxePage, pickaxe_body};
use gitweb_render::project_index::{ProjectIndexEntry, ProjectIndexView, project_index};
use gitweb_render::project_list::{
    ProjectLinks, ProjectList, ProjectListPage, ProjectRow, SortHeader, project_list,
    project_list_body,
};
use gitweb_render::refs::RefMarkerView;
use gitweb_render::remotes::{RemoteBlockView, RemoteUrlLine, RemotesPage, remotes_body};
use gitweb_render::search::{SearchEntryView, SearchPage, SearchPaging, SnippetView, search_body};
use gitweb_render::search_help::{SearchHelpPage, search_help_body};
use gitweb_render::shortlog::{
    ShortlogEntryView, ShortlogPage, ShortlogTable, shortlog_body, shortlog_table,
};
use gitweb_render::summary::{
    HeadsSection, ShortlogSection, SummaryMetadata, SummaryPage, TagsSection, summary_body,
};
use gitweb_render::tag::{TagAuthorView, TagPage, TaggedObjectView, tag_body};
use gitweb_render::tags::{TagEntryView, TagReftype, TagsPage, TagsTable, tags_body, tags_table};
use gitweb_render::tree::{
    TreeLink, TreePage, TreeParentRow, TreeRowView, TreeTable, tree_body, tree_table,
};

#[derive(Debug, Default, World)]
struct RenderWorld {
    input: String,
    age: Option<AgeClass>,
    crumbs: Vec<Crumb>,
    nav_items: Vec<NavItem>,
    footer_links: Vec<FooterLink>,
    feed_links: Vec<FeedLink>,
    scripts: Vec<ScriptLink>,
    project_rows: Vec<ProjectRow>,
    forks_enabled: bool,
    project_index_entries: Vec<ProjectIndexEntry>,
    opml_site_name: Option<String>,
    opml_filter: Option<String>,
    opml_rows: Vec<OpmlRowView>,
    head_entries: Vec<HeadEntryView>,
    heads_more: Option<MoreLink>,
    tags_more: Option<MoreLink>,
    shortlog_entries: Vec<ShortlogEntryView>,
    shortlog_more: Option<MoreLink>,
    search_entries: Vec<SearchEntryView>,
    search_paging: SearchPaging,
    search_no_match: bool,
    grep_files: Vec<GrepFileBlock>,
    grep_no_match: bool,
    grep_trimmed: bool,
    pickaxe_rows: Vec<PickaxeEntryView>,
    history_entries: Vec<HistoryEntryView>,
    history_object_label: Option<String>,
    history_more: Option<MoreLink>,
    log_entries: Vec<LogEntryView>,
    log_more: Option<MoreLink>,
    tag_entries: Vec<TagEntryView>,
    tag_page: Option<TagPage>,
    summary_description: Option<String>,
    summary_owner: Option<String>,
    summary_has_last_change: bool,
    summary_clone_urls: Vec<String>,
    summary_readme: Option<String>,
    summary_shortlog_href: Option<String>,
    summary_tags_href: Option<String>,
    summary_heads_href: Option<String>,
    remotes_title: Option<String>,
    remote_blocks: Vec<RemoteBlockView>,
    tree_rows: Vec<TreeRowView>,
    tree_parent: Option<TreeParentRow>,
    tree_size_off: bool,
    blob_lines: Vec<BlobLine>,
    blob_image: Option<(String, String)>,
    blob_binary: Option<String>,
    domain_error: Option<DomainError>,
    status: Option<HttpStatus>,
    output: Option<String>,
    feed_view: Option<FeedView>,
    diff_url: Option<String>,
    diff_viewer_module_src: Option<String>,
    diff_title: Option<String>,
    diff_path: Option<String>,
    diff_formats: Vec<FormatLink>,
    blame_group_meta: Option<(String, String, String)>,
    blame_group_lines: Vec<BlameTableLine>,
    blame_shell_lines: Vec<BlameShellLine>,
    blame_data_group: Option<BlameDataGroup>,
    blame_data_url: Option<String>,
    blame_project_url: Option<String>,
}

// ---- Given: the text under escaping -----------------------------------------

#[given(regex = r#"^the text "(.*)"$"#)]
fn given_text(world: &mut RenderWorld, text: String) {
    world.input = text;
}

#[given("a double-quote character")]
fn given_double_quote(world: &mut RenderWorld) {
    world.input = "\"".to_owned();
}

#[given("a tab character")]
fn given_tab(world: &mut RenderWorld) {
    world.input = "\t".to_owned();
}

#[given("a newline character")]
fn given_newline(world: &mut RenderWorld) {
    world.input = "\n".to_owned();
}

#[given("a bell character")]
fn given_bell(world: &mut RenderWorld) {
    world.input = "\u{07}".to_owned();
}

#[given("a NUL character")]
fn given_nul(world: &mut RenderWorld) {
    world.input = "\0".to_owned();
}

#[given("the control byte 0x01")]
fn given_ctrl_01(world: &mut RenderWorld) {
    world.input = "\u{01}".to_owned();
}

// ---- When: apply one escaping function --------------------------------------

#[when("I escape it for HTML")]
fn when_esc_html(world: &mut RenderWorld) {
    world.output = Some(esc_html(&world.input));
}

#[when("I escape it for HTML keeping whitespace")]
fn when_esc_html_nbsp(world: &mut RenderWorld) {
    world.output = Some(esc_html_nbsp(&world.input));
}

#[when("I escape it as a path")]
fn when_esc_path(world: &mut RenderWorld) {
    world.output = Some(esc_path(&world.input));
}

#[when("I escape it for an HTML attribute")]
fn when_esc_attr(world: &mut RenderWorld) {
    world.output = Some(esc_attr(&world.input));
}

#[when("I escape it for a URL")]
fn when_esc_url(world: &mut RenderWorld) {
    world.output = Some(esc_url(&world.input));
}

#[when("I escape it as a URL parameter")]
fn when_esc_param(world: &mut RenderWorld) {
    world.output = Some(esc_param(&world.input));
}

#[when("I escape it as path info")]
fn when_esc_path_info(world: &mut RenderWorld) {
    world.output = Some(esc_path_info(&world.input));
}

#[when("I quote it as a project index field")]
fn when_esc_index_field(world: &mut RenderWorld) {
    world.output = Some(esc_index_field(&world.input));
}

// ---- Given: an age classification -------------------------------------------

#[given("an age classification of unknown")]
fn given_age_unknown(world: &mut RenderWorld) {
    world.age = Some(AgeClass::Unknown);
}

#[given("an age classification of fresh")]
fn given_age_fresh(world: &mut RenderWorld) {
    world.age = Some(AgeClass::Fresh);
}

#[given("an age classification of recent")]
fn given_age_recent(world: &mut RenderWorld) {
    world.age = Some(AgeClass::Recent);
}

#[given("an age classification of old")]
fn given_age_old(world: &mut RenderWorld) {
    world.age = Some(AgeClass::Old);
}

#[when("I ask for its CSS class")]
fn when_age_class_name(world: &mut RenderWorld) {
    let class: AgeClass = world.age.expect("scenario must set an age classification");
    world.output = Some(age_class_name(class).to_owned());
}

// ---- When: the raw-HTML safe sink -------------------------------------------

#[when("I render it as untrusted template content")]
fn when_render_untrusted(world: &mut RenderWorld) {
    let text: &str = world.input.as_str();
    let rendered: Markup = html! { (text) };
    world.output = Some(rendered.into_string());
}

#[when("I render it through the raw-HTML safe sink")]
fn when_render_raw(world: &mut RenderWorld) {
    let rendered: Markup = raw(world.input.clone());
    world.output = Some(rendered.into_string());
}

#[when("I escape it for HTML and render it through the raw-HTML safe sink")]
fn when_esc_then_raw(world: &mut RenderWorld) {
    let escaped: String = esc_html(&world.input);
    let rendered: Markup = raw(escaped);
    world.output = Some(rendered.into_string());
}

#[when("I escape it for HTML and render it as untrusted template content")]
fn when_esc_then_untrusted(world: &mut RenderWorld) {
    let escaped: String = esc_html(&world.input);
    let rendered: Markup = html! { (escaped) };
    world.output = Some(rendered.into_string());
}

// ---- Chrome: document skeleton ----------------------------------------------

#[given(regex = r#"^a client script at "([^"]*)"$"#)]
fn given_client_script(world: &mut RenderWorld, src: String) {
    world.scripts.push(ScriptLink { src });
}

#[when(regex = r#"^I render a document titled "(.*)" with stylesheet "(.*)" and body "(.*)"$"#)]
fn when_render_document(world: &mut RenderWorld, title: String, stylesheet: String, body: String) {
    let head: DocumentHead = DocumentHead {
        title,
        stylesheet_href: stylesheet,
        favicon_href: None,
        feeds: Vec::new(),
        scripts: std::mem::take(&mut world.scripts),
    };
    let foot: PageFooter = PageFooter {
        description: None,
        links: std::mem::take(&mut world.footer_links),
    };
    world.output = Some(document(&head, &foot, raw(body)).into_string());
}

#[when(regex = r#"^I render a document with a favicon "(.*)"$"#)]
fn when_render_document_favicon(world: &mut RenderWorld, favicon: String) {
    let head: DocumentHead = DocumentHead {
        title: "x".to_owned(),
        stylesheet_href: "/s".to_owned(),
        favicon_href: Some(favicon),
        feeds: Vec::new(),
        scripts: Vec::new(),
    };
    let foot: PageFooter = PageFooter {
        description: None,
        links: Vec::new(),
    };
    world.output = Some(document(&head, &foot, raw("")).into_string());
}

#[when(regex = r#"^I render a document with an RSS feed "(.*)" titled "(.*)"$"#)]
fn when_render_document_feed(world: &mut RenderWorld, href: String, title: String) {
    let feed: FeedLink = FeedLink {
        title,
        href,
        mime: "application/rss+xml".to_owned(),
    };
    let head: DocumentHead = DocumentHead {
        title: "x".to_owned(),
        stylesheet_href: "/s".to_owned(),
        favicon_href: None,
        feeds: vec![feed],
        scripts: Vec::new(),
    };
    let foot: PageFooter = PageFooter {
        description: None,
        links: Vec::new(),
    };
    world.output = Some(document(&head, &foot, raw("")).into_string());
}

// ---- Chrome: feed auto-discovery link assembly ------------------------------

#[when(regex = r#"^I assemble the project feed links for "(.*)" titled "(.*)"$"#)]
fn when_assemble_project_feed_links(world: &mut RenderWorld, project: String, title: String) {
    let hrefs: FeedHrefs = FeedHrefs {
        rss: "/rss".to_owned(),
        rss_no_merges: "/rss-nm".to_owned(),
        atom: "/atom".to_owned(),
        atom_no_merges: "/atom-nm".to_owned(),
    };
    world.feed_links = project_feed_links(&project, &title, &hrefs);
}

#[when(regex = r#"^I assemble the project-list feed links for site "(.*)"$"#)]
fn when_assemble_project_list_feed_links(world: &mut RenderWorld, site: String) {
    world.feed_links = project_list_feed_links(&site, "/index", "/opml");
}

#[then(regex = r#"^the feed links number (\d+)$"#)]
fn then_feed_links_number(world: &mut RenderWorld, expected: usize) {
    assert_eq!(world.feed_links.len(), expected);
}

#[then(regex = r#"^feed link (\d+) is titled "(.*)" linking "(.*)" of type "(.*)"$"#)]
fn then_feed_link_is(
    world: &mut RenderWorld,
    index: usize,
    title: String,
    href: String,
    mime: String,
) {
    let link: &FeedLink = &world.feed_links[index];
    assert_eq!(link.title, title);
    assert_eq!(link.href, href);
    assert_eq!(link.mime, mime);
}

// ---- Chrome: breadcrumbs and header -----------------------------------------

#[given(regex = r#"^a breadcrumb link "(.*)" to "(.*)"$"#)]
fn given_crumb_link(world: &mut RenderWorld, label: String, href: String) {
    world.crumbs.push(Crumb {
        label,
        href: Some(href),
    });
}

#[given(regex = r#"^a current breadcrumb "(.*)"$"#)]
fn given_crumb_current(world: &mut RenderWorld, label: String) {
    world.crumbs.push(Crumb { label, href: None });
}

#[when("I render the breadcrumbs")]
fn when_render_breadcrumbs(world: &mut RenderWorld) {
    world.output = Some(breadcrumbs(&world.crumbs).into_string());
}

#[when("I render the page header without a logo")]
fn when_page_header_no_logo(world: &mut RenderWorld) {
    world.output = Some(page_header(None, &world.crumbs).into_string());
}

#[when(
    regex = r#"^I render the page header with a logo linking "(.*)" image "(.*)" labelled "(.*)"$"#
)]
fn when_page_header_logo(world: &mut RenderWorld, href: String, image_src: String, label: String) {
    let logo: Logo = Logo {
        href,
        image_src,
        label,
    };
    world.output = Some(page_header(Some(&logo), &world.crumbs).into_string());
}

// ---- Chrome: search form ----------------------------------------------------

#[when(
    regex = r#"^I render the standard search form with query "(.*)" type "(.*)" regexp (on|off)$"#
)]
fn when_search_form(world: &mut RenderWorld, query: String, kind: String, toggle: String) {
    let use_regexp: bool = toggle == "on";
    let make: fn(&str, &str) -> SearchOption = |value: &str, kind: &str| SearchOption {
        value: value.to_owned(),
        label: value.to_owned(),
        selected: value == kind,
    };
    let options: Vec<SearchOption> = vec![
        make("commit", &kind),
        make("grep", &kind),
        make("author", &kind),
        make("committer", &kind),
        make("pickaxe", &kind),
    ];
    let hidden: Vec<HiddenField> = vec![
        HiddenField {
            name: "p".to_owned(),
            value: "myrepo".to_owned(),
        },
        HiddenField {
            name: "a".to_owned(),
            value: "search".to_owned(),
        },
    ];
    let form: SearchForm = SearchForm {
        action: "/myrepo/search".to_owned(),
        hidden,
        options,
        query,
        use_regexp,
        help_href: "/search_help".to_owned(),
    };
    world.output = Some(search_form(&form).into_string());
}

// ---- Chrome: page navigation ------------------------------------------------

#[given(regex = r#"^a navigation link "(.*)" to "(.*)"$"#)]
fn given_nav_link(world: &mut RenderWorld, label: String, href: String) {
    world.nav_items.push(NavItem {
        label,
        href: Some(href),
    });
}

#[given(regex = r#"^a navigation item "(.*)" current$"#)]
fn given_nav_current(world: &mut RenderWorld, label: String) {
    world.nav_items.push(NavItem { label, href: None });
}

#[when("I render the page navigation")]
fn when_page_nav(world: &mut RenderWorld) {
    world.output = Some(page_nav(&world.nav_items, None).into_string());
}

#[when(regex = r#"^I render the page navigation with extra "(.*)"$"#)]
fn when_page_nav_extra(world: &mut RenderWorld, extra: String) {
    world.output = Some(page_nav(&world.nav_items, Some(raw(extra))).into_string());
}

// ---- Chrome: footer ---------------------------------------------------------

#[given(regex = r#"^a footer link "(.*)" to "(.*)" titled "(.*)"$"#)]
fn given_footer_link(world: &mut RenderWorld, label: String, href: String, title: String) {
    world.footer_links.push(FooterLink { label, href, title });
}

#[when(regex = r#"^I render the footer with description "(.*)"$"#)]
fn when_footer_with_desc(world: &mut RenderWorld, description: String) {
    let page_footer: PageFooter = PageFooter {
        description: Some(description),
        links: std::mem::take(&mut world.footer_links),
    };
    world.output = Some(footer(&page_footer).into_string());
}

#[when("I render the footer without a description")]
fn when_footer_no_desc(world: &mut RenderWorld) {
    let page_footer: PageFooter = PageFooter {
        description: None,
        links: std::mem::take(&mut world.footer_links),
    };
    world.output = Some(footer(&page_footer).into_string());
}

#[when(regex = r#"^I assemble the project footer titled "(.*)" with rss "(.*)" and atom "(.*)"$"#)]
fn when_assemble_project_footer(world: &mut RenderWorld, title: String, rss: String, atom: String) {
    let hrefs: FooterFeedHrefs = FooterFeedHrefs { rss, atom };
    let links: Vec<FooterLink> = project_footer_links(&title, &hrefs);
    let page_footer: PageFooter = PageFooter {
        description: None,
        links,
    };
    world.output = Some(footer(&page_footer).into_string());
}

#[when(regex = r#"^I assemble the projects-list footer with opml "(.*)" and index "(.*)"$"#)]
fn when_assemble_list_footer(world: &mut RenderWorld, opml: String, index: String) {
    let links: Vec<FooterLink> = project_list_footer_links(&opml, &index);
    let page_footer: PageFooter = PageFooter {
        description: None,
        links,
    };
    world.output = Some(footer(&page_footer).into_string());
}

// ---- Projects-list table ----------------------------------------------------

/// Synthesizes a row's quick links from its summary href, so the rendered table
/// carries four distinct, labelled action links.
fn project_links(href: &str) -> ProjectLinks {
    ProjectLinks {
        summary: href.to_owned(),
        shortlog: format!("{href}/shortlog"),
        log: format!("{href}/log"),
        tree: format!("{href}/tree"),
    }
}

/// A column header: plain text (no link) when `key` is the active sort `order`,
/// otherwise a link to re-sort by it.
fn sort_header(label: &str, key: &str, order: &str) -> SortHeader {
    SortHeader {
        label: label.to_owned(),
        href: if key == order {
            None
        } else {
            Some(format!("/?o={key}"))
        },
    }
}

#[given(regex = r#"^a listed project "(.*)" at "([^"]*)"$"#)]
fn given_listed_project(world: &mut RenderWorld, name: String, href: String) {
    world.project_rows.push(ProjectRow {
        links: project_links(&href),
        forks_href: format!("{href}/forks"),
        name,
        href,
        description: None,
        owner: None,
        age: None,
        fork_state: ForkState::NotForkable,
    });
}

#[given(regex = r#"^a listed project "([^"]*)" at "([^"]*)" with (\d+) forks?$"#)]
fn given_listed_project_with_forks(
    world: &mut RenderWorld,
    name: String,
    href: String,
    fork_count: usize,
) {
    world.forks_enabled = true;
    let fork_state: ForkState = match NonZeroUsize::new(fork_count) {
        Some(count) => ForkState::Forked(count),
        None => ForkState::NotForkable,
    };
    world.project_rows.push(ProjectRow {
        links: project_links(&href),
        forks_href: format!("{href}/forks"),
        name,
        href,
        description: None,
        owner: None,
        age: None,
        fork_state,
    });
}

#[given(regex = r#"^a listed project "([^"]*)" at "([^"]*)" with an empty fork container$"#)]
fn given_listed_project_empty_container(world: &mut RenderWorld, name: String, href: String) {
    world.forks_enabled = true;
    world.project_rows.push(ProjectRow {
        links: project_links(&href),
        forks_href: format!("{href}/forks"),
        name,
        href,
        description: None,
        owner: None,
        age: None,
        fork_state: ForkState::EmptyContainer,
    });
}

#[given(
    regex = r#"^a listed project "([^"]*)" at "([^"]*)" described "(.*)" owned by "([^"]*)" aged (\d+)$"#
)]
fn given_listed_project_full(
    world: &mut RenderWorld,
    name: String,
    href: String,
    description: String,
    owner: String,
    age_seconds: i64,
) {
    world.project_rows.push(ProjectRow {
        links: project_links(&href),
        forks_href: format!("{href}/forks"),
        name,
        href,
        description: Some(description),
        owner: Some(owner),
        age: Some(Age::from_seconds(age_seconds)),
        fork_state: ForkState::NotForkable,
    });
}

#[given(regex = r#"^a listed project "([^"]*)" at "([^"]*)" with no commits$"#)]
fn given_listed_project_no_commits(world: &mut RenderWorld, name: String, href: String) {
    world.project_rows.push(ProjectRow {
        links: project_links(&href),
        forks_href: format!("{href}/forks"),
        name,
        href,
        description: None,
        owner: None,
        age: None,
        fork_state: ForkState::NotForkable,
    });
}

/// Builds the table view-model from the rows collected so far, carrying the
/// fork-column flag a "with N forks" project turned on.
fn take_project_list(world: &mut RenderWorld, order: &str) -> ProjectList {
    ProjectList {
        project_header: sort_header("Project", "project", order),
        description_header: sort_header("Description", "descr", order),
        owner_header: sort_header("Owner", "owner", order),
        age_header: sort_header("Last Change", "age", order),
        forks_enabled: world.forks_enabled,
        rows: std::mem::take(&mut world.project_rows),
    }
}

#[when(regex = r#"^I render the project list sorted by "([^"]*)"$"#)]
fn when_render_project_list(world: &mut RenderWorld, order: String) {
    let list: ProjectList = take_project_list(world, &order);
    world.output = Some(project_list(&list).into_string());
}

#[when(regex = r#"^I render the project list page sorted by "([^"]*)"$"#)]
fn when_render_project_list_page(world: &mut RenderWorld, order: String) {
    let list: ProjectList = take_project_list(world, &order);
    let page: ProjectListPage = ProjectListPage {
        site_name: "Git Repositories".to_owned(),
        list,
    };
    world.output = Some(project_list_body(&page).into_string());
}

#[when(regex = r#"^I render the forks page for "([^"]*)" sorted by "([^"]*)"$"#)]
fn when_render_forks_page(world: &mut RenderWorld, project: String, order: String) {
    world.forks_enabled = true;
    let list: ProjectList = take_project_list(world, &order);
    let page: ForksPage = ForksPage {
        crumbs: std::mem::take(&mut world.crumbs),
        nav: std::mem::take(&mut world.nav_items),
        title: format!("{project} forks"),
        list,
    };
    world.output = Some(forks_body(&page).into_string());
}

// ---- Project index ----------------------------------------------------------

#[given(regex = r#"^a project index entry "([^"]*)" owned by "([^"]*)"$"#)]
fn given_index_entry(world: &mut RenderWorld, path: String, owner: String) {
    world
        .project_index_entries
        .push(ProjectIndexEntry { path, owner });
}

#[given("an empty project index")]
fn given_empty_index(world: &mut RenderWorld) {
    world.project_index_entries.clear();
}

#[when("I render the project index")]
fn when_render_index(world: &mut RenderWorld) {
    let view: ProjectIndexView = ProjectIndexView {
        entries: std::mem::take(&mut world.project_index_entries),
    };
    world.output = Some(project_index(&view));
}

#[then("the rendered body is the newline-ended lines:")]
fn then_rendered_body_lines(world: &mut RenderWorld, step: &Step) {
    let expected: String = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n')
        .lines()
        .map(|line: &str| format!("{line}\n"))
        .collect();
    assert_eq!(world.output.as_deref(), Some(expected.as_str()));
}

#[then("the index body is empty")]
fn then_index_body_empty(world: &mut RenderWorld) {
    assert_eq!(world.output.as_deref(), Some(""));
}

// ---- OPML outline -----------------------------------------------------------

#[given(regex = r#"^an opml outline for site "([^"]*)"$"#)]
fn given_opml_site(world: &mut RenderWorld, site: String) {
    world.opml_site_name = Some(site);
}

#[given(regex = r#"^the opml outline is filtered to subdirectory "([^"]*)"$"#)]
fn given_opml_filter(world: &mut RenderWorld, subdir: String) {
    world.opml_filter = Some(subdir);
}

#[given(regex = r#"^an opml project "([^"]*)" with feed "([^"]*)" and summary "([^"]*)"$"#)]
fn given_opml_project(world: &mut RenderWorld, path: String, feed: String, summary: String) {
    world.opml_rows.push(OpmlRowView {
        text: path,
        xml_url: feed,
        html_url: summary,
    });
}

#[when("I render the opml outline")]
fn when_render_opml(world: &mut RenderWorld) {
    let view: OpmlView = OpmlView {
        site_name: world.opml_site_name.take().unwrap_or_default(),
        filter: world.opml_filter.take(),
        rows: std::mem::take(&mut world.opml_rows),
    };
    world.output = Some(opml(&view));
}

// ---- Heads table ------------------------------------------------------------

/// Builds a head row from a base href: the name links to `base` (its shortlog),
/// and the per-branch log / tree links hang off it, so one base yields three
/// distinct, labelled links.
fn head_entry(name: &str, base: &str, age: Option<Age>, current: bool) -> HeadEntryView {
    HeadEntryView {
        name: name.to_owned(),
        shortlog: base.to_owned(),
        log: format!("{base}/log"),
        tree: format!("{base}/tree"),
        age,
        current,
    }
}

#[given(regex = r#"^a head "([^"]*)" at "([^"]*)" aged (\d+)$"#)]
fn given_head(world: &mut RenderWorld, name: String, base: String, age_seconds: i64) {
    world.head_entries.push(head_entry(
        &name,
        &base,
        Some(Age::from_seconds(age_seconds)),
        false,
    ));
}

#[given(regex = r#"^a current head "([^"]*)" at "([^"]*)" aged (\d+)$"#)]
fn given_current_head(world: &mut RenderWorld, name: String, base: String, age_seconds: i64) {
    world.head_entries.push(head_entry(
        &name,
        &base,
        Some(Age::from_seconds(age_seconds)),
        true,
    ));
}

#[given(regex = r#"^a head "([^"]*)" at "([^"]*)" with an unknown age$"#)]
fn given_head_unknown_age(world: &mut RenderWorld, name: String, base: String) {
    world
        .head_entries
        .push(head_entry(&name, &base, None, false));
}

#[when("I render the heads table")]
fn when_render_heads_table(world: &mut RenderWorld) {
    let table: HeadsTable = HeadsTable {
        rows: std::mem::take(&mut world.head_entries),
        more: world.heads_more.take(),
    };
    world.output = Some(heads_table(&table).into_string());
}

#[when("I render the heads page")]
fn when_render_heads_page(world: &mut RenderWorld) {
    let table: HeadsTable = HeadsTable {
        rows: std::mem::take(&mut world.head_entries),
        more: world.heads_more.take(),
    };
    let page: HeadsPage = HeadsPage {
        crumbs: std::mem::take(&mut world.crumbs),
        ref_views: std::mem::take(&mut world.nav_items),
        table,
    };
    world.output = Some(heads_body(&page).into_string());
}

/// Builds a shortlog row from its display fields and a base href: the commit
/// link is the base, and the commitdiff / tree links hang off it.
fn shortlog_entry(
    author: &str,
    author_short: &str,
    title: &str,
    title_short: &str,
    displayed: &str,
    tooltip: &str,
    base: &str,
) -> ShortlogEntryView {
    ShortlogEntryView {
        date_displayed: displayed.to_owned(),
        date_tooltip: tooltip.to_owned(),
        author: author.to_owned(),
        author_short: author_short.to_owned(),
        title: title.to_owned(),
        title_short: title_short.to_owned(),
        commit: base.to_owned(),
        commitdiff: format!("{base}/diff"),
        tree: format!("{base}/tree"),
        refs: Vec::new(),
    }
}

#[given(
    regex = r#"^a shortlog commit by "([^"]*)" titled "([^"]*)" dated "([^"]*)" at "([^"]*)"$"#
)]
fn given_shortlog_commit(
    world: &mut RenderWorld,
    author: String,
    title: String,
    displayed: String,
    base: String,
) {
    world.shortlog_entries.push(shortlog_entry(
        &author, &author, &title, &title, &displayed, &displayed, &base,
    ));
}

#[given(
    regex = r#"^a shortlog commit by "([^"]*)" titled "([^"]*)" dated "([^"]*)" tooltip "([^"]*)" at "([^"]*)"$"#
)]
fn given_shortlog_commit_tooltip(
    world: &mut RenderWorld,
    author: String,
    title: String,
    displayed: String,
    tooltip: String,
    base: String,
) {
    world.shortlog_entries.push(shortlog_entry(
        &author, &author, &title, &title, &displayed, &tooltip, &base,
    ));
}

#[given(regex = r#"^a shortlog commit authored "([^"]*)" shortened to "(.*)" at "([^"]*)"$"#)]
fn given_shortlog_chopped_author(
    world: &mut RenderWorld,
    author: String,
    author_short: String,
    base: String,
) {
    world.shortlog_entries.push(shortlog_entry(
        &author,
        &author_short,
        "x",
        "x",
        "now",
        "now",
        &base,
    ));
}

#[given(regex = r#"^a shortlog commit subject "([^"]*)" shortened to "(.*)" at "([^"]*)"$"#)]
fn given_shortlog_chopped_subject(
    world: &mut RenderWorld,
    title: String,
    title_short: String,
    base: String,
) {
    world.shortlog_entries.push(shortlog_entry(
        "Alice",
        "Alice",
        &title,
        &title_short,
        "now",
        "now",
        &base,
    ));
}

#[given(regex = r#"^the shortlog offers more at "([^"]*)" labelled "([^"]*)"$"#)]
fn given_shortlog_more(world: &mut RenderWorld, href: String, label: String) {
    world.shortlog_more = Some(MoreLink { href, label });
}

#[given(
    regex = r#"^a shortlog commit badged a "([^"]*)" ref "([^"]*)" titled "([^"]*)" linking "([^"]*)"$"#
)]
fn given_shortlog_badged(
    world: &mut RenderWorld,
    class: String,
    name: String,
    title: String,
    href: String,
) {
    let mut entry: ShortlogEntryView =
        shortlog_entry("Alice", "Alice", "x", "x", "now", "now", "/c");
    entry.refs.push(RefMarkerView {
        name,
        class,
        title,
        href,
    });
    world.shortlog_entries.push(entry);
}

#[given(regex = r#"^the heads offer more at "([^"]*)" labelled "([^"]*)"$"#)]
fn given_heads_more(world: &mut RenderWorld, href: String, label: String) {
    world.heads_more = Some(MoreLink { href, label });
}

#[given(regex = r#"^the tags offer more at "([^"]*)" labelled "([^"]*)"$"#)]
fn given_tags_more(world: &mut RenderWorld, href: String, label: String) {
    world.tags_more = Some(MoreLink { href, label });
}

#[when("I render the shortlog table")]
fn when_render_shortlog_table(world: &mut RenderWorld) {
    let table: ShortlogTable = ShortlogTable {
        rows: std::mem::take(&mut world.shortlog_entries),
        more: world.shortlog_more.take(),
    };
    world.output = Some(shortlog_table(&table).into_string());
}

#[when("I render the shortlog page")]
fn when_render_shortlog_page(world: &mut RenderWorld) {
    let table: ShortlogTable = ShortlogTable {
        rows: std::mem::take(&mut world.shortlog_entries),
        more: world.shortlog_more.take(),
    };
    let page: ShortlogPage = ShortlogPage {
        crumbs: std::mem::take(&mut world.crumbs),
        nav: std::mem::take(&mut world.nav_items),
        table,
    };
    world.output = Some(shortlog_body(&page).into_string());
}

// ---- search results ----------------------------------------------------------

/// Builds a search result row from its display fields and a commit href, with
/// the given highlighted snippets. The author and subject are left unchopped (the
/// chop is the use case's call), and the per-commit links hang off the href.
fn search_entry(
    author: &str,
    title: &str,
    displayed: &str,
    href: &str,
    snippets: Vec<SnippetView>,
) -> SearchEntryView {
    SearchEntryView {
        date_displayed: displayed.to_owned(),
        date_tooltip: displayed.to_owned(),
        author: author.to_owned(),
        author_short: author.to_owned(),
        title: title.to_owned(),
        title_short: title.to_owned(),
        commit: href.to_owned(),
        commitdiff: format!("{href}/diff"),
        tree: format!("{href}/tree"),
        snippets,
    }
}

#[given(regex = r#"^a search result by "([^"]*)" titled "([^"]*)" dated "([^"]*)" at "([^"]*)"$"#)]
fn given_search_result(
    world: &mut RenderWorld,
    author: String,
    title: String,
    dated: String,
    href: String,
) {
    world
        .search_entries
        .push(search_entry(&author, &title, &dated, &href, Vec::new()));
}

#[given(regex = r#"^a search result highlighting lead "([^"]*)" match "([^"]*)" trail "([^"]*)"$"#)]
fn given_search_highlight(world: &mut RenderWorld, lead: String, matched: String, trail: String) {
    let snippet: SnippetView = SnippetView {
        lead,
        matched,
        trail,
    };
    world
        .search_entries
        .push(search_entry("Ada", "msg", "now", "/c", vec![snippet]));
}

#[given("the search has no matches")]
fn given_search_no_matches(world: &mut RenderWorld) {
    world.search_no_match = true;
}

#[given(regex = r#"^the search offers a next page at "([^"]*)"$"#)]
fn given_search_next(world: &mut RenderWorld, href: String) {
    world.search_paging.next = Some(href);
}

#[given(regex = r#"^the search offers first and previous pages at "([^"]*)" and "([^"]*)"$"#)]
fn given_search_first_prev(world: &mut RenderWorld, first: String, prev: String) {
    world.search_paging.first = Some(first);
    world.search_paging.prev = Some(prev);
}

#[when("I render the search page")]
fn when_render_search(world: &mut RenderWorld) {
    let page: SearchPage = SearchPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        paging: std::mem::take(&mut world.search_paging),
        header_title: "base commit".to_owned(),
        rows: std::mem::take(&mut world.search_entries),
        no_match: world.search_no_match,
    };
    world.output = Some(search_body(&page).into_string());
}

// ---- grep results ------------------------------------------------------------

/// Pushes a row onto the most recently declared grep file block.
fn push_grep_row(world: &mut RenderWorld, row: GrepRowView) {
    world
        .grep_files
        .last_mut()
        .expect("declare a grep file first")
        .rows
        .push(row);
}

#[given(regex = r#"^a grep file "([^"]*)" at "([^"]*)"$"#)]
fn given_grep_file(world: &mut RenderWorld, path: String, blob_href: String) {
    world.grep_files.push(GrepFileBlock {
        path,
        blob_href,
        rows: Vec::new(),
    });
}

#[given(
    regex = r#"^a grep line (\d+) at "([^"]*)" highlighting lead "(.*)" match "(.*)" trail "(.*)"$"#
)]
fn given_grep_highlighted_line(
    world: &mut RenderWorld,
    number: usize,
    line_href: String,
    lead: String,
    matched: String,
    trail: String,
) {
    push_grep_row(
        world,
        GrepRowView::Line {
            number,
            line_href,
            content: GrepLineView::Highlighted {
                lead,
                matched,
                trail,
            },
        },
    );
}

#[given(regex = r#"^a grep line (\d+) at "([^"]*)" plain "(.*)"$"#)]
fn given_grep_plain_line(world: &mut RenderWorld, number: usize, line_href: String, text: String) {
    push_grep_row(
        world,
        GrepRowView::Line {
            number,
            line_href,
            content: GrepLineView::Plain(text),
        },
    );
}

#[given("a grep binary row")]
fn given_grep_binary(world: &mut RenderWorld) {
    push_grep_row(world, GrepRowView::Binary);
}

#[given("the grep has no matches")]
fn given_grep_no_matches(world: &mut RenderWorld) {
    world.grep_no_match = true;
}

#[given("the grep listing is trimmed")]
fn given_grep_trimmed(world: &mut RenderWorld) {
    world.grep_trimmed = true;
}

#[when("I render the grep page")]
fn when_render_grep(world: &mut RenderWorld) {
    let page: GrepPage = GrepPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        header_title: "grep base".to_owned(),
        files: std::mem::take(&mut world.grep_files),
        no_match: world.grep_no_match,
        trimmed: world.grep_trimmed,
    };
    world.output = Some(grep_body(&page).into_string());
}

// ---- pickaxe results ---------------------------------------------------------

#[given(regex = r#"^a pickaxe commit "([^"]*)" by "([^"]*)" at commit "([^"]*)" tree "([^"]*)"$"#)]
fn given_pickaxe_commit(
    world: &mut RenderWorld,
    title: String,
    author: String,
    commit_href: String,
    tree_href: String,
) {
    world.pickaxe_rows.push(PickaxeEntryView {
        date_displayed: "2026-01-01".to_owned(),
        date_tooltip: "1 day ago".to_owned(),
        author: author.clone(),
        author_short: author,
        title: title.clone(),
        title_short: title,
        commit_href,
        tree_href,
        files: Vec::new(),
    });
}

#[given(
    regex = r#"^a pickaxe commit "([^"]*)" by "([^"]*)" short "([^"]*)" at commit "([^"]*)" tree "([^"]*)"$"#
)]
fn given_pickaxe_commit_short_author(
    world: &mut RenderWorld,
    title: String,
    author: String,
    author_short: String,
    commit_href: String,
    tree_href: String,
) {
    world.pickaxe_rows.push(PickaxeEntryView {
        date_displayed: "2026-01-01".to_owned(),
        date_tooltip: "1 day ago".to_owned(),
        author,
        author_short,
        title: title.clone(),
        title_short: title,
        commit_href,
        tree_href,
        files: Vec::new(),
    });
}

#[given(regex = r#"^the pickaxe commit changed file "([^"]*)" at "([^"]*)"$"#)]
fn given_pickaxe_changed_file(world: &mut RenderWorld, path: String, blob_href: String) {
    world
        .pickaxe_rows
        .last_mut()
        .expect("declare a pickaxe commit first")
        .files
        .push(PickaxeFileLink { path, blob_href });
}

#[given(regex = r#"^the pickaxe commit is dated "([^"]*)" tooltip "([^"]*)"$"#)]
fn given_pickaxe_commit_dated(world: &mut RenderWorld, displayed: String, tooltip: String) {
    let row: &mut PickaxeEntryView = world
        .pickaxe_rows
        .last_mut()
        .expect("declare a pickaxe commit first");
    row.date_displayed = displayed;
    row.date_tooltip = tooltip;
}

#[when("I render the pickaxe page")]
fn when_render_pickaxe(world: &mut RenderWorld) {
    let page: PickaxePage = PickaxePage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        header_title: "pickaxe base".to_owned(),
        rows: std::mem::take(&mut world.pickaxe_rows),
    };
    world.output = Some(pickaxe_body(&page).into_string());
}

// ---- history table ----------------------------------------------------------

/// Builds a history *blob* row from its display fields and a base href: the
/// ftype link and raw hang off the base. The tree shape (no raw) is built by its
/// own step; the diff-to-current link is added by a later step.
fn history_blob_entry(
    author: &str,
    author_short: &str,
    title: &str,
    title_short: &str,
    displayed: &str,
    tooltip: &str,
    base: &str,
) -> HistoryEntryView {
    HistoryEntryView {
        date_displayed: displayed.to_owned(),
        date_tooltip: tooltip.to_owned(),
        author: author.to_owned(),
        author_short: author_short.to_owned(),
        title: title.to_owned(),
        title_short: title_short.to_owned(),
        commit: base.to_owned(),
        object: format!("{base}/blob"),
        commitdiff: format!("{base}/diff"),
        raw: Some(format!("{base}/raw")),
        diff_to_current: None,
        refs: Vec::new(),
    }
}

#[given(
    regex = r#"^a history blob row by "([^"]*)" titled "([^"]*)" dated "([^"]*)" at "([^"]*)"$"#
)]
fn given_history_blob_row(
    world: &mut RenderWorld,
    author: String,
    title: String,
    displayed: String,
    base: String,
) {
    world.history_object_label = Some("blob".to_owned());
    world.history_entries.push(history_blob_entry(
        &author, &author, &title, &title, &displayed, &displayed, &base,
    ));
}

#[given(
    regex = r#"^a history blob row by "([^"]*)" titled "([^"]*)" dated "([^"]*)" tooltip "([^"]*)" at "([^"]*)"$"#
)]
fn given_history_blob_row_tooltip(
    world: &mut RenderWorld,
    author: String,
    title: String,
    displayed: String,
    tooltip: String,
    base: String,
) {
    world.history_object_label = Some("blob".to_owned());
    world.history_entries.push(history_blob_entry(
        &author, &author, &title, &title, &displayed, &tooltip, &base,
    ));
}

#[given(regex = r#"^a history blob row authored "([^"]*)" shortened to "(.*)" at "([^"]*)"$"#)]
fn given_history_chopped_author(
    world: &mut RenderWorld,
    author: String,
    author_short: String,
    base: String,
) {
    world.history_object_label = Some("blob".to_owned());
    world.history_entries.push(history_blob_entry(
        &author,
        &author_short,
        "x",
        "x",
        "now",
        "now",
        &base,
    ));
}

#[given(regex = r#"^a history blob row subject "([^"]*)" shortened to "(.*)" at "([^"]*)"$"#)]
fn given_history_chopped_subject(
    world: &mut RenderWorld,
    title: String,
    title_short: String,
    base: String,
) {
    world.history_object_label = Some("blob".to_owned());
    world.history_entries.push(history_blob_entry(
        "Alice",
        "Alice",
        &title,
        &title_short,
        "now",
        "now",
        &base,
    ));
}

#[given(
    regex = r#"^a history tree row by "([^"]*)" titled "([^"]*)" dated "([^"]*)" at "([^"]*)"$"#
)]
fn given_history_tree_row(
    world: &mut RenderWorld,
    author: String,
    title: String,
    displayed: String,
    base: String,
) {
    world.history_object_label = Some("tree".to_owned());
    world.history_entries.push(HistoryEntryView {
        date_displayed: displayed.clone(),
        date_tooltip: displayed,
        author: author.clone(),
        author_short: author,
        title: title.clone(),
        title_short: title,
        commit: base.clone(),
        object: format!("{base}/tree"),
        commitdiff: format!("{base}/diff"),
        raw: None,
        diff_to_current: None,
        refs: Vec::new(),
    });
}

#[given(regex = r#"^the history row offers a diff to current at "([^"]*)"$"#)]
fn given_history_diff_to_current(world: &mut RenderWorld, href: String) {
    world
        .history_entries
        .last_mut()
        .expect("a history row to attach the diff to")
        .diff_to_current = Some(href);
}

#[given(regex = r#"^the history offers more at "([^"]*)" labelled "([^"]*)"$"#)]
fn given_history_more(world: &mut RenderWorld, href: String, label: String) {
    world.history_more = Some(MoreLink { href, label });
}

#[given(
    regex = r#"^a history blob row badged a "([^"]*)" ref "([^"]*)" titled "([^"]*)" linking "([^"]*)"$"#
)]
fn given_history_badged(
    world: &mut RenderWorld,
    class: String,
    name: String,
    title: String,
    href: String,
) {
    world.history_object_label = Some("blob".to_owned());
    let mut entry: HistoryEntryView =
        history_blob_entry("Alice", "Alice", "x", "x", "now", "now", "/c");
    entry.refs.push(RefMarkerView {
        name,
        class,
        title,
        href,
    });
    world.history_entries.push(entry);
}

#[when("I render the history table")]
fn when_render_history_table(world: &mut RenderWorld) {
    let table: HistoryTable = HistoryTable {
        object_label: world
            .history_object_label
            .take()
            .unwrap_or_else(|| "blob".to_owned()),
        rows: std::mem::take(&mut world.history_entries),
        more: world.history_more.take(),
    };
    world.output = Some(history_table(&table).into_string());
}

#[when("I render the history page")]
fn when_render_history_page(world: &mut RenderWorld) {
    let table: HistoryTable = HistoryTable {
        object_label: world
            .history_object_label
            .take()
            .unwrap_or_else(|| "blob".to_owned()),
        rows: std::mem::take(&mut world.history_entries),
        more: world.history_more.take(),
    };
    let page: HistoryPage = HistoryPage {
        crumbs: std::mem::take(&mut world.crumbs),
        nav: std::mem::take(&mut world.nav_items),
        file_name: "file.txt".to_owned(),
        table,
    };
    world.output = Some(history_body(&page).into_string());
}

// ---- Verbose log ------------------------------------------------------------

/// Builds a log entry from its display fields and a base href. The timestamp is a
/// fixed instant (`1780842645 +0200`, a Sunday afternoon UTC) so the badge
/// scenario can assert on it; the body defaults to the subject as its one line.
fn log_entry(author: &str, title: &str, age: &str, base: &str) -> LogEntryView {
    LogEntryView {
        age: age.to_owned(),
        title: title.to_owned(),
        author: author.to_owned(),
        timestamp: Timestamp::new(1_780_842_645, "+0200"),
        comment: vec![LogLine::Text(title.to_owned())],
        commit: base.to_owned(),
        commitdiff: format!("{base}/diff"),
        tree: format!("{base}/tree"),
        refs: Vec::new(),
    }
}

/// The most recently declared log entry, for a follow-up step that sets its body.
fn last_log_entry(world: &mut RenderWorld) -> &mut LogEntryView {
    world
        .log_entries
        .last_mut()
        .expect("declare a log entry first")
}

#[given(regex = r#"^a log entry by "([^"]*)" titled "([^"]*)" aged "([^"]*)" at "([^"]*)"$"#)]
fn given_log_entry(
    world: &mut RenderWorld,
    author: String,
    title: String,
    age: String,
    base: String,
) {
    world
        .log_entries
        .push(log_entry(&author, &title, &age, &base));
}

#[given(regex = r#"^its body is the sign-off "(.*)"$"#)]
fn given_log_body_signoff(world: &mut RenderWorld, line: String) {
    last_log_entry(world).comment = vec![LogLine::Signoff(line)];
}

#[given(regex = r#"^its body links "([^"]*)" to "([^"]*)"$"#)]
fn given_log_body_autolink(world: &mut RenderWorld, label: String, url: String) {
    last_log_entry(world).comment = vec![LogLine::Autolink { label, url }];
}

#[given(regex = r#"^the log offers more at "([^"]*)" labelled "([^"]*)"$"#)]
fn given_log_more(world: &mut RenderWorld, href: String, label: String) {
    world.log_more = Some(MoreLink { href, label });
}

#[given(
    regex = r#"^a log entry badged a "([^"]*)" ref "([^"]*)" titled "([^"]*)" linking "([^"]*)"$"#
)]
fn given_log_badged(
    world: &mut RenderWorld,
    class: String,
    name: String,
    title: String,
    href: String,
) {
    let mut entry: LogEntryView = log_entry("Alice", "x", "now", "/c");
    entry.refs.push(RefMarkerView {
        name,
        class,
        title,
        href,
    });
    world.log_entries.push(entry);
}

#[when("I render the log entries")]
fn when_render_log_entries(world: &mut RenderWorld) {
    let entries: Vec<LogEntryView> = std::mem::take(&mut world.log_entries);
    world.output = Some(log_entries(&entries, world.log_more.as_ref()).into_string());
}

#[when("I render the log page")]
fn when_render_log_page(world: &mut RenderWorld) {
    let page: LogPage = LogPage {
        crumbs: std::mem::take(&mut world.crumbs),
        nav: std::mem::take(&mut world.nav_items),
        entries: std::mem::take(&mut world.log_entries),
        more: world.log_more.take(),
    };
    world.output = Some(log_body(&page).into_string());
}

// ---- Tags table -------------------------------------------------------------

/// Builds a tag row from a base href: the name links to `base` (the tagged
/// object), the single `tag` view hangs off it, and the reftype-dependent links
/// hang off it too, so one base yields distinct, labelled links.
fn tag_entry(
    name: &str,
    base: &str,
    subject: Option<String>,
    age: TagAge,
    reftype: TagReftype,
) -> TagEntryView {
    let annotated: bool = subject.is_some();
    TagEntryView {
        age,
        name: name.to_owned(),
        object_href: base.to_owned(),
        subject,
        tag_view_href: format!("{base}/tag"),
        annotated,
        reftype,
    }
}

/// The `shortlog` / `log` links a commit tag hangs off `base`.
fn commit_reftype(base: &str) -> TagReftype {
    TagReftype::Commit {
        shortlog: format!("{base}/shortlog"),
        log: format!("{base}/log"),
    }
}

#[given(regex = r#"^an annotated commit tag "([^"]*)" at "([^"]*)" subject "(.*)" aged (\d+)$"#)]
fn given_annotated_commit_tag(
    world: &mut RenderWorld,
    name: String,
    base: String,
    subject: String,
    age_seconds: i64,
) {
    world.tag_entries.push(tag_entry(
        &name,
        &base,
        Some(subject),
        TagAge::Known(Age::from_seconds(age_seconds)),
        commit_reftype(&base),
    ));
}

#[given(
    regex = r#"^an annotated commit tag "([^"]*)" at "([^"]*)" subject "(.*)" with an unknown age$"#
)]
fn given_annotated_commit_tag_unknown_age(
    world: &mut RenderWorld,
    name: String,
    base: String,
    subject: String,
) {
    world.tag_entries.push(tag_entry(
        &name,
        &base,
        Some(subject),
        TagAge::Unknown,
        commit_reftype(&base),
    ));
}

#[given(regex = r#"^a lightweight blob tag "([^"]*)" at "([^"]*)" with no age cell$"#)]
fn given_lightweight_blob_tag_absent(world: &mut RenderWorld, name: String, base: String) {
    let reftype: TagReftype = TagReftype::Blob {
        raw: format!("{base}/raw"),
    };
    world
        .tag_entries
        .push(tag_entry(&name, &base, None, TagAge::Absent, reftype));
}

#[given(regex = r#"^a lightweight commit tag "([^"]*)" at "([^"]*)" aged (\d+)$"#)]
fn given_lightweight_commit_tag(
    world: &mut RenderWorld,
    name: String,
    base: String,
    age_seconds: i64,
) {
    world.tag_entries.push(tag_entry(
        &name,
        &base,
        None,
        TagAge::Known(Age::from_seconds(age_seconds)),
        commit_reftype(&base),
    ));
}

#[given(regex = r#"^an annotated blob tag "([^"]*)" at "([^"]*)" subject "(.*)" aged (\d+)$"#)]
fn given_annotated_blob_tag(
    world: &mut RenderWorld,
    name: String,
    base: String,
    subject: String,
    age_seconds: i64,
) {
    let reftype: TagReftype = TagReftype::Blob {
        raw: format!("{base}/raw"),
    };
    world.tag_entries.push(tag_entry(
        &name,
        &base,
        Some(subject),
        TagAge::Known(Age::from_seconds(age_seconds)),
        reftype,
    ));
}

#[given(regex = r#"^an annotated tree tag "([^"]*)" at "([^"]*)" subject "(.*)" aged (\d+)$"#)]
fn given_annotated_tree_tag(
    world: &mut RenderWorld,
    name: String,
    base: String,
    subject: String,
    age_seconds: i64,
) {
    world.tag_entries.push(tag_entry(
        &name,
        &base,
        Some(subject),
        TagAge::Known(Age::from_seconds(age_seconds)),
        TagReftype::Tree,
    ));
}

#[when("I render the tags table")]
fn when_render_tags_table(world: &mut RenderWorld) {
    let table: TagsTable = TagsTable {
        rows: std::mem::take(&mut world.tag_entries),
        more: world.tags_more.take(),
    };
    world.output = Some(tags_table(&table).into_string());
}

/// Renders a search-help page documenting `topics`, with no project chrome so the
/// assertions see only the help body.
fn render_search_help(world: &mut RenderWorld, topics: Vec<SearchHelpTopic>) {
    let page: SearchHelpPage = SearchHelpPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        topics,
    };
    world.output = Some(search_help_body(&page).into_string());
}

#[when("I render the search help page documenting every type")]
fn when_render_search_help_all(world: &mut RenderWorld) {
    render_search_help(
        world,
        vec![
            SearchHelpTopic::Commit,
            SearchHelpTopic::Grep,
            SearchHelpTopic::Author,
            SearchHelpTopic::Committer,
            SearchHelpTopic::Pickaxe,
        ],
    );
}

#[when("I render the search help page documenting only the always-available types")]
fn when_render_search_help_minimal(world: &mut RenderWorld) {
    render_search_help(
        world,
        vec![
            SearchHelpTopic::Commit,
            SearchHelpTopic::Author,
            SearchHelpTopic::Committer,
        ],
    );
}

#[when("I render the tags page with no tags")]
fn when_render_tags_page_empty(world: &mut RenderWorld) {
    let page: TagsPage = TagsPage {
        crumbs: Vec::new(),
        ref_views: Vec::new(),
        table: TagsTable {
            rows: Vec::new(),
            more: None,
        },
    };
    world.output = Some(tags_body(&page).into_string());
}

// ---- Single tag view --------------------------------------------------------

/// Splits a `"Name <email>"` tagger ident into its parts. The render givens
/// always supply an email; the email-absent case is a domain-layer concern.
fn tagger_parts(ident: &str) -> (String, String) {
    let (name, rest): (&str, &str) = ident.split_once(" <").expect("ident has an email");
    (name.to_owned(), rest.trim_end_matches('>').to_owned())
}

/// Assembles a tag page with no project chrome, so the assertions see only the
/// tag-specific markup.
fn tag_page_of(
    name: &str,
    object_id: &str,
    href: &str,
    kind_label: &str,
    tagger: Option<TagAuthorView>,
    message_lines: Vec<String>,
) -> TagPage {
    TagPage {
        crumbs: Vec::new(),
        name: name.to_owned(),
        object: TaggedObjectView {
            id: object_id.to_owned(),
            kind_label: kind_label.to_owned(),
            href: href.to_owned(),
        },
        tagger,
        message_lines,
    }
}

/// A tagger view from an ident and an absolute tag epoch + timezone.
fn tagger_view(ident: &str, epoch: i64, tz: &str) -> TagAuthorView {
    let (name, email): (String, String) = tagger_parts(ident);
    TagAuthorView {
        name,
        email: Some(email),
        timestamp: Timestamp::new(epoch, tz),
    }
}

/// Sets up a tag page with its object row and message but no tagger yet — the
/// "tagged by …" step attaches one. gitweb's object_header row and the tagger
/// authorship rows are separate facts, so they are separate Givens here.
fn set_up_tag(
    world: &mut RenderWorld,
    kind: &str,
    name: &str,
    object_id: &str,
    href: &str,
    message_lines: Vec<String>,
) {
    world.tag_page = Some(tag_page_of(
        name,
        object_id,
        href,
        kind,
        None,
        message_lines,
    ));
}

#[given(
    regex = r#"^a (commit|blob|tree) tag "([^"]*)" pointing at "([^"]*)" at "([^"]*)" with message "(.*)"$"#
)]
fn given_tag_with_message(
    world: &mut RenderWorld,
    kind: String,
    name: String,
    object_id: String,
    href: String,
    message: String,
) {
    set_up_tag(world, &kind, &name, &object_id, &href, vec![message]);
}

#[given(
    regex = r#"^a commit tag "([^"]*)" pointing at "([^"]*)" at "([^"]*)" with a two-line message$"#
)]
fn given_tag_multiline(world: &mut RenderWorld, name: String, object_id: String, href: String) {
    let lines: Vec<String> = vec!["First line".to_owned(), "Second line".to_owned()];
    set_up_tag(world, "commit", &name, &object_id, &href, lines);
}

#[given(regex = r#"^tagged by "([^"]*)" at epoch (\d+) ([-+]\d{4})$"#)]
fn given_tagged_by(world: &mut RenderWorld, ident: String, epoch: i64, tz: String) {
    let page: &mut TagPage = world.tag_page.as_mut().expect("a tag page set up first");
    page.tagger = Some(tagger_view(&ident, epoch, &tz));
}

#[when("I render the tag page")]
fn when_render_tag_page(world: &mut RenderWorld) {
    let page: TagPage = world.tag_page.take().expect("a tag page was set up");
    world.output = Some(tag_body(&page).into_string());
}

// ---- Error responses (die_error) --------------------------------------------

fn record_error(world: &mut RenderWorld, response: ErrorResponse) {
    world.status = Some(response.status);
    world.output = Some(response.body.into_string());
}

fn record_page(world: &mut RenderWorld, status: HttpStatus, body: Markup) {
    world.status = Some(status);
    world.output = Some(body.into_string());
}

#[given(regex = r#"^a not-found failure "(.*)"$"#)]
fn given_not_found(world: &mut RenderWorld, what: String) {
    world.domain_error = Some(DomainError::NotFound(what));
}

#[given(regex = r#"^an invalid-input failure "(.*)"$"#)]
fn given_invalid(world: &mut RenderWorld, what: String) {
    world.domain_error = Some(DomainError::Invalid(what));
}

#[given(regex = r#"^a forbidden failure "(.*)"$"#)]
fn given_forbidden(world: &mut RenderWorld, what: String) {
    world.domain_error = Some(DomainError::Forbidden(what));
}

#[given(regex = r#"^a backend failure "(.*)"$"#)]
fn given_backend(world: &mut RenderWorld, why: String) {
    world.domain_error = Some(DomainError::Backend(why));
}

#[when("I render the error response")]
fn when_render_error_response(world: &mut RenderWorld) {
    let error: &DomainError = world
        .domain_error
        .as_ref()
        .expect("scenario must set a domain failure");
    record_error(world, error_response(error));
}

#[when(regex = r#"^I render a 400 error page saying "(.*)" with detail "(.*)"$"#)]
fn when_render_400_detail(world: &mut RenderWorld, message: String, detail: String) {
    let body: Markup = error_page(HttpStatus::BAD_REQUEST, &message, Some(raw(detail)));
    record_page(world, HttpStatus::BAD_REQUEST, body);
}

#[when(regex = r#"^I render a 404 error page saying "(.*)"$"#)]
fn when_render_404(world: &mut RenderWorld, message: String) {
    let body: Markup = error_page(HttpStatus::NOT_FOUND, &message, None);
    record_page(world, HttpStatus::NOT_FOUND, body);
}

#[when(regex = r#"^I render a 500 error page saying "(.*)"$"#)]
fn when_render_500(world: &mut RenderWorld, message: String) {
    let body: Markup = error_page(HttpStatus::INTERNAL, &message, None);
    record_page(world, HttpStatus::INTERNAL, body);
}

#[when(regex = r#"^I render a 503 error page saying "(.*)"$"#)]
fn when_render_503(world: &mut RenderWorld, message: String) {
    let body: Markup = error_page(HttpStatus::UNAVAILABLE, &message, None);
    record_page(world, HttpStatus::UNAVAILABLE, body);
}

#[then(regex = r"^the HTTP status is (\d+)$")]
fn then_http_status(world: &mut RenderWorld, code: u16) {
    let status: HttpStatus = world.status.expect("a rendered error sets a status");
    assert_eq!(status.code, code);
}

#[then(regex = r#"^the status title is "(.*)"$"#)]
fn then_status_title(world: &mut RenderWorld, expected: String) {
    let status: HttpStatus = world.status.expect("a rendered error sets a status");
    assert_eq!(status.title(), expected);
}

#[then(regex = r#"^the body contains "(.*)"$"#)]
fn then_body_contains(world: &mut RenderWorld, expected: String) {
    let output: &str = world.output.as_deref().expect("a rendered body");
    assert!(
        output.contains(&expected),
        "expected body to contain {expected:?}\n  got: {output}"
    );
}

#[then(regex = r#"^the body does not contain "(.*)"$"#)]
fn then_body_excludes(world: &mut RenderWorld, unexpected: String) {
    let output: &str = world.output.as_deref().expect("a rendered body");
    assert!(
        !output.contains(&unexpected),
        "expected body NOT to contain {unexpected:?}\n  got: {output}"
    );
}

// ---- Then: assert the rendered result ---------------------------------------

#[then(regex = r#"^the result is "(.*)"$"#)]
fn then_result_is(world: &mut RenderWorld, expected: String) {
    assert_eq!(world.output.as_deref(), Some(expected.as_str()));
}

#[then("the result is a single tab character")]
fn then_result_is_tab(world: &mut RenderWorld) {
    assert_eq!(world.output.as_deref(), Some("\t"));
}

// ---- tree table -------------------------------------------------------------

/// Builds a file row from its name, size, and a base href: the name links to the
/// blob view, and the links cell offers blob | history | raw off the same base.
fn tree_file_row(name: &str, size: &str, base: &str) -> TreeRowView {
    TreeRowView {
        mode: "-rw-r--r--".to_owned(),
        size: Some(size.to_owned()),
        name: name.to_owned(),
        name_href: Some(format!("{base}/blob")),
        symlink_target: None,
        links: vec![
            TreeLink {
                label: "blob".to_owned(),
                href: format!("{base}/blob"),
            },
            TreeLink {
                label: "history".to_owned(),
                href: format!("{base}/history"),
            },
            TreeLink {
                label: "raw".to_owned(),
                href: format!("{base}/raw"),
            },
        ],
    }
}

#[given(regex = r#"^a tree file row "([^"]*)" sized "([^"]*)" at "([^"]*)"$"#)]
fn given_tree_file_row(world: &mut RenderWorld, name: String, size: String, base: String) {
    world.tree_rows.push(tree_file_row(&name, &size, &base));
}

#[given(regex = r#"^a tree symlink row "([^"]*)" pointing to "([^"]*)" at "([^"]*)"$"#)]
fn given_tree_symlink_row(world: &mut RenderWorld, name: String, target: String, base: String) {
    world.tree_rows.push(TreeRowView {
        mode: "lrwxrwxrwx".to_owned(),
        size: Some(target.len().to_string()),
        name,
        name_href: Some(format!("{base}/blob")),
        symlink_target: Some(target),
        links: vec![TreeLink {
            label: "blob".to_owned(),
            href: format!("{base}/blob"),
        }],
    });
}

#[given(regex = r#"^a tree directory row "([^"]*)" at "([^"]*)"$"#)]
fn given_tree_directory_row(world: &mut RenderWorld, name: String, base: String) {
    world.tree_rows.push(TreeRowView {
        mode: "drwxr-xr-x".to_owned(),
        size: Some("-".to_owned()),
        name,
        name_href: Some(format!("{base}/tree")),
        symlink_target: None,
        links: vec![TreeLink {
            label: "tree".to_owned(),
            href: format!("{base}/tree"),
        }],
    });
}

#[given(regex = r#"^a tree submodule row "([^"]*)"$"#)]
fn given_tree_submodule_row(world: &mut RenderWorld, name: String) {
    world.tree_rows.push(TreeRowView {
        mode: "m---------".to_owned(),
        size: Some("-".to_owned()),
        name,
        name_href: None,
        symlink_target: None,
        links: Vec::new(),
    });
}

#[given("the tree size column is off")]
fn given_tree_size_off(world: &mut RenderWorld) {
    world.tree_size_off = true;
}

#[given(regex = r#"^the tree offers a parent row at "([^"]*)"$"#)]
fn given_tree_parent(world: &mut RenderWorld, href: String) {
    world.tree_parent = Some(TreeParentRow { href });
}

#[when("I render the tree table")]
fn when_render_tree_table(world: &mut RenderWorld) {
    let table: TreeTable = TreeTable {
        show_sizes: !world.tree_size_off,
        parent: world.tree_parent.take(),
        rows: std::mem::take(&mut world.tree_rows),
    };
    world.output = Some(tree_table(&table).into_string());
}

#[when("I render the tree page")]
fn when_render_tree_page(world: &mut RenderWorld) {
    let table: TreeTable = TreeTable {
        show_sizes: !world.tree_size_off,
        parent: world.tree_parent.take(),
        rows: std::mem::take(&mut world.tree_rows),
    };
    let page: TreePage = TreePage {
        crumbs: std::mem::take(&mut world.crumbs),
        nav: std::mem::take(&mut world.nav_items),
        title: "main tree".to_owned(),
        path: None,
        table,
    };
    world.output = Some(tree_body(&page).into_string());
}

#[then(regex = r#"^the result contains "(.*)"$"#)]
fn then_result_contains(world: &mut RenderWorld, expected: String) {
    let output: &str = world.output.as_deref().expect("a rendered result");
    assert!(
        output.contains(&expected),
        "expected output to contain {expected:?}\n  got: {output}"
    );
}

#[then(regex = r#"^the result does not contain "(.*)"$"#)]
fn then_result_excludes(world: &mut RenderWorld, unexpected: String) {
    let output: &str = world.output.as_deref().expect("a rendered result");
    assert!(
        !output.contains(&unexpected),
        "expected output NOT to contain {unexpected:?}\n  got: {output}"
    );
}

// ---- blob content -----------------------------------------------------------

/// Builds the blob content the givens described: an image or binary if one was
/// declared, otherwise the (possibly empty) text lines.
fn built_blob_content(world: &mut RenderWorld) -> BlobContent {
    if let Some((src, alt)) = world.blob_image.take() {
        return BlobContent::Image { src, alt };
    }
    if let Some(raw_href) = world.blob_binary.take() {
        return BlobContent::Binary { raw_href };
    }
    BlobContent::Text {
        lines: std::mem::take(&mut world.blob_lines),
    }
}

#[given(regex = r#"^a blob line (\d+) reading "(.*)"$"#)]
fn given_blob_line(world: &mut RenderWorld, number: usize, text: String) {
    world.blob_lines.push(BlobLine { number, text });
}

#[given(regex = r#"^the blob is an image at "([^"]*)" described as "([^"]*)"$"#)]
fn given_blob_image(world: &mut RenderWorld, src: String, alt: String) {
    world.blob_image = Some((src, alt));
}

#[given(regex = r#"^the blob is a binary download at "([^"]*)"$"#)]
fn given_blob_binary(world: &mut RenderWorld, raw_href: String) {
    world.blob_binary = Some(raw_href);
}

#[when("I render the blob content")]
fn when_render_blob_content(world: &mut RenderWorld) {
    let content: BlobContent = built_blob_content(world);
    world.output = Some(blob_content(&content).into_string());
}

#[when("I render the blob page")]
fn when_render_blob_page(world: &mut RenderWorld) {
    let content: BlobContent = built_blob_content(world);
    let page: BlobPage = BlobPage {
        crumbs: std::mem::take(&mut world.crumbs),
        nav: std::mem::take(&mut world.nav_items),
        formats: Vec::new(),
        title: "readme.txt".to_owned(),
        path: Some("readme.txt".to_owned()),
        content,
    };
    world.output = Some(blob_body(&page).into_string());
}

#[then("the result is:")]
fn then_result_is_docstring(world: &mut RenderWorld, step: &Step) {
    // gherkin frames docstring content with the newlines next to its `"""`
    // delimiters; strip those so the spec shows the exact HTML fragment.
    let expected: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    assert_eq!(world.output.as_deref(), Some(expected));
}

// ---- diff-viewer host page --------------------------------------------------

#[given(regex = r#"^a diff host fetching "([^"]*)"$"#)]
fn given_diff_host(world: &mut RenderWorld, diff_url: String) {
    world.diff_url = Some(diff_url);
}

#[given(regex = r#"^the viewer module is served at "([^"]*)"$"#)]
fn given_diff_viewer_module(world: &mut RenderWorld, src: String) {
    world.diff_viewer_module_src = Some(src);
}

#[given(regex = r#"^the diff host title is "([^"]*)"$"#)]
fn given_diff_title(world: &mut RenderWorld, title: String) {
    world.diff_title = Some(title);
}

#[given(regex = r#"^the diff host is scoped to file "([^"]*)"$"#)]
fn given_diff_path(world: &mut RenderWorld, path: String) {
    world.diff_path = Some(path);
}

#[given(regex = r#"^the diff host offers format "([^"]*)" at "([^"]*)"$"#)]
fn given_diff_format(world: &mut RenderWorld, label: String, href: String) {
    world.diff_formats.push(FormatLink { label, href });
}

#[when("I render the diff host page")]
fn when_render_diff_host(world: &mut RenderWorld) {
    let page: DiffHostPage = DiffHostPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        formats: std::mem::take(&mut world.diff_formats),
        title: world.diff_title.take().unwrap_or_default(),
        path: world.diff_path.take(),
        diff_url: world.diff_url.take().expect("a diff URL must be set"),
        viewer_module_src: world
            .diff_viewer_module_src
            .take()
            .expect("a viewer module src must be set"),
    };
    world.output = Some(diff_host_body(&page).into_string());
}

// ---- Summary page -----------------------------------------------------------

#[given(regex = r#"^a summary described as "([^"]*)"$"#)]
fn given_summary_described(world: &mut RenderWorld, description: String) {
    world.summary_description = Some(description);
}

#[given(regex = r#"^a summary described as "([^"]*)" owned by "([^"]*)"$"#)]
fn given_summary_described_owned(world: &mut RenderWorld, description: String, owner: String) {
    world.summary_description = Some(description);
    world.summary_owner = Some(owner);
}

#[given("a summary with a last change")]
fn given_summary_last_change(world: &mut RenderWorld) {
    world.summary_has_last_change = true;
}

#[given(regex = r#"^a summary with clone url "([^"]*)"$"#)]
fn given_summary_clone_url(world: &mut RenderWorld, url: String) {
    world.summary_clone_urls.push(url);
}

#[given(regex = r#"^a summary with README "([^"]*)"$"#)]
fn given_summary_readme(world: &mut RenderWorld, readme: String) {
    world.summary_readme = Some(readme);
}

#[given(regex = r#"^a summary with a shortlog section at "([^"]*)"$"#)]
fn given_summary_shortlog(world: &mut RenderWorld, href: String) {
    world.summary_shortlog_href = Some(href);
}

#[given(regex = r#"^a summary with a tags section at "([^"]*)"$"#)]
fn given_summary_tags(world: &mut RenderWorld, href: String) {
    world.summary_tags_href = Some(href);
}

#[given(regex = r#"^a summary with a heads section at "([^"]*)"$"#)]
fn given_summary_heads(world: &mut RenderWorld, href: String) {
    world.summary_heads_href = Some(href);
}

#[when("I render the summary")]
fn when_render_summary(world: &mut RenderWorld) {
    let metadata: SummaryMetadata = SummaryMetadata {
        description: world
            .summary_description
            .clone()
            .unwrap_or_else(|| "none".to_owned()),
        owner: world.summary_owner.clone(),
        last_change: world
            .summary_has_last_change
            .then(|| Timestamp::from_epoch(1_577_880_000)),
        clone_urls: std::mem::take(&mut world.summary_clone_urls),
    };
    let page: SummaryPage = SummaryPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        metadata,
        readme: world.summary_readme.take(),
        shortlog: world
            .summary_shortlog_href
            .take()
            .map(|href: String| ShortlogSection {
                href,
                table: ShortlogTable {
                    rows: Vec::new(),
                    more: None,
                },
            }),
        tags: world
            .summary_tags_href
            .take()
            .map(|href: String| TagsSection {
                href,
                table: TagsTable {
                    rows: Vec::new(),
                    more: None,
                },
            }),
        heads: world
            .summary_heads_href
            .take()
            .map(|href: String| HeadsSection {
                href,
                table: HeadsTable {
                    rows: Vec::new(),
                    more: None,
                },
            }),
    };
    world.output = Some(summary_body(&page).into_string());
}

// ---- Remotes page -----------------------------------------------------------

/// The last declared remote block, which the per-block givens (URL lines,
/// tracking branches) accrue onto.
fn last_remote_block(world: &mut RenderWorld) -> &mut RemoteBlockView {
    world
        .remote_blocks
        .last_mut()
        .expect("declare a remote block first")
}

/// An empty heads table, the starting point a block's tracking branches accrue
/// onto.
fn empty_heads_table() -> HeadsTable {
    HeadsTable {
        rows: Vec::new(),
        more: None,
    }
}

#[given(regex = r#"^a remotes page titled "([^"]*)"$"#)]
fn given_remotes_page(world: &mut RenderWorld, title: String) {
    world.remotes_title = Some(title);
}

#[given(regex = r#"^a remote block "([^"]*)" linking to "([^"]*)"$"#)]
fn given_remote_block_linked(world: &mut RenderWorld, name: String, href: String) {
    world.remote_blocks.push(RemoteBlockView {
        name,
        href: Some(href),
        urls: Vec::new(),
        heads: empty_heads_table(),
    });
}

#[given(regex = r#"^a remote block "([^"]*)" with no link$"#)]
fn given_remote_block_unlinked(world: &mut RenderWorld, name: String) {
    world.remote_blocks.push(RemoteBlockView {
        name,
        href: None,
        urls: Vec::new(),
        heads: empty_heads_table(),
    });
}

#[given(regex = r#"^the remote block has a URL line labelled "([^"]*)" of "([^"]*)"$"#)]
fn given_remote_url_line(world: &mut RenderWorld, label: String, value: String) {
    last_remote_block(world).urls.push(RemoteUrlLine {
        label: Some(label),
        value,
    });
}

#[given(regex = r#"^the remote block has an unlabelled URL line of "([^"]*)"$"#)]
fn given_remote_url_line_unlabelled(world: &mut RenderWorld, value: String) {
    last_remote_block(world)
        .urls
        .push(RemoteUrlLine { label: None, value });
}

#[given(regex = r#"^the remote block tracks branch "([^"]*)" at "([^"]*)"$"#)]
fn given_remote_tracking_branch(world: &mut RenderWorld, name: String, base: String) {
    let row: HeadEntryView = head_entry(&name, &base, Some(Age::from_seconds(600)), false);
    last_remote_block(world).heads.rows.push(row);
}

#[when("I render the remotes page")]
fn when_render_remotes_page(world: &mut RenderWorld) {
    let page: RemotesPage = RemotesPage {
        crumbs: std::mem::take(&mut world.crumbs),
        ref_views: std::mem::take(&mut world.nav_items),
        title: world.remotes_title.take().unwrap_or_default(),
        blocks: std::mem::take(&mut world.remote_blocks),
    };
    world.output = Some(remotes_body(&page).into_string());
}

// ---- feed: RSS / Atom serialization -----------------------------------------

/// A canonical one-entry, one-file feed exercising the metadata, the date forms,
/// per-site escaping (an author with `<>`, a path with `&`), and the optional
/// `history` link. `latest` toggles the with-commits vs empty-feed shape.
fn sample_feed(latest: bool) -> FeedView {
    let stamp: Timestamp = Timestamp::from_epoch(1_700_000_000);
    let entries: Vec<FeedEntryView> = if latest {
        vec![FeedEntryView {
            title: "fix <stuff>".to_owned(),
            author_full: "Ada Lovelace <ada@example.com>".to_owned(),
            author_name: "Ada Lovelace".to_owned(),
            author_email: Some("ada@example.com".to_owned()),
            committer_name: "Ada Lovelace".to_owned(),
            committer_email: Some("ada@example.com".to_owned()),
            timestamp: stamp.clone(),
            commitdiff_url: "http://h/?p=repo;a=commitdiff;h=abc".to_owned(),
            comment: vec!["fix <stuff>".to_owned(), "more".to_owned()],
            files: vec![FeedFileView {
                to_path: "a&b.txt".to_owned(),
                blobdiff_url: "http://h/?p=repo;a=blobdiff;f=a%26b.txt".to_owned(),
                blame_url: None,
                history_url: Some("http://h/?p=repo;a=history;f=a%26b.txt".to_owned()),
            }],
        }]
    } else {
        Vec::new()
    };
    FeedView {
        title: "Untitled - repo log".to_owned(),
        description: "desc".to_owned(),
        owner: "Ada Lovelace".to_owned(),
        generator: "gitweb-x/2.0".to_owned(),
        logo: "static/git-logo.png".to_owned(),
        favicon: "static/git-favicon.png".to_owned(),
        alt_url: "http://h/?p=repo;a=summary".to_owned(),
        id_url: "http://h/?p=repo".to_owned(),
        self_url: "http://h?p=repo;a=atom".to_owned(),
        xml_base: "http://h/".to_owned(),
        content_type: "application/atom+xml".to_owned(),
        latest: latest.then_some(stamp),
        entries,
    }
}

#[given("a feed with one commit")]
fn given_feed_one_commit(world: &mut RenderWorld) {
    world.feed_view = Some(sample_feed(true));
}

#[given("an empty feed")]
fn given_empty_feed(world: &mut RenderWorld) {
    world.feed_view = Some(sample_feed(false));
}

#[when("I render the feed as RSS")]
fn when_render_rss(world: &mut RenderWorld) {
    let view: &FeedView = world
        .feed_view
        .as_ref()
        .expect("a feed must be built first");
    world.output = Some(rss(view));
}

#[when("I render the feed as Atom")]
fn when_render_atom(world: &mut RenderWorld) {
    let view: &FeedView = world
        .feed_view
        .as_ref()
        .expect("a feed must be built first");
    world.output = Some(atom(view));
}

// --- commit view rendering ---------------------------------------------------

/// An author/committer row with a fixed zone-aware date (the same instant the tag
/// scenarios use, so its rendered badge is known).
fn author_row_of(name: &str, email: Option<&str>) -> AuthorRow {
    AuthorRow {
        name: name.to_owned(),
        email: email.map(str::to_owned),
        timestamp: Timestamp::new(1_780_842_645, "+0200"),
    }
}

/// A labelled action link for a changed-files row.
fn action_link(label: &str, href: &str) -> FormatLink {
    FormatLink {
        label: label.to_owned(),
        href: href.to_owned(),
    }
}

#[when("I render a single-parent commit page")]
fn render_single_parent_commit(world: &mut RenderWorld) {
    let page: CommitPage = CommitPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        parent_nav: ParentNav::Single(ParentNavLink {
            short: "parent0".to_owned(),
            href: "/r/commit/parent01".to_owned(),
        }),
        title: "Add the analytical engine".to_owned(),
        commit_id: "c0ffeec0ffee".to_owned(),
        tree: LinkedId {
            id: "treeid".to_owned(),
            href: "/r/tree/c0ffee".to_owned(),
        },
        author: author_row_of("Ada Lovelace", Some("ada@example.com")),
        committer: author_row_of("Linus Torvalds", Some("linus@example.com")),
        parents: vec![ParentRow {
            id: "parent01".to_owned(),
            commit_href: "/r/commit/parent01".to_owned(),
            diff_href: "/r/commitdiff/parent01".to_owned(),
        }],
        comment: vec![LogLine::Text("Add the analytical engine".to_owned())],
        changed: vec![
            ChangedRow {
                path: "engine.rs".to_owned(),
                path_href: Some("/r/blob/engine".to_owned()),
                note: ChangeNoteView::Text {
                    category: "changed".to_owned(),
                    text: "changed mode: 0644->0755".to_owned(),
                },
                links: vec![
                    action_link("blob", "/r/blob/engine"),
                    action_link("diff", "/r/blobdiff/engine"),
                    action_link("history", "/r/history/engine"),
                ],
            },
            ChangedRow {
                path: "new.rs".to_owned(),
                path_href: Some("/r/blob/new".to_owned()),
                note: ChangeNoteView::Rename {
                    category: "moved".to_owned(),
                    from_path: "old.rs".to_owned(),
                    from_href: "/r/blob/old".to_owned(),
                    similarity: 95,
                    mode: None,
                },
                links: vec![action_link("blob", "/r/blob/new")],
            },
            ChangedRow {
                path: "README".to_owned(),
                path_href: Some("/r/blob/readme".to_owned()),
                note: ChangeNoteView::Text {
                    category: "new".to_owned(),
                    text: "new file with mode: 0644".to_owned(),
                },
                links: vec![action_link("blob", "/r/blob/readme")],
            },
        ],
    };
    world.output = Some(commit_body(&page).into_string());
}

#[when("I render a root commit page")]
fn render_root_commit(world: &mut RenderWorld) {
    let page: CommitPage = CommitPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        parent_nav: ParentNav::Initial,
        title: "Initial commit".to_owned(),
        commit_id: "1n1t1n1t".to_owned(),
        tree: LinkedId {
            id: "tinit".to_owned(),
            href: "/r/tree/init".to_owned(),
        },
        author: author_row_of("Tester", Some("t@example.com")),
        committer: author_row_of("Tester", Some("t@example.com")),
        parents: Vec::new(),
        comment: vec![LogLine::Text("Initial commit".to_owned())],
        changed: vec![ChangedRow {
            path: "LICENSE".to_owned(),
            path_href: Some("/r/blob/license".to_owned()),
            note: ChangeNoteView::Text {
                category: "new".to_owned(),
                text: "new file with mode: 0644".to_owned(),
            },
            links: vec![action_link("blob", "/r/blob/license")],
        }],
    };
    world.output = Some(commit_body(&page).into_string());
}

#[when("I render a merge commit page")]
fn render_merge_commit(world: &mut RenderWorld) {
    let page: CommitPage = CommitPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        parent_nav: ParentNav::Merge(vec![
            ParentNavLink {
                short: "par1".to_owned(),
                href: "/r/commit/par1".to_owned(),
            },
            ParentNavLink {
                short: "par2".to_owned(),
                href: "/r/commit/par2".to_owned(),
            },
        ]),
        title: "Merge branch topic".to_owned(),
        commit_id: "mer9emer9e".to_owned(),
        tree: LinkedId {
            id: "tmerge".to_owned(),
            href: "/r/tree/merge".to_owned(),
        },
        author: author_row_of("Tester", Some("t@example.com")),
        committer: author_row_of("Tester", Some("t@example.com")),
        parents: vec![
            ParentRow {
                id: "par1".to_owned(),
                commit_href: "/r/commit/par1".to_owned(),
                diff_href: "/r/commitdiff/par1".to_owned(),
            },
            ParentRow {
                id: "par2".to_owned(),
                commit_href: "/r/commit/par2".to_owned(),
                diff_href: "/r/commitdiff/par2".to_owned(),
            },
        ],
        comment: vec![LogLine::Text("Merge branch topic".to_owned())],
        changed: vec![ChangedRow {
            path: "conflict.txt".to_owned(),
            path_href: Some("/r/blob/conflict".to_owned()),
            note: ChangeNoteView::None,
            links: vec![action_link("blob", "/r/blob/conflict")],
        }],
    };
    world.output = Some(commit_body(&page).into_string());
}

// ---- Commitdiff host page ---------------------------------------------------

/// A changed-files row in commitdiff context: a path with the patch-anchor and
/// blob links the web boundary builds for `git_commitdiff`.
fn commitdiff_changed_row(path: &str) -> ChangedRow {
    ChangedRow {
        path: path.to_owned(),
        path_href: Some(format!("/r/blob/{path}")),
        note: ChangeNoteView::None,
        links: vec![
            action_link("patch", "#patch1"),
            action_link("blob", &format!("/r/blob/{path}")),
        ],
    }
}

#[when("I render a single-parent commitdiff page")]
fn render_single_parent_commitdiff(world: &mut RenderWorld) {
    let page: CommitdiffPage = CommitdiffPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        formats: vec![
            action_link("raw", "/r/raw/c0ffee"),
            action_link("patch", "/r/patch/c0ffee"),
        ],
        parentage: CommitdiffNav::SingleParent(ParentNavLink {
            short: "parent0".to_owned(),
            href: "/r/commit/parent01".to_owned(),
        }),
        title: "Rework the engine".to_owned(),
        author: author_row_of("Ada Lovelace", Some("ada@example.com")),
        committer: author_row_of("Linus Torvalds", Some("linus@example.com")),
        comment: vec![
            LogLine::Text("Rework the engine".to_owned()),
            LogLine::Text("Detailed body line.".to_owned()),
        ],
        changed: vec![commitdiff_changed_row("README")],
        diff_url: "/r/diff/c0ffee".to_owned(),
        viewer_module_src: "/static/diff-viewer.js".to_owned(),
    };
    world.output = Some(commitdiff_body(&page).into_string());
}

#[when("I render a root commitdiff page")]
fn render_root_commitdiff(world: &mut RenderWorld) {
    let page: CommitdiffPage = CommitdiffPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        formats: vec![action_link("raw", "/r/raw/1n1t")],
        parentage: CommitdiffNav::Initial,
        title: "Initial import".to_owned(),
        author: author_row_of("Tester", Some("t@example.com")),
        committer: author_row_of("Tester", Some("t@example.com")),
        comment: vec![LogLine::Text("Initial import".to_owned())],
        changed: vec![commitdiff_changed_row("LICENSE")],
        diff_url: "/r/diff/1n1t".to_owned(),
        viewer_module_src: "/static/diff-viewer.js".to_owned(),
    };
    world.output = Some(commitdiff_body(&page).into_string());
}

#[when("I render a merge commitdiff page")]
fn render_merge_commitdiff(world: &mut RenderWorld) {
    let page: CommitdiffPage = CommitdiffPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        formats: vec![action_link("raw", "/r/raw/mer9e")],
        parentage: CommitdiffNav::Merge(vec![
            ParentNavLink {
                short: "par1".to_owned(),
                href: "/r/commit/par1".to_owned(),
            },
            ParentNavLink {
                short: "par2".to_owned(),
                href: "/r/commit/par2".to_owned(),
            },
        ]),
        title: "Merge side branches".to_owned(),
        author: author_row_of("Tester", Some("t@example.com")),
        committer: author_row_of("Tester", Some("t@example.com")),
        comment: vec![LogLine::Text("Merge side branches".to_owned())],
        changed: vec![commitdiff_changed_row("file.txt")],
        diff_url: "/r/diff/mer9e".to_owned(),
        viewer_module_src: "/static/diff-viewer.js".to_owned(),
    };
    world.output = Some(commitdiff_body(&page).into_string());
}

#[when("I render a commitdiff page from a chosen parent")]
fn render_commitdiff_from_parent(world: &mut RenderWorld) {
    let page: CommitdiffPage = CommitdiffPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        formats: vec![action_link("raw", "/r/raw/mer9e")],
        parentage: CommitdiffNav::FromParent {
            number: Some(2),
            link: ParentNavLink {
                short: "par2".to_owned(),
                href: "/r/commit/par2".to_owned(),
            },
        },
        title: "Merge side branches".to_owned(),
        author: author_row_of("Tester", Some("t@example.com")),
        committer: author_row_of("Tester", Some("t@example.com")),
        comment: vec![LogLine::Text("Merge side branches".to_owned())],
        changed: vec![commitdiff_changed_row("file.txt")],
        diff_url: "/r/diff/mer9e".to_owned(),
        viewer_module_src: "/static/diff-viewer.js".to_owned(),
    };
    world.output = Some(commitdiff_body(&page).into_string());
}

// ---- Blame ------------------------------------------------------------------

#[given(regex = r#"^a blame group of (\d+) lines? by "([^"]*)" dated "([^"]*)"$"#)]
fn given_blame_group(world: &mut RenderWorld, _lines: usize, author: String, date: String) {
    let tooltip: String = format!("{author}, {date}");
    world.blame_group_meta = Some((String::new(), String::new(), tooltip));
}

#[given(regex = r#"^the group commit is "([^"]*)" linking to "([^"]*)"$"#)]
fn given_blame_group_commit(world: &mut RenderWorld, short: String, href: String) {
    let meta: &mut (String, String, String) = world
        .blame_group_meta
        .as_mut()
        .expect("a blame group was declared");
    meta.0 = short;
    meta.1 = href;
}

#[given(regex = r#"^the group line (\d+) is final (\d+) reading "(.*)" linking to "([^"]*)"$"#)]
fn given_blame_group_line(
    world: &mut RenderWorld,
    _index: usize,
    final_lineno: usize,
    text: String,
    linenr_href: String,
) {
    world.blame_group_lines.push(BlameTableLine {
        final_lineno,
        orig_lineno: final_lineno,
        linenr_href,
        text,
    });
}

#[when("I render the blame table")]
fn when_render_blame_table(world: &mut RenderWorld) {
    let (short_commit, commit_href, tooltip): (String, String, String) = world
        .blame_group_meta
        .clone()
        .expect("a blame group was declared");
    let group: BlameTableGroup = BlameTableGroup {
        short_commit,
        commit_href,
        tooltip,
        lines: std::mem::take(&mut world.blame_group_lines),
    };
    world.output = Some(blame_table(&[group]).into_string());
}

#[given(regex = r#"^an incremental blame line (\d+) reading "(.*)"$"#)]
fn given_incremental_line(world: &mut RenderWorld, final_lineno: usize, text: String) {
    world
        .blame_shell_lines
        .push(BlameShellLine { final_lineno, text });
}

#[given(regex = r#"^the blame data url is "([^"]*)"$"#)]
fn given_blame_data_url(world: &mut RenderWorld, url: String) {
    world.blame_data_url = Some(url);
}

#[given("the blame data url has a tab then a space")]
fn given_blame_data_url_control(world: &mut RenderWorld) {
    // A control character (tab, 0x09) must become a `	` escape; the space
    // (0x20) sits just above the guard and must stay a literal space — so the
    // bootstrap never carries a raw control byte yet does not over-escape.
    world.blame_data_url = Some("/p/\t /x".to_owned());
}

#[given(regex = r#"^the blame project url is "([^"]*)"$"#)]
fn given_blame_project_url(world: &mut RenderWorld, url: String) {
    world.blame_project_url = Some(url);
}

#[when("I render the incremental blame shell")]
fn when_render_incremental_shell(world: &mut RenderWorld) {
    let page: BlameIncrementalPage = BlameIncrementalPage {
        crumbs: Vec::new(),
        nav: Vec::new(),
        formats: Vec::new(),
        title: "subject".to_owned(),
        path: "f.txt".to_owned(),
        lines: std::mem::take(&mut world.blame_shell_lines),
        data_url: world.blame_data_url.clone().unwrap_or_default(),
        project_url: world.blame_project_url.clone().unwrap_or_default(),
        module_src: "/static/blame-incremental.js".to_owned(),
    };
    world.output = Some(blame_incremental_body(&page).into_string());
}

#[given(regex = r#"^a blame data group "([^"]*)" orig (\d+) final (\d+) span (\d+)$"#)]
fn given_blame_data_group(
    world: &mut RenderWorld,
    commit: String,
    orig: usize,
    final_lineno: usize,
    span: usize,
) {
    world.blame_data_group = Some(BlameDataGroup {
        commit,
        orig_lineno: orig,
        final_lineno,
        numlines: span,
        author: String::new(),
        author_time: 0,
        author_tz: String::new(),
        filename: String::new(),
    });
}

#[given(regex = r#"^the data group author is "([^"]*)"$"#)]
fn given_data_group_author(world: &mut RenderWorld, author: String) {
    world
        .blame_data_group
        .as_mut()
        .expect("a data group was declared")
        .author = author;
}

#[given(regex = r#"^the data group author time is (\d+) zone "([^"]*)"$"#)]
fn given_data_group_time(world: &mut RenderWorld, time: i64, zone: String) {
    let group: &mut BlameDataGroup = world
        .blame_data_group
        .as_mut()
        .expect("a data group was declared");
    group.author_time = time;
    group.author_tz = zone;
}

#[given(regex = r#"^the data group filename is "([^"]*)"$"#)]
fn given_data_group_filename(world: &mut RenderWorld, filename: String) {
    world
        .blame_data_group
        .as_mut()
        .expect("a data group was declared")
        .filename = filename;
}

#[when("I render the blame data")]
fn when_render_blame_data(world: &mut RenderWorld) {
    let group: BlameDataGroup = world
        .blame_data_group
        .clone()
        .expect("a data group was declared");
    world.output = Some(blame_data(&[group]));
}

#[then("the result ends after a filename line")]
fn then_data_ends_after_filename(world: &mut RenderWorld) {
    let output: &str = world.output.as_ref().expect("the data was rendered");
    let last_record: &str = output.trim_end_matches("END\n").trim_end();
    assert!(
        last_record
            .lines()
            .next_back()
            .is_some_and(|line: &str| line.starts_with("filename ")),
        "expected the last record line to be a filename line, got: {last_record:?}"
    );
}

#[tokio::main]
async fn main() {
    RenderWorld::run("features/render").await;
}
