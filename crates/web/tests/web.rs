//! Gherkin-driven BDD harness for the web boundary.
//!
//! The `Given` lays out a real project root on disk with the gix fixture builder
//! and wires the gix `ProjectStore` adapter over it, as the composition root
//! would. The `When` drives [`resolve`] with a raw query string and PATH_INFO
//! and stores the raw [`Result`], so each `Then` asserts one fact — a resolved
//! field, or the exact failure mode — without any branching in the step body.
//! cucumber supplies its own `main`, so this target sets `harness = false`.

use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Request as HttpRequest, header};
use cucumber::{World, given, then, when};
use tower::ServiceExt;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;
use gitweb_web::handlers::{
    HeadsHandler, ProjectListHandler, ShortlogHandler, TagHandler, TagsHandler,
};
use gitweb_web::request::{ResolvedRequest, resolve};
use gitweb_web::response::View;
use gitweb_web::{Dispatcher, Handler, router};

#[derive(Debug, Default, World)]
struct WebWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    resolved: Option<Result<ResolvedRequest, DomainError>>,
    dispatcher: Dispatcher,
    response_status: Option<u16>,
    response_content_type: Option<String>,
    response_body: Option<String>,
}

// --- dispatch fixtures -------------------------------------------------------

/// A stand-in for a capability bead's handler: it ignores the request and
/// returns a fixed view, so the harness can prove the dispatcher routed to it.
struct StubHandler {
    content_type: &'static str,
    body: String,
}

impl Handler for StubHandler {
    fn handle(&self, _request: &Request) -> Result<View, DomainError> {
        Ok(View::text(self.content_type, self.body.clone()))
    }
}

/// Registers a stub handler for `action_name` that serves `body` as `content_type`.
fn register_stub(
    world: &mut WebWorld,
    action_name: &str,
    content_type: &'static str,
    body: String,
) {
    let action: Action = Action::parse(action_name).expect("the stub names a valid action");
    let handler: Arc<dyn Handler> = Arc::new(StubHandler { content_type, body });
    world.dispatcher.register(action, handler);
}

// --- fixture construction ----------------------------------------------------

/// Creates the project root and wires the adapter over it, once per scenario.
fn ensure_root(world: &mut WebWorld) {
    if world.root.is_some() {
        return;
    }
    let root: ProjectRoot = ProjectRoot::new();
    let store: GixProjectStore = GixProjectStore::new(root.path().to_path_buf());
    world.root = Some(root);
    world.store = Some(store);
}

fn root(world: &WebWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

fn store(world: &WebWorld) -> &GixProjectStore {
    world.store.as_ref().expect("a store must be wired first")
}

/// The successful resolution, or a panic if the scenario produced an error.
fn ok(world: &WebWorld) -> &ResolvedRequest {
    world
        .resolved
        .as_ref()
        .expect("resolve the request first")
        .as_ref()
        .expect("resolution succeeded")
}

/// The failure, or a panic if the scenario produced a success.
fn err(world: &WebWorld) -> &DomainError {
    match world.resolved.as_ref().expect("resolve the request first") {
        Ok(_) => panic!("expected resolution to fail"),
        Err(error) => error,
    }
}

/// Splits a raw query string ("k=v&k=v") into gitweb's short-name pairs.
fn parse_query(query: &str) -> Vec<(String, String)> {
    if query.is_empty() {
        return Vec::new();
    }
    query
        .split('&')
        .map(|pair: &str| match pair.split_once('=') {
            Some((key, value)) => (key.to_owned(), value.to_owned()),
            None => (pair.to_owned(), String::new()),
        })
        .collect()
}

// --- Givens ------------------------------------------------------------------

#[given("an empty project root")]
fn given_empty_root(world: &mut WebWorld) {
    ensure_root(world);
}

#[given(regex = r#"^a project root containing repository "([^"]*)"$"#)]
fn given_root_with_repo(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_repo(&name);
}

#[given(regex = r#"^the root also contains repository "([^"]*)"$"#)]
fn given_also_repo(world: &mut WebWorld, name: String) {
    root(world).add_repo(&name);
}

#[given(regex = r#"^a repository "([^"]*)" with an unborn HEAD$"#)]
fn given_unborn_repo(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_empty_repo(&name);
}

#[given(regex = r#"^the repository "([^"]*)" has branch "([^"]*)" committed at (\d+)$"#)]
fn given_repo_branch(world: &mut WebWorld, name: String, branch: String, epoch: i64) {
    root(world).add_branch_at(&name, &branch, epoch);
}

#[given(
    regex = r#"^the repository "([^"]*)" has an annotated tag "([^"]*)" of a commit at (\d+) with subject "(.*)"$"#
)]
fn given_repo_annotated_tag(
    world: &mut WebWorld,
    name: String,
    tag: String,
    epoch: i64,
    subject: String,
) {
    root(world).add_annotated_tag_at(&name, &tag, epoch, &subject);
}

#[given(
    regex = r#"^the repository "([^"]*)" has a lightweight tag "([^"]*)" of a commit at (\d+)$"#
)]
fn given_repo_lightweight_tag(world: &mut WebWorld, name: String, tag: String, epoch: i64) {
    root(world).add_tag_at(&name, &tag, epoch);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r#"^I resolve the path "(.*)" with no query$"#)]
fn resolve_path_no_query(world: &mut WebWorld, path: String) {
    world.resolved = Some(resolve(store(world), &[], &path));
}

#[when(regex = r#"^I resolve the path "(.*)" with the query "(.*)"$"#)]
fn resolve_path_with_query(world: &mut WebWorld, path: String, query: String) {
    let params: Vec<(String, String)> = parse_query(&query);
    world.resolved = Some(resolve(store(world), &params, &path));
}

#[when(regex = r#"^I resolve the query "(.*)"$"#)]
fn resolve_query_only(world: &mut WebWorld, query: String) {
    let params: Vec<(String, String)> = parse_query(&query);
    world.resolved = Some(resolve(store(world), &params, ""));
}

// --- Thens: resolved project / action ----------------------------------------

#[then(regex = r#"^the resolved project is "([^"]*)"$"#)]
fn then_project_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.project.as_deref(),
        Some(expected.as_str())
    );
}

#[then("no project is resolved")]
fn then_no_project(world: &mut WebWorld) {
    assert_eq!(ok(world).request.project, None);
}

#[then(regex = r#"^the resolved action is "([^"]*)"$"#)]
fn then_action_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.action.map(Action::as_str),
        Some(expected.as_str())
    );
}

#[then("no action is resolved")]
fn then_no_action(world: &mut WebWorld) {
    assert_eq!(ok(world).request.action, None);
}

// --- Thens: resolved refs and paths ------------------------------------------

#[then(regex = r#"^the resolved hash is "([^"]*)"$"#)]
fn then_hash_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.hash.as_ref().map(SafeRef::as_str),
        Some(expected.as_str())
    );
}

#[then("no hash is resolved")]
fn then_no_hash(world: &mut WebWorld) {
    assert_eq!(ok(world).request.hash, None);
}

#[then(regex = r#"^the resolved hash base is "([^"]*)"$"#)]
fn then_hash_base_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.hash_base.as_ref().map(SafeRef::as_str),
        Some(expected.as_str())
    );
}

#[then("no hash base is resolved")]
fn then_no_hash_base(world: &mut WebWorld) {
    assert_eq!(ok(world).request.hash_base, None);
}

#[then(regex = r#"^the resolved file is "([^"]*)"$"#)]
fn then_file_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.file_name.as_ref().map(SafePath::as_str),
        Some(expected.as_str())
    );
}

// --- Thens: view preferences -------------------------------------------------

#[then(regex = r#"^the diff style is "([^"]*)"$"#)]
fn then_diff_style_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).view.diff_style.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the content tag is "([^"]*)"$"#)]
fn then_content_tag_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).view.content_tag.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the javascript marker is "([^"]*)"$"#)]
fn then_javascript_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).view.javascript.as_deref(),
        Some(expected.as_str())
    );
}

// --- Thens: failure modes ----------------------------------------------------

#[then("resolution fails as invalid")]
fn then_fails_invalid(world: &mut WebWorld) {
    assert!(matches!(err(world), DomainError::Invalid(_)));
}

#[then("resolution fails as not found")]
fn then_fails_not_found(world: &mut WebWorld) {
    assert!(matches!(err(world), DomainError::NotFound(_)));
}

#[then("resolution fails as forbidden")]
fn then_fails_forbidden(world: &mut WebWorld) {
    assert!(matches!(err(world), DomainError::Forbidden(_)));
}

// --- Givens: registering stub handlers ---------------------------------------

#[given(regex = r#"^a stub "([^"]*)" page handler$"#)]
fn given_stub_page_handler(world: &mut WebWorld, action_name: String) {
    let body: String = format!("<p>STUB:{action_name}</p>");
    register_stub(world, &action_name, "text/html; charset=utf-8", body);
}

#[given(regex = r#"^a stub "([^"]*)" plain-text handler$"#)]
fn given_stub_plain_handler(world: &mut WebWorld, action_name: String) {
    let body: String = format!("STUB:{action_name}");
    register_stub(world, &action_name, "text/plain; charset=utf-8", body);
}

// --- Given: registering real capability handlers -----------------------------

#[given("the project-list landing page is served")]
fn given_project_list_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(ProjectListHandler::new(store, settings));
    world.dispatcher.register(Action::ProjectList, handler);
}

#[given("the heads action is served")]
fn given_heads_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(HeadsHandler::new(store, settings));
    world.dispatcher.register(Action::Heads, handler);
}

#[given("the shortlog action is served")]
fn given_shortlog_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(ShortlogHandler::new(store, settings));
    world.dispatcher.register(Action::Shortlog, handler);
}

#[given("the tags action is served")]
fn given_tags_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(TagsHandler::new(store, settings));
    world.dispatcher.register(Action::Tags, handler);
}

#[given("the tag action is served")]
fn given_tag_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(TagHandler::new(store, settings));
    world.dispatcher.register(Action::Tag, handler);
}

// --- When: drive the assembled router with one in-process request ------------

#[when(regex = r#"^I GET "([^"]*)"$"#)]
async fn when_get(world: &mut WebWorld, uri: String) {
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let dispatcher: Arc<Dispatcher> = Arc::new(world.dispatcher.clone());
    let app: Router = router(store, dispatcher);

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

// --- Thens: the served response ----------------------------------------------

#[then(regex = r#"^the response status is (\d+)$"#)]
fn then_response_status_is(world: &mut WebWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected));
}

#[then(regex = r#"^the response content type is "([^"]*)"$"#)]
fn then_response_content_type_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        world.response_content_type.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the response body contains "([^"]*)"$"#)]
fn then_response_body_contains(world: &mut WebWorld, needle: String) {
    let body: &str = world
        .response_body
        .as_deref()
        .expect("a response body must have been captured");
    assert!(
        body.contains(&needle),
        "body did not contain {needle:?}: {body}"
    );
}

#[tokio::main]
async fn main() {
    WebWorld::run("features/web").await;
}
