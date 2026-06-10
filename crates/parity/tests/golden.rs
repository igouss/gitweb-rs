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
use gitweb_domain::model::blobdiff_plain::BlobdiffPlain;
use gitweb_domain::model::commitdiff_plain::CommitdiffPlain;
use gitweb_domain::model::content_type::PlainHeaders;
use gitweb_domain::model::feed::Feed;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::project_info::ProjectInfo;
use gitweb_domain::model::settings::Settings;
use gitweb_domain::model::timestamp::Timestamp;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_domain::usecase::blobdiff_plain::assemble_blobdiff_plain;
use gitweb_domain::usecase::commitdiff_plain::assemble_commitdiff_plain;
use gitweb_domain::usecase::feed::assemble_feed;
use gitweb_domain::usecase::opml::{Opml, assemble_opml};
use gitweb_domain::usecase::patch::{assemble_patch, assemble_patches};
use gitweb_domain::usecase::project_index::{ProjectIndex, assemble_project_index};
use gitweb_fixtures::ObjectId as FixtureOid;
use gitweb_git::{GixProjectStore, GixRepository};
use gitweb_parity::corpus::{self, Corpus};
use gitweb_parity::golden::Golden;
use gitweb_render::feed::{atom, rss};
use gitweb_render::opml::{OpmlView, opml};
use gitweb_render::project_index::{ProjectIndexView, project_index};
use gitweb_web::handlers::feed::{FeedFormat, FeedRequest, assemble_feed_view};
use gitweb_web::handlers::opml::{OPML_CONTENT_TYPE, OPML_DISPOSITION, assemble_opml_view};
use gitweb_web::handlers::project_index::{
    INDEX_CONTENT_TYPE, INDEX_DISPOSITION, assemble_project_index_view,
};
use gitweb_web::url::self_url;

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
    /// The serialized project_index body — the exact bytes the handler emits over
    /// the corpus.
    index_body: Option<String>,
    /// The serialized opml body — the exact bytes the handler emits over the
    /// corpus.
    opml_body: Option<String>,
    /// The serialized commitdiff_plain body and the Content-Disposition our
    /// endpoint derives — the exact bytes and header the handler emits over the
    /// corpus root commit.
    commitdiff_plain_body: Option<String>,
    commitdiff_plain_disposition: Option<String>,
    /// The serialized blobdiff_plain body and the Content-Disposition our
    /// endpoint derives — the exact bytes and header the handler emits over the
    /// corpus `diffs` branch for one single-file surface.
    blobdiff_plain_body: Option<String>,
    blobdiff_plain_disposition: Option<String>,
    /// The serialized patch (format-patch) mail stream and the Content-Disposition
    /// our endpoint derives — the exact bytes and header the handler emits over the
    /// corpus `texts` branch commit.
    patch_body: Option<String>,
    patch_disposition: Option<String>,
    /// The serialized patches (format-patch range) mail stream and the
    /// Content-Disposition our endpoint derives — the exact bytes and header the
    /// handler emits over the corpus `texts` branch tip.
    patches_body: Option<String>,
    patches_disposition: Option<String>,
    /// The serialized binary `patch` mail over the corpus `binmix` head, and
    /// `binary_golden`, gitweb's real output for it — the base85 `GIT binary patch`
    /// body the port now reproduces byte-for-byte (af6).
    binary_patch_body: Option<String>,
    binary_golden: Option<Golden>,
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

#[when("I serve the project index")]
fn serve_project_index(world: &mut GoldenWorld) {
    let project_root: PathBuf = corpus(world).project_root.clone();
    let store: GixProjectStore = GixProjectStore::new(project_root);
    // Exactly what the handler emits: the use case over the adapter, mapped to
    // the render view, serialized — no boundary URL building, no settings.
    let index: ProjectIndex =
        assemble_project_index(&store, None).expect("assemble the corpus project index");
    let view: ProjectIndexView = assemble_project_index_view(&index);
    world.index_body = Some(project_index(&view));
    world.golden = Some(Golden::load("project_index/index"));
}

/// The serialized project_index body, or a panic if none was served.
fn index_body(world: &GoldenWorld) -> &str {
    world.index_body.as_deref().expect("serve the index first")
}

#[when("I serve the opml outline")]
fn serve_opml(world: &mut GoldenWorld) {
    let project_root: PathBuf = corpus(world).project_root.clone();
    let store: GixProjectStore = GixProjectStore::new(project_root);
    // Exactly what the handler emits: the use case over the adapter, mapped to
    // the render view with the CGI default site URL and the built-in site name,
    // serialized.
    let outline: Opml = assemble_opml(&store, None).expect("assemble the corpus opml");
    let settings: Settings = Settings::builtin();
    let view: OpmlView = assemble_opml_view(&outline, &settings, "http://localhost", None);
    world.opml_body = Some(opml(&view));
    world.golden = Some(Golden::load("opml/opml"));
}

/// The serialized opml body, or a panic if none was served.
fn opml_body(world: &GoldenWorld) -> &str {
    world.opml_body.as_deref().expect("serve the opml first")
}

#[when("I serve the commitdiff_plain of the corpus root commit")]
fn serve_commitdiff_plain(world: &mut GoldenWorld) {
    // Exactly what the handler emits over the by-hash request gitweb was
    // captured with: the use case over the adapter for the corpus HEAD oid, the
    // body rendered with the request's own self URL (gitweb's self_url), and the
    // inline filename both stamped with that hash.
    let head: ObjectId = repo(world)
        .resolve("HEAD")
        .expect("resolve the corpus HEAD");
    let plain: CommitdiffPlain = assemble_commitdiff_plain(repo(world), Some(head.as_str()), None)
        .expect("assemble the corpus commitdiff_plain");
    let link: String = self_url(
        "http://localhost",
        &[
            ("p", Corpus::PROJECT),
            ("a", "commitdiff_plain"),
            ("h", head.as_str()),
        ],
    );
    world.commitdiff_plain_body = Some(plain.render(&link));
    world.commitdiff_plain_disposition = Some(format!(
        "inline; filename=\"{}-{}.patch\"",
        Corpus::PROJECT,
        head.as_str()
    ));
    world.golden = Some(Golden::load("commitdiff_plain/root"));
}

/// The serialized commitdiff_plain body, or a panic if none was served.
fn commitdiff_plain_body(world: &GoldenWorld) -> &str {
    world
        .commitdiff_plain_body
        .as_deref()
        .expect("serve the commitdiff_plain first")
}

/// The single-file surface a blobdiff_plain scenario diffs: the new-side path,
/// and the from-path the link carries for a rename (`None` otherwise).
fn diff_surface(name: &str) -> (&'static str, Option<&'static str>) {
    match name {
        "text" => (Corpus::DIFF_TEXT, None),
        "mode" => (Corpus::DIFF_MODE, None),
        "binary" => (Corpus::DIFF_BINARY, None),
        "rename" => (Corpus::DIFF_RENAME_TO, Some(Corpus::DIFF_RENAME_FROM)),
        other => panic!("unknown blobdiff_plain surface {other:?}"),
    }
}

#[when(regex = r#"^I serve the "([^"]*)" blobdiff_plain of the corpus diffs branch$"#)]
fn serve_blobdiff_plain(world: &mut GoldenWorld, name: String) {
    // Exactly what the handler emits over the by-hash request gitweb was captured
    // with: the use case over the adapter for the corpus `diffs` head/parent, the
    // body rendered with the request's own self URL, and the inline filename
    // stamped with the file path.
    let head: String = corpus(world).diff_head.to_string();
    let parent: String = corpus(world).diff_parent.to_string();
    let (file, file_parent): (&str, Option<&str>) = diff_surface(&name);
    let plain: BlobdiffPlain = assemble_blobdiff_plain(repo(world), &head, &parent, file)
        .expect("assemble the corpus blobdiff_plain");
    let mut params: Vec<(&str, &str)> =
        vec![("p", Corpus::PROJECT), ("a", "blobdiff_plain"), ("f", file)];
    if let Some(from) = file_parent {
        params.push(("fp", from));
    }
    params.push(("hb", &head));
    params.push(("hpb", &parent));
    let link: String = self_url("http://localhost", &params);
    world.blobdiff_plain_body = Some(plain.render(&link));
    world.blobdiff_plain_disposition = Some(format!("inline; filename=\"{file}.patch\""));
    world.golden = Some(Golden::load(&format!("blobdiff_plain/{name}")));
}

/// The serialized blobdiff_plain body, or a panic if none was served.
fn blobdiff_plain_body(world: &GoldenWorld) -> &str {
    world
        .blobdiff_plain_body
        .as_deref()
        .expect("serve the blobdiff_plain first")
}

/// The git version captured alongside the patch golden — the value git stamped
/// on the format-patch `-- ` signature. git's version is not part of the mail
/// format and is not byte-stable across machines, so injecting the captured
/// value keeps the rest of the mail a byte-exact comparison.
fn read_patch_version() -> String {
    let path: PathBuf = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("goldens/patch/version");
    std::fs::read_to_string(&path)
        .unwrap_or_else(|err: std::io::Error| panic!("read {}: {err}", path.display()))
}

#[when("I serve the patch of the corpus texts commit")]
fn serve_patch(world: &mut GoldenWorld) {
    // Exactly what the handler emits over the by-hash request gitweb was captured
    // with: the use case over the adapter for the corpus `texts` commit, the
    // patches limit at its built-in default, the signature stamped with the
    // captured git version, and the inline filename stamped with that hash.
    let commit: String = corpus(world).text_commit.to_string();
    let version: String = read_patch_version();
    let stream = assemble_patch(repo(world), Some(&commit), 16, &version)
        .expect("assemble the corpus patch");
    world.patch_body = Some(stream.render());
    world.patch_disposition = Some(format!(
        "inline; filename=\"{}-{}.patch\"",
        Corpus::PROJECT,
        commit
    ));
    world.golden = Some(Golden::load("patch/single"));
}

/// The serialized patch body, or a panic if none was served.
fn patch_body(world: &GoldenWorld) -> &str {
    world.patch_body.as_deref().expect("serve the patch first")
}

#[when("I serve the patches of the corpus texts range")]
fn serve_patches(world: &mut GoldenWorld) {
    // Exactly what the handler emits over the by-hash request gitweb was captured
    // with: the use case over the adapter for the corpus `texts` branch tip, the
    // patches limit at its built-in default, each signature stamped with the
    // captured git version, and the inline filename stamped with that tip hash.
    let tip: String = corpus(world).texts_head.to_string();
    let version: String = read_patch_version();
    let stream = assemble_patches(repo(world), Some(&tip), 16, &version)
        .expect("assemble the corpus patches");
    world.patches_body = Some(stream.render());
    world.patches_disposition = Some(format!(
        "inline; filename=\"{}-{}.patch\"",
        Corpus::PROJECT,
        tip
    ));
    world.golden = Some(Golden::load("patches/range"));
}

/// The serialized patches body, or a panic if none was served.
fn patches_body(world: &GoldenWorld) -> &str {
    world
        .patches_body
        .as_deref()
        .expect("serve the patches first")
}

#[when("I serve the patch of the corpus binmix commit")]
fn serve_binary_patch(world: &mut GoldenWorld) {
    // Exactly what the handler emits over the by-hash request gitweb was captured
    // with: the use case over the adapter for the corpus `binmix` head (a text +
    // binary modify), the patches limit at its built-in default, the signature
    // stamped with the captured git version. Both references the body is judged
    // against were captured over the same commit with the same flags.
    let commit: String = corpus(world).binmix_head.to_string();
    let version: String = read_patch_version();
    let stream = assemble_patch(repo(world), Some(&commit), 16, &version)
        .expect("assemble the corpus binary patch");
    world.binary_patch_body = Some(stream.render());
    world.binary_golden = Some(Golden::load("patch/binary"));
}

/// The serialized binary patch body, or a panic if none was served.
fn binary_patch_body(world: &GoldenWorld) -> &str {
    world
        .binary_patch_body
        .as_deref()
        .expect("serve the binary patch first")
}

/// gitweb's real binary `patch` reference (the `GIT binary patch` body).
fn binary_golden(world: &GoldenWorld) -> &Golden {
    world
        .binary_golden
        .as_ref()
        .expect("serve the binary patch first")
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

#[then("the project index body matches gitweb's reference output")]
fn index_body_matches(world: &mut GoldenWorld) {
    assert_eq!(index_body(world).as_bytes(), golden(world).body());
}

#[then("the project index content type matches gitweb's")]
fn index_content_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-type => 'text/plain', -charset => 'utf-8'`, so the whole
    // Content-Type (with charset) is its own choice and matches ours exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!(INDEX_CONTENT_TYPE, theirs);
}

#[then("the project index content disposition matches gitweb's")]
fn index_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(INDEX_DISPOSITION, theirs);
}

#[then("the opml body matches gitweb's reference output")]
fn opml_body_matches(world: &mut GoldenWorld) {
    assert_eq!(opml_body(world).as_bytes(), golden(world).body());
}

#[then("the opml content type matches gitweb's")]
fn opml_content_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-type => 'text/xml', -charset => 'utf-8'`, so the whole
    // Content-Type (with charset) is its own choice and matches ours exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!(OPML_CONTENT_TYPE, theirs);
}

#[then("the opml content disposition matches gitweb's")]
fn opml_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(OPML_DISPOSITION, theirs);
}

#[then("the commitdiff_plain body matches gitweb's reference output")]
fn commitdiff_plain_body_matches(world: &mut GoldenWorld) {
    assert_eq!(
        commitdiff_plain_body(world).as_bytes(),
        golden(world).body()
    );
}

#[then("the commitdiff_plain content type matches gitweb's")]
fn commitdiff_plain_content_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-type => 'text/plain', -charset => 'utf-8'`; our endpoint
    // serves the same fixed type, so the whole Content-Type matches exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!("text/plain; charset=utf-8", theirs);
}

#[then("the commitdiff_plain content disposition matches gitweb's")]
fn commitdiff_plain_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(world.commitdiff_plain_disposition.as_deref(), Some(theirs));
}

#[then("the blobdiff_plain body matches gitweb's reference output")]
fn blobdiff_plain_body_matches(world: &mut GoldenWorld) {
    assert_eq!(blobdiff_plain_body(world).as_bytes(), golden(world).body());
}

#[then("the blobdiff_plain content type matches gitweb's")]
fn blobdiff_plain_content_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-type => 'text/plain', -charset => 'utf-8'`; our endpoint
    // serves the same fixed type, so the whole Content-Type matches exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!("text/plain; charset=utf-8", theirs);
}

#[then("the blobdiff_plain content disposition matches gitweb's")]
fn blobdiff_plain_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(world.blobdiff_plain_disposition.as_deref(), Some(theirs));
}

#[then("the patch body matches gitweb's reference output")]
fn patch_body_matches(world: &mut GoldenWorld) {
    assert_eq!(patch_body(world).as_bytes(), golden(world).body());
}

#[then("the patch content type matches gitweb's")]
fn patch_content_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-type => 'text/plain', -charset => 'utf-8'`; our endpoint
    // serves the same fixed type, so the whole Content-Type matches exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!("text/plain; charset=utf-8", theirs);
}

#[then("the patch content disposition matches gitweb's")]
fn patch_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(world.patch_disposition.as_deref(), Some(theirs));
}

#[then("the patches body matches gitweb's reference output")]
fn patches_body_matches(world: &mut GoldenWorld) {
    assert_eq!(patches_body(world).as_bytes(), golden(world).body());
}

#[then("the patches content type matches gitweb's")]
fn patches_content_type_matches(world: &mut GoldenWorld) {
    // gitweb sets `-type => 'text/plain', -charset => 'utf-8'`; our endpoint
    // serves the same fixed type, so the whole Content-Type matches exactly.
    let theirs: &str = golden(world)
        .header("Content-Type")
        .expect("gitweb declares a Content-Type");
    assert_eq!("text/plain; charset=utf-8", theirs);
}

#[then("the patches content disposition matches gitweb's")]
fn patches_disposition_matches(world: &mut GoldenWorld) {
    let theirs: &str = golden(world)
        .header("Content-Disposition")
        .expect("gitweb declares a Content-Disposition");
    assert_eq!(world.patches_disposition.as_deref(), Some(theirs));
}

#[then("the binary patch matches gitweb byte for byte")]
fn binary_patch_matches(world: &mut GoldenWorld) {
    // The whole mail is byte-for-byte gitweb's `git format-patch` output: the
    // mailbox header, the diffstat (incl. the `Bin <old> -> <new> bytes` row), the
    // text file's abbreviated-index diff, and — the af6 deliverable — the binary
    // file's FULL `index` and its base85 `GIT binary patch` body (the forward and
    // reverse blocks, each git's deflated literal or delta), then the signature.
    assert_eq!(
        binary_patch_body(world).as_bytes(),
        binary_golden(world).body()
    );
}

#[tokio::main]
async fn main() {
    GoldenWorld::run("features/golden").await;
}
