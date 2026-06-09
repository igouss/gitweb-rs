//! Gherkin conformance for the gix adapter's `grep` operation.
//!
//! The `Given` builds a deterministic fixture with gix (no git binary): a single
//! commit whose tree mixes text files, a binary blob, a non-UTF-8 blob, a
//! symlink, a gitlink, an executable, and a subdirectory — so the adapter's tree
//! walk has every entry kind to include or skip and a stable path order to
//! preserve. A second fixture fills the match cap exactly or by one, to pin the
//! trim boundary. The `When` drives `Repository::grep` and stores the raw
//! [`Result`]; the `Then` asserts counts, per-match path/line/text, and the
//! `trimmed` flag with no branching in the step bodies.
//!
//! A separate conformance target with its own `World`. cucumber supplies its own
//! `main`, so the target sets `harness = false` in Cargo.toml.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::grep::{GREP_MATCH_LIMIT, GrepMatch, GrepResults};
use gitweb_domain::model::grep_pattern::GrepPattern;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct GrepWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    results: Option<Result<GrepResults, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// One pinned identity, so every object id is stable across runs.
fn who() -> Identity {
    Identity {
        name: "Ada Lovelace".to_owned(),
        email: "ada@example.com".to_owned(),
        epoch_seconds: 1_000,
        timezone_offset_seconds: 0,
    }
}

/// Converts a fixture (gix) object id into the domain's, for assertions.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// One tree entry naming a freshly written blob with the given mode.
fn entry(builder: &RepoBuilder, name: &str, mode: Mode, content: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_owned(),
        mode,
        oid: builder.blob(content),
    }
}

/// The mixed-files fixture documented in the feature: one commit whose tree
/// holds text, binary, non-UTF-8, symlink, gitlink, executable, and subtree
/// entries. The gitlink points at a real (empty-tree) commit so the tree is
/// well-formed; the symlink's blob content is the path "alpha.txt".
fn build_mixed(world: &mut GrepWorld) {
    let builder: RepoBuilder = RepoBuilder::init();

    let empty_tree: FixtureOid = builder.tree(&[]);
    let submodule: FixtureOid = builder.commit(&CommitSpec {
        tree: empty_tree,
        parents: Vec::new(),
        author: who(),
        committer: who(),
        message: "submodule head\n".to_owned(),
    });

    let deep: FixtureOid = builder.tree(&[entry(
        &builder,
        "deep.txt",
        Mode::File,
        b"alpha in subdir\n",
    )]);

    let root: FixtureOid = builder.tree(&[
        entry(
            &builder,
            "alpha.txt",
            Mode::File,
            b"alpha\nbeta\nalpha gamma\n",
        ),
        entry(&builder, "beta.txt", Mode::File, b"beta\n"),
        entry(&builder, "data.bin", Mode::File, b"alpha\0binary"),
        entry(&builder, "latin.txt", Mode::File, b"caf\xe9 noir\n"),
        entry(&builder, "link", Mode::Symlink, b"alpha.txt"),
        TreeEntry {
            name: "modz".to_owned(),
            mode: Mode::Gitlink,
            oid: submodule,
        },
        entry(&builder, "run.sh", Mode::Executable, b"alpha script\n"),
        TreeEntry {
            name: "sub".to_owned(),
            mode: Mode::Directory,
            oid: deep,
        },
    ]);

    let commit: FixtureOid = builder.commit(&CommitSpec {
        tree: root,
        parents: Vec::new(),
        author: who(),
        committer: who(),
        message: "mixed tree\n".to_owned(),
    });
    builder.branch("main", commit);
    builder.set_head("main");

    open_with(world, builder, &[("commit", commit), ("tree", root)]);
}

/// A one-file fixture whose only file holds `lines` lines that each contain the
/// pattern "x", so a grep for "x" yields exactly `lines` candidate matches —
/// used to drive the adapter's output cap at and just past its limit.
fn build_lines(world: &mut GrepWorld, lines: usize) {
    let builder: RepoBuilder = RepoBuilder::init();
    let content: String = "x\n".repeat(lines);
    let root: FixtureOid =
        builder.tree(&[entry(&builder, "big.txt", Mode::File, content.as_bytes())]);
    let commit: FixtureOid = builder.commit(&CommitSpec {
        tree: root,
        parents: Vec::new(),
        author: who(),
        committer: who(),
        message: "many lines\n".to_owned(),
    });
    builder.branch("main", commit);
    builder.set_head("main");
    open_with(world, builder, &[("commit", commit), ("tree", root)]);
}

/// Opens the adapter over the freshly built fixture and records named ids.
fn open_with(world: &mut GrepWorld, builder: RepoBuilder, names: &[(&str, FixtureOid)]) {
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

fn repo(world: &GrepWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &GrepWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture object named {name}"))
}

fn ok_results(world: &GrepWorld) -> &GrepResults {
    world
        .results
        .as_ref()
        .expect("run a grep first")
        .as_ref()
        .expect("the grep succeeded")
}

fn matches(world: &GrepWorld) -> &[GrepMatch] {
    ok_results(world).matches()
}

// --- Givens ------------------------------------------------------------------

#[given("a tree of mixed files")]
fn given_mixed(world: &mut GrepWorld) {
    build_mixed(world);
}

#[given("a file with as many matching lines as the cap allows")]
fn given_cap_full(world: &mut GrepWorld) {
    build_lines(world, GREP_MATCH_LIMIT);
}

#[given("a file with one matching line beyond the cap")]
fn given_cap_over(world: &mut GrepWorld) {
    build_lines(world, GREP_MATCH_LIMIT + 1);
}

// --- Whens -------------------------------------------------------------------

/// A fixed (`-F`, case-sensitive) grep matcher — the default search mode.
fn fixed(pattern: &str) -> GrepPattern {
    GrepPattern::new(pattern, false).expect("a fixed pattern always builds")
}

/// A regexp (`-E -i`, case-insensitive) grep matcher — the *re*-box search mode.
fn regexp(pattern: &str) -> GrepPattern {
    GrepPattern::new(pattern, true).expect("the scenario's regexp is well-formed")
}

#[when(regex = r#"^I grep "(.*)" at the commit$"#)]
fn grep_at_commit(world: &mut GrepWorld, pattern: String) {
    let base: ObjectId = oid(world, "commit");
    world.results = Some(repo(world).grep(&base, &fixed(&pattern)));
}

#[when(regex = r#"^I grep regexp "(.*)" at the commit$"#)]
fn grep_regexp_at_commit(world: &mut GrepWorld, pattern: String) {
    let base: ObjectId = oid(world, "commit");
    world.results = Some(repo(world).grep(&base, &regexp(&pattern)));
}

#[when(regex = r#"^I grep "(.*)" at the tree$"#)]
fn grep_at_tree(world: &mut GrepWorld, pattern: String) {
    let base: ObjectId = oid(world, "tree");
    world.results = Some(repo(world).grep(&base, &fixed(&pattern)));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) grep match(?:es)? (?:is|are) found$")]
fn grep_count(world: &mut GrepWorld, count: usize) {
    assert_eq!(matches(world).len(), count);
}

#[then(regex = r#"^grep match (\d+) is line (\d+) "(.*)" in "([^"]*)"$"#)]
fn grep_line_is(world: &mut GrepWorld, index: usize, line: usize, text: String, path: String) {
    let hit: &GrepMatch = &matches(world)[index];
    assert_eq!(hit.path(), path);
    assert_eq!(hit.line_no(), Some(line));
    assert_eq!(hit.text(), Some(text.as_str()));
}

#[then(regex = r#"^grep match (\d+) is binary file "([^"]*)"$"#)]
fn grep_binary_is(world: &mut GrepWorld, index: usize, path: String) {
    let hit: &GrepMatch = &matches(world)[index];
    assert!(hit.is_binary());
    assert_eq!(hit.path(), path);
    assert_eq!(hit.text(), None);
}

#[then("the result fills the cap exactly")]
fn fills_cap(world: &mut GrepWorld) {
    assert_eq!(matches(world).len(), GREP_MATCH_LIMIT);
}

#[then("the listing is trimmed")]
fn is_trimmed(world: &mut GrepWorld) {
    assert!(ok_results(world).trimmed());
}

#[then("the listing is not trimmed")]
fn is_not_trimmed(world: &mut GrepWorld) {
    assert!(!ok_results(world).trimmed());
}

#[tokio::main]
async fn main() {
    GrepWorld::run("features/grep").await;
}
