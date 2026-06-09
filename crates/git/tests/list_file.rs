//! Gherkin conformance for the gix [`ProjectStore`] adapter in file mode.
//!
//! The `Given` lays out repositories under a root and writes a `$projects_list`
//! file (from a docstring) that the store reads instead of scanning. The `When`
//! drives `list` or `info` and stores the raw [`Result`], so the `Then` asserts
//! the listed set or the resolved owner without branching. cucumber supplies its
//! own `main`, so this target sets `harness = false` in Cargo.toml.

use std::path::PathBuf;

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_filter::ProjectFilter;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;

#[derive(Debug, Default, World)]
struct ListFileWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    listed: Option<Result<Vec<Project>, DomainError>>,
    info: Option<Result<ProjectInfo, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

fn root(world: &ListFileWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

fn store(world: &ListFileWorld) -> &GixProjectStore {
    world
        .store
        .as_ref()
        .expect("the projects-list file must be declared first")
}

fn ok_listed(world: &ListFileWorld) -> &[Project] {
    world
        .listed
        .as_ref()
        .expect("list the projects first")
        .as_ref()
        .expect("listing the projects succeeded")
}

fn ok_info(world: &ListFileWorld) -> &ProjectInfo {
    world
        .info
        .as_ref()
        .expect("read the metadata first")
        .as_ref()
        .expect("reading the metadata succeeded")
}

// --- Givens ------------------------------------------------------------------

#[given("a project root")]
fn given_project_root(world: &mut ListFileWorld) {
    world.root = Some(ProjectRoot::new());
}

#[given(regex = r#"^a repository "([^"]*)"$"#)]
fn given_repository(world: &mut ListFileWorld, name: String) {
    root(world).add_repo(&name);
}

#[given(regex = r#"^"([^"]*)" has gitweb config "([^"]*)" set to "(.*)"$"#)]
fn given_gitweb_config(world: &mut ListFileWorld, name: String, key: String, value: String) {
    root(world).set_gitweb_config(&name, &key, &value);
}

#[given("a projects-list file:")]
fn given_projects_list_file(world: &mut ListFileWorld, step: &Step) {
    let body: &str = step
        .docstring
        .as_deref()
        .expect("the projects-list file step needs a docstring");
    let path: PathBuf = root(world).write_projects_list(body);
    let store: GixProjectStore =
        GixProjectStore::from_list_file(root(world).path().to_path_buf(), path);
    world.store = Some(store);
}

// --- Whens -------------------------------------------------------------------

#[when("I list the projects")]
fn list_projects(world: &mut ListFileWorld) {
    world.listed = Some(store(world).list(None));
}

#[when(regex = r#"^I list the projects under "([^"]*)"$"#)]
fn list_projects_filtered(world: &mut ListFileWorld, subdir: String) {
    let filter: ProjectFilter = ProjectFilter::new(subdir);
    world.listed = Some(store(world).list(Some(&filter)));
}

#[when(regex = r#"^I read the metadata of "([^"]*)"$"#)]
fn read_metadata(world: &mut ListFileWorld, name: String) {
    world.info = Some(store(world).info(&name));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) projects? (?:is|are) listed$")]
fn projects_listed(world: &mut ListFileWorld, count: usize) {
    assert_eq!(ok_listed(world).len(), count);
}

#[then(regex = r#"^the project "([^"]*)" is listed$"#)]
fn project_is_listed(world: &mut ListFileWorld, name: String) {
    let found: bool = ok_listed(world)
        .iter()
        .any(|candidate: &Project| candidate.name() == name);
    assert!(found, "expected a project named {name}");
}

#[then(regex = r#"^the project "([^"]*)" is not listed$"#)]
fn project_is_not_listed(world: &mut ListFileWorld, name: String) {
    let found: bool = ok_listed(world)
        .iter()
        .any(|candidate: &Project| candidate.name() == name);
    assert!(!found, "expected no project named {name}");
}

#[then(regex = r#"^the owner is "(.*)"$"#)]
fn owner_is(world: &mut ListFileWorld, expected: String) {
    assert_eq!(ok_info(world).owner(), Some(expected.as_str()));
}

#[tokio::main]
async fn main() {
    ListFileWorld::run("features/list_file").await;
}
