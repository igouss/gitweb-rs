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
use gitweb_git::{GixProjectStore, UserDirectory};

#[derive(Debug, Default, World)]
struct MetadataWorld {
    root: Option<ProjectRoot>,
    /// The name the pinned user directory returns for the repository
    /// directory's owning uid, exercising gitweb's `get_file_owner` fallback.
    /// Absent means the host has no passwd entry for the uid.
    dir_owner: Option<String>,
    info: Option<Result<ProjectInfo, DomainError>>,
}

/// A pinned [`UserDirectory`]: every uid resolves to the same configured name
/// (or to none), so the filesystem owner fallback is deterministic in a spec
/// instead of depending on the host's passwd database.
#[derive(Debug, Default)]
struct FakeUserDirectory {
    name: Option<String>,
}

impl UserDirectory for FakeUserDirectory {
    fn display_name(&self, _uid: u32) -> Option<String> {
        self.name.clone()
    }
}

// --- fixture construction ----------------------------------------------------

fn ensure_root(world: &mut MetadataWorld) {
    if world.root.is_some() {
        return;
    }
    world.root = Some(ProjectRoot::new());
}

fn root(world: &MetadataWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
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

#[given(regex = r#"^the repository directory's operating-system owner resolves to "(.*)"$"#)]
fn given_dir_owner(world: &mut MetadataWorld, owner: String) {
    world.dir_owner = Some(owner);
}

#[given(regex = r#"^a project root containing an empty repository "([^"]*)"$"#)]
fn given_root_with_empty_repo(world: &mut MetadataWorld, name: String) {
    ensure_root(world);
    root(world).add_empty_repo(&name);
}

#[given(regex = r#"^"([^"]*)" has a branch "([^"]*)" committed at (\d+)$"#)]
fn given_branch_at(world: &mut MetadataWorld, name: String, branch: String, epoch: i64) {
    root(world).add_branch_at(&name, &branch, epoch);
}

#[given(regex = r#"^"([^"]*)" has a tag "([^"]*)" committed at (\d+)$"#)]
fn given_tag_at(world: &mut MetadataWorld, name: String, tag: String, epoch: i64) {
    root(world).add_tag_at(&name, &tag, epoch);
}

// --- When --------------------------------------------------------------------

#[when(regex = r#"^I read the metadata of "([^"]*)"$"#)]
fn read_metadata(world: &mut MetadataWorld, name: String) {
    let users: Box<dyn UserDirectory> = Box::new(FakeUserDirectory {
        name: world.dir_owner.clone(),
    });
    let store: GixProjectStore =
        GixProjectStore::new(root(world).path().to_path_buf()).with_user_directory(users);
    world.info = Some(store.info(&name));
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

// --- Thens: last activity ----------------------------------------------------

#[then(regex = r"^the last activity timestamp is (\d+)$")]
fn last_activity_is(world: &mut MetadataWorld, expected: i64) {
    assert_eq!(ok_info(world).last_activity(), Some(expected));
}

#[then("there is no last activity")]
fn no_last_activity(world: &mut MetadataWorld) {
    assert_eq!(ok_info(world).last_activity(), None);
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
