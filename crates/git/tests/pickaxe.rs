//! Gherkin conformance for the gix adapter's `pickaxe` operation.
//!
//! The `Given` builds a deterministic fixture history with gix (no git binary): a
//! five-commit chain that adds, edits, and removes a tracked string so the
//! "banana" occurrence count rises, holds, falls, and rises again across several
//! files. The `When` drives `Repository::pickaxe` and stores the raw [`Result`];
//! the `Then` asserts which commits matched (by pinned newest-first order) and the
//! count-changing files under each — by path, so a step never loops or branches.
//!
//! A separate conformance target from `search` (message/author/committer): a
//! single capability, its own fixture, its own `World`, since pickaxe returns a
//! different shape (commits with their changed files, not a bare commit list).
//! cucumber supplies its own `main`, so the target sets `harness = false`.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::pickaxe::{PickaxeChange, PickaxeMatch};
use gitweb_domain::model::pickaxe_pattern::PickaxePattern;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct PickaxeWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    results: Option<Result<Vec<PickaxeMatch>, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// One pinned identity, stamped at `epoch` so every object id is stable and the
/// commit times along the chain are strictly monotonic.
fn who(name: &str, email: &str, epoch: i64) -> Identity {
    Identity {
        name: name.to_owned(),
        email: email.to_owned(),
        epoch_seconds: epoch,
        timezone_offset_seconds: 0,
    }
}

/// Converts a fixture (gix) object id into the domain's, for assertions.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// A tree mapping each name to a freshly written blob, so two trees differ at a
/// path exactly when that file's bytes differ.
fn tree_of(builder: &RepoBuilder, files: &[(&str, &[u8])]) -> FixtureOid {
    let entries: Vec<TreeEntry> = files
        .iter()
        .map(|(name, bytes): &(&str, &[u8])| TreeEntry {
            name: (*name).to_owned(),
            mode: Mode::File,
            oid: builder.blob(bytes),
        })
        .collect();
    builder.tree(&entries)
}

/// The five-commit chain c1..c5 documented in the feature, tracking the "banana"
/// count so pickaxe has commits that add it (c2, c5), leave it unchanged (c3),
/// remove it (c4), and change it across two files at once (c5).
fn build_pickaxe_history(world: &mut PickaxeWorld) {
    let builder: RepoBuilder = RepoBuilder::init();

    let recipe_v1: &[u8] = b"banana bread\n";

    let t1: FixtureOid = tree_of(
        &builder,
        &[("notes.txt", b"alpha\n"), ("todo.txt", b"TODO one\n")],
    );
    let t2: FixtureOid = tree_of(
        &builder,
        &[
            ("notes.txt", b"alpha\n"),
            ("todo.txt", b"TODO one\n"),
            ("recipe.txt", recipe_v1),
        ],
    );
    let t3: FixtureOid = tree_of(
        &builder,
        &[
            ("notes.txt", b"alpha\n"),
            ("todo.txt", b"TODO one\n"),
            ("recipe.txt", b"banana cake\n"),
        ],
    );
    let t4: FixtureOid = tree_of(
        &builder,
        &[("notes.txt", b"alpha\n"), ("todo.txt", b"TODO one\n")],
    );
    let t5: FixtureOid = tree_of(
        &builder,
        &[
            ("notes.txt", b"alpha\nbanana\n"),
            ("todo.txt", b"TODO two\n"),
            ("extra.txt", b"banana\n"),
        ],
    );

    let ada1: Identity = who("Ada Lovelace", "ada@example.com", 1_000);
    let ada2: Identity = who("Ada Lovelace", "ada@example.com", 1_100);
    let ada3: Identity = who("Ada Lovelace", "ada@example.com", 1_200);
    let ada4: Identity = who("Ada Lovelace", "ada@example.com", 1_300);
    let ada5: Identity = who("Ada Lovelace", "ada@example.com", 1_400);

    let c1: FixtureOid = builder.commit(&CommitSpec {
        tree: t1,
        parents: Vec::new(),
        author: ada1.clone(),
        committer: ada1,
        message: "Initial import\n".to_owned(),
    });
    let c2: FixtureOid = builder.commit(&CommitSpec {
        tree: t2,
        parents: vec![c1],
        author: ada2.clone(),
        committer: ada2,
        message: "Add banana recipe\n".to_owned(),
    });
    let c3: FixtureOid = builder.commit(&CommitSpec {
        tree: t3,
        parents: vec![c2],
        author: ada3.clone(),
        committer: ada3,
        message: "Fix recipe typo\n".to_owned(),
    });
    let c4: FixtureOid = builder.commit(&CommitSpec {
        tree: t4,
        parents: vec![c3],
        author: ada4.clone(),
        committer: ada4,
        message: "Remove recipe\n".to_owned(),
    });
    let c5: FixtureOid = builder.commit(&CommitSpec {
        tree: t5,
        parents: vec![c4],
        author: ada5.clone(),
        committer: ada5,
        message: "Spread banana around\n".to_owned(),
    });

    builder.branch("main", c5);
    builder.set_head("main");

    let recipe_v1_blob: FixtureOid = builder.blob(recipe_v1);
    open_with(
        world,
        builder,
        &[
            ("c1", c1),
            ("c2", c2),
            ("c3", c3),
            ("c4", c4),
            ("c5", c5),
            ("recipe-v1", recipe_v1_blob),
        ],
    );
}

/// Opens the adapter over the freshly built fixture and records the named ids
/// (commits, and the one blob a scenario links a change to) for assertions.
fn open_with(world: &mut PickaxeWorld, builder: RepoBuilder, names: &[(&str, FixtureOid)]) {
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    let named: BTreeMap<String, ObjectId> = names
        .iter()
        .map(|(name, oid): &(&str, FixtureOid)| ((*name).to_owned(), to_domain(*oid)))
        .collect();
    world.named = named;
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- accessors ---------------------------------------------------------------

fn repo(world: &PickaxeWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &PickaxeWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture id named {name}"))
}

fn ok_results(world: &PickaxeWorld) -> &[PickaxeMatch] {
    world
        .results
        .as_ref()
        .expect("run a pickaxe first")
        .as_ref()
        .expect("the pickaxe succeeded")
}

/// The match for the named commit, or `None` when that commit was not a hit.
fn match_of<'a>(world: &'a PickaxeWorld, name: &str) -> Option<&'a PickaxeMatch> {
    let target: ObjectId = oid(world, name);
    ok_results(world)
        .iter()
        .find(|m: &&PickaxeMatch| m.commit().id() == &target)
}

/// The named commit's match, asserting it was a hit.
fn require_match<'a>(world: &'a PickaxeWorld, name: &str) -> &'a PickaxeMatch {
    match_of(world, name).unwrap_or_else(|| panic!("commit {name} was expected to match"))
}

/// The change to `path` within the named commit's match, if the commit listed it.
fn change_of<'a>(world: &'a PickaxeWorld, name: &str, path: &str) -> Option<&'a PickaxeChange> {
    require_match(world, name)
        .changes()
        .iter()
        .find(|change: &&PickaxeChange| change.path() == path)
}

/// The base the pickaxe roots at: a named fixture commit, or HEAD's target when
/// the scenario names none (gitweb's default `$hash`).
fn pickaxe_base(world: &PickaxeWorld, base_name: Option<&str>) -> ObjectId {
    match base_name {
        Some(name) => oid(world, name),
        None => repo(world)
            .head()
            .expect("the fixture has a HEAD")
            .target()
            .clone(),
    }
}

fn run_pickaxe(world: &mut PickaxeWorld, pattern: &str, use_regexp: bool, base_name: Option<&str>) {
    let base: ObjectId = pickaxe_base(world, base_name);
    let matcher: PickaxePattern =
        PickaxePattern::new(pattern, use_regexp).expect("a valid pattern");
    world.results = Some(repo(world).pickaxe(&base, &matcher));
}

// --- Givens ------------------------------------------------------------------

#[given("a pickaxe history")]
fn given_history(world: &mut PickaxeWorld) {
    build_pickaxe_history(world);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r#"^I pickaxe-search for "([^"]*)"$"#)]
fn pickaxe_search(world: &mut PickaxeWorld, pattern: String) {
    run_pickaxe(world, &pattern, false, None);
}

#[when(regex = r#"^I regexp-pickaxe-search for "([^"]*)"$"#)]
fn regexp_pickaxe_search(world: &mut PickaxeWorld, pattern: String) {
    run_pickaxe(world, &pattern, true, None);
}

#[when(regex = r#"^I pickaxe-search for "([^"]*)" rooted at "([^"]*)"$"#)]
fn pickaxe_search_rooted(world: &mut PickaxeWorld, pattern: String, base: String) {
    run_pickaxe(world, &pattern, false, Some(&base));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) commits? match(?:es)?$")]
fn commits_match(world: &mut PickaxeWorld, count: usize) {
    assert_eq!(ok_results(world).len(), count);
}

#[then(regex = r#"^matching commit (\d+) is "([^"]*)"$"#)]
fn matching_commit_is(world: &mut PickaxeWorld, index: usize, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_results(world)[index].commit().id(), &target);
}

#[then(regex = r#"^commit "([^"]*)" is not a match$"#)]
fn commit_is_not_a_match(world: &mut PickaxeWorld, name: String) {
    assert!(match_of(world, &name).is_none());
}

#[then(regex = r#"^commit "([^"]*)" changed (\d+) files?$"#)]
fn commit_changed_files(world: &mut PickaxeWorld, name: String, count: usize) {
    assert_eq!(require_match(world, &name).changes().len(), count);
}

#[then(regex = r#"^commit "([^"]*)" changed file "([^"]*)"$"#)]
fn commit_changed_file(world: &mut PickaxeWorld, name: String, path: String) {
    let change: &PickaxeChange =
        change_of(world, &name, &path).unwrap_or_else(|| panic!("{name} did not list {path}"));
    assert!(!change.is_deleted());
}

#[then(regex = r#"^commit "([^"]*)" deleted file "([^"]*)"$"#)]
fn commit_deleted_file(world: &mut PickaxeWorld, name: String, path: String) {
    let change: &PickaxeChange =
        change_of(world, &name, &path).unwrap_or_else(|| panic!("{name} did not list {path}"));
    assert!(change.is_deleted());
}

#[then(regex = r#"^commit "([^"]*)" did not change file "([^"]*)"$"#)]
fn commit_did_not_change_file(world: &mut PickaxeWorld, name: String, path: String) {
    assert!(change_of(world, &name, &path).is_none());
}

#[then(regex = r#"^commit "([^"]*)" changed file "([^"]*)" holding blob "([^"]*)"$"#)]
fn commit_changed_file_blob(world: &mut PickaxeWorld, name: String, path: String, blob: String) {
    let expected: ObjectId = oid(world, &blob);
    let change: &PickaxeChange =
        change_of(world, &name, &path).unwrap_or_else(|| panic!("{name} did not list {path}"));
    assert_eq!(change.to_blob(), Some(&expected));
}

#[tokio::main]
async fn main() {
    PickaxeWorld::run("features/pickaxe").await;
}
