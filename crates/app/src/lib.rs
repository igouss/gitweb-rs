//! The composition root: the one crate that knows every adapter.
//!
//! Hexagonally, the domain and the other crates depend only inward, on ports —
//! never on a concrete adapter. *Something* has to bind those ports to real
//! implementations, and that something is here, at the outermost layer. This
//! crate wires the gix-backed [`GixProjectStore`] into the `ProjectStore` port,
//! registers each capability's [`Handler`] in the [`Dispatcher`], and hands the
//! result to [`router`] (the web boundary) to assemble the axum [`Router`].
//!
//! [`build_router`] is that assembly, factored out of the binary so the BDD
//! harness drives the exact same wiring the server runs. [`load_settings`] reads
//! the global config the way the binary does. The binary itself ([`main`]) is
//! the thinnest possible shell: load, build, bind, serve.
//!
//! [`main`]: ../gitweb_rs/index.html

use std::path::{Path, PathBuf};
use std::sync::Arc;

use axum::Router;
use gitweb_config::ConfigError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::config_chain::{ConfigChain, ConfigSlot};
use gitweb_domain::model::settings::Settings;
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_git::GixProjectStore;
use gitweb_web::{
    BlobHandler, BlobPlainHandler, CommitHandler, CommitdiffHandler, CommitdiffPlainHandler,
    Dispatcher, FeedHandler, Handler, HeadsHandler, HistoryHandler, LogHandler, ObjectHandler,
    OpmlHandler, ProjectIndexHandler, ProjectListHandler, RemotesHandler, ShortlogHandler,
    SummaryHandler, TagHandler, TagsHandler, TreeHandler, router,
};
use tower_http::services::ServeDir;

/// Assembles the full gitweb-rs router: a gix project store rooted at
/// `projectroot`, the dispatch table populated with every handler this build
/// serves, and the web boundary wrapped around them. The binary binds a listener
/// around the returned [`Router`]; the BDD harness drives it in-process.
pub fn build_router(projectroot: PathBuf, settings: Arc<Settings>) -> Router {
    let store: Arc<dyn ProjectStore + Send + Sync> = Arc::new(GixProjectStore::new(projectroot));
    let dispatcher: Arc<Dispatcher> = Arc::new(build_dispatcher(Arc::clone(&store), settings));
    mount_vendor_assets(router(store, dispatcher))
}

/// Mounts the vendored `@pierre/diffs` viewer bundle under `/static/vendor`,
/// served from the `vendor/` subdirectory of the static directory (gitweb's
/// `static/`). The bundle is built on demand by `scripts/vendor-pierre.sh` and
/// git-ignored (~11 MB of Shiki grammars); when it is absent, [`ServeDir`] simply
/// returns `404` for those URLs, so the binary still serves every other route and
/// the diff host page degrades to its no-JavaScript raw-diff link. The diff
/// boot module ([`gitweb_render::assets::DIFF_VIEWER_JS`]) imports the bundle from
/// `/static/vendor/pierre/`, the prefix this mount strips.
fn mount_vendor_assets(router: Router) -> Router {
    let vendor_dir: PathBuf = static_dir().join("vendor");
    router.nest_service("/static/vendor", ServeDir::new(vendor_dir))
}

/// The directory the runtime-served static assets live under — gitweb's
/// `static/`. Read from `GITWEB_STATIC_DIR`, defaulting to `./static` relative to
/// the working directory. Only the vendored viewer bundle is served from here;
/// the stylesheet, favicon and diff boot module are baked into the binary.
fn static_dir() -> PathBuf {
    std::env::var("GITWEB_STATIC_DIR")
        .ok()
        .filter(|value: &String| !value.is_empty())
        .map_or_else(|| PathBuf::from("static"), PathBuf::from)
}

/// gitweb's `%actions` table, populated with the handlers this build serves.
///
/// This is the single registration point: as each capability bead lands its
/// handler, it adds exactly one line here. Every action without a registered
/// handler takes gitweb's `die_error(400, "Unknown action")` path until then.
fn build_dispatcher(
    store: Arc<dyn ProjectStore + Send + Sync>,
    settings: Arc<Settings>,
) -> Dispatcher {
    let mut dispatcher: Dispatcher = Dispatcher::new();

    let project_list: Arc<dyn Handler> = Arc::new(ProjectListHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::ProjectList, project_list);

    let project_index: Arc<dyn Handler> = Arc::new(ProjectIndexHandler::new(Arc::clone(&store)));
    dispatcher.register(Action::ProjectIndex, project_index);

    let summary: Arc<dyn Handler> = Arc::new(SummaryHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::Summary, summary);

    let heads: Arc<dyn Handler> =
        Arc::new(HeadsHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Heads, heads);

    let shortlog: Arc<dyn Handler> = Arc::new(ShortlogHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::Shortlog, shortlog);

    let log: Arc<dyn Handler> =
        Arc::new(LogHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Log, log);

    let history: Arc<dyn Handler> = Arc::new(HistoryHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::History, history);

    let tags: Arc<dyn Handler> =
        Arc::new(TagsHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Tags, tags);

    let tag: Arc<dyn Handler> =
        Arc::new(TagHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Tag, tag);

    let tree: Arc<dyn Handler> =
        Arc::new(TreeHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Tree, tree);

    let blob: Arc<dyn Handler> =
        Arc::new(BlobHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Blob, blob);

    let commit: Arc<dyn Handler> = Arc::new(CommitHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::Commit, commit);

    let commitdiff: Arc<dyn Handler> = Arc::new(CommitdiffHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::Commitdiff, commitdiff);

    let blob_plain: Arc<dyn Handler> = Arc::new(BlobPlainHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
    ));
    dispatcher.register(Action::BlobPlain, blob_plain);

    // The feeds, OPML, and the commitdiff_plain self-link are absolute against
    // the site URL; gitweb derives it per-request (SERVER_NAME), but a standalone
    // daemon takes a configured canonical base (GITWEB_BASE_URL), defaulting to
    // the CGI default host. The generator stamps the feed with this build's version.
    let base: String =
        std::env::var("GITWEB_BASE_URL").unwrap_or_else(|_| "http://localhost".to_owned());
    let generator: String = concat!("gitweb-rs/", env!("CARGO_PKG_VERSION")).to_owned();

    // commitdiff_plain's `X-Git-Url` line is the request's absolute self URL.
    let commitdiff_plain: Arc<dyn Handler> = Arc::new(CommitdiffPlainHandler::new(
        Arc::clone(&store),
        base.clone(),
    ));
    dispatcher.register(Action::CommitdiffPlain, commitdiff_plain);

    // The object action redirects (gitweb's `href(-full => 1, …)`), absolute
    // against the same site URL.
    let object: Arc<dyn Handler> = Arc::new(ObjectHandler::new(Arc::clone(&store), base.clone()));
    dispatcher.register(Action::Object, object);

    let rss: Arc<dyn Handler> = Arc::new(FeedHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
        base.clone(),
        generator.clone(),
    ));
    dispatcher.register(Action::Rss, rss);
    let atom: Arc<dyn Handler> = Arc::new(FeedHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
        base.clone(),
        generator,
    ));
    dispatcher.register(Action::Atom, atom);

    // OPML links are absolute against the same site URL the feeds use.
    let opml: Arc<dyn Handler> = Arc::new(OpmlHandler::new(
        Arc::clone(&store),
        Arc::clone(&settings),
        base,
    ));
    dispatcher.register(Action::Opml, opml);

    let remotes: Arc<dyn Handler> = Arc::new(RemotesHandler::new(store, settings));
    dispatcher.register(Action::Remotes, remotes);

    dispatcher
}

/// Loads the effective global settings the way the binary does: select the
/// config files from the `GITWEB_CONFIG*` environment (gitweb's
/// `evaluate_gitweb_config` precedence, the pure rule lives in the domain),
/// then read and resolve them over the built-in defaults. With no config files
/// present, this yields the built-in defaults.
///
/// gitweb's config files are executable Perl; gitweb-rs uses declarative TOML
/// instead (see the `gitweb-config` crate). The format never leaks past it.
///
/// # Errors
///
/// Returns a [`ConfigError`] if a selected config file cannot be read, parsed,
/// or names an unknown feature.
pub fn load_settings() -> Result<Settings, ConfigError> {
    let chain: ConfigChain = config_chain_from_env();
    let order: Vec<String> = chain.load_order(|path: &str| Path::new(path).exists());
    let paths: Vec<PathBuf> = order.into_iter().map(PathBuf::from).collect();
    gitweb_config::load(&paths)
}

/// Builds gitweb's three config slots from the environment. There is no
/// compile-time install step for a standalone binary, so each slot's default is
/// empty — config is opt-in via the environment, and an unset slot names no file.
fn config_chain_from_env() -> ConfigChain {
    ConfigChain::new(
        env_slot("GITWEB_CONFIG_COMMON"),
        env_slot("GITWEB_CONFIG"),
        env_slot("GITWEB_CONFIG_SYSTEM"),
    )
}

/// One config slot from environment variable `var` (gitweb's `$ENV{X}`), with no
/// compile-time default.
fn env_slot(var: &str) -> ConfigSlot {
    ConfigSlot::new(std::env::var(var).ok(), None)
}
