//! Gherkin-driven BDD harness for the composition root.
//!
//! The `Given` lays out a real project root on disk with the gix fixture builder.
//! The `When` calls the app crate's [`build_router`] — the exact assembly the
//! binary uses — over that root and the built-in settings, then drives the
//! resulting [`Router`] with a single in-process request (no socket bind), so
//! each `Then` asserts one fact about the served response. cucumber supplies its
//! own `main`, so this target sets `harness = false`.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request as HttpRequest, header};
use cucumber::{World, given, then, when};
use tower::ServiceExt;

use gitweb_app::build_router;
use gitweb_domain::model::settings::Settings;
use gitweb_fixtures::ProjectRoot;

#[derive(Debug, Default, World)]
struct AppWorld {
    root: Option<ProjectRoot>,
    response_status: Option<u16>,
    response_content_type: Option<String>,
    response_body: Option<String>,
}

/// The project root, or a panic if a scenario forgot to lay one down first.
fn root(world: &AppWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

// --- Givens ------------------------------------------------------------------

#[given("a project root")]
fn given_project_root(world: &mut AppWorld) {
    world.root = Some(ProjectRoot::new());
}

#[given(regex = r#"^the root contains repository "([^"]*)"$"#)]
fn given_root_contains_repo(world: &mut AppWorld, name: String) {
    root(world).add_repo(&name);
}

// --- When: boot the assembled binary and drive one request -------------------

#[when(regex = r#"^I GET "([^"]*)"$"#)]
async fn when_get(world: &mut AppWorld, uri: String) {
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let app: Router = build_router(root(world).path().to_path_buf(), settings);

    let request: HttpRequest<Body> = HttpRequest::builder()
        .uri(uri)
        .body(Body::empty())
        .expect("the test request builds");
    let response = app.oneshot(request).await.expect("the router responds");

    let status: u16 = response.status().as_u16();
    let content_type: Option<String> = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .map(str::to_owned);
    let bytes: axum::body::Bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the response body collects");

    world.response_status = Some(status);
    world.response_content_type = content_type;
    world.response_body = Some(String::from_utf8_lossy(&bytes).into_owned());
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r#"^the response status is (\d+)$"#)]
fn then_status_is(world: &mut AppWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected));
}

#[then(regex = r#"^the response content type is "([^"]*)"$"#)]
fn then_content_type_is(world: &mut AppWorld, expected: String) {
    assert_eq!(
        world.response_content_type.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the response body contains "(.*)"$"#)]
fn then_body_contains(world: &mut AppWorld, needle: String) {
    let body: &str = world
        .response_body
        .as_deref()
        .expect("a response body must have been captured");
    assert!(
        body.contains(&needle),
        "body did not contain {needle:?}: {body}"
    );
}

#[then(regex = r#"^the response body does not contain "(.*)"$"#)]
fn then_body_excludes(world: &mut AppWorld, needle: String) {
    let body: &str = world
        .response_body
        .as_deref()
        .expect("a response body must have been captured");
    assert!(
        !body.contains(&needle),
        "body unexpectedly contained {needle:?}: {body}"
    );
}

#[tokio::main]
async fn main() {
    AppWorld::run("features/app").await;
}
