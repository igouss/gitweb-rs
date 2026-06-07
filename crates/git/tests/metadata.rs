//! Gherkin conformance for per-project metadata in the gix [`ProjectStore`]
//! adapter.
//!
//! The `Given` lays out a real repository on disk and writes its metadata
//! sources — a `description`/`category`/`cloneurl` file, or a `gitweb.*` config
//! value. The `When` drives `info` and stores the raw [`Result`], so the `Then`
//! asserts either the resolved metadata or the exact failure mode without
//! branching in the step bodies. cucumber supplies its own `main`, so this
//! target sets `harness = false` in Cargo.toml.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;

#[derive(Debug, Default, World)]
struct MetadataWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    info: Option<Result<ProjectInfo, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

fn ensure_root(world: &mut MetadataWorld) {
    if world.root.is_some() {
        return;
    }
    let root: ProjectRoot = ProjectRoot::new();
    let store: GixProjectStore = GixProjectStore::new(root.path().to_path_buf());
    world.root = Some(root);
    world.store = Some(store);
}

fn root(world: &MetadataWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

fn store(world: &MetadataWorld) -> &GixProjectStore {
    world.store.as_ref().expect("a store must be opened first")
}

fn ok_info(world: &MetadataWorld) -> &ProjectInfo {
    world
        .info
        .as_ref()
        .expect("read the metadata first")
        .as_ref()
        .expect("reading the metadata succeeded")
}

// --- Givens ------------------------------------------------------------------

#[given("an empty project root")]
fn given_empty_root(world: &mut MetadataWorld) {
    ensure_root(world);
}

#[given(regex = r#"^a project root containing repository "([^"]*)"$"#)]
fn given_root_with_repo(world: &mut MetadataWorld, name: String) {
    ensure_root(world);
    root(world).add_repo(&name);
}

#[given(regex = r#"^"([^"]*)" has the description file "(.*)"$"#)]
fn given_description_file(world: &mut MetadataWorld, name: String, text: String) {
    root(world).set_description(&name, &text);
}

#[given(regex = r#"^"([^"]*)" has no description file$"#)]
fn given_no_description_file(world: &mut MetadataWorld, name: String) {
    root(world).remove_description(&name);
}

#[given(regex = r#"^"([^"]*)" has the category file "(.*)"$"#)]
fn given_category_file(world: &mut MetadataWorld, name: String, text: String) {
    root(world).set_category(&name, &text);
}

#[given(regex = r#"^"([^"]*)" has the clone URLs "([^"]*)" and "([^"]*)"$"#)]
fn given_clone_urls(world: &mut MetadataWorld, name: String, first: String, second: String) {
    root(world).set_clone_urls(&name, &[&first, &second]);
}

#[given(regex = r#"^"([^"]*)" has gitweb config "([^"]*)" set to "(.*)"$"#)]
fn given_gitweb_config(world: &mut MetadataWorld, name: String, key: String, value: String) {
    root(world).set_gitweb_config(&name, &key, &value);
}

// --- When --------------------------------------------------------------------

#[when(regex = r#"^I read the metadata of "([^"]*)"$"#)]
fn read_metadata(world: &mut MetadataWorld, name: String) {
    world.info = Some(store(world).info(&name));
}

// --- Thens: resolved metadata ------------------------------------------------

#[then(regex = r#"^the description is "(.*)"$"#)]
fn description_is(world: &mut MetadataWorld, expected: String) {
    assert_eq!(ok_info(world).description(), Some(expected.as_str()));
}

#[then("there is no description")]
fn no_description(world: &mut MetadataWorld) {
    assert_eq!(ok_info(world).description(), None);
}

#[then(regex = r#"^the owner is "(.*)"$"#)]
fn owner_is(world: &mut MetadataWorld, expected: String) {
    assert_eq!(ok_info(world).owner(), Some(expected.as_str()));
}

#[then("there is no owner")]
fn no_owner(world: &mut MetadataWorld) {
    assert_eq!(ok_info(world).owner(), None);
}

#[then(regex = r#"^the category is "(.*)"$"#)]
fn category_is(world: &mut MetadataWorld, expected: String) {
    assert_eq!(ok_info(world).category(), Some(expected.as_str()));
}

#[then(regex = r"^there (?:is|are) (\d+) clone URLs?$")]
fn clone_url_count(world: &mut MetadataWorld, count: usize) {
    assert_eq!(ok_info(world).clone_urls().len(), count);
}

#[then(regex = r#"^the clone URLs include "([^"]*)"$"#)]
fn clone_urls_include(world: &mut MetadataWorld, url: String) {
    let found: bool = ok_info(world)
        .clone_urls()
        .iter()
        .any(|candidate: &String| candidate == &url);
    assert!(found, "expected the clone URLs to include {url}");
}

// --- Thens: failure modes ----------------------------------------------------

#[then("reading metadata fails as not found")]
fn metadata_not_found(world: &mut MetadataWorld) {
    let result: &Result<ProjectInfo, DomainError> =
        world.info.as_ref().expect("read the metadata first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[then("reading metadata fails as invalid")]
fn metadata_invalid(world: &mut MetadataWorld) {
    let result: &Result<ProjectInfo, DomainError> =
        world.info.as_ref().expect("read the metadata first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

#[tokio::main]
async fn main() {
    MetadataWorld::run("features/metadata").await;
}
