//! Gherkin conformance for the gix adapter's `combined_diff` operation.
//!
//! Each `Given` builds a merge with gix (no git binary): two or three parent
//! commits and a merge commit whose result tree is chosen so the combined-diff
//! rule — show a path only if it differs from EVERY parent — is exercised. The
//! `When` drives `Repository::combined_diff` and stores the raw [`Result`], so
//! each `Then` looks a changed path up by name and asserts one fact, with no
//! branching in the step bodies.
//!
//! cucumber supplies its own `main`, so this target sets `harness = false`.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::diff::{CombinedDiff, CombinedDiffEntry};
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct CombinedWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    merge_oid: Option<ObjectId>,
    result: Option<Result<CombinedDiff, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// One pinned identity, so every built object id is stable across runs.
fn ada() -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: 1_000,
        timezone_offset_seconds: 0,
    }
}

/// Converts a fixture (gix) object id into the domain's, for the `When` call.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// A tree entry naming a freshly written regular-file blob.
fn file_entry(builder: &RepoBuilder, name: &str, bytes: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_owned(),
        mode: Mode::File,
        oid: builder.blob(bytes),
    }
}

/// A root commit snapshotting `entries`, used to stand in for a merge parent.
fn parent_commit(builder: &RepoBuilder, entries: &[TreeEntry]) -> FixtureOid {
    builder.commit(&CommitSpec {
        tree: builder.tree(entries),
        parents: Vec::new(),
        author: ada(),
        committer: ada(),
        message: "parent\n".to_owned(),
    })
}

/// A merge commit snapshotting `entries`, with the given parents.
fn merge_commit(
    builder: &RepoBuilder,
    entries: &[TreeEntry],
    parents: Vec<FixtureOid>,
) -> FixtureOid {
    builder.commit(&CommitSpec {
        tree: builder.tree(entries),
        parents,
        author: ada(),
        committer: ada(),
        message: "merge\n".to_owned(),
    })
}

/// Opens the adapter over `builder` and records the merge to diff.
fn open_with(world: &mut CombinedWorld, builder: RepoBuilder, merge: ObjectId) {
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.merge_oid = Some(merge);
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- accessors ---------------------------------------------------------------

fn repo(world: &CombinedWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn ok_diff(world: &CombinedWorld) -> &CombinedDiff {
    world
        .result
        .as_ref()
        .expect("compute the combined diff first")
        .as_ref()
        .expect("the combined diff succeeded")
}

fn entry_to<'a>(world: &'a CombinedWorld, path: &str) -> &'a CombinedDiffEntry {
    ok_diff(world)
        .entries()
        .iter()
        .find(|entry: &&CombinedDiffEntry| entry.to_path() == path)
        .unwrap_or_else(|| panic!("no combined change to {path}"))
}

fn zeros() -> String {
    "0".repeat(40)
}

// --- Givens ------------------------------------------------------------------

#[given("a two-parent merge where one file differs from both parents")]
fn given_differs_both(world: &mut CombinedWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let p1: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "only.txt", b"p1\n")]);
    let p2: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "only.txt", b"p2\n")]);
    let merge: FixtureOid = merge_commit(
        &builder,
        &[file_entry(&builder, "only.txt", b"merged\n")],
        vec![p1, p2],
    );
    open_with(world, builder, to_domain(merge));
}

#[given("a two-parent merge where one file matches a parent")]
fn given_matches_parent(world: &mut CombinedWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let p1: FixtureOid = parent_commit(
        &builder,
        &[
            file_entry(&builder, "same.txt", b"v\n"),
            file_entry(&builder, "matches.txt", b"RESULT\n"),
            file_entry(&builder, "only.txt", b"p1\n"),
        ],
    );
    let p2: FixtureOid = parent_commit(
        &builder,
        &[
            file_entry(&builder, "same.txt", b"v\n"),
            file_entry(&builder, "matches.txt", b"p2\n"),
            file_entry(&builder, "only.txt", b"p2\n"),
        ],
    );
    let merge: FixtureOid = merge_commit(
        &builder,
        &[
            file_entry(&builder, "same.txt", b"v\n"),
            file_entry(&builder, "matches.txt", b"RESULT\n"),
            file_entry(&builder, "only.txt", b"merged\n"),
        ],
        vec![p1, p2],
    );
    open_with(world, builder, to_domain(merge));
}

#[given("a two-parent merge that adds a file new to both parents")]
fn given_combined_add(world: &mut CombinedWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let p1: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "a.txt", b"x\n")]);
    let p2: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "a.txt", b"x\n")]);
    let merge: FixtureOid = merge_commit(
        &builder,
        &[
            file_entry(&builder, "a.txt", b"x\n"),
            file_entry(&builder, "added.txt", b"new\n"),
        ],
        vec![p1, p2],
    );
    open_with(world, builder, to_domain(merge));
}

#[given("a two-parent merge that deletes a file present in both parents")]
fn given_combined_delete(world: &mut CombinedWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let p1: FixtureOid = parent_commit(
        &builder,
        &[
            file_entry(&builder, "a.txt", b"x\n"),
            file_entry(&builder, "gone.txt", b"old\n"),
        ],
    );
    let p2: FixtureOid = parent_commit(
        &builder,
        &[
            file_entry(&builder, "a.txt", b"x\n"),
            file_entry(&builder, "gone.txt", b"old\n"),
        ],
    );
    let merge: FixtureOid = merge_commit(
        &builder,
        &[file_entry(&builder, "a.txt", b"x\n")],
        vec![p1, p2],
    );
    open_with(world, builder, to_domain(merge));
}

#[given("a three-parent merge where one file differs from all parents")]
fn given_octopus(world: &mut CombinedWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let p1: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "octo.txt", b"1\n")]);
    let p2: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "octo.txt", b"2\n")]);
    let p3: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "octo.txt", b"3\n")]);
    let merge: FixtureOid = merge_commit(
        &builder,
        &[file_entry(&builder, "octo.txt", b"merged\n")],
        vec![p1, p2, p3],
    );
    open_with(world, builder, to_domain(merge));
}

#[given("a missing merge commit id")]
fn given_missing(world: &mut CombinedWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let _seed: FixtureOid = parent_commit(&builder, &[file_entry(&builder, "a.txt", b"x\n")]);
    let missing: ObjectId =
        ObjectId::parse(&"f".repeat(40)).expect("forty f's is a valid object id");
    open_with(world, builder, missing);
}

// --- When --------------------------------------------------------------------

#[when("I compute the combined diff")]
fn when_compute(world: &mut CombinedWorld) {
    let merge: ObjectId = world.merge_oid.clone().expect("a merge oid must be set");
    let result: Result<CombinedDiff, DomainError> = repo(world).combined_diff(&merge);
    world.result = Some(result);
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) combined paths? changed$")]
fn combined_paths_changed(world: &mut CombinedWorld, count: usize) {
    assert_eq!(ok_diff(world).entries().len(), count);
}

#[then(regex = r#"^the merged path "([^"]*)" has (\d+) parents$"#)]
fn merged_path_parents(world: &mut CombinedWorld, path: String, count: usize) {
    assert_eq!(entry_to(world, &path).nparents(), count);
}

#[then(regex = r#"^the change to "([^"]*)" against parent (\d+) is "(\w+)"$"#)]
fn change_against_parent(world: &mut CombinedWorld, path: String, parent: usize, kind: String) {
    let status: String = format!(
        "{:?}",
        entry_to(world, &path).parents()[parent - 1].status().kind()
    );
    assert_eq!(status, kind);
}

#[then(regex = r#"^the from-oid of "([^"]*)" against parent (\d+) is all zeros$"#)]
fn from_oid_zeros(world: &mut CombinedWorld, path: String, parent: usize) {
    assert_eq!(
        entry_to(world, &path).parents()[parent - 1]
            .from_oid()
            .as_str(),
        zeros()
    );
}

#[then(regex = r#"^the merged path "([^"]*)" is a deletion$"#)]
fn merged_path_deletion(world: &mut CombinedWorld, path: String) {
    assert!(entry_to(world, &path).is_deleted());
}

#[then(regex = r#"^no combined path "([^"]*)" changed$"#)]
fn no_combined_path(world: &mut CombinedWorld, path: String) {
    let found: Option<&CombinedDiffEntry> = ok_diff(world)
        .entries()
        .iter()
        .find(|entry: &&CombinedDiffEntry| entry.to_path() == path);
    assert!(found.is_none());
}

#[then("the combined diff fails to find an object")]
fn combined_not_found(world: &mut CombinedWorld) {
    let result: &Result<CombinedDiff, DomainError> = world
        .result
        .as_ref()
        .expect("compute the combined diff first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[tokio::main]
async fn main() {
    CombinedWorld::run("features/combined_diff").await;
}
