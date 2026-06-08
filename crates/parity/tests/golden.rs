//! Golden differential conformance for gitweb-rs's format-stable endpoints.
//!
//! The `Given` rebuilds the parity corpus with gix — the same builder the
//! goldens were captured over, so object ids match. The `When` reads our
//! format-stable output through the real adapter and loads the captured
//! reference. The `Then` asserts the bytes are identical, with no branching in
//! any step body. cucumber supplies its own `main`, so this target sets
//! `harness = false` in Cargo.toml.

use std::path::PathBuf;

use cucumber::{World, given, then, when};
use tempfile::TempDir;

use gitweb_domain::model::blob::Blob;
use gitweb_domain::model::content_type::PlainHeaders;
use gitweb_domain::model::feed::Feed;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::model::timestamp::Timestamp;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::feed::assemble_feed;
use gitweb_fixtures::ObjectId as FixtureOid;
use gitweb_git::{GixProjectStore, GixRepository};
use gitweb_parity::corpus::{self, Corpus};
use gitweb_parity::golden::Golden;
use gitweb_render::feed::{atom, rss};
use gitweb_web::handlers::feed::{FeedFormat, FeedRequest, assemble_feed_view};

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
    /// The serialized feed body, its media type, and its Last-Modified value —
    /// the exact bytes and headers the feed handler emits over the corpus.
    feed_body: Option<String>,
    feed_media_type: Option<&'static str>,
    feed_last_modified: Option<String>,
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

/// The request-time clock the feed window is computed against. The corpus has a
/// single commit (index 0), which the window always keeps regardless of `now`,
/// and `now` appears nowhere in the body, so any value yields the same feed.
const FEED_NOW: i64 = 1_700_000_000;

#[when(regex = r#"^I serve the "(rss|atom)" feed$"#)]
fn serve_feed(world: &mut GoldenWorld, format_name: String) {
    let project_root: PathBuf = corpus(world).project_root.clone();
    let store: GixProjectStore = GixProjectStore::new(project_root);
    let repo: Box<dyn Repository> = store
        .open(Corpus::PROJECT)
        .expect("open the corpus repository");
    let info: ProjectInfo = store
        .info(Corpus::PROJECT)
        .expect("read the corpus project info");
    let feed: Feed =
        assemble_feed(repo.as_ref(), None, None, FEED_NOW).expect("assemble the corpus feed");
    let format: FeedFormat = match format_name.as_str() {
        "atom" => FeedFormat::Atom,
        _ => FeedFormat::Rss,
    };
    // gitweb's default config: the built-in settings, the captured generator
    // composite, and the CGI default site URL (no SERVER_NAME -> http://localhost).
    let settings: Settings = Settings::builtin();
    let generator: String = read_generator();
    let request: FeedRequest<'_> = FeedRequest {
        format,
        project: Corpus::PROJECT,
        hash: None,
        file_name: None,
    };
    let view = assemble_feed_view(
        &feed,
        &request,
        &info,
        &settings,
        "http://localhost",
        &generator,
    );
    let body: String = match format {
        FeedFormat::Rss => rss(&view),
        FeedFormat::Atom => atom(&view),
    };
    world.feed_body = Some(body);
    world.feed_media_type = Some(format.content_type_header());
    world.feed_last_modified = feed.latest().map(Timestamp::rfc2822);
    world.golden = Some(Golden::load(&format!("feed/{format_name}")));
}

/// The generator version composite captured alongside the feed goldens — the
/// `$version/$git_version` gitweb stamped the body with. Injecting it keeps the
/// rest of the body a byte-exact comparison (git's version is not part of the
/// feed format and is not byte-stable across machines).
fn read_generator() -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("goldens/feed/generator");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err: std::io::Error| panic!("read {}: {err}", path.display()))
}

/// The serialized feed body, or a panic if none was served.
fn feed_body(world: &GoldenWorld) -> &str {
    world.feed_body.as_deref().expect("serve a feed first")
}

// --- Then --------------------------------------------------------------------

#[then("its body matches gitweb's reference output")]
fn body_matches(world: &mut GoldenWorld) {
    assert_eq!(served(world).bytes(), golden(world).body());
}

#[then("the feed body matches gitweb's reference output")]
fn feed_body_matches(world: &mut GoldenWorld) {
    assert_eq!(feed_body(world).as_bytes(), golden(world).body());
}

#[then("the feed media type matches gitweb's")]
fn feed_media_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-charset => 'utf-8'` on the feed, so the whole Content-Type
    // (with charset) is its own choice and matches ours exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!(world.feed_media_type, Some(theirs));
}

#[then("the feed last-modified matches gitweb's")]
fn feed_last_modified_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Last-Modified")
        .expect("gitweb declares a Last-Modified");
    assert_eq!(world.feed_last_modified.as_deref(), Some(theirs));
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
