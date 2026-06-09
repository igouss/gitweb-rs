//! Gherkin conformance for the gix [`ProjectStore`] adapter.
//!
//! The `Given` lays out a real project root on disk — bare repositories built
//! with gix at various depths, plus the odd plain directory — and opens the
//! adapter over it. The `When` drives `list` or `open` and stores the raw
//! [`Result`], so the `Then` asserts either the discovered set or the exact
//! failure mode without any branching in the step bodies. cucumber supplies its
//! own `main`, so this target sets `harness = false` in Cargo.toml.

use std::fmt;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_filter::ProjectFilter;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;

/// An opened repository, wrapped so the `World` can derive `Debug` over a result
/// whose `Ok` is a trait object that is not itself `Debug`.
struct Opened(Box<dyn Repository>);

impl fmt::Debug for Opened {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Opened(<repository>)")
    }
}

#[derive(Debug, Default, World)]
struct DiscoveryWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    listed: Option<Result<Vec<Project>, DomainError>>,
    opened: Option<Result<Opened, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// Creates the project root and opens the adapter over it, once per scenario.
fn ensure_root(world: &mut DiscoveryWorld) {
    if world.root.is_some() {
        return;
    }
    let root: ProjectRoot = ProjectRoot::new();
    let store: GixProjectStore = GixProjectStore::new(root.path().to_path_buf());
    world.root = Some(root);
    world.store = Some(store);
}

fn root(world: &DiscoveryWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

fn store(world: &DiscoveryWorld) -> &GixProjectStore {
    world.store.as_ref().expect("a store must be opened first")
}

// --- accessors ---------------------------------------------------------------

fn ok_listed(world: &DiscoveryWorld) -> &[Project] {
    world
        .listed
        .as_ref()
        .expect("list the projects first")
        .as_ref()
        .expect("listing the projects succeeded")
}

fn ok_opened(world: &DiscoveryWorld) -> &Opened {
    world
        .opened
        .as_ref()
        .expect("open a project first")
        .as_ref()
        .expect("opening the project succeeded")
}

// --- Givens ------------------------------------------------------------------

#[given("an empty project root")]
fn given_empty_root(world: &mut DiscoveryWorld) {
    ensure_root(world);
}

#[given(regex = r#"^a project root containing repository "([^"]*)"$"#)]
fn given_root_with_repo(world: &mut DiscoveryWorld, name: String) {
    ensure_root(world);
    root(world).add_repo(&name);
}

#[given(regex = r#"^the root also contains repository "([^"]*)"$"#)]
fn given_also_repo(world: &mut DiscoveryWorld, name: String) {
    root(world).add_repo(&name);
}

#[given(regex = r#"^the root also contains a plain directory "([^"]*)"$"#)]
fn given_also_plain_dir(world: &mut DiscoveryWorld, name: String) {
    root(world).add_dir(&name);
}

// --- Whens -------------------------------------------------------------------

#[when("I list the projects")]
fn list_projects(world: &mut DiscoveryWorld) {
    world.listed = Some(store(world).list(None));
}

#[when(regex = r#"^I list the projects under "([^"]*)"$"#)]
fn list_projects_filtered(world: &mut DiscoveryWorld, subdir: String) {
    let filter: ProjectFilter = ProjectFilter::new(subdir);
    world.listed = Some(store(world).list(Some(&filter)));
}

#[when(regex = r#"^I open the project "([^"]*)"$"#)]
fn open_project(world: &mut DiscoveryWorld, name: String) {
    let result: Result<Box<dyn Repository>, DomainError> = store(world).open(&name);
    world.opened = Some(result.map(Opened));
}

// --- Thens: list -------------------------------------------------------------

#[then(regex = r"^(\d+) projects? (?:is|are) listed$")]
fn projects_listed(world: &mut DiscoveryWorld, count: usize) {
    assert_eq!(ok_listed(world).len(), count);
}

#[then(regex = r#"^the project "([^"]*)" is listed$"#)]
fn project_is_listed(world: &mut DiscoveryWorld, name: String) {
    let found: bool = ok_listed(world)
        .iter()
        .any(|candidate: &Project| candidate.name() == name);
    assert!(found, "expected a project named {name}");
}

#[then(regex = r#"^the project "([^"]*)" is not listed$"#)]
fn project_is_not_listed(world: &mut DiscoveryWorld, name: String) {
    let found: bool = ok_listed(world)
        .iter()
        .any(|candidate: &Project| candidate.name() == name);
    assert!(!found, "expected no project named {name}");
}

// --- Thens: open -------------------------------------------------------------

#[then("the project opens successfully")]
fn opens_successfully(world: &mut DiscoveryWorld) {
    let result: &Result<Opened, DomainError> = world.opened.as_ref().expect("open a project first");
    assert!(result.is_ok());
}

#[then(regex = r#"^the opened repository has (\d+) references? under "([^"]*)"$"#)]
fn opened_reference_count(world: &mut DiscoveryWorld, count: usize, prefix: String) {
    let refs: Vec<Reference> = ok_opened(world)
        .0
        .references(&prefix)
        .expect("listing references on the opened repository succeeded");
    assert_eq!(refs.len(), count);
}

#[then("opening fails as not found")]
fn open_not_found(world: &mut DiscoveryWorld) {
    let result: &Result<Opened, DomainError> = world.opened.as_ref().expect("open a project first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[then("opening fails as invalid")]
fn open_invalid(world: &mut DiscoveryWorld) {
    let result: &Result<Opened, DomainError> = world.opened.as_ref().expect("open a project first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

#[tokio::main]
async fn main() {
    DiscoveryWorld::run("features/discovery").await;
}
