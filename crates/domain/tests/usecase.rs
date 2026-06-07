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
use gitweb_domain::model::blame::Blame;
use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::diff::{CombinedDiff, Diff};
use gitweb_domain::model::grep::GrepResults;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::patch::Patch;
use gitweb_domain::model::project::Project;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::ref_name::RefName;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::model::signature::Signature;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tree::Tree;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::{
    ArchiveFormat, Page, RenameDetection, Repository, SearchQuery,
};
use gitweb_domain::usecase::heads::{HeadRow, HeadsView, assemble_heads};
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
    head: Option<String>,
    branches: Vec<FakeBranch>,
    heads_result: Option<Result<HeadsView, DomainError>>,
}

/// One branch in the fake repository: its short name, the id of its tip commit,
/// and that commit's committer epoch.
#[derive(Debug, Clone)]
struct FakeBranch {
    name: String,
    tip: ObjectId,
    epoch: i64,
}

/// A deterministic 40-hex object id derived from `seed`, so distinct branch
/// names get distinct tips while a test can still alias two branches onto one
/// commit by deriving both from the same seed.
fn fake_oid(seed: &str) -> ObjectId {
    let mut hex: String = seed.bytes().map(|byte: u8| format!("{byte:02x}")).collect();
    hex.truncate(40);
    while hex.len() < 40 {
        hex.push('0');
    }
    ObjectId::parse(&hex).expect("a 40-character hex object id")
}

/// An in-memory [`Repository`] over a fixed set of branches. It serves `head`,
/// `references` and `find_commit` — all the heads use case reads — and leaves
/// every other port method unimplemented, since the use case never reaches them.
struct FakeRepository {
    head: Option<String>,
    branches: Vec<FakeBranch>,
}

impl FakeRepository {
    fn branch_ref(branch: &FakeBranch) -> Reference {
        Reference::new(
            RefName::new(format!("refs/heads/{}", branch.name)),
            branch.tip.clone(),
        )
    }
}

impl Repository for FakeRepository {
    fn head(&self) -> Result<Reference, DomainError> {
        let name: &String = self
            .head
            .as_ref()
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?;
        let branch: &FakeBranch = self
            .branches
            .iter()
            .find(|branch: &&FakeBranch| &branch.name == name)
            .ok_or_else(|| DomainError::NotFound("HEAD".to_owned()))?;
        Ok(Self::branch_ref(branch))
    }

    fn references(&self, prefix: &str) -> Result<Vec<Reference>, DomainError> {
        Ok(self
            .branches
            .iter()
            .map(Self::branch_ref)
            .filter(|reference: &Reference| reference.name().full().starts_with(prefix))
            .collect())
    }

    fn find_commit(&self, oid: &ObjectId) -> Result<Commit, DomainError> {
        let branch: &FakeBranch = self
            .branches
            .iter()
            .find(|branch: &&FakeBranch| &branch.tip == oid)
            .ok_or_else(|| DomainError::NotFound(oid.as_str().to_owned()))?;
        let who: Signature =
            Signature::parse(&format!("Tester <t@example.com> {} +0000", branch.epoch))
                .expect("a valid fixture signature");
        Ok(Commit::new(
            oid.clone(),
            fake_oid("tree"),
            Vec::new(),
            who.clone(),
            who,
            "msg\n".to_owned(),
        ))
    }

    fn resolve(&self, _rev: &str) -> Result<ObjectId, DomainError> {
        unimplemented!("the heads use case never resolves a revision")
    }

    fn object_kind(&self, _oid: &ObjectId) -> Result<ObjectKind, DomainError> {
        unimplemented!("the heads use case never reads an object kind")
    }

    fn find_tree(&self, _oid: &ObjectId) -> Result<Tree, DomainError> {
        unimplemented!("the heads use case never reads a tree")
    }

    fn find_blob(&self, _oid: &ObjectId) -> Result<Blob, DomainError> {
        unimplemented!("the heads use case never reads a blob")
    }

    fn find_tag(&self, _oid: &ObjectId) -> Result<Tag, DomainError> {
        unimplemented!("the heads use case never reads a tag")
    }

    fn history(
        &self,
        _start: &ObjectId,
        _path: Option<&str>,
        _page: Page,
    ) -> Result<Vec<Commit>, DomainError> {
        unimplemented!("the heads use case never walks history")
    }

    fn diff(
        &self,
        _from: Option<&ObjectId>,
        _to: &ObjectId,
        _detection: RenameDetection,
    ) -> Result<Diff, DomainError> {
        unimplemented!("the heads use case never diffs")
    }

    fn patch(
        &self,
        _from: Option<&ObjectId>,
        _to: &ObjectId,
        _detection: RenameDetection,
    ) -> Result<Patch, DomainError> {
        unimplemented!("the heads use case never builds a patch")
    }

    fn combined_diff(&self, _commit: &ObjectId) -> Result<CombinedDiff, DomainError> {
        unimplemented!("the heads use case never builds a combined diff")
    }

    fn blame(&self, _at: &ObjectId, _path: &str) -> Result<Blame, DomainError> {
        unimplemented!("the heads use case never blames")
    }

    fn archive(&self, _tree: &ObjectId, _format: ArchiveFormat) -> Result<Vec<u8>, DomainError> {
        unimplemented!("the heads use case never archives")
    }

    fn search(&self, _query: &SearchQuery, _page: Page) -> Result<Vec<Commit>, DomainError> {
        unimplemented!("the heads use case never searches")
    }

    fn grep(&self, _revision: &ObjectId, _pattern: &str) -> Result<GrepResults, DomainError> {
        unimplemented!("the heads use case never greps")
    }
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

// --- heads: accessors --------------------------------------------------------

/// The assembled heads view, or a panic if the scenario produced an error.
fn heads_view(world: &UsecaseWorld) -> &HeadsView {
    world
        .heads_result
        .as_ref()
        .expect("assemble the heads first")
        .as_ref()
        .expect("assembly succeeded")
}

/// The assembled head row for `name`, or a panic if it is absent.
fn head_row<'a>(world: &'a UsecaseWorld, name: &str) -> &'a HeadRow {
    heads_view(world)
        .rows()
        .iter()
        .find(|row: &&HeadRow| row.name() == name)
        .unwrap_or_else(|| panic!("no head row for {name}"))
}

/// The committer epoch of an already-declared branch, for aliasing a second
/// branch onto the same commit.
fn branch_epoch(world: &UsecaseWorld, name: &str) -> i64 {
    world
        .branches
        .iter()
        .find(|branch: &&FakeBranch| branch.name == name)
        .unwrap_or_else(|| panic!("no branch {name} declared yet"))
        .epoch
}

// --- heads: Givens -----------------------------------------------------------

#[given(regex = r#"^the repository HEAD is branch "([^"]*)"$"#)]
fn head_is_branch(world: &mut UsecaseWorld, name: String) {
    world.head = Some(name);
}

#[given(regex = r#"^the repository HEAD is the unborn branch "([^"]*)"$"#)]
fn head_is_unborn(world: &mut UsecaseWorld, name: String) {
    world.head = Some(name);
}

#[given(regex = r#"^the repository has branch "([^"]*)" committed at (\d+)$"#)]
fn repo_has_branch(world: &mut UsecaseWorld, name: String, epoch: i64) {
    let tip: ObjectId = fake_oid(&name);
    world.branches.push(FakeBranch { name, tip, epoch });
}

#[given(regex = r#"^the repository has branch "([^"]*)" at the same commit as "([^"]*)"$"#)]
fn repo_has_aliased_branch(world: &mut UsecaseWorld, name: String, other: String) {
    let tip: ObjectId = fake_oid(&other);
    let epoch: i64 = branch_epoch(world, &other);
    world.branches.push(FakeBranch { name, tip, epoch });
}

// --- heads: When -------------------------------------------------------------

#[when("I assemble the heads")]
fn assemble_the_heads(world: &mut UsecaseWorld) {
    let repo: FakeRepository = FakeRepository {
        head: world.head.clone(),
        branches: world.branches.clone(),
    };
    world.heads_result = Some(assemble_heads(&repo, world.now));
}

// --- heads: Thens ------------------------------------------------------------

#[then("no heads are listed")]
fn no_heads_listed(world: &mut UsecaseWorld) {
    assert!(heads_view(world).rows().is_empty());
}

#[then(regex = r#"^the listed heads are "(.*)"$"#)]
fn listed_heads_are(world: &mut UsecaseWorld, expected: String) {
    let names: Vec<&str> = heads_view(world)
        .rows()
        .iter()
        .map(|row: &HeadRow| row.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the head "([^"]*)" is current$"#)]
fn head_is_current(world: &mut UsecaseWorld, name: String) {
    assert!(head_row(world, &name).current());
}

#[then(regex = r#"^the head "([^"]*)" is not current$"#)]
fn head_is_not_current(world: &mut UsecaseWorld, name: String) {
    assert!(!head_row(world, &name).current());
}

#[then(regex = r#"^the head "([^"]*)" shows the age "([^"]*)"$"#)]
fn head_shows_age(world: &mut UsecaseWorld, name: String, expected: String) {
    let humanized: String = head_row(world, &name)
        .age()
        .expect("the head has an age")
        .humanized();
    assert_eq!(humanized, expected);
}

#[then(regex = r#"^the head "([^"]*)" has no age$"#)]
fn head_has_no_age(world: &mut UsecaseWorld, name: String) {
    assert_eq!(head_row(world, &name).age(), None);
}

#[tokio::main]
async fn main() {
    UsecaseWorld::run("features/usecase").await;
}
