//! The runnable gitweb-rs server: gitweb's CGI entry point as a standalone HTTP
//! daemon.
//!
//! The thinnest possible shell over the composition root: load the global
//! settings, resolve where the repositories live, bind a listener, and serve the
//! assembled router until interrupted. All the wiring lives in the library
//! ([`gitweb_app`]); this file only reads the deployment knobs and starts the
//! server.
//!
//! Deployment knobs (environment):
//! - `GITWEB_CONFIG`, `GITWEB_CONFIG_SYSTEM`, `GITWEB_CONFIG_COMMON` — config
//!   files, gitweb's precedence (see [`gitweb_app::load_settings`]).
//! - `GITWEB_PROJECTROOT` — the directory the repositories live under; overrides
//!   the config's `projectroot` so a quick run needs no config file.
//! - `GITWEB_ADDR` — the listen address (default `127.0.0.1:8080`).
//! - `GITWEB_STATIC_DIR` — the directory the vendored diff-viewer bundle is
//!   served from (default `./static`). Build it with `scripts/vendor-pierre.sh`;
//!   its absence only disables inline diff rendering (see `static/vendor/`).

use std::net::SocketAddr;
use std::path::PathBuf;
use std::process::ExitCode;
use std::sync::Arc;

use axum::Router;
use gitweb_app::{build_router, load_settings};
use gitweb_domain::model::settings::Settings;
use tokio::net::TcpListener;

/// The default listen address when `GITWEB_ADDR` is unset.
const DEFAULT_ADDR: &str = "127.0.0.1:8080";

#[tokio::main]
async fn main() -> ExitCode {
    match serve().await {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("gitweb-rs: {error}");
            ExitCode::FAILURE
        }
    }
}

/// Loads the configuration, assembles the router, and serves it until a shutdown
/// signal arrives.
///
/// # Errors
///
/// Returns the config-loading error, or the I/O error from binding or serving.
async fn serve() -> Result<(), Box<dyn std::error::Error>> {
    let settings: Settings = load_settings()?;
    let projectroot: PathBuf = resolve_projectroot(&settings);
    let addr: SocketAddr = bind_addr()?;
    let app: Router = build_router(projectroot, Arc::new(settings));

    let listener: TcpListener = TcpListener::bind(addr).await?;
    eprintln!("gitweb-rs: serving on http://{addr}");
    axum::serve(listener, app)
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Resolves where the repositories live: the `GITWEB_PROJECTROOT` override if
/// set and non-empty, otherwise the config's `projectroot`.
fn resolve_projectroot(settings: &Settings) -> PathBuf {
    std::env::var("GITWEB_PROJECTROOT")
        .ok()
        .filter(|value: &String| !value.is_empty())
        .unwrap_or_else(|| settings.projectroot().to_owned())
        .into()
}

/// Parses the listen address from `GITWEB_ADDR`, falling back to
/// [`DEFAULT_ADDR`].
///
/// # Errors
///
/// Returns the parse error when `GITWEB_ADDR` is set but not a valid socket
/// address.
fn bind_addr() -> Result<SocketAddr, std::net::AddrParseError> {
    match std::env::var("GITWEB_ADDR") {
        Ok(value) if !value.is_empty() => value.parse(),
        _ => Ok(DEFAULT_ADDR.parse().expect("the default address is valid")),
    }
}

/// Resolves when the process receives Ctrl-C, so the server drains in-flight
/// requests before exiting instead of dropping connections.
async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
}
