//! Gherkin conformance for the gix [`Repository`] adapter.
//!
//! The `Given` builds a deterministic fixture repository with gix (no git
//! binary); the `When` drives the adapter and stores the raw [`Result`], so the
//! `Then` can assert either parsed fields or the exact failure mode without any
//! branching in the step bodies. cucumber supplies its own `main`, so this
//! target sets `harness = false` in Cargo.toml.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::object_kind::ObjectKind;
use gitweb_domain::model::reference::Reference;
use gitweb_domain::model::tag::Tag;
use gitweb_domain::model::tree::{Tree, TreeEntry as DomainTreeEntry};
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{
    CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TagSpec, TargetKind, TreeEntry,
};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct AdapterWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    head: Option<Result<Reference, DomainError>>,
    refs: Option<Result<Vec<Reference>, DomainError>>,
    resolved: Option<Result<ObjectId, DomainError>>,
    kind: Option<Result<ObjectKind, DomainError>>,
    commit: Option<Result<Commit, DomainError>>,
    tree: Option<Result<Tree, DomainError>>,
    blob: Option<Result<Blob, DomainError>>,
    tag: Option<Result<Tag, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// The single pinned identity used throughout the fixture, so every object id
/// is stable across runs and machines.
fn ada() -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: 1_700_000_000,
        timezone_offset_seconds: 0,
    }
}

/// Converts a fixture (gix) object id into the domain's, for assertions.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// Builds the canonical history and opens the adapter over it.
fn build_populated(world: &mut AdapterWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let who: Identity = ada();

    let hello: FixtureOid = builder.blob(b"hello\n");
    let bin: FixtureOid = builder.blob(&[0x00, 0x01, 0x02]);
    let empty: FixtureOid = builder.tree(&[]);
    let submod: FixtureOid = FixtureOid::from_hex(b"1111111111111111111111111111111111111111")
        .expect("a placeholder gitlink id");

    let t_full: FixtureOid = builder.tree(&[
        TreeEntry {
            name: "file.txt".to_owned(),
            mode: Mode::File,
            oid: hello,
        },
        TreeEntry {
            name: "run.sh".to_owned(),
            mode: Mode::Executable,
            oid: hello,
        },
        TreeEntry {
            name: "link".to_owned(),
            mode: Mode::Symlink,
            oid: hello,
        },
        TreeEntry {
            name: "sub".to_owned(),
            mode: Mode::Directory,
            oid: empty,
        },
        TreeEntry {
            name: "mod".to_owned(),
            mode: Mode::Gitlink,
            oid: submod,
        },
    ]);

    let c1: FixtureOid = builder.commit(&CommitSpec {
        tree: t_full,
        parents: Vec::new(),
        author: who.clone(),
        committer: who.clone(),
        message: "first commit\n".to_owned(),
    });
    let c2: FixtureOid = builder.commit(&CommitSpec {
        tree: empty,
        parents: vec![c1],
        author: who.clone(),
        committer: who.clone(),
        message: "second\n".to_owned(),
    });
    let merge: FixtureOid = builder.commit(&CommitSpec {
        tree: t_full,
        parents: vec![c1, c2],
        author: who.clone(),
        committer: who.clone(),
        message: "merge\n".to_owned(),
    });

    builder.branch("main", c2);
    builder.set_head("main");
    builder.lightweight_tag("lw", c1);
    let tagobj: FixtureOid = builder.annotated_tag(&TagSpec {
        name: "v1".to_owned(),
        target: c2,
        target_kind: TargetKind::Commit,
        tagger: who,
        message: "release\n".to_owned(),
    });

    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");

    let mut named: BTreeMap<String, ObjectId> = BTreeMap::new();
    named.insert("hello".to_owned(), to_domain(hello));
    named.insert("bin".to_owned(), to_domain(bin));
    named.insert("empty".to_owned(), to_domain(empty));
    named.insert("t_full".to_owned(), to_domain(t_full));
    named.insert("c1".to_owned(), to_domain(c1));
    named.insert("c2".to_owned(), to_domain(c2));
    named.insert("merge".to_owned(), to_domain(merge));
    named.insert("tagobj".to_owned(), to_domain(tagobj));

    world.named = named;
    world.builder = Some(builder);
    world.repo = Some(repo);
}

/// An unborn repository: HEAD points at a branch that does not exist yet.
fn build_unborn(world: &mut AdapterWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    builder.set_head("main");
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the unborn fixture repository");
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- accessors ---------------------------------------------------------------

fn repo(world: &AdapterWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &AdapterWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture object named {name}"))
}

fn absent_oid() -> ObjectId {
    ObjectId::parse(&"a".repeat(40)).expect("forty a's is valid hex")
}

fn ok_head(world: &AdapterWorld) -> &Reference {
    world
        .head
        .as_ref()
        .expect("read HEAD first")
        .as_ref()
        .expect("reading HEAD succeeded")
}

fn ok_refs(world: &AdapterWorld) -> &[Reference] {
    world
        .refs
        .as_ref()
        .expect("list references first")
        .as_ref()
        .expect("listing references succeeded")
}

fn ok_resolved(world: &AdapterWorld) -> &ObjectId {
    world
        .resolved
        .as_ref()
        .expect("resolve first")
        .as_ref()
        .expect("resolving succeeded")
}

fn ok_kind(world: &AdapterWorld) -> &ObjectKind {
    world
        .kind
        .as_ref()
        .expect("read the kind first")
        .as_ref()
        .expect("reading the kind succeeded")
}

fn ok_commit(world: &AdapterWorld) -> &Commit {
    world
        .commit
        .as_ref()
        .expect("read the commit first")
        .as_ref()
        .expect("reading the commit succeeded")
}

fn ok_tree(world: &AdapterWorld) -> &Tree {
    world
        .tree
        .as_ref()
        .expect("read the tree first")
        .as_ref()
        .expect("reading the tree succeeded")
}

fn ok_blob(world: &AdapterWorld) -> &Blob {
    world
        .blob
        .as_ref()
        .expect("read the blob first")
        .as_ref()
        .expect("reading the blob succeeded")
}

fn ok_tag(world: &AdapterWorld) -> &Tag {
    world
        .tag
        .as_ref()
        .expect("read the tag first")
        .as_ref()
        .expect("reading the tag succeeded")
}

fn entry<'a>(tree: &'a Tree, name: &str) -> &'a DomainTreeEntry {
    tree.entries()
        .iter()
        .find(|candidate: &&DomainTreeEntry| candidate.name() == name)
        .unwrap_or_else(|| panic!("no tree entry named {name}"))
}

// --- Givens ------------------------------------------------------------------

#[given("a populated repository")]
fn given_populated(world: &mut AdapterWorld) {
    build_populated(world);
}

#[given("an unborn repository")]
fn given_unborn(world: &mut AdapterWorld) {
    build_unborn(world);
}

// --- Whens -------------------------------------------------------------------

#[when("I read HEAD")]
fn read_head(world: &mut AdapterWorld) {
    world.head = Some(repo(world).head());
}

#[when(regex = r#"^I list references under "([^"]*)"$"#)]
fn list_refs(world: &mut AdapterWorld, prefix: String) {
    world.refs = Some(repo(world).references(&prefix));
}

#[when(regex = r#"^I resolve "([^"]*)"$"#)]
fn resolve_rev(world: &mut AdapterWorld, rev: String) {
    world.resolved = Some(repo(world).resolve(&rev));
}

#[when(regex = r#"^I read the kind of "([^"]*)"$"#)]
fn read_kind(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    world.kind = Some(repo(world).object_kind(&target));
}

#[when("I read the kind of an absent object")]
fn read_kind_absent(world: &mut AdapterWorld) {
    let target: ObjectId = absent_oid();
    world.kind = Some(repo(world).object_kind(&target));
}

#[when(regex = r#"^I read the commit "([^"]*)"$"#)]
fn read_commit(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    world.commit = Some(repo(world).find_commit(&target));
}

#[when(regex = r#"^I read the tree "([^"]*)"$"#)]
fn read_tree(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    world.tree = Some(repo(world).find_tree(&target));
}

#[when(regex = r#"^I read the blob "([^"]*)"$"#)]
fn read_blob(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    world.blob = Some(repo(world).find_blob(&target));
}

#[when(regex = r#"^I read the tag "([^"]*)"$"#)]
fn read_tag(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    world.tag = Some(repo(world).find_tag(&target));
}

// --- Thens: HEAD -------------------------------------------------------------

#[then(regex = r#"^the head ref is "([^"]*)"$"#)]
fn head_ref_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(ok_head(world).name().full(), expected);
}

#[then(regex = r#"^the head points at "([^"]*)"$"#)]
fn head_points_at(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_head(world).target(), &target);
}

#[then("reading HEAD fails as not found")]
fn head_not_found(world: &mut AdapterWorld) {
    let result: &Result<Reference, DomainError> = world.head.as_ref().expect("read HEAD first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

// --- Thens: references -------------------------------------------------------

#[then(regex = r"^(\d+) references? (?:is|are) listed$")]
fn references_listed(world: &mut AdapterWorld, count: usize) {
    assert_eq!(ok_refs(world).len(), count);
}

#[then(regex = r#"^the reference "([^"]*)" points at "([^"]*)"$"#)]
fn reference_points_at(world: &mut AdapterWorld, full: String, name: String) {
    let target: ObjectId = oid(world, &name);
    let found: &Reference = ok_refs(world)
        .iter()
        .find(|candidate: &&Reference| candidate.name().full() == full)
        .unwrap_or_else(|| panic!("no listed reference named {full}"));
    assert_eq!(found.target(), &target);
}

// --- Thens: resolve ----------------------------------------------------------

#[then(regex = r#"^it resolves to "([^"]*)"$"#)]
fn resolves_to(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_resolved(world), &target);
}

#[then("resolving fails as not found")]
fn resolving_not_found(world: &mut AdapterWorld) {
    let result: &Result<ObjectId, DomainError> = world.resolved.as_ref().expect("resolve first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

// --- Thens: object_kind ------------------------------------------------------

#[then(regex = r#"^the kind is "(\w+)"$"#)]
fn kind_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(format!("{:?}", ok_kind(world)), expected);
}

#[then("reading the kind fails as not found")]
fn kind_not_found(world: &mut AdapterWorld) {
    let result: &Result<ObjectKind, DomainError> =
        world.kind.as_ref().expect("read the kind first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

// --- Thens: find_commit ------------------------------------------------------

#[then(regex = r#"^the commit author is "([^"]*)"$"#)]
fn commit_author_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(ok_commit(world).author().name(), expected);
}

#[then(regex = r#"^the commit author email is "([^"]*)"$"#)]
fn commit_email_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(ok_commit(world).author().email(), Some(expected.as_str()));
}

#[then(regex = r"^the commit timestamp is (-?\d+)$")]
fn commit_timestamp_is(world: &mut AdapterWorld, expected: i64) {
    assert_eq!(ok_commit(world).author().epoch(), expected);
}

#[then(regex = r#"^the commit title is "([^"]*)"$"#)]
fn commit_title_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(ok_commit(world).title(), expected);
}

#[then(regex = r#"^the commit snapshots the tree "([^"]*)"$"#)]
fn commit_snapshots_tree(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_commit(world).tree(), &target);
}

#[then(regex = r"^the commit has (\d+) parents$")]
fn commit_has_parents(world: &mut AdapterWorld, count: usize) {
    assert_eq!(ok_commit(world).parents().len(), count);
}

#[then("the commit is a merge")]
fn commit_is_merge(world: &mut AdapterWorld) {
    assert!(ok_commit(world).is_merge());
}

#[then("the commit is not a merge")]
fn commit_is_not_merge(world: &mut AdapterWorld) {
    assert!(!ok_commit(world).is_merge());
}

#[then("reading the commit fails as invalid")]
fn commit_invalid(world: &mut AdapterWorld) {
    let result: &Result<Commit, DomainError> =
        world.commit.as_ref().expect("read the commit first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

// --- Thens: find_tree --------------------------------------------------------

#[then(regex = r"^the tree has (\d+) entries$")]
fn tree_has_entries(world: &mut AdapterWorld, count: usize) {
    assert_eq!(ok_tree(world).entries().len(), count);
}

#[then(regex = r#"^the tree entry "([^"]*)" has long type "([^"]*)"$"#)]
fn tree_entry_long_type(world: &mut AdapterWorld, name: String, expected: String) {
    assert_eq!(entry(ok_tree(world), &name).mode().long_type(), expected);
}

#[then(regex = r#"^the tree entry "([^"]*)" points at "([^"]*)"$"#)]
fn tree_entry_points_at(world: &mut AdapterWorld, name: String, target_name: String) {
    let target: ObjectId = oid(world, &target_name);
    assert_eq!(entry(ok_tree(world), &name).oid(), &target);
}

#[then("reading the tree fails as invalid")]
fn tree_invalid(world: &mut AdapterWorld) {
    let result: &Result<Tree, DomainError> = world.tree.as_ref().expect("read the tree first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

// --- Thens: find_blob --------------------------------------------------------

#[then(regex = r"^the blob is (\d+) bytes$")]
fn blob_size_is(world: &mut AdapterWorld, size: usize) {
    assert_eq!(ok_blob(world).size(), size);
}

#[then("the blob is binary")]
fn blob_is_binary(world: &mut AdapterWorld) {
    assert!(ok_blob(world).is_binary());
}

#[then("the blob is text")]
fn blob_is_text(world: &mut AdapterWorld) {
    assert!(!ok_blob(world).is_binary());
}

#[then("reading the blob fails as invalid")]
fn blob_invalid(world: &mut AdapterWorld) {
    let result: &Result<Blob, DomainError> = world.blob.as_ref().expect("read the blob first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

// --- Thens: find_tag ---------------------------------------------------------

#[then(regex = r#"^the tag name is "([^"]*)"$"#)]
fn tag_name_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(ok_tag(world).name(), expected);
}

#[then(regex = r#"^the tag points at "([^"]*)"$"#)]
fn tag_points_at(world: &mut AdapterWorld, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_tag(world).object(), &target);
}

#[then(regex = r#"^the tagged kind is "(\w+)"$"#)]
fn tagged_kind_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(format!("{:?}", ok_tag(world).object_kind()), expected);
}

#[then(regex = r#"^the tag tagger is "([^"]*)"$"#)]
fn tag_tagger_is(world: &mut AdapterWorld, expected: String) {
    assert_eq!(
        ok_tag(world).tagger().expect("an annotated tagger").name(),
        expected
    );
}

#[then("reading the tag fails as invalid")]
fn tag_invalid(world: &mut AdapterWorld) {
    let result: &Result<Tag, DomainError> = world.tag.as_ref().expect("read the tag first");
    assert!(matches!(result, Err(DomainError::Invalid(_))));
}

#[tokio::main]
async fn main() {
    AdapterWorld::run("features/conformance").await;
}
