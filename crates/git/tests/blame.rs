//! Gherkin conformance for the gix adapter's `blame` operation.
//!
//! The `Given` builds a deterministic fixture with gix (no git binary); the
//! `When` drives `Repository::blame` and stores the raw [`Result`], so each
//! `Then` asserts one fact — a line count, a per-line attribution, a line's text
//! or numbering, or the exact failure — with no branching in the step body.
//!
//! The line attributions are pinned against real `git blame -p` output, so the
//! adapter is verified to match git's behaviour exactly, including following a
//! file across a whole-file rename.
//!
//! A separate conformance target: one capability, its own fixtures, its own
//! `World`. cucumber supplies `main`, so the target sets `harness = false`.

use std::collections::BTreeMap;

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::blame::Blame;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct BlameWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    blame: Option<Result<Blame, DomainError>>,
}

// --- fixture construction ----------------------------------------------------

/// A pinned identity, named so distinct commits carry distinct authors and the
/// resulting object ids stay stable across runs.
fn who(name: &str, epoch: i64) -> Identity {
    Identity {
        name: name.to_owned(),
        email: format!("{}@example.com", name.to_ascii_lowercase()),
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

/// Writes one commit authored and committed by `author`.
fn commit(
    builder: &RepoBuilder,
    tree: FixtureOid,
    parents: Vec<FixtureOid>,
    author: &Identity,
    message: &str,
) -> FixtureOid {
    builder.commit(&CommitSpec {
        tree,
        parents,
        author: author.clone(),
        committer: author.clone(),
        message: message.to_owned(),
    })
}

/// A root commit that writes the whole file at once.
fn build_single(world: &mut BlameWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let tree: FixtureOid = tree_of(&builder, &[("poem.txt", b"alpha\nbeta\ngamma\n")]);
    let c1: FixtureOid = commit(&builder, tree, Vec::new(), &who("Ada", 1_000), "root\n");
    builder.branch("main", c1);
    builder.set_head("main");
    open_with(world, builder, &[("c1", c1)]);
}

/// Three commits by three authors: c1 writes the file, c2 appends a line, c3
/// rewrites the second line. Pinned against real `git blame -p`: line 1 → c1,
/// line 2 → c3, line 3 → c1, line 4 → c2.
fn build_layered(world: &mut BlameWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let t1: FixtureOid = tree_of(&builder, &[("f.txt", b"alpha\nbeta\ngamma\n")]);
    let t2: FixtureOid = tree_of(&builder, &[("f.txt", b"alpha\nbeta\ngamma\ndelta\n")]);
    let t3: FixtureOid = tree_of(&builder, &[("f.txt", b"alpha\nBETA\ngamma\ndelta\n")]);
    let c1: FixtureOid = commit(&builder, t1, Vec::new(), &who("Ada", 1_000), "c1\n");
    let c2: FixtureOid = commit(&builder, t2, vec![c1], &who("Bob", 1_100), "c2\n");
    let c3: FixtureOid = commit(&builder, t3, vec![c2], &who("Carol", 1_200), "c3\n");
    builder.branch("main", c3);
    builder.set_head("main");
    open_with(world, builder, &[("c1", c1), ("c2", c2), ("c3", c3)]);
}

/// c1 writes `old.txt`, c2 renames it to `new.txt` unchanged, c3 appends a line.
/// Blame must follow the rename: the untouched lines stay attributed to c1.
fn build_renamed(world: &mut BlameWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let t1: FixtureOid = tree_of(&builder, &[("old.txt", b"one\ntwo\nthree\n")]);
    let t2: FixtureOid = tree_of(&builder, &[("new.txt", b"one\ntwo\nthree\n")]);
    let t3: FixtureOid = tree_of(&builder, &[("new.txt", b"one\ntwo\nthree\nfour\n")]);
    let c1: FixtureOid = commit(&builder, t1, Vec::new(), &who("Ada", 1_000), "write old\n");
    let c2: FixtureOid = commit(&builder, t2, vec![c1], &who("Bob", 1_100), "rename\n");
    let c3: FixtureOid = commit(&builder, t3, vec![c2], &who("Carol", 1_200), "append\n");
    builder.branch("main", c3);
    builder.set_head("main");
    open_with(world, builder, &[("c1", c1), ("c2", c2), ("c3", c3)]);
}

/// A root commit holding a single empty file: zero lines to attribute.
fn build_empty(world: &mut BlameWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let tree: FixtureOid = tree_of(&builder, &[("empty.txt", b"")]);
    let c1: FixtureOid = commit(&builder, tree, Vec::new(), &who("Ada", 1_000), "empty\n");
    builder.branch("main", c1);
    builder.set_head("main");
    open_with(world, builder, &[("c1", c1)]);
}

/// Opens the adapter over a freshly built fixture and records named commits.
fn open_with(world: &mut BlameWorld, builder: RepoBuilder, names: &[(&str, FixtureOid)]) {
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

fn repo(world: &BlameWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &BlameWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture commit named {name}"))
}

/// The all-zero-ish id that is syntactically valid hex but names no object.
fn absent_oid() -> ObjectId {
    ObjectId::parse(&"f".repeat(40)).expect("forty f's is a valid object id")
}

fn ok_blame(world: &BlameWorld) -> &Blame {
    world
        .blame
        .as_ref()
        .expect("take the blame first")
        .as_ref()
        .expect("blaming succeeded")
}

// --- Givens ------------------------------------------------------------------

#[given("a file first written whole by one commit")]
fn given_single(world: &mut BlameWorld) {
    build_single(world);
}

#[given("a file edited over three commits")]
fn given_layered(world: &mut BlameWorld) {
    build_layered(world);
}

#[given("a file renamed then appended to")]
fn given_renamed(world: &mut BlameWorld) {
    build_renamed(world);
}

#[given("a commit holding an empty file")]
fn given_empty(world: &mut BlameWorld) {
    build_empty(world);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r#"^I blame "([^"]*)" at "([^"]*)"$"#)]
fn blame_at(world: &mut BlameWorld, path: String, at: String) {
    let at_oid: ObjectId = oid(world, &at);
    world.blame = Some(repo(world).blame(&at_oid, &path));
}

#[when(regex = r#"^I blame "([^"]*)" at a missing commit$"#)]
fn blame_at_missing(world: &mut BlameWorld, path: String) {
    let at_oid: ObjectId = absent_oid();
    world.blame = Some(repo(world).blame(&at_oid, &path));
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) lines? (?:is|are) blamed$")]
fn lines_blamed(world: &mut BlameWorld, count: usize) {
    assert_eq!(ok_blame(world).lines().len(), count);
}

#[then(regex = r#"^line (\d+) is attributed to "([^"]*)"$"#)]
fn line_attributed_to(world: &mut BlameWorld, line: usize, name: String) {
    let target: ObjectId = oid(world, &name);
    assert_eq!(ok_blame(world).lines()[line - 1].commit(), &target);
}

#[then(regex = r#"^line (\d+) reads "([^"]*)"$"#)]
fn line_reads(world: &mut BlameWorld, line: usize, text: String) {
    assert_eq!(ok_blame(world).lines()[line - 1].content(), text);
}

#[then(regex = r"^line (\d+) has original line number (\d+)$")]
fn line_orig_lineno(world: &mut BlameWorld, line: usize, orig: usize) {
    assert_eq!(ok_blame(world).lines()[line - 1].orig_lineno(), orig);
}

#[then(regex = r"^line (\d+) has final line number (\d+)$")]
fn line_final_lineno(world: &mut BlameWorld, line: usize, final_no: usize) {
    assert_eq!(ok_blame(world).lines()[line - 1].final_lineno(), final_no);
}

#[then("the blame fails to find it")]
fn blame_not_found(world: &mut BlameWorld) {
    let result: &Result<Blame, DomainError> = world.blame.as_ref().expect("take the blame first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[tokio::main]
async fn main() {
    BlameWorld::run("features/blame").await;
}
