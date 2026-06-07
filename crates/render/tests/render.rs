//! Gherkin-driven BDD harness for the render layer.
//!
//! Runs every `.feature` under `features/render`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_domain::model::age::AgeClass;
use gitweb_render::age::age_class_name;
use gitweb_render::chrome::{
    Crumb, DocumentHead, FeedLink, FooterLink, HiddenField, Logo, NavItem, PageFooter, SearchForm,
    SearchOption, breadcrumbs, document, footer, page_header, page_nav, search_form,
};
use gitweb_render::escape::{
    esc_attr, esc_html, esc_html_nbsp, esc_param, esc_path, esc_path_info, esc_url,
};
use gitweb_render::markup::{Markup, html, raw};

#[derive(Debug, Default, World)]
struct RenderWorld {
    input: String,
    age: Option<AgeClass>,
    crumbs: Vec<Crumb>,
    nav_items: Vec<NavItem>,
    footer_links: Vec<FooterLink>,
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
