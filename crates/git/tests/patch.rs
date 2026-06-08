//! Gherkin conformance for the gix adapter's `patch` operation.
//!
//! Each `Given` builds a deterministic before/after pair with gix (no git
//! binary) and records the two commits to diff; the `When` drives
//! `Repository::patch` and stores the raw [`Result`], so the `Then` steps assert
//! on the rendered patch text with no branching in the step bodies. Combined
//! merge diffs and byte-exact copy similarity are separate capabilities and are
//! not exercised here.
//!
//! cucumber supplies its own `main`, so this target sets `harness = false`.

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::patch::Patch;
use gitweb_domain::port::repository::{RenameDetection, Repository};
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct PatchWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    from_oid: Option<ObjectId>,
    to_oid: Option<ObjectId>,
    patch: Option<Result<Patch, DomainError>>,
    rendered: Option<String>,
    abbrev_length: Option<usize>,
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

fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

fn blob_entry(builder: &RepoBuilder, name: &str, mode: Mode, bytes: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_owned(),
        mode,
        oid: builder.blob(bytes),
    }
}

fn gitlink_entry(name: &str, commit: FixtureOid) -> TreeEntry {
    TreeEntry {
        name: name.to_owned(),
        mode: Mode::Gitlink,
        oid: commit,
    }
}

fn commit_of(builder: &RepoBuilder, tree: FixtureOid, parents: Vec<FixtureOid>) -> FixtureOid {
    builder.commit(&CommitSpec {
        tree,
        parents,
        author: ada(),
        committer: ada(),
        message: "change\n".to_owned(),
    })
}

/// Writes a root commit for `before` and a child for `after`, opens the adapter,
/// and records the two commits to diff.
fn build_pair(
    world: &mut PatchWorld,
    builder: RepoBuilder,
    before: &[TreeEntry],
    after: &[TreeEntry],
) {
    let parent: FixtureOid = commit_of(&builder, builder.tree(before), Vec::new());
    let child: FixtureOid = commit_of(&builder, builder.tree(after), vec![parent]);
    open_pair(world, builder, Some(parent), to_domain(child));
}

fn open_pair(world: &mut PatchWorld, builder: RepoBuilder, from: Option<FixtureOid>, to: ObjectId) {
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.from_oid = from.map(to_domain);
    world.to_oid = Some(to);
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- accessors ---------------------------------------------------------------

fn repo(world: &PatchWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn rendered(world: &PatchWorld) -> &str {
    world
        .rendered
        .as_deref()
        .expect("take the patch before asserting on it")
}

// --- Givens ------------------------------------------------------------------

#[given("a commit that changes nothing")]
fn given_no_change(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let unchanged: [TreeEntry; 1] = [blob_entry(&builder, "a.txt", Mode::File, b"x\n")];
    build_pair(world, builder, &unchanged, &unchanged);
}

#[given("a commit that modifies one file")]
fn given_modify_one(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"one\n")];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"two\n")];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds a file")]
fn given_add_file(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "keep.txt", Mode::File, b"keep\n")];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "keep.txt", Mode::File, b"keep\n"),
        blob_entry(
            &builder,
            "added.txt",
            Mode::File,
            b"a brand new file here\n",
        ),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that deletes a file")]
fn given_delete_file(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![
        blob_entry(&builder, "keep.txt", Mode::File, b"keep\n"),
        blob_entry(
            &builder,
            "gone.txt",
            Mode::File,
            b"this file will be removed\n",
        ),
    ];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "keep.txt", Mode::File, b"keep\n")];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that makes a file executable")]
fn given_make_executable(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"same bytes\n")];
    let after: Vec<TreeEntry> = vec![blob_entry(
        &builder,
        "a.txt",
        Mode::Executable,
        b"same bytes\n",
    )];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that changes a binary file")]
fn given_change_binary(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(
        &builder,
        "logo.bin",
        Mode::File,
        b"\x00\x01\x02PNG\x00\xff",
    )];
    let after: Vec<TreeEntry> = vec![blob_entry(
        &builder,
        "logo.bin",
        Mode::File,
        b"\x00\x09\x08GIF\x00\xfe",
    )];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that renames a file")]
fn given_rename(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let content: &[u8] = b"some unique content that survives a rename unchanged\n";
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "old.txt", Mode::File, content)];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "new.txt", Mode::File, content)];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds a symlink")]
fn given_add_symlink(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"x\n")];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "a.txt", Mode::File, b"x\n"),
        blob_entry(&builder, "link", Mode::Symlink, b"a.txt"),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds a submodule")]
fn given_add_submodule(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let sub_tree: FixtureOid =
        builder.tree(&[blob_entry(&builder, "readme", Mode::File, b"sub\n")]);
    let sub_commit: FixtureOid = commit_of(&builder, sub_tree, Vec::new());
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"x\n")];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "a.txt", Mode::File, b"x\n"),
        gitlink_entry("sub", sub_commit),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that modifies a latin-1 file")]
fn given_modify_latin1(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"caf\xe9\n")];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"caf\xe9!\n")];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that creates a two-line file")]
fn given_create_two_line(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let tree: FixtureOid = builder.tree(&[blob_entry(
        &builder,
        "two.txt",
        Mode::File,
        b"alpha\nbeta\n",
    )]);
    let commit: FixtureOid = commit_of(&builder, tree, Vec::new());
    // A root commit (no from-side) diffs against the empty tree — a creation.
    open_pair(world, builder, None, to_domain(commit));
}

#[given("a commit and a missing object id")]
fn given_missing(world: &mut PatchWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let tree: FixtureOid = builder.tree(&[blob_entry(&builder, "a.txt", Mode::File, b"x\n")]);
    let commit: FixtureOid = commit_of(&builder, tree, Vec::new());
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.from_oid = Some(to_domain(commit));
    world.to_oid = Some(ObjectId::parse(&"f".repeat(40)).expect("forty f's is a valid object id"));
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- When --------------------------------------------------------------------

#[when("I take the patch")]
fn when_take_patch(world: &mut PatchWorld) {
    let to: ObjectId = world.to_oid.clone().expect("a to-oid must be set");
    let from: Option<ObjectId> = world.from_oid.clone();
    let result: Result<Patch, DomainError> =
        repo(world).patch(from.as_ref(), &to, RenameDetection::RenamesOnly);
    world.rendered = result.as_ref().ok().map(Patch::render);
    world.patch = Some(result);
}

#[when("I read the default abbreviation length")]
fn when_read_abbrev(world: &mut PatchWorld) {
    world.abbrev_length = Some(
        repo(world)
            .abbrev_length()
            .expect("the adapter reports an abbreviation length"),
    );
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r#"^the patch contains "(.*)"$"#)]
fn then_patch_contains(world: &mut PatchWorld, fragment: String) {
    assert!(
        rendered(world).contains(&fragment),
        "expected patch to contain {fragment:?}, got:\n{}",
        rendered(world)
    );
}

#[then(regex = r#"^the patch does not contain "(.*)"$"#)]
fn then_patch_not_contains(world: &mut PatchWorld, fragment: String) {
    assert!(
        !rendered(world).contains(&fragment),
        "expected patch not to contain {fragment:?}, got:\n{}",
        rendered(world)
    );
}

#[then("the patch is empty")]
fn then_patch_is_empty(world: &mut PatchWorld) {
    assert_eq!(rendered(world), "");
}

#[then("the patch fails to find an object")]
fn then_patch_not_found(world: &mut PatchWorld) {
    let result: &Result<Patch, DomainError> = world.patch.as_ref().expect("take the patch first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[then(regex = r#"^the default abbreviation length is (\d+)$"#)]
fn then_abbrev_length_is(world: &mut PatchWorld, expected: usize) {
    assert_eq!(
        world
            .abbrev_length
            .expect("read the abbreviation length first"),
        expected
    );
}

#[then("the hunk text is:")]
fn then_hunk_text_is(world: &mut PatchWorld, step: &Step) {
    let expected: &str = step
        .docstring
        .as_deref()
        .expect("scenario must supply a docstring")
        .trim_matches('\n');
    // Slice from the first `@@` so the assertion is over the hunk body, not the
    // non-deterministic `index` blob ids above it.
    let rendered: &str = rendered(world);
    let at: usize = rendered.find("@@").expect("a hunk header");
    assert_eq!(rendered[at..].trim_end_matches('\n'), expected);
}

#[tokio::main]
async fn main() {
    PatchWorld::run("features/patch").await;
}
