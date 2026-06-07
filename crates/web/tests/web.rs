//! Gherkin-driven BDD harness for the web boundary.
//!
//! The `Given` lays out a real project root on disk with the gix fixture builder
//! and wires the gix `ProjectStore` adapter over it, as the composition root
//! would. The `When` drives [`resolve`] with a raw query string and PATH_INFO
//! and stores the raw [`Result`], so each `Then` asserts one fact — a resolved
//! field, or the exact failure mode — without any branching in the step body.
//! cucumber supplies its own `main`, so this target sets `harness = false`.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;
use gitweb_web::request::{ResolvedRequest, resolve};

#[derive(Debug, Default, World)]
struct WebWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    resolved: Option<Result<ResolvedRequest, DomainError>>,
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

#[tokio::main]
async fn main() {
    WebWorld::run("features/web").await;
}
