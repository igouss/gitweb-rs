//! Gherkin conformance for the gix adapter's `remotes()` port method.
//!
//! The `Given` builds a deterministic fixture repository with gix (no git
//! binary), configuring `[remote "<name>"]` sections and remote-tracking refs;
//! the `When` drives the adapter and stores the raw [`Result`], so the `Then`
//! asserts either the parsed remotes or the listed refs without branching in the
//! step bodies. cucumber supplies its own `main`, so this target sets
//! `harness = false` in Cargo.toml.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::model::remote::Remote;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{CommitSpec, Identity, Mode, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct RemotesWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    remotes: Option<Result<Vec<Remote>, DomainError>>,
    refs: Option<Result<Vec<Reference>, DomainError>>,
}

/// The single pinned identity used throughout the fixture, so every object id is
/// stable across runs and machines.
fn ada() -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: 1_700_000_000,
        timezone_offset_seconds: 0,
    }
}

/// A repository with three remotes — `origin` (url only), `mirror` (url plus a
/// distinct pushurl), and `pushonly` (pushurl only) — and two remote-tracking
/// branches under `origin`.
fn build_with_remotes(world: &mut RemotesWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let who: Identity = ada();
    let blob: gitweb_fixtures::ObjectId = builder.blob(b"hello\n");
    let tree: gitweb_fixtures::ObjectId = builder.tree(&[TreeEntry {
        name: "file.txt".to_owned(),
        mode: Mode::File,
        oid: blob,
    }]);
    let c1: gitweb_fixtures::ObjectId = builder.commit(&CommitSpec {
        tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "first\n".to_owned(),
    });
    let c2: gitweb_fixtures::ObjectId = builder.commit(&CommitSpec {
        tree,
        parents: vec![c1],
        author: who.clone(),
        committer: who,
        message: "second\n".to_owned(),
    });

    builder.remote("origin", Some("git://h/origin"), None);
    builder.remote("mirror", Some("git://h/mirror"), Some("ssh://h/mirror"));
    builder.remote("pushonly", None, Some("ssh://h/pushonly"));
    builder.remote_branch("origin", "main", c1);
    builder.remote_branch("origin", "topic", c2);

    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.builder = Some(builder);
    world.repo = Some(repo);
}

/// A repository with a commit but no configured remotes.
fn build_no_remotes(world: &mut RemotesWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.builder = Some(builder);
    world.repo = Some(repo);
}

fn repo(world: &RemotesWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn ok_remotes(world: &RemotesWorld) -> &[Remote] {
    world
        .remotes
        .as_ref()
        .expect("read the remotes first")
        .as_ref()
        .expect("reading the remotes succeeded")
}

fn ok_refs(world: &RemotesWorld) -> &[Reference] {
    world
        .refs
        .as_ref()
        .expect("list references first")
        .as_ref()
        .expect("listing references succeeded")
}

fn remote<'a>(world: &'a RemotesWorld, name: &str) -> &'a Remote {
    ok_remotes(world)
        .iter()
        .find(|remote: &&Remote| remote.name() == name)
        .unwrap_or_else(|| panic!("no remote named {name}"))
}

// --- Givens ------------------------------------------------------------------

#[given("a repository with remotes")]
fn given_with_remotes(world: &mut RemotesWorld) {
    build_with_remotes(world);
}

#[given("a repository with no remotes")]
fn given_no_remotes(world: &mut RemotesWorld) {
    build_no_remotes(world);
}

// --- Whens -------------------------------------------------------------------

#[when("I read the remotes")]
fn read_remotes(world: &mut RemotesWorld) {
    world.remotes = Some(repo(world).remotes());
}

#[when(regex = r#"^I list references under "([^"]*)"$"#)]
fn list_refs(world: &mut RemotesWorld, prefix: String) {
    world.refs = Some(repo(world).references(&prefix));
}

// --- Thens -------------------------------------------------------------------

#[then("no remotes are listed")]
fn no_remotes_listed(world: &mut RemotesWorld) {
    assert!(ok_remotes(world).is_empty());
}

#[then(regex = r#"^the listed remotes are "(.*)"$"#)]
fn listed_remotes_are(world: &mut RemotesWorld, expected: String) {
    let names: Vec<&str> = ok_remotes(world)
        .iter()
        .map(|remote: &Remote| remote.name())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[then(regex = r#"^the remote "([^"]*)" fetch URL is "([^"]*)"$"#)]
fn remote_fetch_url_is(world: &mut RemotesWorld, name: String, expected: String) {
    assert_eq!(remote(world, &name).fetch_url(), Some(expected.as_str()));
}

#[then(regex = r#"^the remote "([^"]*)" push URL is "([^"]*)"$"#)]
fn remote_push_url_is(world: &mut RemotesWorld, name: String, expected: String) {
    assert_eq!(remote(world, &name).push_url(), Some(expected.as_str()));
}

#[then(regex = r#"^the remote "([^"]*)" has no fetch URL$"#)]
fn remote_has_no_fetch_url(world: &mut RemotesWorld, name: String) {
    assert_eq!(remote(world, &name).fetch_url(), None);
}

#[then(regex = r"^(\d+) remote references? (?:is|are) listed$")]
fn remote_references_listed(world: &mut RemotesWorld, count: usize) {
    assert_eq!(ok_refs(world).len(), count);
}

#[tokio::main]
async fn main() {
    RemotesWorld::run("features/remotes").await;
}
