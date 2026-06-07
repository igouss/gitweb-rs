//! Gherkin conformance for the deterministic fixture builder.
//!
//! Golden object ids anchored on git's canonical empty-blob / empty-tree ids,
//! plus byte-exact round trips and ref resolution. cucumber supplies its own
//! `main`, so this target sets `harness = false` in Cargo.toml.

use std::collections::BTreeMap;

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use gitweb_fixtures::{
    CommitSpec, Identity, Mode, ObjectId, RepoBuilder, TagSpec, TargetKind, TreeEntry,
};

#[derive(Debug, Default, World)]
struct FixtureWorld {
    builder: Option<RepoBuilder>,
    identity: Option<Identity>,
    named: BTreeMap<String, ObjectId>,
    last_oid: Option<ObjectId>,
    read_back: Option<Vec<u8>>,
    resolved: Option<Option<ObjectId>>,
}

fn repo(world: &FixtureWorld) -> &RepoBuilder {
    world
        .builder
        .as_ref()
        .expect("a fixture repository must be created first")
}

fn ident(world: &FixtureWorld) -> Identity {
    world
        .identity
        .clone()
        .expect("the fixture identity must be set first")
}

fn lookup(world: &FixtureWorld, name: &str) -> ObjectId {
    *world
        .named
        .get(name)
        .unwrap_or_else(|| panic!("no object named {name}"))
}

fn parse_identity(spec: &str, epoch: i64) -> Identity {
    let (name, rest): (&str, &str) = spec.split_once(" <").expect("a \"Name <email>\" identity");
    let email: &str = rest.strip_suffix('>').expect("a trailing > on the email");
    Identity {
        name: name.to_owned(),
        email: email.to_owned(),
        epoch_seconds: epoch,
        timezone_offset_seconds: 0,
    }
}

fn unescape(text: &str) -> Vec<u8> {
    text.replace("\\0", "\0")
        .replace("\\t", "\t")
        .replace("\\n", "\n")
        .into_bytes()
}

fn parse_hex_bytes(text: &str) -> Vec<u8> {
    text.split_whitespace()
        .map(|byte: &str| u8::from_str_radix(byte, 16).expect("a valid hex byte"))
        .collect()
}

fn mode_of(text: &str) -> Mode {
    match text {
        "file" => Mode::File,
        "executable" => Mode::Executable,
        "symlink" => Mode::Symlink,
        "directory" => Mode::Directory,
        "gitlink" => Mode::Gitlink,
        other => panic!("unknown file mode: {other}"),
    }
}

fn parse_oid(hex: &str) -> ObjectId {
    ObjectId::from_hex(hex.as_bytes()).expect("a 40-char hex object id")
}

// --- Givens: build the repository and its named objects ----------------------

#[given("a fresh fixture repository")]
fn fresh_repo(world: &mut FixtureWorld) {
    world.builder = Some(RepoBuilder::init());
}

#[given(regex = r#"^the fixture identity "([^"]*)" at epoch (\d+)$"#)]
fn set_identity(world: &mut FixtureWorld, spec: String, epoch: i64) {
    world.identity = Some(parse_identity(&spec, epoch));
}

#[given(regex = r#"^a blob containing "([^"]*)" stored as "([^"]*)"$"#)]
fn given_named_blob(world: &mut FixtureWorld, content: String, name: String) {
    let oid: ObjectId = repo(world).blob(&unescape(&content));
    world.named.insert(name, oid);
}

#[given(regex = r#"^an empty tree stored as "([^"]*)"$"#)]
fn given_named_empty_tree(world: &mut FixtureWorld, name: String) {
    let oid: ObjectId = repo(world).tree(&[]);
    world.named.insert(name, oid);
}

#[given(regex = r#"^a placeholder object id "([^"]*)" stored as "([^"]*)"$"#)]
fn given_placeholder_oid(world: &mut FixtureWorld, hex: String, name: String) {
    world.named.insert(name, parse_oid(&hex));
}

#[given(regex = r#"^a tree with the file "([^"]*)" pointing at "([^"]*)" stored as "([^"]*)"$"#)]
fn given_named_one_file_tree(world: &mut FixtureWorld, file: String, target: String, name: String) {
    let entry: TreeEntry = TreeEntry {
        name: file,
        mode: Mode::File,
        oid: lookup(world, &target),
    };
    let oid: ObjectId = repo(world).tree(&[entry]);
    world.named.insert(name, oid);
}

#[given(regex = r#"^a commit "([^"]*)" with tree "([^"]*)" and message "([^"]*)"$"#)]
fn given_named_commit(world: &mut FixtureWorld, name: String, tree: String, message: String) {
    let who: Identity = ident(world);
    let spec: CommitSpec = CommitSpec {
        tree: lookup(world, &tree),
        parents: Vec::new(),
        author: who.clone(),
        committer: who,
        message: String::from_utf8(unescape(&message)).expect("a UTF-8 message"),
    };
    let oid: ObjectId = repo(world).commit(&spec);
    world.named.insert(name, oid);
}

// --- Whens: write the object under test --------------------------------------

#[when("I write a blob with no bytes")]
fn write_empty_blob(world: &mut FixtureWorld) {
    world.last_oid = Some(repo(world).blob(&[]));
}

#[when(regex = r#"^I write a blob containing "([^"]*)"$"#)]
fn write_text_blob(world: &mut FixtureWorld, content: String) {
    world.last_oid = Some(repo(world).blob(&unescape(&content)));
}

#[when(regex = r#"^I write a blob of the bytes "([^"]*)"$"#)]
fn write_byte_blob(world: &mut FixtureWorld, hex: String) {
    world.last_oid = Some(repo(world).blob(&parse_hex_bytes(&hex)));
}

#[when("I write a tree with no entries")]
fn write_empty_tree(world: &mut FixtureWorld) {
    world.last_oid = Some(repo(world).tree(&[]));
}

#[when(regex = r#"^I write a tree with the file "([^"]*)" pointing at "([^"]*)"$"#)]
fn write_one_file_tree(world: &mut FixtureWorld, file: String, target: String) {
    let entry: TreeEntry = TreeEntry {
        name: file,
        mode: Mode::File,
        oid: lookup(world, &target),
    };
    world.last_oid = Some(repo(world).tree(&[entry]));
}

#[when("I write a tree with entries:")]
fn write_tree_with_entries(world: &mut FixtureWorld, step: &Step) {
    let table: &cucumber::gherkin::Table = step.table.as_ref().expect("a data table of entries");
    let entries: Vec<TreeEntry> = table
        .rows
        .iter()
        .skip(1)
        .map(|row: &Vec<String>| TreeEntry {
            name: row[0].clone(),
            mode: mode_of(&row[1]),
            oid: lookup(world, &row[2]),
        })
        .collect();
    world.last_oid = Some(repo(world).tree(&entries));
}

#[when(regex = r#"^I write a commit with tree "([^"]*)" and message "([^"]*)"$"#)]
fn write_root_commit(world: &mut FixtureWorld, tree: String, message: String) {
    let who: Identity = ident(world);
    let spec: CommitSpec = CommitSpec {
        tree: lookup(world, &tree),
        parents: Vec::new(),
        author: who.clone(),
        committer: who,
        message: String::from_utf8(unescape(&message)).expect("a UTF-8 message"),
    };
    world.last_oid = Some(repo(world).commit(&spec));
}

#[when(
    regex = r#"^I write a commit with tree "([^"]*)", parents "([^"]*)" and "([^"]*)", and message "([^"]*)"$"#
)]
fn write_merge_commit(
    world: &mut FixtureWorld,
    tree: String,
    first: String,
    second: String,
    message: String,
) {
    let who: Identity = ident(world);
    let spec: CommitSpec = CommitSpec {
        tree: lookup(world, &tree),
        parents: vec![lookup(world, &first), lookup(world, &second)],
        author: who.clone(),
        committer: who,
        message: String::from_utf8(unescape(&message)).expect("a UTF-8 message"),
    };
    world.last_oid = Some(repo(world).commit(&spec));
}

#[when(regex = r#"^I set HEAD to branch "([^"]*)"$"#)]
fn when_set_head(world: &mut FixtureWorld, branch: String) {
    repo(world).set_head(&branch);
}

#[given(regex = r#"^HEAD is set to branch "([^"]*)"$"#)]
fn given_set_head(world: &mut FixtureWorld, branch: String) {
    repo(world).set_head(&branch);
}

#[when(regex = r#"^I create branch "([^"]*)" at "([^"]*)"$"#)]
fn create_branch(world: &mut FixtureWorld, name: String, target: String) {
    let oid: ObjectId = lookup(world, &target);
    repo(world).branch(&name, oid);
}

#[when(regex = r#"^I create lightweight tag "([^"]*)" at "([^"]*)"$"#)]
fn create_lightweight_tag(world: &mut FixtureWorld, name: String, target: String) {
    let oid: ObjectId = lookup(world, &target);
    repo(world).lightweight_tag(&name, oid);
}

#[when(regex = r#"^I create annotated tag "([^"]*)" on "([^"]*)" with message "([^"]*)"$"#)]
fn create_annotated_tag(world: &mut FixtureWorld, name: String, target: String, message: String) {
    let spec: TagSpec = TagSpec {
        name,
        target: lookup(world, &target),
        target_kind: TargetKind::Commit,
        tagger: ident(world),
        message: String::from_utf8(unescape(&message)).expect("a UTF-8 message"),
    };
    world.last_oid = Some(repo(world).annotated_tag(&spec));
}

// --- Thens: assert ids, bytes, and ref resolution ----------------------------

#[then(regex = r#"^the new object id is "([^"]*)"$"#)]
fn new_object_id_is(world: &mut FixtureWorld, expected: String) {
    assert_eq!(world.last_oid, Some(parse_oid(&expected)));
}

#[then(regex = r#"^reading that blob back yields the bytes "([^"]*)"$"#)]
fn read_back_yields(world: &mut FixtureWorld, hex: String) {
    let oid: ObjectId = world.last_oid.expect("a blob must be written first");
    world.read_back = Some(repo(world).read_blob(oid));
    assert_eq!(
        world.read_back.as_deref(),
        Some(parse_hex_bytes(&hex).as_slice())
    );
}

#[then(regex = r#"^"([^"]*)" does not resolve$"#)]
fn does_not_resolve(world: &mut FixtureWorld, full_ref: String) {
    world.resolved = Some(repo(world).id_of(&full_ref));
    assert_eq!(world.resolved, Some(None));
}

#[then(regex = r#"^"([^"]*)" resolves to "([^"]*)"$"#)]
fn resolves_to_named(world: &mut FixtureWorld, full_ref: String, target: String) {
    world.resolved = Some(repo(world).id_of(&full_ref));
    assert_eq!(world.resolved, Some(Some(lookup(world, &target))));
}

#[then(regex = r#"^"([^"]*)" resolves to the new object id$"#)]
fn resolves_to_new(world: &mut FixtureWorld, full_ref: String) {
    world.resolved = Some(repo(world).id_of(&full_ref));
    assert_eq!(world.resolved, Some(world.last_oid));
}

#[tokio::main]
async fn main() {
    FixtureWorld::run("features/conformance").await;
}
