//! Gherkin-driven BDD harness for the render layer.
//!
//! Runs every `.feature` under `features/render`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_domain::error::DomainError;
use gitweb_domain::model::age::{Age, AgeClass};
use gitweb_render::age::age_class_name;
use gitweb_render::chrome::{
    Crumb, DocumentHead, FeedLink, FooterLink, HiddenField, Logo, NavItem, PageFooter, SearchForm,
    SearchOption, breadcrumbs, document, footer, page_header, page_nav, search_form,
};
use gitweb_render::error::{ErrorResponse, HttpStatus, error_page, error_response};
use gitweb_render::escape::{
    esc_attr, esc_html, esc_html_nbsp, esc_param, esc_path, esc_path_info, esc_url,
};
use gitweb_render::markup::{Markup, html, raw};
use gitweb_render::project_list::{
    ProjectLinks, ProjectList, ProjectRow, SortHeader, project_list,
};

#[derive(Debug, Default, World)]
struct RenderWorld {
    input: String,
    age: Option<AgeClass>,
    crumbs: Vec<Crumb>,
    nav_items: Vec<NavItem>,
    footer_links: Vec<FooterLink>,
    project_rows: Vec<ProjectRow>,
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
