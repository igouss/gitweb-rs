//! Gherkin conformance for the gix adapter's `dereferenced_references()` port
//! method — gitweb's `git_get_references` (`show-ref --dereference`).
//!
//! The `Given` builds a deterministic fixture repository with gix (no git
//! binary): one commit carrying two branches, a lightweight tag, and an annotated
//! tag whose ref points at a tag object that peels back to the commit. The `When`
//! drives the adapter and stores the raw [`Result`], so the `Then` asserts the
//! peeled targets, the indirect flag, and the name ordering without branching in
//! the step bodies. cucumber supplies its own `main`, so this target sets
//! `harness = false` in Cargo.toml.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::reference::DereferencedRef;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{
    CommitSpec, Identity, Mode, ObjectId, RepoBuilder, TagSpec, TargetKind, TreeEntry,
};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct RefsWorld {
    /// Held only to keep the fixture's temp directory alive for the repo's life.
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    /// The hex id of the single commit every ref ultimately decorates.
    commit: Option<String>,
    /// The hex id of the annotated tag's tag object (the unpeeled target).
    tag_object: Option<String>,
    refs: Option<Result<Vec<DereferencedRef>, DomainError>>,
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

/// A repository with one commit reachable from `refs/heads/main` and
/// `refs/heads/topic`, a lightweight tag `light` pointing straight at it, and an
/// annotated tag `annot` whose ref points at a tag object that peels to it.
fn build_repo(world: &mut RefsWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let who: Identity = ada();
    let blob: ObjectId = builder.blob(b"hello\n");
    let tree: ObjectId = builder.tree(&[TreeEntry {
        name: "file.txt".to_owned(),
        mode: Mode::File,
        oid: blob,
    }]);
    let commit: ObjectId = builder.commit(&CommitSpec {
        tree,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "first\n".to_owned(),
    });
    builder.branch("main", commit);
    builder.branch("topic", commit);
    builder.lightweight_tag("light", commit);
    let tag_object: ObjectId = builder.annotated_tag(&TagSpec {
        name: "annot".to_owned(),
        target: commit,
        target_kind: TargetKind::Commit,
        tagger: who,
        message: "release\n".to_owned(),
    });

    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.commit = Some(commit.to_string());
    world.tag_object = Some(tag_object.to_string());
    world.builder = Some(builder);
    world.repo = Some(repo);
}

fn repo(world: &RefsWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be built first")
}

fn ok_refs(world: &RefsWorld) -> &[DereferencedRef] {
    world
        .refs
        .as_ref()
        .expect("read the references first")
        .as_ref()
        .expect("reading the references succeeded")
}

fn deref<'a>(world: &'a RefsWorld, name: &str) -> &'a DereferencedRef {
    ok_refs(world)
        .iter()
        .find(|reference: &&DereferencedRef| reference.name().full() == name)
        .unwrap_or_else(|| panic!("no dereferenced ref named {name}"))
}

fn commit_hex(world: &RefsWorld) -> &str {
    world.commit.as_deref().expect("the commit id is recorded")
}

// --- Givens ------------------------------------------------------------------

#[given("a repository whose commit carries two branches, a lightweight tag and an annotated tag")]
fn given_repo(world: &mut RefsWorld) {
    build_repo(world);
}

// --- Whens -------------------------------------------------------------------

#[when("I read the dereferenced references")]
fn read_refs(world: &mut RefsWorld) {
    world.refs = Some(repo(world).dereferenced_references());
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r#"^the ref "([^"]*)" targets the commit and is direct$"#)]
fn ref_targets_commit_direct(world: &mut RefsWorld, name: String) {
    let reference: &DereferencedRef = deref(world, &name);
    assert_eq!(reference.target().as_str(), commit_hex(world));
    assert!(!reference.indirect());
}

#[then(regex = r#"^the ref "([^"]*)" targets the commit and is indirect$"#)]
fn ref_targets_commit_indirect(world: &mut RefsWorld, name: String) {
    let reference: &DereferencedRef = deref(world, &name);
    assert_eq!(reference.target().as_str(), commit_hex(world));
    assert!(reference.indirect());
}

#[then(regex = r#"^the ref "([^"]*)" does not target the tag object$"#)]
fn ref_not_tag_object(world: &mut RefsWorld, name: String) {
    let tag_object: &str = world
        .tag_object
        .as_deref()
        .expect("the tag object id is recorded");
    assert_ne!(deref(world, &name).target().as_str(), tag_object);
}

#[then(regex = r#"^the dereferenced references are "(.*)"$"#)]
fn deref_refs_are(world: &mut RefsWorld, expected: String) {
    let names: Vec<&str> = ok_refs(world)
        .iter()
        .map(|reference: &DereferencedRef| reference.name().full())
        .collect();
    assert_eq!(names.join(", "), expected);
}

#[tokio::main]
async fn main() {
    RefsWorld::run("features/refs").await;
}
