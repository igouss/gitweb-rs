//! Gherkin-driven BDD harness for the web boundary.
//!
//! The `Given` lays out a real project root on disk with the gix fixture builder
//! and wires the gix `ProjectStore` adapter over it, as the composition root
//! would. The `When` drives [`resolve`] with a raw query string and PATH_INFO
//! and stores the raw [`Result`], so each `Then` asserts one fact — a resolved
//! field, or the exact failure mode — without any branching in the step body.
//! cucumber supplies its own `main`, so this target sets `harness = false`.

use std::collections::BTreeMap;
use std::sync::Arc;

use axum::Router;
use axum::body::{Body, to_bytes};
use axum::http::{Method, Request as HttpRequest, header};
use cucumber::{World, given, then, when};
use tower::ServiceExt;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::commit::Commit;
use gitweb_domain::model::object_id::ObjectId;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::model::settings::{FeatureLayer, FeatureName, Settings, SettingsLayer};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::port::repository::Repository;
use gitweb_fixtures::ProjectRoot;
use gitweb_git::GixProjectStore;
use gitweb_web::handlers::{
    BlobHandler, BlobPlainHandler, BlobdiffHandler, BlobdiffPlainHandler, CommitHandler,
    CommitdiffHandler, CommitdiffPlainHandler, DefaultObjectResolver, FeedHandler, ForksHandler,
    HeadsHandler, HistoryHandler, LogHandler, ObjectHandler, OpmlHandler, PatchHandler,
    PatchesHandler, ProjectIndexHandler, ProjectListHandler, RemotesHandler, SearchHandler,
    SearchHelpHandler, ShortlogHandler, SnapshotHandler, SummaryHandler, TagHandler, TagsHandler,
    TreeHandler,
};
use gitweb_web::request::{ResolvedRequest, resolve};
use gitweb_web::response::View;
use gitweb_web::url::{href_full, self_url};
use gitweb_web::{Dispatcher, Handler, ObjectKindResolver, router};

#[derive(Debug, Default, World)]
struct WebWorld {
    root: Option<ProjectRoot>,
    store: Option<GixProjectStore>,
    resolved: Option<Result<ResolvedRequest, DomainError>>,
    dispatcher: Dispatcher,
    response_status: Option<u16>,
    response_content_type: Option<String>,
    response_content_disposition: Option<String>,
    response_last_modified: Option<String>,
    response_expires: Option<String>,
    response_location: Option<String>,
    response_body: Option<String>,
    response_body_bytes: Option<Vec<u8>>,
    built_url: Option<String>,
}

// --- dispatch fixtures -------------------------------------------------------

/// A stand-in for a capability bead's handler: it ignores the request and
/// returns a fixed view, so the harness can prove the dispatcher routed to it.
struct StubHandler {
    content_type: &'static str,
    body: String,
}

impl Handler for StubHandler {
    fn handle(&self, _request: &Request) -> Result<View, DomainError> {
        Ok(View::text(self.content_type, self.body.clone()))
    }
}

/// Registers a stub handler for `action_name` that serves `body` as `content_type`.
fn register_stub(
    world: &mut WebWorld,
    action_name: &str,
    content_type: &'static str,
    body: String,
) {
    let action: Action = Action::parse(action_name).expect("the stub names a valid action");
    let handler: Arc<dyn Handler> = Arc::new(StubHandler { content_type, body });
    world.dispatcher.register(action, handler);
}

// --- fixture construction ----------------------------------------------------

/// Creates the project root and wires the adapter over it, once per scenario.
fn ensure_root(world: &mut WebWorld) {
    if world.root.is_some() {
        return;
    }
    let root: ProjectRoot = ProjectRoot::new();
    let store: GixProjectStore = GixProjectStore::new(root.path().to_path_buf());
    world.root = Some(root);
    world.store = Some(store);
}

fn root(world: &WebWorld) -> &ProjectRoot {
    world
        .root
        .as_ref()
        .expect("a project root must exist first")
}

fn store(world: &WebWorld) -> &GixProjectStore {
    world.store.as_ref().expect("a store must be wired first")
}

/// The successful resolution, or a panic if the scenario produced an error.
fn ok(world: &WebWorld) -> &ResolvedRequest {
    world
        .resolved
        .as_ref()
        .expect("resolve the request first")
        .as_ref()
        .expect("resolution succeeded")
}

/// The failure, or a panic if the scenario produced a success.
fn err(world: &WebWorld) -> &DomainError {
    match world.resolved.as_ref().expect("resolve the request first") {
        Ok(_) => panic!("expected resolution to fail"),
        Err(error) => error,
    }
}

/// Splits a raw query string ("k=v&k=v") into gitweb's short-name pairs.
fn parse_query(query: &str) -> Vec<(String, String)> {
    if query.is_empty() {
        return Vec::new();
    }
    query
        .split('&')
        .map(|pair: &str| match pair.split_once('=') {
            Some((key, value)) => (key.to_owned(), value.to_owned()),
            None => (pair.to_owned(), String::new()),
        })
        .collect()
}

// --- Givens ------------------------------------------------------------------

#[given("an empty project root")]
fn given_empty_root(world: &mut WebWorld) {
    ensure_root(world);
}

#[given(regex = r#"^a project root containing repository "([^"]*)"$"#)]
fn given_root_with_repo(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_repo(&name);
}

#[given(regex = r#"^the root also contains repository "([^"]*)"$"#)]
fn given_also_repo(world: &mut WebWorld, name: String) {
    root(world).add_repo(&name);
}

#[given(regex = r#"^the root also contains a plain directory "([^"]*)"$"#)]
fn given_also_plain_dir(world: &mut WebWorld, name: String) {
    root(world).add_dir(&name);
}

#[given(regex = r#"^a repository "([^"]*)" with an unborn HEAD$"#)]
fn given_unborn_repo(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_empty_repo(&name);
}

#[given(regex = r#"^the repository "([^"]*)" has branch "([^"]*)" committed at (\d+)$"#)]
fn given_repo_branch(world: &mut WebWorld, name: String, branch: String, epoch: i64) {
    root(world).add_branch_at(&name, &branch, epoch);
}

#[given(
    regex = r#"^the repository "([^"]*)" has an annotated tag "([^"]*)" of a commit at (\d+) with subject "(.*)"$"#
)]
fn given_repo_annotated_tag(
    world: &mut WebWorld,
    name: String,
    tag: String,
    epoch: i64,
    subject: String,
) {
    root(world).add_annotated_tag_at(&name, &tag, epoch, &subject);
}

#[given(
    regex = r#"^the repository "([^"]*)" has a lightweight tag "([^"]*)" of a commit at (\d+)$"#
)]
fn given_repo_lightweight_tag(world: &mut WebWorld, name: String, tag: String, epoch: i64) {
    root(world).add_tag_at(&name, &tag, epoch);
}

#[given(regex = r#"^the repository "([^"]*)" has a lightweight tag "([^"]*)" of a blob$"#)]
fn given_repo_lightweight_blob_tag(world: &mut WebWorld, name: String, tag: String) {
    root(world).add_lightweight_blob_tag(&name, &tag);
}

// --- Whens -------------------------------------------------------------------

#[when(regex = r#"^I resolve the path "(.*)" with no query$"#)]
fn resolve_path_no_query(world: &mut WebWorld, path: String) {
    world.resolved = Some(resolve(store(world), &[], &path));
}

#[when(regex = r#"^I resolve the path "(.*)" with the query "(.*)"$"#)]
fn resolve_path_with_query(world: &mut WebWorld, path: String, query: String) {
    let params: Vec<(String, String)> = parse_query(&query);
    world.resolved = Some(resolve(store(world), &params, &path));
}

#[when(regex = r#"^I resolve the query "(.*)"$"#)]
fn resolve_query_only(world: &mut WebWorld, query: String) {
    let params: Vec<(String, String)> = parse_query(&query);
    world.resolved = Some(resolve(store(world), &params, ""));
}

// --- Whens: byte-faithful URL building (feeds) -------------------------------

/// Splits a space-separated `"k=v k=v"` list into ordered pairs (empty for "").
fn parse_pairs(pairs: &str) -> Vec<(String, String)> {
    pairs
        .split_whitespace()
        .filter_map(|token: &str| token.split_once('='))
        .map(|(key, value): (&str, &str)| (key.to_owned(), value.to_owned()))
        .collect()
}

/// Borrows owned pairs as the `&[(&str, &str)]` the URL builders take.
fn borrow_pairs(pairs: &[(String, String)]) -> Vec<(&str, &str)> {
    pairs
        .iter()
        .map(|(key, value): &(String, String)| (key.as_str(), value.as_str()))
        .collect()
}

#[when(regex = r#"^I build a full URL at "([^"]*)" with no params$"#)]
fn build_full_url_empty(world: &mut WebWorld, base: String) {
    world.built_url = Some(href_full(&base, &[]));
}

#[when(regex = r#"^I build a full URL at "([^"]*)" with params "([^"]*)"$"#)]
fn build_full_url(world: &mut WebWorld, base: String, pairs: String) {
    let owned: Vec<(String, String)> = parse_pairs(&pairs);
    world.built_url = Some(href_full(&base, &borrow_pairs(&owned)));
}

#[when(regex = r#"^I build a full URL at "([^"]*)" with param "([^"]*)" set to "(.*)"$"#)]
fn build_full_url_single(world: &mut WebWorld, base: String, key: String, value: String) {
    world.built_url = Some(href_full(&base, &[(key.as_str(), value.as_str())]));
}

#[when(regex = r#"^I build a self URL at "([^"]*)" with params "([^"]*)"$"#)]
fn build_self_url(world: &mut WebWorld, base: String, pairs: String) {
    let owned: Vec<(String, String)> = parse_pairs(&pairs);
    world.built_url = Some(self_url(&base, &borrow_pairs(&owned)));
}

#[then(regex = r#"^the URL is "([^"]*)"$"#)]
fn then_url_is(world: &mut WebWorld, expected: String) {
    assert_eq!(world.built_url.as_deref(), Some(expected.as_str()));
}

// --- Thens: resolved project / action ----------------------------------------

#[then(regex = r#"^the resolved project is "([^"]*)"$"#)]
fn then_project_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.project.as_deref(),
        Some(expected.as_str())
    );
}

#[then("no project is resolved")]
fn then_no_project(world: &mut WebWorld) {
    assert_eq!(ok(world).request.project, None);
}

#[then(regex = r#"^the resolved action is "([^"]*)"$"#)]
fn then_action_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.action.map(Action::as_str),
        Some(expected.as_str())
    );
}

#[then("no action is resolved")]
fn then_no_action(world: &mut WebWorld) {
    assert_eq!(ok(world).request.action, None);
}

// --- Thens: resolved refs and paths ------------------------------------------

#[then(regex = r#"^the resolved hash is "([^"]*)"$"#)]
fn then_hash_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.hash.as_ref().map(SafeRef::as_str),
        Some(expected.as_str())
    );
}

#[then("no hash is resolved")]
fn then_no_hash(world: &mut WebWorld) {
    assert_eq!(ok(world).request.hash, None);
}

#[then(regex = r#"^the resolved hash base is "([^"]*)"$"#)]
fn then_hash_base_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.hash_base.as_ref().map(SafeRef::as_str),
        Some(expected.as_str())
    );
}

#[then("no hash base is resolved")]
fn then_no_hash_base(world: &mut WebWorld) {
    assert_eq!(ok(world).request.hash_base, None);
}

#[then(regex = r#"^the resolved file is "([^"]*)"$"#)]
fn then_file_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).request.file_name.as_ref().map(SafePath::as_str),
        Some(expected.as_str())
    );
}

// --- Thens: view preferences -------------------------------------------------

#[then(regex = r#"^the diff style is "([^"]*)"$"#)]
fn then_diff_style_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).view.diff_style.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the content tag is "([^"]*)"$"#)]
fn then_content_tag_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).view.content_tag.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the javascript marker is "([^"]*)"$"#)]
fn then_javascript_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        ok(world).view.javascript.as_deref(),
        Some(expected.as_str())
    );
}

// --- Thens: failure modes ----------------------------------------------------

#[then("resolution fails as invalid")]
fn then_fails_invalid(world: &mut WebWorld) {
    assert!(matches!(err(world), DomainError::Invalid(_)));
}

#[then("resolution fails as not found")]
fn then_fails_not_found(world: &mut WebWorld) {
    assert!(matches!(err(world), DomainError::NotFound(_)));
}

#[then("resolution fails as forbidden")]
fn then_fails_forbidden(world: &mut WebWorld) {
    assert!(matches!(err(world), DomainError::Forbidden(_)));
}

// --- Givens: registering stub handlers ---------------------------------------

#[given(regex = r#"^a stub "([^"]*)" page handler$"#)]
fn given_stub_page_handler(world: &mut WebWorld, action_name: String) {
    let body: String = format!("<p>STUB:{action_name}</p>");
    register_stub(world, &action_name, "text/html; charset=utf-8", body);
}

#[given(regex = r#"^a stub "([^"]*)" plain-text handler$"#)]
fn given_stub_plain_handler(world: &mut WebWorld, action_name: String) {
    let body: String = format!("STUB:{action_name}");
    register_stub(world, &action_name, "text/plain; charset=utf-8", body);
}

// --- Given: registering real capability handlers -----------------------------

#[given("the project-list landing page is served")]
fn given_project_list_served(world: &mut WebWorld) {
    register_project_list(world, Vec::new());
}

#[given(regex = r#"^the project-list landing page is served with extra branch refs "([^"]*)"$"#)]
fn given_project_list_served_with_extra_refs(world: &mut WebWorld, extra_refs: String) {
    let extra: Vec<String> = extra_refs
        .split(',')
        .map(|token: &str| token.trim().to_owned())
        .collect();
    register_project_list(world, extra);
}

/// Wires the project-list handler over the root's store, injecting the
/// extra-branch-refs options the composition root resolves from settings (so the
/// "Last Change" age spans those branch directories).
fn register_project_list(world: &mut WebWorld, extra_branch_refs: Vec<String>) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> = Arc::new(
        GixProjectStore::new(root(world).path().to_path_buf())
            .with_extra_branch_refs(extra_branch_refs),
    );
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(ProjectListHandler::new(store, settings));
    world.dispatcher.register(Action::ProjectList, handler);
}

#[given("the forks feature is served")]
fn given_forks_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(forks_settings());
    let list: Arc<dyn Handler> = Arc::new(ProjectListHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    world.dispatcher.register(Action::ProjectList, list);
    let forks: Arc<dyn Handler> = Arc::new(ForksHandler::new(store, settings));
    world.dispatcher.register(Action::Forks, forks);
}

#[given("the project index is served")]
fn given_project_index_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(ProjectIndexHandler::new(store));
    world.dispatcher.register(Action::ProjectIndex, handler);
}

#[given("the opml action is served")]
fn given_opml_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(OpmlHandler::new(
        store,
        settings,
        "http://localhost".to_owned(),
    ));
    world.dispatcher.register(Action::Opml, handler);
}

#[given("the heads action is served")]
fn given_heads_served(world: &mut WebWorld) {
    register_heads(world, Settings::builtin());
}

#[given(regex = r#"^the heads action is served with extra branch refs "([^"]*)"$"#)]
fn given_heads_served_with_extra_refs(world: &mut WebWorld, extra_refs: String) {
    register_heads(world, extra_branch_refs_settings(&extra_refs));
}

/// Wires the heads handler over the root's store and the given settings.
fn register_heads(world: &mut WebWorld, settings: Settings) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(HeadsHandler::new(store, Arc::new(settings)));
    world.dispatcher.register(Action::Heads, handler);
}

#[given("the shortlog action is served")]
fn given_shortlog_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(ShortlogHandler::new(store, settings));
    world.dispatcher.register(Action::Shortlog, handler);
}

#[given("the log action is served")]
fn given_log_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(LogHandler::new(store, settings));
    world.dispatcher.register(Action::Log, handler);
}

#[given(regex = r#"^a repository "([^"]*)" with a file history$"#)]
fn given_repo_file_history(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_file_history(&name);
}

#[given("the history action is served")]
fn given_history_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(HistoryHandler::new(store, settings));
    world.dispatcher.register(Action::History, handler);
}

#[given("the feed actions are served")]
fn given_feed_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let rss: Arc<dyn Handler> = Arc::new(FeedHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
        "http://localhost".to_owned(),
        "gitweb-test/1".to_owned(),
    ));
    let atom: Arc<dyn Handler> = Arc::new(FeedHandler::new(
        store,
        settings,
        "http://localhost".to_owned(),
        "gitweb-test/1".to_owned(),
    ));
    world.dispatcher.register(Action::Rss, rss);
    world.dispatcher.register(Action::Atom, atom);
}

#[given(regex = r#"^a repository "([^"]*)" with a tree$"#)]
fn given_repo_with_tree(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_tree_repo(&name);
}

#[given("the tree action is served")]
fn given_tree_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(TreeHandler::new(store, settings));
    world.dispatcher.register(Action::Tree, handler);
}

#[given(regex = r#"^a repository "([^"]*)" with blobs$"#)]
fn given_repo_with_blobs(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_blob_repo(&name);
}

#[given("the blob action is served")]
fn given_blob_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(BlobHandler::new(store, settings));
    world.dispatcher.register(Action::Blob, handler);
}

#[given(regex = r#"^a project root containing a commit repository "([^"]*)"$"#)]
fn given_commit_repo(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_commit_repo(&name);
}

#[given(regex = r#"^a project root containing a merge repository "([^"]*)"$"#)]
fn given_merge_repo(world: &mut WebWorld, name: String) {
    ensure_root(world);
    root(world).add_merge_repo(&name);
}

#[given("the commit action is served")]
fn given_commit_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(CommitHandler::new(store, settings));
    world.dispatcher.register(Action::Commit, handler);
}

#[given("the commitdiff action is served")]
fn given_commitdiff_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(CommitdiffHandler::new(store, settings));
    world.dispatcher.register(Action::Commitdiff, handler);
}

#[given("the blob_plain action is served")]
fn given_blob_plain_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(BlobPlainHandler::new(store, settings));
    world.dispatcher.register(Action::BlobPlain, handler);
}

#[given("the object action is served")]
fn given_object_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> =
        Arc::new(ObjectHandler::new(store, "http://localhost".to_owned()));
    world.dispatcher.register(Action::Object, handler);
}

#[given("the no-action object dispatch is enabled")]
fn given_object_dispatch_enabled(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let resolver: Arc<dyn ObjectKindResolver> = Arc::new(DefaultObjectResolver::new(store));
    world.dispatcher.set_object_resolver(resolver);
}

#[given("the commitdiff_plain action is served")]
fn given_commitdiff_plain_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(CommitdiffPlainHandler::new(
        store,
        "http://localhost".to_owned(),
    ));
    world.dispatcher.register(Action::CommitdiffPlain, handler);
}

/// The git version the patch scenarios stamp on the mail signature — a fixed
/// value, since the test does not run git; only that it is echoed matters.
const PATCH_GIT_VERSION: &str = "2.54.0";

/// Settings with the `patches` feature on (the built-in `16`) or off (`0`), the
/// `$patch_max` the patch view's 403 gate reads.
fn patches_settings(enabled: bool) -> Settings {
    if enabled {
        return Settings::builtin();
    }
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Patches,
        FeatureLayer {
            default: Some(vec!["0".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

/// Settings with the `forks` feature on, so the landing page folds forks and the
/// forks action lists them.
fn forks_settings() -> Settings {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Forks,
        FeatureLayer {
            default: Some(vec!["1".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

fn register_patch(world: &mut WebWorld, enabled: bool) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(patches_settings(enabled));
    let handler: Arc<dyn Handler> = Arc::new(PatchHandler::new(
        store,
        settings,
        PATCH_GIT_VERSION.to_owned(),
    ));
    world.dispatcher.register(Action::Patch, handler);
}

#[given("the patch action is served")]
fn given_patch_served(world: &mut WebWorld) {
    register_patch(world, true);
}

#[given("the patch action is served with the patches feature off")]
fn given_patch_served_disabled(world: &mut WebWorld) {
    register_patch(world, false);
}

fn register_patches(world: &mut WebWorld, enabled: bool) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(patches_settings(enabled));
    let handler: Arc<dyn Handler> = Arc::new(PatchesHandler::new(
        store,
        settings,
        PATCH_GIT_VERSION.to_owned(),
    ));
    world.dispatcher.register(Action::Patches, handler);
}

#[given("the patches action is served")]
fn given_patches_served(world: &mut WebWorld) {
    register_patches(world, true);
}

#[given("the patches action is served with the patches feature off")]
fn given_patches_served_disabled(world: &mut WebWorld) {
    register_patches(world, false);
}

#[given("the blobdiff_plain action is served")]
fn given_blobdiff_plain_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(BlobdiffPlainHandler::new(
        store,
        "http://localhost".to_owned(),
    ));
    world.dispatcher.register(Action::BlobdiffPlain, handler);
}

#[given("the blobdiff action is served")]
fn given_blobdiff_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(BlobdiffHandler::new(store, settings));
    world.dispatcher.register(Action::Blobdiff, handler);
}

#[given("the tags action is served")]
fn given_tags_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(TagsHandler::new(store, settings));
    world.dispatcher.register(Action::Tags, handler);
}

#[given("the tag action is served")]
fn given_tag_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(TagHandler::new(store, settings));
    world.dispatcher.register(Action::Tag, handler);
}

// --- remotes: fixtures and the served handler --------------------------------

#[given(regex = r#"^the repository "([^"]*)" has a remote "([^"]*)" fetching "([^"]*)"$"#)]
fn given_repo_remote(world: &mut WebWorld, name: String, remote: String, url: String) {
    root(world).add_remote(&name, &remote, Some(&url), None);
}

#[given(
    regex = r#"^the repository "([^"]*)" remote "([^"]*)" tracks "([^"]*)" committed at (\d+)$"#
)]
fn given_repo_remote_tracks(
    world: &mut WebWorld,
    name: String,
    remote: String,
    branch: String,
    epoch: i64,
) {
    root(world).add_remote_branch(&name, &remote, &branch, epoch);
}

/// Settings with the `remote_heads` feature on or off, the gate the remotes view
/// reads (gitweb's `gitweb_check_feature('remote_heads')`).
fn remote_heads_settings(enabled: bool) -> Settings {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    let default: Vec<String> = if enabled {
        vec!["1".to_owned()]
    } else {
        vec!["0".to_owned()]
    };
    features.insert(
        FeatureName::RemoteHeads,
        FeatureLayer {
            default: Some(default),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

#[given("the remotes action is served")]
fn given_remotes_served(world: &mut WebWorld) {
    register_remotes(world, true);
}

#[given("the remotes action is served with the remote_heads feature disabled")]
fn given_remotes_served_disabled(world: &mut WebWorld) {
    register_remotes(world, false);
}

/// Registers the remotes handler with the `remote_heads` feature on or off.
fn register_remotes(world: &mut WebWorld, enabled: bool) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(remote_heads_settings(enabled));
    let handler: Arc<dyn Handler> = Arc::new(RemotesHandler::new(store, settings));
    world.dispatcher.register(Action::Remotes, handler);
}

/// Settings that turn the `grep` and `pickaxe` features off, so the search-help
/// page documents only the always-available types.
fn grep_pickaxe_off_settings() -> Settings {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Grep,
        FeatureLayer {
            default: Some(vec!["0".to_owned()]),
            overridable: None,
        },
    );
    features.insert(
        FeatureName::Pickaxe,
        FeatureLayer {
            default: Some(vec!["0".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

#[given("the search_help action is served")]
fn given_search_help_served(world: &mut WebWorld) {
    register_search_help(world, Settings::builtin());
}

#[given("the search_help action is served with grep and pickaxe disabled")]
fn given_search_help_served_disabled(world: &mut WebWorld) {
    register_search_help(world, grep_pickaxe_off_settings());
}

/// Registers the search-help handler over the given settings.
fn register_search_help(world: &mut WebWorld, settings: Settings) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(SearchHelpHandler::new(store, Arc::new(settings)));
    world.dispatcher.register(Action::SearchHelp, handler);
}

#[given("the search action is served")]
fn given_search_served(world: &mut WebWorld) {
    register_search(world, Settings::builtin());
}

#[given("the search action is served with search disabled")]
fn given_search_served_disabled(world: &mut WebWorld) {
    register_search(world, search_off_settings());
}

#[given("the search action is served with grep disabled")]
fn given_search_served_grep_disabled(world: &mut WebWorld) {
    register_search(world, grep_pickaxe_off_settings());
}

#[given("the search action is served with pickaxe disabled")]
fn given_search_served_pickaxe_disabled(world: &mut WebWorld) {
    register_search(world, pickaxe_off_settings());
}

/// Registers the search handler over the given settings.
fn register_search(world: &mut WebWorld, settings: Settings) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(SearchHandler::new(store, Arc::new(settings)));
    world.dispatcher.register(Action::Search, handler);
}

/// Settings whose `search` feature is turned off (gitweb's 403 "Search is
/// disabled").
fn search_off_settings() -> Settings {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Search,
        FeatureLayer {
            default: Some(vec!["0".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

/// Settings whose `pickaxe` feature is turned off, search left on — so a pickaxe
/// request is gitweb's 403 "Pickaxe search is disabled" from the second gate.
fn pickaxe_off_settings() -> Settings {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Pickaxe,
        FeatureLayer {
            default: Some(vec!["0".to_owned()]),
            overridable: None,
        },
    );
    let layer: SettingsLayer = SettingsLayer {
        features,
        ..SettingsLayer::default()
    };
    Settings::resolve(&[layer])
}

// --- snapshot: the served handler with site-configured formats ---------------

/// Settings whose `snapshot` feature enables exactly `formats` (a comma-separated
/// list, empty for none) — gitweb's `$feature{snapshot}{default}`.
/// A settings layer that enables the `snapshot` feature with the given formats.
fn snapshot_settings_layer(formats: &str) -> SettingsLayer {
    let options: Vec<String> = if formats.trim().is_empty() {
        Vec::new()
    } else {
        formats
            .split(',')
            .map(|token: &str| token.trim().to_owned())
            .collect()
    };
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::Snapshot,
        FeatureLayer {
            default: Some(options),
            overridable: None,
        },
    );
    SettingsLayer {
        features,
        ..SettingsLayer::default()
    }
}

fn snapshot_settings(formats: &str) -> Settings {
    Settings::resolve(&[snapshot_settings_layer(formats)])
}

/// The snapshot settings with the `extra-branch-refs` feature set as well, for the
/// branch-ref-naming proofs.
fn snapshot_settings_with_extra_refs(formats: &str, extra_refs: &str) -> Settings {
    let extra_options: Vec<String> = if extra_refs.trim().is_empty() {
        Vec::new()
    } else {
        extra_refs
            .split(',')
            .map(|token: &str| token.trim().to_owned())
            .collect()
    };
    let mut layer: SettingsLayer = snapshot_settings_layer(formats);
    layer.features.insert(
        FeatureName::ExtraBranchRefs,
        FeatureLayer {
            default: Some(extra_options),
            overridable: None,
        },
    );
    Settings::resolve(&[layer])
}

/// Settings with only the `extra-branch-refs` feature set, for the heads-listing
/// branch-directory proof.
fn extra_branch_refs_settings(extra_refs: &str) -> Settings {
    let options: Vec<String> = if extra_refs.trim().is_empty() {
        Vec::new()
    } else {
        extra_refs
            .split(',')
            .map(|token: &str| token.trim().to_owned())
            .collect()
    };
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    features.insert(
        FeatureName::ExtraBranchRefs,
        FeatureLayer {
            default: Some(options),
            overridable: None,
        },
    );
    Settings::resolve(&[SettingsLayer {
        features,
        ..SettingsLayer::default()
    }])
}

#[given(regex = r#"^the repository "([^"]*)" has a ref "([^"]*)" committed at (\d+)$"#)]
fn given_repo_custom_ref(world: &mut WebWorld, name: String, full_ref: String, epoch: i64) {
    root(world).add_ref_at(&name, &full_ref, epoch);
}

#[given(regex = r#"^the snapshot action is served with formats "([^"]*)"$"#)]
fn given_snapshot_served(world: &mut WebWorld, formats: String) {
    register_snapshot(world, snapshot_settings(&formats));
}

#[given(
    regex = r#"^the snapshot action is served with formats "([^"]*)" and extra branch refs "([^"]*)"$"#
)]
fn given_snapshot_served_with_extra_refs(
    world: &mut WebWorld,
    formats: String,
    extra_refs: String,
) {
    register_snapshot(
        world,
        snapshot_settings_with_extra_refs(&formats, &extra_refs),
    );
}

/// Wires the snapshot handler over the root's store and the given settings.
fn register_snapshot(world: &mut WebWorld, settings: Settings) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let handler: Arc<dyn Handler> = Arc::new(SnapshotHandler::new(store, Arc::new(settings)));
    world.dispatcher.register(Action::Snapshot, handler);
}

/// The raw response bytes, or a panic if no response was captured.
fn response_bytes(world: &WebWorld) -> &[u8] {
    world
        .response_body_bytes
        .as_deref()
        .expect("a response body must have been captured")
}

#[then("the response body begins with the gzip magic")]
fn then_body_gzip_magic(world: &mut WebWorld) {
    let bytes: &[u8] = response_bytes(world);
    assert!(
        bytes.starts_with(&[0x1f, 0x8b]),
        "body did not begin with the gzip magic: {:02x?}",
        &bytes[..bytes.len().min(4)]
    );
}

#[then("the response body begins with the zip magic")]
fn then_body_zip_magic(world: &mut WebWorld) {
    let bytes: &[u8] = response_bytes(world);
    assert!(
        bytes.starts_with(b"PK\x03\x04"),
        "body did not begin with the zip magic: {:02x?}",
        &bytes[..bytes.len().min(4)]
    );
}

#[then("the response has a last-modified header")]
fn then_has_last_modified(world: &mut WebWorld) {
    assert!(
        world.response_last_modified.is_some(),
        "no Last-Modified header was captured"
    );
}

#[then("the response has an expires header")]
fn then_has_expires(world: &mut WebWorld) {
    assert!(
        world.response_expires.is_some(),
        "no Expires header was captured"
    );
}

#[then("the response has no expires header")]
fn then_no_expires(world: &mut WebWorld) {
    assert_eq!(
        world.response_expires, None,
        "an Expires header was captured but none was expected"
    );
}

#[then(regex = r#"^the response content disposition contains "(.*)"$"#)]
fn then_disposition_contains(world: &mut WebWorld, needle: String) {
    let disposition: &str = world
        .response_content_disposition
        .as_deref()
        .expect("a Content-Disposition header must have been captured");
    assert!(
        disposition.contains(&needle),
        "disposition did not contain {needle:?}: {disposition}"
    );
}

// --- summary: fixture metadata and the served handler ------------------------

#[given(regex = r#"^"([^"]*)" has the description file "(.*)"$"#)]
fn given_summary_description(world: &mut WebWorld, name: String, text: String) {
    root(world).set_description(&name, &text);
}

#[given(regex = r#"^"([^"]*)" has a README of "(.*)"$"#)]
fn given_summary_readme(world: &mut WebWorld, name: String, contents: String) {
    root(world).set_readme_html(&name, &contents);
}

#[given(regex = r#"^"([^"]*)" has an empty README$"#)]
fn given_summary_empty_readme(world: &mut WebWorld, name: String) {
    root(world).set_readme_html(&name, "");
}

#[given(regex = r#"^the repository "([^"]*)" has 17 branches$"#)]
fn given_seventeen_branches(world: &mut WebWorld, name: String) {
    // Fixture construction (not assertion logic): 17 distinct branches puts the
    // heads section one past gitweb's cap of 16, so its "..." link must appear.
    for index in 0..17 {
        let branch: String = format!("b{index:02}");
        let epoch: i64 = 1000 + i64::from(index);
        root(world).add_branch_at(&name, &branch, epoch);
    }
}

#[given("the summary action is served")]
fn given_summary_served(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let settings: Arc<Settings> = Arc::new(Settings::builtin());
    let handler: Arc<dyn Handler> = Arc::new(SummaryHandler::new(store, settings));
    world.dispatcher.register(Action::Summary, handler);
}

#[given("the summary action is served with XSS prevention")]
fn given_summary_served_xss(world: &mut WebWorld) {
    ensure_root(world);
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let layer: SettingsLayer = SettingsLayer {
        prevent_xss: Some(true),
        ..SettingsLayer::default()
    };
    let settings: Arc<Settings> = Arc::new(Settings::resolve(&[layer]));
    let handler: Arc<dyn Handler> = Arc::new(SummaryHandler::new(store, settings));
    world.dispatcher.register(Action::Summary, handler);
}

// --- When: drive the assembled router with one in-process request ------------

#[when(regex = r#"^I GET "([^"]*)"$"#)]
async fn when_get(world: &mut WebWorld, uri: String) {
    dispatch_capture(world, uri).await;
}

#[when(regex = r#"^I GET "([^"]*)" if not modified since "([^"]*)"$"#)]
async fn when_get_if_modified_since(world: &mut WebWorld, uri: String, since: String) {
    let headers: [(header::HeaderName, String); 1] = [(header::IF_MODIFIED_SINCE, since)];
    dispatch_capture_with(world, uri, Method::GET, &headers).await;
}

#[when(regex = r#"^I GET "([^"]*)" accepting "([^"]*)"$"#)]
async fn when_get_accepting(world: &mut WebWorld, uri: String, accept: String) {
    let headers: [(header::HeaderName, String); 1] = [(header::ACCEPT, accept)];
    dispatch_capture_with(world, uri, Method::GET, &headers).await;
}

#[when(regex = r#"^I HEAD "([^"]*)"$"#)]
async fn when_head(world: &mut WebWorld, uri: String) {
    dispatch_capture_with(world, uri, Method::HEAD, &[]).await;
}

/// Resolves the parent base for a blobdiff request: the project's HEAD commit and
/// its first parent, so a single-file diff between them is addressable by hash.
/// gitweb's blobdiff links carry the actual commit ids; `^`/`~` rev syntax is
/// rejected by `is_valid_refname`, so the test resolves the ids the same way.
fn head_and_parent(world: &WebWorld, project: &str) -> (String, String) {
    let store: GixProjectStore = GixProjectStore::new(root(world).path().to_path_buf());
    let repository: Box<dyn Repository> = store.open(project).expect("open the project");
    let head: ObjectId = repository.resolve("HEAD").expect("resolve HEAD");
    let commit: Commit = repository.find_commit(&head).expect("read the HEAD commit");
    let parent: &ObjectId = commit
        .parents()
        .first()
        .expect("the HEAD commit has a parent to diff against");
    (head.as_str().to_owned(), parent.as_str().to_owned())
}

/// Resolves the merge's HEAD commit and its SECOND parent, so a commitdiff can be
/// addressed against an explicit non-first parent (gitweb's "from parent 2"
/// link) — the base-aware changed-files case.
fn head_and_second_parent(world: &WebWorld, project: &str) -> (String, String) {
    let store: GixProjectStore = GixProjectStore::new(root(world).path().to_path_buf());
    let repository: Box<dyn Repository> = store.open(project).expect("open the project");
    let head: ObjectId = repository.resolve("HEAD").expect("resolve HEAD");
    let commit: Commit = repository.find_commit(&head).expect("read the HEAD commit");
    let parent: &ObjectId = commit
        .parents()
        .get(1)
        .expect("the HEAD merge has a second parent");
    (head.as_str().to_owned(), parent.as_str().to_owned())
}

#[when(regex = r#"^I GET the commitdiff of "([^"]*)" against its second parent$"#)]
async fn when_get_commitdiff_second_parent(world: &mut WebWorld, project: String) {
    let (head, parent): (String, String) = head_and_second_parent(world, &project);
    let uri: String = format!("/?p={project}&a=commitdiff&h={head}&hp={parent}");
    dispatch_capture(world, uri).await;
}

#[when(regex = r#"^I GET the blobdiff_plain of "([^"]*)" in "([^"]*)"$"#)]
async fn when_get_blobdiff_plain(world: &mut WebWorld, file: String, project: String) {
    let (head, parent): (String, String) = head_and_parent(world, &project);
    let uri: String = format!("/?p={project}&a=blobdiff_plain&hb={head}&hpb={parent}&f={file}");
    dispatch_capture(world, uri).await;
}

#[when(regex = r#"^I GET the blobdiff_plain of "([^"]*)" renamed from "([^"]*)" in "([^"]*)"$"#)]
async fn when_get_blobdiff_plain_renamed(
    world: &mut WebWorld,
    file: String,
    file_parent: String,
    project: String,
) {
    let (head, parent): (String, String) = head_and_parent(world, &project);
    let uri: String =
        format!("/?p={project}&a=blobdiff_plain&hb={head}&hpb={parent}&f={file}&fp={file_parent}");
    dispatch_capture(world, uri).await;
}

#[when(regex = r#"^I GET the blobdiff of "([^"]*)" in "([^"]*)"$"#)]
async fn when_get_blobdiff(world: &mut WebWorld, file: String, project: String) {
    let (head, parent): (String, String) = head_and_parent(world, &project);
    let uri: String = format!("/?p={project}&a=blobdiff&hb={head}&hpb={parent}&f={file}");
    dispatch_capture(world, uri).await;
}

/// A blobdiff whose base is the symbolic `HEAD` ref (the parent base stays a
/// literal oid) — so gitweb's dual-oid Expires gate is NOT met and no freshness
/// window is stamped.
#[when(regex = r#"^I GET the blobdiff of "([^"]*)" in "([^"]*)" with a ref base$"#)]
async fn when_get_blobdiff_ref_base(world: &mut WebWorld, file: String, project: String) {
    let (_head, parent): (String, String) = head_and_parent(world, &project);
    let uri: String = format!("/?p={project}&a=blobdiff&hb=HEAD&hpb={parent}&f={file}");
    dispatch_capture(world, uri).await;
}

/// The raw single-file diff with the symbolic `HEAD` ref as its base — the same
/// dual-oid Expires miss as the host page's ref-base case.
#[when(regex = r#"^I GET the blobdiff_plain of "([^"]*)" in "([^"]*)" with a ref base$"#)]
async fn when_get_blobdiff_plain_ref_base(world: &mut WebWorld, file: String, project: String) {
    let (_head, parent): (String, String) = head_and_parent(world, &project);
    let uri: String = format!("/?p={project}&a=blobdiff_plain&hb=HEAD&hpb={parent}&f={file}");
    dispatch_capture(world, uri).await;
}

/// Resolves a blobdiff request's bases as TREE ids rather than commit ids: the
/// HEAD commit's tree and its first parent's tree. A tree is tree-ish, so the
/// single-file diff between them is the same — but a tree is not a commit, the
/// hand-crafted URI gitweb's `git_blobdiff` `else` branch renders degenerately.
fn head_tree_and_parent_tree(world: &WebWorld, project: &str) -> (String, String) {
    let store: GixProjectStore = GixProjectStore::new(root(world).path().to_path_buf());
    let repository: Box<dyn Repository> = store.open(project).expect("open the project");
    let head: ObjectId = repository.resolve("HEAD").expect("resolve HEAD");
    let commit: Commit = repository.find_commit(&head).expect("read the HEAD commit");
    let parent: &ObjectId = commit
        .parents()
        .first()
        .expect("the HEAD commit has a parent to diff against");
    let parent_commit: Commit = repository
        .find_commit(parent)
        .expect("read the parent commit");
    (
        commit.tree().as_str().to_owned(),
        parent_commit.tree().as_str().to_owned(),
    )
}

#[when(regex = r#"^I GET the blobdiff of "([^"]*)" in "([^"]*)" with tree bases$"#)]
async fn when_get_blobdiff_tree_bases(world: &mut WebWorld, file: String, project: String) {
    let (head_tree, parent_tree): (String, String) = head_tree_and_parent_tree(world, &project);
    let uri: String = format!("/?p={project}&a=blobdiff&hb={head_tree}&hpb={parent_tree}&f={file}");
    dispatch_capture(world, uri).await;
}

#[when(regex = r#"^I GET the file diff of "([^"]*)" in "([^"]*)"$"#)]
async fn when_get_file_diff(world: &mut WebWorld, file: String, project: String) {
    let (head, parent): (String, String) = head_and_parent(world, &project);
    let uri: String = format!("/diff?p={project}&h={head}&hp={parent}&f={file}");
    dispatch_capture(world, uri).await;
}

/// The full hex object id a project's HEAD resolves to — for a by-oid request
/// that exercises gitweb's `$expires = "+1d"` (an `Expires` header only when the
/// request hash is a literal oid).
fn head_full_oid(world: &WebWorld, project: &str) -> String {
    let store: GixProjectStore = GixProjectStore::new(root(world).path().to_path_buf());
    let repository: Box<dyn Repository> = store.open(project).expect("open the project");
    repository
        .resolve("HEAD")
        .expect("resolve HEAD")
        .as_str()
        .to_owned()
}

#[when(regex = r#"^I GET the "([^"]*)" of "([^"]*)" by its full object id$"#)]
async fn when_get_action_by_oid(world: &mut WebWorld, action: String, project: String) {
    let oid: String = head_full_oid(world, &project);
    let uri: String = format!("/?p={project}&a={action}&h={oid}");
    dispatch_capture(world, uri).await;
}

#[when(regex = r#"^I GET the blob "([^"]*)" of "([^"]*)" by its full object id$"#)]
async fn when_get_blob_by_oid(world: &mut WebWorld, file: String, project: String) {
    let store: GixProjectStore = GixProjectStore::new(root(world).path().to_path_buf());
    let repository: Box<dyn Repository> = store.open(&project).expect("open the project");
    let head: ObjectId = repository.resolve("HEAD").expect("resolve HEAD");
    let blob: ObjectId = repository
        .path_id(&head, &file)
        .expect("path lookup")
        .expect("the file exists at HEAD");
    let uri: String = format!("/?p={project}&a=blob&h={}", blob.as_str());
    dispatch_capture(world, uri).await;
}

/// Drives the router with a GET for `uri` and captures the response — the shared
/// core of every plain GET step.
async fn dispatch_capture(world: &mut WebWorld, uri: String) {
    dispatch_capture_with(world, uri, Method::GET, &[]).await;
}

/// Drives the router with `uri` under `method` and the given request headers,
/// capturing the response status, headers, and body into the world. The
/// header-aware core behind the conditional-GET / HEAD / Accept steps.
async fn dispatch_capture_with(
    world: &mut WebWorld,
    uri: String,
    method: Method,
    headers: &[(header::HeaderName, String)],
) {
    let store: Arc<dyn ProjectStore + Send + Sync> =
        Arc::new(GixProjectStore::new(root(world).path().to_path_buf()));
    let dispatcher: Arc<Dispatcher> = Arc::new(world.dispatcher.clone());
    let app: Router = router(store, dispatcher);

    let mut builder = HttpRequest::builder().method(method).uri(uri);
    for (name, value) in headers {
        builder = builder.header(name, value);
    }
    let request: HttpRequest<Body> = builder
        .body(Body::empty())
        .expect("the test request builds");
    let response = app.oneshot(request).await.expect("the router responds");

    let status: u16 = response.status().as_u16();
    let content_type: Option<String> = response
        .headers()
        .get(header::CONTENT_TYPE)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .map(str::to_owned);
    let content_disposition: Option<String> = response
        .headers()
        .get(header::CONTENT_DISPOSITION)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .map(str::to_owned);
    let last_modified: Option<String> = response
        .headers()
        .get(header::LAST_MODIFIED)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .map(str::to_owned);
    let expires: Option<String> = response
        .headers()
        .get(header::EXPIRES)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .map(str::to_owned);
    let location: Option<String> = response
        .headers()
        .get(header::LOCATION)
        .and_then(|value: &header::HeaderValue| value.to_str().ok())
        .map(str::to_owned);
    let bytes: axum::body::Bytes = to_bytes(response.into_body(), usize::MAX)
        .await
        .expect("the response body collects");

    world.response_status = Some(status);
    world.response_content_type = content_type;
    world.response_content_disposition = content_disposition;
    world.response_last_modified = last_modified;
    world.response_expires = expires;
    world.response_location = location;
    world.response_body = Some(String::from_utf8_lossy(&bytes).into_owned());
    world.response_body_bytes = Some(bytes.to_vec());
}

// --- Thens: the served response ----------------------------------------------

#[then(regex = r#"^the response status is (\d+)$"#)]
fn then_response_status_is(world: &mut WebWorld, expected: u16) {
    assert_eq!(world.response_status, Some(expected));
}

#[then(regex = r#"^the response content type is "([^"]*)"$"#)]
fn then_response_content_type_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        world.response_content_type.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the response last-modified is "([^"]*)"$"#)]
fn then_response_last_modified_is(world: &mut WebWorld, expected: String) {
    assert_eq!(
        world.response_last_modified.as_deref(),
        Some(expected.as_str())
    );
}

#[then(regex = r#"^the response redirects to "([^"]*)"$"#)]
fn then_response_redirects_to(world: &mut WebWorld, expected: String) {
    assert_eq!(world.response_location.as_deref(), Some(expected.as_str()));
}

#[then(regex = r#"^the response location contains "([^"]*)"$"#)]
fn then_response_location_contains(world: &mut WebWorld, needle: String) {
    let location: &str = world
        .response_location
        .as_deref()
        .expect("a redirect location must have been captured");
    assert!(
        location.contains(&needle),
        "location did not contain {needle:?}: {location}"
    );
}

#[then(regex = r#"^the response is offered inline as "([^"]*)"$"#)]
fn then_response_offered_inline_as(world: &mut WebWorld, file_name: String) {
    assert_eq!(
        world.response_content_disposition.as_deref(),
        Some(format!(r#"inline; filename="{file_name}""#).as_str())
    );
}

#[then(regex = r#"^the response body contains "(.*)"$"#)]
fn then_response_body_contains(world: &mut WebWorld, needle: String) {
    let body: &str = world
        .response_body
        .as_deref()
        .expect("a response body must have been captured");
    assert!(
        body.contains(&needle),
        "body did not contain {needle:?}: {body}"
    );
}

#[then(regex = r#"^the response body does not contain "(.*)"$"#)]
fn then_response_body_excludes(world: &mut WebWorld, needle: String) {
    let body: &str = world
        .response_body
        .as_deref()
        .expect("a response body must have been captured");
    assert!(
        !body.contains(&needle),
        "body unexpectedly contained {needle:?}: {body}"
    );
}

#[then("the response body is empty")]
fn then_response_body_empty(world: &mut WebWorld) {
    let body: &[u8] = world
        .response_body_bytes
        .as_deref()
        .expect("a response body must have been captured");
    assert!(body.is_empty(), "body was not empty: {body:02x?}");
}

#[tokio::main]
async fn main() {
    WebWorld::run("features/web").await;
}
