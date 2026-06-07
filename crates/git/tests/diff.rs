//! Gherkin conformance for the gix adapter's `diff` operation.
//!
//! Each `Given` builds a deterministic before/after pair with gix (no git
//! binary) and records the two commits to diff; the `When` drives
//! `Repository::diff` and stores the raw [`Result`], so the `Then` steps assert
//! per-path statuses, modes, and ids by looking changes up by path — no
//! branching in the step bodies. Copy detection (gitweb's `-C` /
//! `--find-copies-harder`) is exercised here via the detection level the `When`
//! steps pass; combined merge diffs are a separate capability, not exercised.
//!
//! cucumber supplies its own `main`, so this target sets `harness = false`.

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::diff::{Diff, DiffEntry};
use gitweb_domain::model::file_mode::FileMode;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::{RenameDetection, Repository};
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct DiffWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    from_oid: Option<ObjectId>,
    to_oid: Option<ObjectId>,
    diff: Option<Result<Diff, DomainError>>,
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

/// Converts a fixture (gix) object id into the domain's, for assertions.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

/// A tree entry naming a freshly written blob, in the given mode.
fn blob_entry(builder: &RepoBuilder, name: &str, mode: Mode, bytes: &[u8]) -> TreeEntry {
    TreeEntry {
        name: name.to_owned(),
        mode,
        oid: builder.blob(bytes),
    }
}

/// A tree entry recording `commit` as a submodule (gitlink).
fn gitlink_entry(name: &str, commit: FixtureOid) -> TreeEntry {
    TreeEntry {
        name: name.to_owned(),
        mode: Mode::Gitlink,
        oid: commit,
    }
}

/// A commit snapshotting `tree`, with the given parents.
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
    world: &mut DiffWorld,
    builder: RepoBuilder,
    before: &[TreeEntry],
    after: &[TreeEntry],
) {
    let parent: FixtureOid = commit_of(&builder, builder.tree(before), Vec::new());
    let child: FixtureOid = commit_of(&builder, builder.tree(after), vec![parent]);
    open_pair(world, builder, Some(parent), to_domain(child));
}

/// Opens the adapter and records the from/to commits. The from side is optional,
/// so a root commit can be diffed against the empty tree.
fn open_pair(world: &mut DiffWorld, builder: RepoBuilder, from: Option<FixtureOid>, to: ObjectId) {
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.from_oid = from.map(to_domain);
    world.to_oid = Some(to);
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- accessors ---------------------------------------------------------------

fn repo(world: &DiffWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn ok_diff(world: &DiffWorld) -> &Diff {
    world
        .diff
        .as_ref()
        .expect("take the diff first")
        .as_ref()
        .expect("the diff succeeded")
}

fn entry_to<'a>(world: &'a DiffWorld, path: &str) -> &'a DiffEntry {
    ok_diff(world)
        .entries()
        .iter()
        .find(|entry: &&DiffEntry| entry.to_path() == path)
        .unwrap_or_else(|| panic!("no change to {path}"))
}

fn zeros() -> String {
    "0".repeat(40)
}

fn take_diff(world: &mut DiffWorld, detection: RenameDetection) {
    let to: ObjectId = world.to_oid.clone().expect("a to-oid must be set");
    let from: Option<ObjectId> = world.from_oid.clone();
    let result: Result<Diff, DomainError> = repo(world).diff(from.as_ref(), &to, detection);
    world.diff = Some(result);
}

// --- Givens ------------------------------------------------------------------

#[given("a commit that changes nothing")]
fn given_no_change(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let unchanged: [TreeEntry; 1] = [blob_entry(&builder, "a.txt", Mode::File, b"x")];
    build_pair(world, builder, &unchanged, &unchanged);
}

#[given("a commit that modifies one file")]
fn given_modify_one(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"one\n")];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"two\n")];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds, deletes, and modifies files")]
fn given_add_delete_modify(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![
        blob_entry(&builder, "kept.txt", Mode::File, b"kept original\n"),
        blob_entry(
            &builder,
            "gone.txt",
            Mode::File,
            b"this file will be removed\n",
        ),
    ];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "kept.txt", Mode::File, b"kept but changed\n"),
        blob_entry(
            &builder,
            "added.txt",
            Mode::File,
            b"a brand new file here\n",
        ),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that makes a file executable")]
fn given_make_executable(world: &mut DiffWorld) {
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

#[given("a commit that turns a file into a symlink")]
fn given_file_to_symlink(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"hello\n")];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::Symlink, b"target/path")];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds a symlink")]
fn given_add_symlink(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"x")];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "a.txt", Mode::File, b"x"),
        blob_entry(&builder, "link", Mode::Symlink, b"a.txt"),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds a submodule")]
fn given_add_submodule(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let sub_tree: FixtureOid =
        builder.tree(&[blob_entry(&builder, "readme", Mode::File, b"sub\n")]);
    let sub_commit: FixtureOid = commit_of(&builder, sub_tree, Vec::new());
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "a.txt", Mode::File, b"x")];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "a.txt", Mode::File, b"x"),
        gitlink_entry("sub", sub_commit),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that changes a binary file")]
fn given_change_binary(world: &mut DiffWorld) {
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

#[given("a root commit with two files")]
fn given_root_commit(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let tree: FixtureOid = builder.tree(&[
        blob_entry(&builder, "a.txt", Mode::File, b"alpha\n"),
        blob_entry(&builder, "b.txt", Mode::File, b"beta\n"),
    ]);
    let root: FixtureOid = commit_of(&builder, tree, Vec::new());
    open_pair(world, builder, None, to_domain(root));
}

#[given("a commit that renames a file")]
fn given_rename(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let content: &[u8] = b"some unique content that survives a rename unchanged\n";
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "old.txt", Mode::File, content)];
    let after: Vec<TreeEntry> = vec![blob_entry(&builder, "new.txt", Mode::File, content)];
    build_pair(world, builder, &before, &after);
}

/// Six distinct lines, the shared baseline for the copy-detection fixtures.
fn six_lines() -> &'static [u8] {
    b"alpha\nbeta\ngamma\ndelta\nepsilon\nzeta\n"
}

/// An unrelated added file, present only so the commit changes more than one
/// path — gix scans for copies only then (see the feature's note).
fn gate_opener(builder: &RepoBuilder) -> TreeEntry {
    blob_entry(
        builder,
        "other.txt",
        Mode::File,
        b"an unrelated brand new file\nwith its own second line\n",
    )
}

#[given("a commit that copies an unchanged file")]
fn given_copy_unchanged(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let content: &[u8] = six_lines();
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "orig.txt", Mode::File, content)];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "orig.txt", Mode::File, content),
        blob_entry(&builder, "copy.txt", Mode::File, content),
        gate_opener(&builder),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that modifies a file and copies its original content")]
fn given_copy_of_modified(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let original: &[u8] = six_lines();
    let modified: &[u8] = b"alpha\nbeta\ngamma CHANGED\ndelta\nepsilon\nzeta\nextra line\n";
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "orig.txt", Mode::File, original)];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "orig.txt", Mode::File, modified),
        blob_entry(&builder, "copy.txt", Mode::File, original),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that copies a file with small edits")]
fn given_near_copy(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let content: &[u8] = six_lines();
    // Five of six lines survive: well above git's 50% similarity threshold.
    let near: &[u8] = b"alpha\nbeta\ngamma\ndelta\nepsilon\nZETA CHANGED\n";
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "orig.txt", Mode::File, content)];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "orig.txt", Mode::File, content),
        blob_entry(&builder, "near.txt", Mode::File, near),
        gate_opener(&builder),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that adds a file barely resembling another")]
fn given_below_threshold(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let content: &[u8] = six_lines();
    // Only two of six lines are shared: a third, below the 50% threshold.
    let different: &[u8] = b"alpha\nbeta\nuno\ndos\ntres\ncuatro\n";
    let before: Vec<TreeEntry> = vec![blob_entry(&builder, "orig.txt", Mode::File, content)];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "orig.txt", Mode::File, content),
        blob_entry(&builder, "different.txt", Mode::File, different),
        gate_opener(&builder),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that renames one file and copies another")]
fn given_rename_and_copy(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let moved: &[u8] = b"content of the file that moves across a rename\nsecond line\nthird line\n";
    let kept: &[u8] =
        b"content of the file that stays put and is copied\nanother line\nyet another\n";
    let before: Vec<TreeEntry> = vec![
        blob_entry(&builder, "moved.txt", Mode::File, moved),
        blob_entry(&builder, "kept.txt", Mode::File, kept),
    ];
    let after: Vec<TreeEntry> = vec![
        blob_entry(&builder, "renamed.txt", Mode::File, moved),
        blob_entry(&builder, "kept.txt", Mode::File, kept),
        blob_entry(&builder, "copy.txt", Mode::File, kept),
    ];
    build_pair(world, builder, &before, &after);
}

#[given("a commit that changes a file inside a subdirectory")]
fn given_subdir_change(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let sub_before: FixtureOid = builder.tree(&[blob_entry(&builder, "a.txt", Mode::File, b"1\n")]);
    let sub_after: FixtureOid = builder.tree(&[blob_entry(&builder, "a.txt", Mode::File, b"2\n")]);
    let before: Vec<TreeEntry> = vec![TreeEntry {
        name: "dir".to_owned(),
        mode: Mode::Directory,
        oid: sub_before,
    }];
    let after: Vec<TreeEntry> = vec![TreeEntry {
        name: "dir".to_owned(),
        mode: Mode::Directory,
        oid: sub_after,
    }];
    build_pair(world, builder, &before, &after);
}

#[given("a commit and a missing object id")]
fn given_missing(world: &mut DiffWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let tree: FixtureOid = builder.tree(&[blob_entry(&builder, "a.txt", Mode::File, b"x")]);
    let commit: FixtureOid = commit_of(&builder, tree, Vec::new());
    let repo: GixRepository =
        GixRepository::open(builder.path()).expect("open the fixture repository");
    world.from_oid = Some(to_domain(commit));
    world.to_oid = Some(ObjectId::parse(&"f".repeat(40)).expect("forty f's is a valid object id"));
    world.builder = Some(builder);
    world.repo = Some(repo);
}

// --- Whens -------------------------------------------------------------------

#[when("I diff them")]
fn when_diff_them(world: &mut DiffWorld) {
    take_diff(world, RenameDetection::RenamesOnly);
}

#[when("I diff the root commit")]
fn when_diff_root(world: &mut DiffWorld) {
    take_diff(world, RenameDetection::RenamesOnly);
}

#[when("I diff them detecting copies")]
fn when_diff_copies(world: &mut DiffWorld) {
    take_diff(world, RenameDetection::Copies);
}

#[when("I diff them detecting copies harder")]
fn when_diff_copies_harder(world: &mut DiffWorld) {
    take_diff(world, RenameDetection::CopiesHarder);
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^(\d+) paths? changed$")]
fn paths_changed(world: &mut DiffWorld, count: usize) {
    assert_eq!(ok_diff(world).entries().len(), count);
}

#[then(regex = r#"^the change to "([^"]*)" is "(\w+)"$"#)]
fn change_is(world: &mut DiffWorld, path: String, kind: String) {
    assert_eq!(
        format!("{:?}", entry_to(world, &path).status().kind()),
        kind
    );
}

#[then(regex = r#"^the change to "([^"]*)" has similarity (\d+)$"#)]
fn change_similarity(world: &mut DiffWorld, path: String, score: u8) {
    assert_eq!(entry_to(world, &path).status().similarity(), score);
}

#[then(regex = r#"^the change to "([^"]*)" comes from "([^"]*)"$"#)]
fn change_comes_from(world: &mut DiffWorld, path: String, source: String) {
    assert_eq!(entry_to(world, &path).from_path(), source);
}

#[then(regex = r#"^the from-mode of "([^"]*)" is "(\d+)"$"#)]
fn from_mode_is(world: &mut DiffWorld, path: String, octal: String) {
    let expected: FileMode = FileMode::from_octal(&octal).expect("a valid octal mode");
    assert_eq!(entry_to(world, &path).from_mode(), expected);
}

#[then(regex = r#"^the to-mode of "([^"]*)" is "(\d+)"$"#)]
fn to_mode_is(world: &mut DiffWorld, path: String, octal: String) {
    let expected: FileMode = FileMode::from_octal(&octal).expect("a valid octal mode");
    assert_eq!(entry_to(world, &path).to_mode(), expected);
}

#[then(regex = r#"^the from-oid of "([^"]*)" is all zeros$"#)]
fn from_oid_zeros(world: &mut DiffWorld, path: String) {
    assert_eq!(entry_to(world, &path).from_oid().as_str(), zeros());
}

#[then(regex = r#"^the to-oid of "([^"]*)" is all zeros$"#)]
fn to_oid_zeros(world: &mut DiffWorld, path: String) {
    assert_eq!(entry_to(world, &path).to_oid().as_str(), zeros());
}

#[then(regex = r#"^no path "([^"]*)" changed$"#)]
fn no_path_changed(world: &mut DiffWorld, path: String) {
    let found: Option<&DiffEntry> = ok_diff(world)
        .entries()
        .iter()
        .find(|entry: &&DiffEntry| entry.to_path() == path);
    assert!(found.is_none());
}

#[then("the diff fails to find an object")]
fn diff_not_found(world: &mut DiffWorld) {
    let result: &Result<Diff, DomainError> = world.diff.as_ref().expect("take the diff first");
    assert!(matches!(result, Err(DomainError::NotFound(_))));
}

#[tokio::main]
async fn main() {
    DiffWorld::run("features/diff").await;
}
