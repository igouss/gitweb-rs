//! Gherkin conformance for the gix adapter's `search` operation.
//!
//! The `Given` builds a deterministic fixture history with gix (no git binary):
//! a five-commit chain with distinct authors, committers, messages, and file
//! content so each search facet has many / one / zero matches to assert against.
//! The `When` drives `Repository::search` and stores the raw [`Result`]; the
//! `Then` asserts counts and newest-first order without any branching in the
//! step bodies. Commit times are pinned and strictly increasing along the chain,
//! so ordering is exact and reproducible.
//!
//! A separate conformance target from `history`: a single capability, its own
//! fixture, its own `World`. cucumber supplies its own `main`, so the target
//! sets `harness = false` in Cargo.toml.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::{Page, Repository, SearchKind, SearchQuery};
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct SearchWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    results: Option<Result<Vec<Commit>, DomainError>>,
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

/// The five-commit chain c1..c5 documented in the feature: distinct authors and
/// committers, recipe.txt added (c2), edited (c3), and removed (c4) so the
/// "banana" count rises then falls, and notes.txt extended at c5 without
/// changing its "alpha" count.
fn build_searchable(world: &mut SearchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();

    let t1: FixtureOid = tree_of(&builder, &[("notes.txt", b"alpha\n")]);
    let t2: FixtureOid = tree_of(
        &builder,
        &[("notes.txt", b"alpha\n"), ("recipe.txt", b"banana bread\n")],
    );
    let t3: FixtureOid = tree_of(
        &builder,
        &[("notes.txt", b"alpha\n"), ("recipe.txt", b"banana cake\n")],
    );
    let t4: FixtureOid = tree_of(&builder, &[("notes.txt", b"alpha\n")]);
    let t5: FixtureOid = tree_of(&builder, &[("notes.txt", b"alpha\nbeta\n")]);

    let ada1: Identity = who("Ada Lovelace", "ada@example.com", 1_000);
    let grace2: Identity = who("Grace Hopper", "grace@example.com", 1_100);
    let ada2: Identity = who("Ada Lovelace", "ada@example.com", 1_100);
    let ada3: Identity = who("Ada Lovelace", "ada@example.com", 1_200);
    let linus4: Identity = who("Linus Torvalds", "linus@example.com", 1_300);
    let ada4: Identity = who("Ada Lovelace", "ada@example.com", 1_300);
    let ada5: Identity = who("Ada Lovelace", "ada@example.com", 1_400);
    let grace5: Identity = who("Grace Hopper", "grace@example.com", 1_400);

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
        author: grace2,
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
        author: linus4,
        committer: ada4,
        message: "Remove recipe\n".to_owned(),
    });
    let c5: FixtureOid = builder.commit(&CommitSpec {
        tree: t5,
        parents: vec![c4],
        author: ada5,
        committer: grace5,
        message: "Update notes\n".to_owned(),
    });

    builder.branch("main", c5);
    builder.set_head("main");

    open_with(
        world,
        builder,
        &[("c1", c1), ("c2", c2), ("c3", c3), ("c4", c4), ("c5", c5)],
    );
}

/// Opens the adapter over the freshly built fixture and records the named
/// commits for assertions.
fn open_with(world: &mut SearchWorld, builder: RepoBuilder, names: &[(&str, FixtureOid)]) {
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

fn repo(world: &SearchWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &SearchWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture commit named {name}"))
}

fn ok_results(world: &SearchWorld) -> &[Commit] {
    world
        .results
        .as_ref()
        .expect("run a search first")
        .as_ref()
        .expect("the search succeeded")
}

fn run_search(world: &mut SearchWorld, kind: SearchKind, pattern: &str, page: Page) {
    let query: SearchQuery = SearchQuery {
        kind,
        pattern: pattern.to_owned(),
    };
    world.results = Some(repo(world).search(&query, page));
}

// --- Givens ------------------------------------------------------------------

#[given("a searchable history")]
fn given_searchable(world: &mut SearchWorld) {
    build_searchable(world);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r#"^I search messages for "([^"]*)"$"#)]
fn search_messages(world: &mut SearchWorld, pattern: String) {
    run_search(world, SearchKind::Commit, &pattern, Page::new(0, 100));
}

#[when(regex = r#"^I search authors for "([^"]*)"$"#)]
fn search_authors(world: &mut SearchWorld, pattern: String) {
    run_search(world, SearchKind::Author, &pattern, Page::new(0, 100));
}

#[when(regex = r#"^I search committers for "([^"]*)"$"#)]
fn search_committers(world: &mut SearchWorld, pattern: String) {
    run_search(world, SearchKind::Committer, &pattern, Page::new(0, 100));
}

#[when(regex = r#"^I pickaxe-search for "([^"]*)"$"#)]
fn search_pickaxe(world: &mut SearchWorld, pattern: String) {
    run_search(world, SearchKind::Pickaxe, &pattern, Page::new(0, 100));
}

#[when(regex = r#"^I search messages for "([^"]*)" on page (\d+) of size (\d+)$"#)]
fn search_messages_paged(world: &mut SearchWorld, pattern: String, page: usize, size: usize) {
    run_search(
        world,
        SearchKind::Commit,
        &pattern,
        Page::from_page(page, size),
    );
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) commits? (?:is|are) found$")]
fn commits_found(world: &mut SearchWorld, count: usize) {
    assert_eq!(ok_results(world).len(), count);
}

#[then(regex = r#"^found commit (\d+) is "([^"]*)"$"#)]
fn found_commit_is(world: &mut SearchWorld, index: usize, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_results(world)[index].id(), &target);
}

#[tokio::main]
async fn main() {
    SearchWorld::run("features/search").await;
}
