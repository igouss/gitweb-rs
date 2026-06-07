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
use gitweb_web::{Dispatcher, Handler, HeadsHandler, ProjectListHandler, TagsHandler, router};

/// Assembles the full gitweb-rs router: a gix project store rooted at
/// `projectroot`, the dispatch table populated with every handler this build
/// serves, and the web boundary wrapped around them. The binary binds a listener
/// around the returned [`Router`]; the BDD harness drives it in-process.
pub fn build_router(projectroot: PathBuf, settings: Arc<Settings>) -> Router {
    let store: Arc<dyn ProjectStore + Send + Sync> = Arc::new(GixProjectStore::new(projectroot));
    let dispatcher: Arc<Dispatcher> = Arc::new(build_dispatcher(Arc::clone(&store), settings));
    router(store, dispatcher)
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

    let heads: Arc<dyn Handler> =
        Arc::new(HeadsHandler::new(Arc::clone(&store), Arc::clone(&settings)));
    dispatcher.register(Action::Heads, heads);

    let tags: Arc<dyn Handler> = Arc::new(TagsHandler::new(store, settings));
    dispatcher.register(Action::Tags, tags);

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
