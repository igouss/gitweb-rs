//! Gherkin-driven BDD harness for the render layer.
//!
//! Runs every `.feature` under `features/render`. cucumber supplies its own
//! `main`, so this test target sets `harness = false` in Cargo.toml.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_render::escape::{
    esc_attr, esc_html, esc_html_nbsp, esc_param, esc_path, esc_path_info, esc_url,
};

#[derive(Debug, Default, World)]
struct RenderWorld {
    input: String,
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

// ---- Then: assert the rendered result ---------------------------------------

#[then(regex = r#"^the result is "(.*)"$"#)]
fn then_result_is(world: &mut RenderWorld, expected: String) {
    assert_eq!(world.output.as_deref(), Some(expected.as_str()));
}

#[then("the result is a single tab character")]
fn then_result_is_tab(world: &mut RenderWorld) {
    assert_eq!(world.output.as_deref(), Some("\t"));
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
