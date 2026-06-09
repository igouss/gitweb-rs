//! Gherkin conformance for the gix adapter's `archive` operation.
//!
//! The `Given` builds a deterministic fixture with gix (no git binary): one
//! commit whose tree mixes a text file, a symlink, an executable, a file inside
//! a subtree, and a gitlink — so the snapshot has every entry kind to include,
//! recurse into, or skip, in gix's stable breadth-first order. The `When` drives
//! [`Repository::archive`] for a format and stores the raw bytes (or the
//! [`Result`], for the not-a-tree case); the `Then` decodes the bytes back —
//! gunzip / bunzip2 / unxz to the tar, or opens the zip — and asserts entry
//! representation, format well-formedness, and byte-for-byte reproducibility,
//! with no branching in the step bodies (the format/decoder match lives in
//! support helpers).
//!
//! A separate conformance target with its own `World`. cucumber supplies its own
//! `main`, so the target sets `harness = false` in Cargo.toml.

use std::collections::BTreeMap;
use std::io::{Cursor, Read};

use cucumber::{World, given, then, when};

use gitweb_domain::error::DomainError;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::{ArchiveFormat, ArchiveOptions, Repository};
use gitweb_fixtures::{CommitSpec, Identity, Mode, ObjectId as FixtureOid, RepoBuilder, TreeEntry};
use gitweb_git::GixRepository;

#[derive(Debug, Default, World)]
struct ArchiveWorld {
    builder: Option<RepoBuilder>,
    repo: Option<GixRepository>,
    named: BTreeMap<String, ObjectId>,
    result: Option<Result<Vec<u8>, DomainError>>,
    format: Option<ArchiveFormat>,
    second: Option<Vec<u8>>,
    second_format: Option<ArchiveFormat>,
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

/// Converts a fixture (gix) object id into the domain's, for the adapter call.
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

/// The mixed-entries fixture documented in the feature: one commit whose tree
/// holds a text file, a symlink, a subtree with a file, an executable, and a
/// gitlink. The gitlink points at a real (empty-tree) commit so the tree is
/// well-formed; the symlink's blob content is its target path.
fn build_mixed(world: &mut ArchiveWorld) {
    let builder: RepoBuilder = RepoBuilder::init();

    let empty_tree: FixtureOid = builder.tree(&[]);
    let submodule: FixtureOid = builder.commit(&CommitSpec {
        tree: empty_tree,
        parents: Vec::new(),
        author: who(),
        committer: who(),
        message: "submodule head\n".to_owned(),
    });

    let nested: FixtureOid = builder.tree(&[entry(&builder, "inner.txt", Mode::File, b"deep\n")]);

    let greeting: FixtureOid = builder.blob(b"hello\n");
    let root: FixtureOid = builder.tree(&[
        TreeEntry {
            name: "greeting.txt".to_owned(),
            mode: Mode::File,
            oid: greeting,
        },
        entry(&builder, "link", Mode::Symlink, b"greeting.txt"),
        TreeEntry {
            name: "nested".to_owned(),
            mode: Mode::Directory,
            oid: nested,
        },
        entry(
            &builder,
            "run.sh",
            Mode::Executable,
            b"#!/bin/sh\necho hi\n",
        ),
        TreeEntry {
            name: "submod".to_owned(),
            mode: Mode::Gitlink,
            oid: submodule,
        },
    ]);

    open_with(world, builder, &[("tree", root), ("blob", greeting)]);
}

/// A fixture whose archived tree is empty: one repository holding the empty tree.
fn build_empty(world: &mut ArchiveWorld) {
    let builder: RepoBuilder = RepoBuilder::init();
    let root: FixtureOid = builder.tree(&[]);
    open_with(world, builder, &[("tree", root)]);
}

/// Opens the adapter over the freshly built fixture and records named ids.
fn open_with(world: &mut ArchiveWorld, builder: RepoBuilder, names: &[(&str, FixtureOid)]) {
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

fn repo(world: &ArchiveWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("a repository must be opened first")
}

fn oid(world: &ArchiveWorld, name: &str) -> ObjectId {
    world
        .named
        .get(name)
        .cloned()
        .unwrap_or_else(|| panic!("no fixture object named {name}"))
}

fn primary_bytes(world: &ArchiveWorld) -> &[u8] {
    world
        .result
        .as_ref()
        .expect("archive the tree first")
        .as_ref()
        .expect("the archive succeeded")
}

fn primary_format(world: &ArchiveWorld) -> ArchiveFormat {
    world.format.expect("an archive format must be recorded")
}

// --- format parsing and decoding (support, not test logic) -------------------

/// The bare archive options this slice uses: no top-level directory and a fixed
/// epoch, so the bytes stay prefix-less and reproducible. The prefix and the
/// commit-time stamp are the snapshot endpoint's concern, exercised there.
fn bare_options() -> ArchiveOptions {
    ArchiveOptions {
        prefix: String::new(),
        modification_time: 0,
    }
}

/// Parses a feature's format token into the port's [`ArchiveFormat`].
fn parse_format(token: &str) -> ArchiveFormat {
    match token {
        "tgz" => ArchiveFormat::TarGz,
        "tbz2" => ArchiveFormat::TarBz2,
        "txz" => ArchiveFormat::TarXz,
        "zip" => ArchiveFormat::Zip,
        other => panic!("unknown archive format {other}"),
    }
}

/// Decompresses a tar-family archive back to its raw tar bytes. The compressors
/// are the inverse of the adapter's: gunzip, bunzip2, unxz. Calling it on a zip
/// is a test bug.
fn to_tar_bytes(bytes: &[u8], format: ArchiveFormat) -> Vec<u8> {
    let mut out: Vec<u8> = Vec::new();
    match format {
        ArchiveFormat::TarGz => {
            let mut dec: flate2::read::GzDecoder<Cursor<&[u8]>> =
                flate2::read::GzDecoder::new(Cursor::new(bytes));
            dec.read_to_end(&mut out).expect("gunzip the archive");
        }
        ArchiveFormat::TarBz2 => {
            let mut dec: bzip2::read::BzDecoder<Cursor<&[u8]>> =
                bzip2::read::BzDecoder::new(Cursor::new(bytes));
            dec.read_to_end(&mut out).expect("bunzip2 the archive");
        }
        ArchiveFormat::TarXz => {
            let mut dec: lzma_rust2::XzReader<Cursor<&[u8]>> =
                lzma_rust2::XzReader::new(Cursor::new(bytes), false);
            dec.read_to_end(&mut out).expect("unxz the archive");
        }
        ArchiveFormat::Zip => panic!("to_tar_bytes called on a zip archive"),
    }
    out
}

/// One archived file, read back from the tar.
#[derive(Debug)]
struct ArchivedFile {
    path: String,
    kind: tar::EntryType,
    mode: u32,
    link_target: Option<String>,
    data: Vec<u8>,
}

/// Parses every entry of a raw tar into the order it appears.
fn parse_tar(tar_bytes: &[u8]) -> Vec<ArchivedFile> {
    let mut archive: tar::Archive<Cursor<&[u8]>> = tar::Archive::new(Cursor::new(tar_bytes));
    archive
        .entries()
        .expect("read tar entries")
        .map(|item: std::io::Result<tar::Entry<'_, Cursor<&[u8]>>>| {
            let mut item: tar::Entry<'_, Cursor<&[u8]>> = item.expect("a tar entry");
            let kind: tar::EntryType = item.header().entry_type();
            let mode: u32 = item.header().mode().expect("entry mode");
            let path: String = item
                .path()
                .expect("entry path")
                .to_string_lossy()
                .into_owned();
            let link_target: Option<String> = item.link_name().expect("entry link name").map(
                |target: std::borrow::Cow<'_, std::path::Path>| {
                    target.to_string_lossy().into_owned()
                },
            );
            let mut data: Vec<u8> = Vec::new();
            item.read_to_end(&mut data).expect("read entry data");
            ArchivedFile {
                path,
                kind,
                mode,
                link_target,
                data,
            }
        })
        .collect()
}

/// The files of the primary archive, decoded by its recorded format.
fn archived_files(world: &ArchiveWorld) -> Vec<ArchivedFile> {
    let tar_bytes: Vec<u8> = to_tar_bytes(primary_bytes(world), primary_format(world));
    parse_tar(&tar_bytes)
}

/// Finds an archived file by exact path.
fn find<'a>(files: &'a [ArchivedFile], name: &str) -> Option<&'a ArchivedFile> {
    files.iter().find(|f: &&ArchivedFile| f.path == name)
}

/// Asserts an archive decodes cleanly in its format — the well-formedness check.
fn assert_well_formed(bytes: &[u8], format: ArchiveFormat) {
    match format {
        ArchiveFormat::Zip => {
            zip::ZipArchive::new(Cursor::new(bytes.to_vec())).expect("open the zip archive");
        }
        tar_family => {
            let _files: Vec<ArchivedFile> = parse_tar(&to_tar_bytes(bytes, tar_family));
        }
    }
}

/// `644` as octal -> the permission bits the tar header carries.
fn octal(digits: &str) -> u32 {
    u32::from_str_radix(digits, 8).expect("octal mode")
}

/// Turns a feature literal like `hello\n` into the bytes it denotes.
fn unescape(text: &str) -> Vec<u8> {
    text.replace("\\n", "\n").into_bytes()
}

// --- Givens ------------------------------------------------------------------

#[given("a tree of mixed entries")]
fn given_mixed(world: &mut ArchiveWorld) {
    build_mixed(world);
}

#[given("an empty tree")]
fn given_empty(world: &mut ArchiveWorld) {
    build_empty(world);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r"^I archive the tree as (\w+)$")]
fn archive_tree(world: &mut ArchiveWorld, format: String) {
    let format: ArchiveFormat = parse_format(&format);
    let tree: ObjectId = oid(world, "tree");
    world.result = Some(repo(world).archive(&tree, format, &bare_options()));
    world.format = Some(format);
}

#[when(regex = r"^I archive the tree as (\w+) twice$")]
fn archive_tree_twice(world: &mut ArchiveWorld, format: String) {
    let format: ArchiveFormat = parse_format(&format);
    let tree: ObjectId = oid(world, "tree");
    let first: Vec<u8> = repo(world)
        .archive(&tree, format, &bare_options())
        .expect("the first archive succeeded");
    let second: Vec<u8> = repo(world)
        .archive(&tree, format, &bare_options())
        .expect("the second archive succeeded");
    world.result = Some(Ok(first));
    world.format = Some(format);
    world.second = Some(second);
    world.second_format = Some(format);
}

#[when(regex = r"^I archive the tree as tgz and as (\w+)$")]
fn archive_tree_two_formats(world: &mut ArchiveWorld, other: String) {
    let other: ArchiveFormat = parse_format(&other);
    let tree: ObjectId = oid(world, "tree");
    let gzipped: Vec<u8> = repo(world)
        .archive(&tree, ArchiveFormat::TarGz, &bare_options())
        .expect("the tgz archive succeeded");
    let compressed: Vec<u8> = repo(world)
        .archive(&tree, other, &bare_options())
        .expect("the other archive succeeded");
    world.result = Some(Ok(gzipped));
    world.format = Some(ArchiveFormat::TarGz);
    world.second = Some(compressed);
    world.second_format = Some(other);
}

#[when(regex = r"^I archive the blob as (\w+)$")]
fn archive_blob(world: &mut ArchiveWorld, format: String) {
    let format: ArchiveFormat = parse_format(&format);
    let blob: ObjectId = oid(world, "blob");
    world.result = Some(repo(world).archive(&blob, format, &bare_options()));
    world.format = Some(format);
}

// --- Thens -------------------------------------------------------------------

#[then(regex = r"^the archive is a well-formed (\w+)$")]
fn well_formed(world: &mut ArchiveWorld, format: String) {
    assert_well_formed(primary_bytes(world), parse_format(&format));
}

#[then("the two archives are byte-identical")]
fn byte_identical(world: &mut ArchiveWorld) {
    let second: &[u8] = world.second.as_ref().expect("a second archive");
    assert_eq!(primary_bytes(world), second);
}

#[then(regex = r"^the archive holds (\d+) files$")]
fn holds_files(world: &mut ArchiveWorld, count: usize) {
    assert_eq!(archived_files(world).len(), count);
}

#[then(regex = r#"^the archived files are "(.*)"$"#)]
fn files_in_order(world: &mut ArchiveWorld, expected: String) {
    let files: Vec<ArchivedFile> = archived_files(world);
    let actual: String = files
        .iter()
        .map(|f: &ArchivedFile| f.path.clone())
        .collect::<Vec<String>>()
        .join(", ");
    assert_eq!(actual, expected);
}

#[then(regex = r#"^archived file "([^"]*)" is a regular file of mode (\d+) holding "(.*)"$"#)]
fn regular_with_content(world: &mut ArchiveWorld, name: String, mode: String, content: String) {
    let files: Vec<ArchivedFile> = archived_files(world);
    let file: &ArchivedFile = find(&files, &name).expect("the named file is archived");
    assert_eq!(file.kind, tar::EntryType::Regular);
    assert_eq!(file.mode, octal(&mode));
    assert_eq!(file.data, unescape(&content));
}

#[then(regex = r#"^archived file "([^"]*)" is a regular file of mode (\d+)$"#)]
fn regular_with_mode(world: &mut ArchiveWorld, name: String, mode: String) {
    let files: Vec<ArchivedFile> = archived_files(world);
    let file: &ArchivedFile = find(&files, &name).expect("the named file is archived");
    assert_eq!(file.kind, tar::EntryType::Regular);
    assert_eq!(file.mode, octal(&mode));
}

#[then(regex = r#"^archived file "([^"]*)" is a symlink to "([^"]*)"$"#)]
fn symlink_to(world: &mut ArchiveWorld, name: String, target: String) {
    let files: Vec<ArchivedFile> = archived_files(world);
    let file: &ArchivedFile = find(&files, &name).expect("the named file is archived");
    assert_eq!(file.kind, tar::EntryType::Symlink);
    assert_eq!(file.link_target, Some(target));
}

#[then(regex = r#"^the archive has no entry named "([^"]*)"$"#)]
fn no_entry(world: &mut ArchiveWorld, name: String) {
    let files: Vec<ArchivedFile> = archived_files(world);
    assert!(find(&files, &name).is_none());
}

#[then("both decompress to identical tar bytes")]
fn identical_tar(world: &mut ArchiveWorld) {
    let primary: Vec<u8> = to_tar_bytes(primary_bytes(world), primary_format(world));
    let second_format: ArchiveFormat = world.second_format.expect("a second format");
    let second_bytes: &[u8] = world.second.as_ref().expect("a second archive");
    let second: Vec<u8> = to_tar_bytes(second_bytes, second_format);
    assert_eq!(primary, second);
}

#[then("archiving fails as invalid")]
fn fails_invalid(world: &mut ArchiveWorld) {
    let error: &DomainError = world
        .result
        .as_ref()
        .expect("archive the blob first")
        .as_ref()
        .expect_err("archiving a non-tree must fail");
    assert!(matches!(error, DomainError::Invalid(_)));
}

#[tokio::main]
async fn main() {
    ArchiveWorld::run("features/archive").await;
}
