//! Golden differential conformance for gitweb-rs's format-stable endpoints.
//!
//! The `Given` rebuilds the parity corpus with gix — the same builder the
//! goldens were captured over, so object ids match. The `When` reads our
//! format-stable output through the real adapter and loads the captured
//! reference. The `Then` asserts the bytes are identical, with no branching in
//! any step body. cucumber supplies its own `main`, so this target sets
//! `harness = false` in Cargo.toml.

use cucumber::{World, given, then, when};
use tempfile::TempDir;

use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::content_type::PlainHeaders;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::ObjectId as FixtureOid;
use gitweb_git::GixRepository;
use gitweb_parity::corpus::{self, Corpus};
use gitweb_parity::golden::Golden;

#[derive(Debug, Default, World)]
struct GoldenWorld {
    /// Owns the corpus directory; dropped (and deleted) with the world.
    tempdir: Option<TempDir>,
    corpus: Option<Corpus>,
    repo: Option<GixRepository>,
    served: Option<Blob>,
    /// The Content-Type and Content-Disposition our `blob_plain` endpoint derives
    /// for the served blob — the exact headers the use case and handler emit.
    headers: Option<PlainHeaders>,
    golden: Option<Golden>,
}

// --- accessors ---------------------------------------------------------------

fn corpus(world: &GoldenWorld) -> &Corpus {
    world.corpus.as_ref().expect("build the corpus first")
}

fn repo(world: &GoldenWorld) -> &GixRepository {
    world
        .repo
        .as_ref()
        .expect("open the corpus repository first")
}

fn served(world: &GoldenWorld) -> &Blob {
    world.served.as_ref().expect("serve a blob first")
}

fn headers(world: &GoldenWorld) -> &PlainHeaders {
    world.headers.as_ref().expect("serve a blob first")
}

fn golden(world: &GoldenWorld) -> &Golden {
    world.golden.as_ref().expect("load a golden first")
}

/// Converts a fixture (gix) object id into the domain's, for the adapter read.
fn to_domain(oid: FixtureOid) -> ObjectId {
    ObjectId::parse(&oid.to_string()).expect("a git object id is valid hex")
}

// --- Given -------------------------------------------------------------------

#[given("the parity corpus")]
fn given_corpus(world: &mut GoldenWorld) {
    let dir: TempDir = tempfile::tempdir().expect("a temp dir for the corpus");
    let built: Corpus = corpus::build(dir.path());
    let repo: GixRepository =
        GixRepository::open(&built.repo_path).expect("open the corpus repository");
    world.tempdir = Some(dir);
    world.corpus = Some(built);
    world.repo = Some(repo);
}

// --- When --------------------------------------------------------------------

#[when(regex = r#"^I serve the "([^"]*)" blob plain$"#)]
fn serve_blob_plain(world: &mut GoldenWorld, name: String) {
    let oid: ObjectId = to_domain(corpus(world).blob(&name));
    let file_name: String = corpus(world).file_name(&name).to_owned();
    let blob: Blob = repo(world).find_blob(&oid).expect("read the corpus blob");
    // Exactly what the use case derives: gitweb's default config — no configured
    // text charset, XSS prevention off — so the bytes serve inline by tree path.
    let headers: PlainHeaders = PlainHeaders::resolve(
        Some(&file_name),
        blob.is_binary(),
        oid.as_str(),
        None,
        false,
    );
    world.served = Some(blob);
    world.headers = Some(headers);
    world.golden = Some(Golden::load(&format!("blob_plain/{name}")));
}

// --- Then --------------------------------------------------------------------

#[then("its body matches gitweb's reference output")]
fn body_matches(world: &mut GoldenWorld) {
    assert_eq!(served(world).bytes(), golden(world).body());
}

#[then("its media type matches gitweb's")]
fn media_type_matches(world: &mut GoldenWorld) {
    let full: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    // gitweb's CGI.pm bolts a `; charset=ISO-8859-1` onto every type; the media
    // type is the part before it, which is what gitweb itself chose.
    let base: &str = full
        .split_once(';')
        .map_or(full, |(media, _): (&str, &str)| media)
        .trim();
    assert_eq!(headers(world).content_type(), base);
}

#[then("its content disposition matches gitweb's")]
fn content_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(headers(world).content_disposition(), theirs);
}

#[then("the reference declares a Content-Type header")]
fn declares_content_type(world: &mut GoldenWorld) {
    assert!(golden(world).header("Content-Type").is_some());
}

#[tokio::main]
async fn main() {
    GoldenWorld::run("features/golden").await;
}
