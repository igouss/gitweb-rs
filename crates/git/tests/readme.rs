//! Gherkin conformance for reading a project's `README.html` through the gix
//! [`ProjectStore`] adapter.
//!
//! The `Given` lays out a real repository on disk and, where a scenario needs
//! one, writes its `README.html`. The `When` drives `readme_html` and stores the
//! raw [`Result`], so each `Then` asserts either the returned contents, its
//! absence, or the exact failure mode without branching in the step bodies.
//! README.html is a deliberate raw-HTML sink, not list metadata, so it gets its
//! own focused harness. cucumber supplies its own `main`, so this target sets
//! `harness = false` in Cargo.toml.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;

#[derive(Debug, Default, World)]
struct ReadmeWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    readme: Option<Result<Option<String>, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

fn ensure_root(world: &mut ReadmeWorld) {
    if world.root.is_some() {
        return;
    }
    let root: ProjectRoot = ProjectRoot::new();
    let store: GixProjectStore = GixProjectStore::new(root.path().to_path_buf());
    world.root = Some(root);
    world.store = Some(store);
}

fn root(world: &ReadmeWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

fn store(world: &ReadmeWorld) -> &GixProjectStore {
    world.store.as_ref().expect("a store must be opened first")
}

fn ok_readme(world: &ReadmeWorld) -> Option<&str> {
    world
        .readme
        .as_ref()
        .expect("read the README first")
        .as_ref()
        .expect("reading the README succeeded")
        .as_deref()
}

// --- Givens ------------------------------------------------------------------

#[given("an empty project root")]
fn given_empty_root(world: &mut ReadmeWorld) {
    ensure_root(world);
}

#[given(regex = r#"^a project root containing repository "([^"]*)"$"#)]
fn given_root_with_repo(world: &mut ReadmeWorld, name: String) {
    ensure_root(world);
    root(world).add_repo(&name);
}

#[given(regex = r#"^"([^"]*)" has the README.html "(.*)"$"#)]
fn given_readme(world: &mut ReadmeWorld, name: String, contents: String) {
    root(world).set_readme_html(&name, &contents);
}

// --- When --------------------------------------------------------------------

#[when(regex = r#"^I read the README of "([^"]*)"$"#)]
fn read_readme(world: &mut ReadmeWorld, name: String) {
    world.readme = Some(store(world).readme_html(&name));
}

// --- Thens: the read result --------------------------------------------------

#[then(regex = r#"^the README is "(.*)"$"#)]
fn readme_is(world: &mut ReadmeWorld, expected: String) {
    assert_eq!(ok_readme(world), Some(expected.as_str()));
}

#[then("there is no README")]
fn no_readme(world: &mut ReadmeWorld) {
    assert_eq!(ok_readme(world), None);
}

#[then("reading the README fails as not found")]
fn readme_not_found(world: &mut ReadmeWorld) {
    let result: &Result<Option<String>, DomainError> =
        world.readme.as_ref().expect("read the README first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[then("reading the README fails as invalid")]
fn readme_invalid(world: &mut ReadmeWorld) {
    let result: &Result<Option<String>, DomainError> =
        world.readme.as_ref().expect("read the README first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

#[tokio::main]
async fn main() {
    ReadmeWorld::run("features/readme").await;
}
