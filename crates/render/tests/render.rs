//! Gherkin-driven BDD harness for the render layer.
//!
//! Runs every `.feature` under `features/render`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_domain::error::DomainError;
use gitweb_domain::model::age::{Age, AgeClass};
use gitweb_domain::model::timestamp::Timestamp;
use gitweb_render::age::age_class_name;
use gitweb_render::chrome::{
    Crumb, DocumentHead, FeedLink, FooterLink, HiddenField, Logo, NavItem, PageFooter, SearchForm,
    SearchOption, breadcrumbs, document, footer, page_header, page_nav, search_form,
};
use gitweb_render::error::{ErrorResponse, HttpStatus, error_page, error_response};
use gitweb_render::escape::{
    esc_attr, esc_html, esc_html_nbsp, esc_param, esc_path, esc_path_info, esc_url,
};
use gitweb_render::heads::{HeadEntryView, HeadsTable, heads_table};
use gitweb_render::markup::{Markup, html, raw};
use gitweb_render::project_list::{
    ProjectLinks, ProjectList, ProjectRow, SortHeader, project_list,
};
use gitweb_render::tag::{TagAuthorView, TagPage, TaggedObjectView, tag_body};
use gitweb_render::tags::{TagEntryView, TagReftype, TagsPage, TagsTable, tags_body, tags_table};

#[derive(Debug, Default, World)]
struct RenderWorld {
    input: String,
    age: Option<AgeClass>,
    crumbs: Vec<Crumb>,
    nav_items: Vec<NavItem>,
    footer_links: Vec<FooterLink>,
    project_rows: Vec<ProjectRow>,
    head_entries: Vec<HeadEntryView>,
    tag_entries: Vec<TagEntryView>,
    tag_page: Option<TagPage>,
    domain_error: Option<DomainError>,
    status: Option<HttpStatus>,
    output: Option<String>,
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

#[when(regex = r#"^I render a document titled "(.*)" with stylesheet "(.*)" and body "(.*)"$"#)]
fn when_render_document(world: &mut RenderWorld, title: String, stylesheet: String, body: String) {
    let head: DocumentHead = DocumentHead {
        title,
        stylesheet_href: stylesheet,
        favicon_href: None,
        feeds: Vec::new(),
    };
    world.output = Some(document(&head, raw(body)).into_string());
}

#[when(regex = r#"^I render a document with a favicon "(.*)"$"#)]
fn when_render_document_favicon(world: &mut RenderWorld, favicon: String) {
    let head: DocumentHead = DocumentHead {
        title: "x".to_owned(),
        stylesheet_href: "/s".to_owned(),
        favicon_href: Some(favicon),
        feeds: Vec::new(),
    };
    world.output = Some(document(&head, raw("")).into_string());
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
    };
    world.output = Some(document(&head, raw("")).into_string());
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
        name,
        href,
        description: None,
        owner: None,
        age: None,
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
        name,
        href,
        description: Some(description),
        owner: Some(owner),
        age: Some(Age::from_seconds(age_seconds)),
    });
}

#[given(regex = r#"^a listed project "([^"]*)" at "([^"]*)" with no commits$"#)]
fn given_listed_project_no_commits(world: &mut RenderWorld, name: String, href: String) {
    world.project_rows.push(ProjectRow {
        links: project_links(&href),
        name,
        href,
        description: None,
        owner: None,
        age: None,
    });
}

#[when(regex = r#"^I render the project list sorted by "([^"]*)"$"#)]
fn when_render_project_list(world: &mut RenderWorld, order: String) {
    let list: ProjectList = ProjectList {
        project_header: sort_header("Project", "project", &order),
        description_header: sort_header("Description", "descr", &order),
        owner_header: sort_header("Owner", "owner", &order),
        age_header: sort_header("Last Change", "age", &order),
        rows: std::mem::take(&mut world.project_rows),
    };
    world.output = Some(project_list(&list).into_string());
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
    };
    world.output = Some(heads_table(&table).into_string());
}

// ---- Tags table -------------------------------------------------------------

/// Builds a tag row from a base href: the name links to `base` (the tagged
/// object), the single `tag` view hangs off it, and the reftype-dependent links
/// hang off it too, so one base yields distinct, labelled links.
fn tag_entry(
    name: &str,
    base: &str,
    subject: Option<String>,
    age: Option<Age>,
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
        Some(Age::from_seconds(age_seconds)),
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
        None,
        commit_reftype(&base),
    ));
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
        Some(Age::from_seconds(age_seconds)),
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
        Some(Age::from_seconds(age_seconds)),
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
        Some(Age::from_seconds(age_seconds)),
        TagReftype::Tree,
    ));
}

#[when("I render the tags table")]
fn when_render_tags_table(world: &mut RenderWorld) {
    let table: TagsTable = TagsTable {
        rows: std::mem::take(&mut world.tag_entries),
    };
    world.output = Some(tags_table(&table).into_string());
}

#[when("I render the tags page with no tags")]
fn when_render_tags_page_empty(world: &mut RenderWorld) {
    let page: TagsPage = TagsPage {
        crumbs: Vec::new(),
        ref_views: Vec::new(),
        table: TagsTable { rows: Vec::new() },
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

#[tokio::main]
async fn main() {
    RenderWorld::run("features/render").await;
}
