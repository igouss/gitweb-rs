//! Gherkin-driven BDD harness for the domain use cases.
//!
//! Use cases orchestrate the ports, so the `Given` builds an in-memory fake
//! [`ProjectStore`] (no adapter, no gix) holding hand-written [`ProjectInfo`],
//! the `When` runs the use case against it, and each `Then` asserts one fact
//! about the resulting view-model or the failure. Verifying the orchestration
//! through a fake keeps it fast and isolated; the gix adapter realizes the same
//! port contract under its own conformance specs, and the web layer exercises
//! the two together end-to-end. cucumber supplies its own `main`, so this target
//! sets `harness = false`.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::project_list::{
    ProjectListRow, ProjectListView, assemble_project_list,
};

/// An in-memory [`ProjectStore`] over a fixed set of projects. It serves
/// `list` and `info` from its metadata; `open` is never reached by the
/// project-list use case, so it is left unimplemented.
struct FakeStore {
    projects: Vec<ProjectInfo>,
}

impl ProjectStore for FakeStore {
    fn list(&self) -> Result<Vec<Project>, DomainError> {
        Ok(self
            .projects
            .iter()
            .map(|info: &ProjectInfo| Project::new(info.name().to_owned()))
            .collect())
    }

    fn open(&self, _name: &str) -> Result<Box<dyn Repository>, DomainError> {
        unimplemented!("the project-list use case never opens a repository")
    }

    fn info(&self, name: &str) -> Result<ProjectInfo, DomainError> {
        self.projects
            .iter()
            .find(|info: &&ProjectInfo| info.name() == name)
            .cloned()
            .ok_or_else(|| DomainError::NotFound(format!("no project {name}")))
    }
}

#[derive(Debug, Default, World)]
struct UsecaseWorld {
    projects: Vec<ProjectInfo>,
    now: i64,
    settings: Settings,
    result: Option<Result<ProjectListView, DomainError>>,
}

/// The successful view, or a panic if the scenario produced an error.
fn view(world: &UsecaseWorld) -> &ProjectListView {
    world
        .result
        .as_ref()
        .expect("assemble the list first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The failure, or a panic if the scenario produced a success.
fn error(world: &UsecaseWorld) -> &DomainError {
    match world.result.as_ref().expect("assemble the list first") {
        Ok(_) => panic!("expected assembly to fail"),
        Err(failure) => failure,
    }
}

/// The assembled row for `name`, or a panic if it is absent.
fn row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a ProjectListRow {
    view(world)
        .rows()
        .iter()
        .find(|row: &&ProjectListRow| row.name() == name)
        .unwrap_or_else(|| panic!("no row for {name}"))
}

// --- Givens ------------------------------------------------------------------

#[given(regex = r#"^the store has project "([^"]*)"$"#)]
fn store_has_project(world: &mut UsecaseWorld, name: String) {
    world.projects.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the store has project "([^"]*)" last changed at (\d+)$"#)]
fn store_has_aged_project(world: &mut UsecaseWorld, name: String, epoch: i64) {
    world
        .projects
        .push(ProjectInfo::named(name).with_last_activity(epoch));
}

#[given(regex = r#"^the store has project "([^"]*)" with no commits$"#)]
fn store_has_uncommitted_project(world: &mut UsecaseWorld, name: String) {
    world.projects.push(ProjectInfo::named(name));
}

#[given(regex = r#"^the store has project "([^"]*)" described as "(.*)" owned by "([^"]*)"$"#)]
fn store_has_described_project(
    world: &mut UsecaseWorld,
    name: String,
    description: String,
    owner: String,
) {
    world.projects.push(
        ProjectInfo::named(name)
            .with_description(description)
            .with_owner(owner),
    );
}

#[given(regex = r"^the current time is (\d+)$")]
fn current_time_is(world: &mut UsecaseWorld, now: i64) {
    world.now = now;
}

// --- Whens -------------------------------------------------------------------

#[when("I assemble the project list")]
fn assemble_default(world: &mut UsecaseWorld) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
    };
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        None,
        world.now,
    ));
}

#[when(regex = r#"^I assemble the project list ordered by "([^"]*)"$"#)]
fn assemble_ordered(world: &mut UsecaseWorld, order: String) {
    let store: FakeStore = FakeStore {
        projects: world.projects.clone(),
    };
    world.result = Some(assemble_project_list(
        &store,
        &world.settings,
        Some(&order),
        world.now,
    ));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r#"^the listed projects are "(.*)"$"#)]
fn listed_projects_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = view(world)
        .rows()
        .iter()
        .map(|row: &ProjectListRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then("assembling fails as not found")]
fn fails_not_found(world: &mut UsecaseWorld) {
    assert!(matches!(error(world), DomainError::NotFound(_)));
}

#[then("assembling fails as invalid")]
fn fails_invalid(world: &mut UsecaseWorld) {
    assert!(matches!(error(world), DomainError::Invalid(_)));
}

#[then(regex = r#"^the project "([^"]*)" shows description "(.*)"$"#)]
fn project_shows_description(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(row(world, &name).description(), Some(expected.as_str()));
}

#[then(regex = r#"^the project "([^"]*)" shows owner "([^"]*)"$"#)]
fn project_shows_owner(world: &mut UsecaseWorld, name: String, expected: String) {
    assert_eq!(row(world, &name).owner(), Some(expected.as_str()));
}

#[then(regex = r#"^the project "([^"]*)" shows the age "([^"]*)"$"#)]
fn project_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = row(world, &name)
        .age()
        .expect("the project has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the project "([^"]*)" has no age$"#)]
fn project_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(row(world, &name).age(), None);
}

#[tokio::main]
async fn main() {
    UsecaseWorld::run("features/usecase").await;
}
