//! Gherkin conformance for the TOML config loader.
//!
//! The `Given` writes config files to a real temp directory (as the composition
//! root would lay them out); the `When` reads them back through [`load`] and
//! stores the raw [`Result`], so each `Then` asserts one resolved field or the
//! failure without branching. cucumber supplies its own `main`, so this target
//! sets `harness = false`.
//!
//! [`load`]: gitweb_config::load

use std::path::PathBuf;

use cucumber::gherkin::Step;
use cucumber::{World, given, then, when};
use tempfile::TempDir;

use gitweb_config::ConfigError;
use gitweb_domain::model::settings::{FeatureName, Settings};

#[derive(Debug, Default, World)]
struct ConfigWorld {
    dir: Option<TempDir>,
    paths: Vec<PathBuf>,
    loaded: Option<Result<Settings, ConfigError>>,
}

fn loaded_ok(world: &ConfigWorld) -> &Settings {
    world
        .loaded
        .as_ref()
        .expect("load the files first")
        .as_ref()
        .expect("loading succeeded")
}

fn named_feature(name: &str) -> FeatureName {
    FeatureName::from_key(name).expect("a known feature key")
}

// --- Givens ------------------------------------------------------------------

#[given("no config files")]
fn given_no_files(_world: &mut ConfigWorld) {}

#[given(regex = r#"^a config file "(.*)":$"#)]
fn given_config_file(world: &mut ConfigWorld, step: &Step, name: String) {
    let path: PathBuf = {
        let dir: &mut TempDir = world
            .dir
            .get_or_insert_with(|| tempfile::tempdir().expect("create a temp config dir"));
        dir.path().join(&name)
    };
    let body: &str = step
        .docstring
        .as_deref()
        .expect("a config file step needs a TOML docstring");
    std::fs::write(&path, body).expect("write the config fixture");
    world.paths.push(path);
}

// --- Whens -------------------------------------------------------------------

#[when("I load the configured files")]
fn when_load(world: &mut ConfigWorld) {
    world.loaded = Some(gitweb_config::load(&world.paths));
}

// --- Thens -------------------------------------------------------------------

#[then("loading succeeds")]
fn loading_succeeds(world: &mut ConfigWorld) {
    let result: &Result<Settings, ConfigError> =
        world.loaded.as_ref().expect("load the files first");
    assert!(
        result.is_ok(),
        "expected loading to succeed, got: {result:?}"
    );
}

#[then("loading fails")]
fn loading_fails(world: &mut ConfigWorld) {
    let result: &Result<Settings, ConfigError> =
        world.loaded.as_ref().expect("load the files first");
    assert!(
        result.is_err(),
        "expected loading to fail, but it succeeded"
    );
}

#[then(regex = r#"^the resolved projectroot is "(.*)"$"#)]
fn resolved_projectroot(world: &mut ConfigWorld, expected: String) {
    assert_eq!(loaded_ok(world).projectroot(), expected);
}

#[then(regex = r#"^the resolved site name is "(.*)"$"#)]
fn resolved_site_name(world: &mut ConfigWorld, expected: String) {
    assert_eq!(loaded_ok(world).site_name(), expected);
}

#[then(regex = r#"^the resolved clone base URLs are "(.*)"$"#)]
fn resolved_clone_urls(world: &mut ConfigWorld, expected: String) {
    assert_eq!(loaded_ok(world).git_base_url_list().join(", "), expected);
}

#[then(regex = r#"^the resolved "(.*)" feature default is "(.*)"$"#)]
fn resolved_feature_default(world: &mut ConfigWorld, name: String, expected: String) {
    let feature: FeatureName = named_feature(&name);
    assert_eq!(
        loaded_ok(world)
            .feature(feature)
            .default_options()
            .join(", "),
        expected
    );
}

#[then(regex = r#"^the resolved "(.*)" feature is overridable$"#)]
fn resolved_feature_overridable(world: &mut ConfigWorld, name: String) {
    let feature: FeatureName = named_feature(&name);
    assert!(loaded_ok(world).feature(feature).is_overridable());
}

#[tokio::main]
async fn main() {
    ConfigWorld::run("features/conformance").await;
}
