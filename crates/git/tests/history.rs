//! Gherkin conformance for the gix adapter's `history` operation.
//!
//! The `Given` builds a deterministic fixture history with gix (no git binary);
//! the `When` drives `Repository::history` and stores the raw [`Result`], so the
//! `Then` can assert counts, order, and parentage without any branching in the
//! step bodies. Commit times are pinned and strictly increasing along each
//! chain, so newest-first ordering is exact and reproducible.
//!
//! This is a separate conformance target from `object_reads`: a single
//! capability, its own fixtures, its own `World`. cucumber supplies its own
//! `main`, so the target sets `harness = false` in Cargo.toml.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::{Page, Repository};
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct HistoryWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    history: Option<Result<Vec<Commit>, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// One pinned identity, stamped at `epoch` so every object id is stable and the
/// commit times along a chain can be made strictly monotonic by the caller.
fn ada(epoch: i64) -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: epoch,
        timezone_offset_seconds: 0,
    }
}

/// Converts a fixture (gix) object id into the domain's, for assertions.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// A single-file tree mapping each name to a freshly written blob, so two trees
/// differ at a path exactly when that file's bytes differ.
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

/// A five-commit chain off a root. `a.txt` is rewritten by c1, c2, c4 and left
/// untouched by c3, c5; `b.txt` is added by c3 and changed by c5. Commit times
/// increase by 100s along the chain.
fn build_line(world: &mut HistoryWorld) {
    let builder: RepoBuilder = RepoBuilder::init();

    let t1: FixtureOid = tree_of(&builder, &[("a.txt", b"1")]);
    let t2: FixtureOid = tree_of(&builder, &[("a.txt", b"2")]);
    let t3: FixtureOid = tree_of(&builder, &[("a.txt", b"2"), ("b.txt", b"x")]);
    let t4: FixtureOid = tree_of(&builder, &[("a.txt", b"3"), ("b.txt", b"x")]);
    let t5: FixtureOid = tree_of(&builder, &[("a.txt", b"3"), ("b.txt", b"y")]);

    let c1: FixtureOid = builder.commit(&CommitSpec {
        tree: t1,
        parents: Vec::new(),
        author: ada(1_000),
        committer: ada(1_000),
        message: "root\n".to_owned(),
    });
    let c2: FixtureOid = builder.commit(&CommitSpec {
        tree: t2,
        parents: vec![c1],
        author: ada(1_100),
        committer: ada(1_100),
        message: "edit a\n".to_owned(),
    });
    let c3: FixtureOid = builder.commit(&CommitSpec {
        tree: t3,
        parents: vec![c2],
        author: ada(1_200),
        committer: ada(1_200),
        message: "add b\n".to_owned(),
    });
    let c4: FixtureOid = builder.commit(&CommitSpec {
        tree: t4,
        parents: vec![c3],
        author: ada(1_300),
        committer: ada(1_300),
        message: "edit a again\n".to_owned(),
    });
    let c5: FixtureOid = builder.commit(&CommitSpec {
        tree: t5,
        parents: vec![c4],
        author: ada(1_400),
        committer: ada(1_400),
        message: "edit b\n".to_owned(),
    });

    builder.branch("main", c5);
    builder.set_head("main");

    open_with(
        world,
        builder,
        &[("c1", c1), ("c2", c2), ("c3", c3), ("c4", c4), ("c5", c5)],
    );
}

/// A root `r` with three independent children `p1`, `p2`, `p3` and an octopus
/// merge `m` of all three. Commit times increase, so newest-first order is
/// `m, p3, p2, p1, r`.
fn build_octopus(world: &mut HistoryWorld) {
    let builder: RepoBuilder = RepoBuilder::init();

    let base: FixtureOid = tree_of(&builder, &[("f", b"r")]);
    let r: FixtureOid = builder.commit(&CommitSpec {
        tree: base,
        parents: Vec::new(),
        author: ada(2_000),
        committer: ada(2_000),
        message: "root\n".to_owned(),
    });
    let p1: FixtureOid = builder.commit(&CommitSpec {
        tree: tree_of(&builder, &[("f", b"r"), ("p1", b"1")]),
        parents: vec![r],
        author: ada(2_100),
        committer: ada(2_100),
        message: "p1\n".to_owned(),
    });
    let p2: FixtureOid = builder.commit(&CommitSpec {
        tree: tree_of(&builder, &[("f", b"r"), ("p2", b"2")]),
        parents: vec![r],
        author: ada(2_200),
        committer: ada(2_200),
        message: "p2\n".to_owned(),
    });
    let p3: FixtureOid = builder.commit(&CommitSpec {
        tree: tree_of(&builder, &[("f", b"r"), ("p3", b"3")]),
        parents: vec![r],
        author: ada(2_300),
        committer: ada(2_300),
        message: "p3\n".to_owned(),
    });
    let m: FixtureOid = builder.commit(&CommitSpec {
        tree: tree_of(
            &builder,
            &[("f", b"r"), ("p1", b"1"), ("p2", b"2"), ("p3", b"3")],
        ),
        parents: vec![p1, p2, p3],
        author: ada(2_400),
        committer: ada(2_400),
        message: "octopus\n".to_owned(),
    });

    builder.branch("main", m);
    builder.set_head("main");

    open_with(
        world,
        builder,
        &[("r", r), ("p1", p1), ("p2", p2), ("p3", p3), ("m", m)],
    );
}

/// Opens the adapter over a freshly built fixture and records the named commits
/// for assertions.
fn open_with(world: &mut HistoryWorld, builder: RepoBuilder, names: &[(&str, FixtureOid)]) {
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

fn repo(world: &HistoryWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &HistoryWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture commit named {name}"))
}

fn ok_history(world: &HistoryWorld) -> &[Commit] {
    world
        .history
        .as_ref()
        .expect("read history first")
        .as_ref()
        .expect("reading history succeeded")
}

// --- Givens ------------------------------------------------------------------

#[given("a line of history")]
fn given_line(world: &mut HistoryWorld) {
    build_line(world);
}

#[given("an octopus merge")]
fn given_octopus(world: &mut HistoryWorld) {
    build_octopus(world);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r#"^I read the history from "([^"]*)"$"#)]
fn read_history(world: &mut HistoryWorld, start: String) {
    let start_oid: ObjectId = oid(world, &start);
    world.history = Some(repo(world).history(&start_oid, None, Page::new(0, 100)));
}

#[when(regex = r#"^I read the history from "([^"]*)" touching "([^"]*)"$"#)]
fn read_history_touching(world: &mut HistoryWorld, start: String, path: String) {
    let start_oid: ObjectId = oid(world, &start);
    world.history = Some(repo(world).history(&start_oid, Some(&path), Page::new(0, 100)));
}

#[when(regex = r#"^I read page (\d+) of size (\d+) from "([^"]*)"$"#)]
fn read_history_page(world: &mut HistoryWorld, page: usize, size: usize, start: String) {
    let start_oid: ObjectId = oid(world, &start);
    world.history = Some(repo(world).history(&start_oid, None, Page::from_page(page, size)));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) commits? (?:is|are) listed$")]
fn commits_listed(world: &mut HistoryWorld, count: usize) {
    assert_eq!(ok_history(world).len(), count);
}

#[then(regex = r#"^commit (\d+) is "([^"]*)"$"#)]
fn commit_at_is(world: &mut HistoryWorld, index: usize, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_history(world)[index].id(), &target);
}

#[then(regex = r"^commit (\d+) has (\d+) parents$")]
fn commit_at_has_parents(world: &mut HistoryWorld, index: usize, count: usize) {
    assert_eq!(ok_history(world)[index].parents().len(), count);
}

#[then(regex = r"^commit (\d+) is a merge$")]
fn commit_at_is_merge(world: &mut HistoryWorld, index: usize) {
    assert!(ok_history(world)[index].is_merge());
}

#[tokio::main]
async fn main() {
    HistoryWorld::run("features/history").await;
}
