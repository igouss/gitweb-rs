//! The axum router: gitweb's CGI entry point as a standalone HTTP server.
//!
//! [`router`] assembles the whole boundary — the static-asset mounts and the
//! catch-all that runs gitweb's request loop for every other URL: decode the
//! path and query, [`resolve`] them into a validated request, [`dispatch`] it,
//! and render the [`View`] (or map a [`DomainError`] to gitweb's `die_error`
//! page). It takes its collaborators — the project store and the dispatcher —
//! as already-wired trait objects, so the composition root (the app crate) owns
//! the concrete adapters and binds a listener around the returned [`Router`].
//!
//! [`dispatch`]: Dispatcher::dispatch

use std::borrow::Cow;
use std::sync::Arc;

use axum::Router;
use axum::extract::State;
use axum::http::Uri;
use axum::response::{IntoResponse, Response};
use axum::routing::get;
use gitweb_domain::error::DomainError;
use gitweb_domain::port::project_store::ProjectStore;

use crate::assets;
use crate::diff_text;
use crate::dispatch::Dispatcher;
use crate::request::resolve;
use crate::response::{View, error_to_response};

/// The state every request handler shares: the project store (for the inbound
/// project-prefix walk) and the dispatcher (the action table).
#[derive(Clone)]
struct AppState {
    store: Arc<dyn ProjectStore + Send + Sync>,
    dispatcher: Arc<Dispatcher>,
}

/// Builds the gitweb-rs router over an already-wired `store` and `dispatcher`:
/// the static-asset mounts plus the catch-all gitweb request handler. The
/// composition root binds a listener around it (`axum::serve`).
pub fn router(store: Arc<dyn ProjectStore + Send + Sync>, dispatcher: Arc<Dispatcher>) -> Router {
    let state: AppState = AppState { store, dispatcher };
    Router::new()
        .route(assets::STYLESHEET_PATH, get(assets::stylesheet))
        .route(assets::FAVICON_PATH, get(assets::favicon))
        .route(assets::DIFF_VIEWER_PATH, get(assets::diff_viewer_js))
        .route(diff_text::DIFF_TEXT_PATH, get(handle_diff_text))
        .fallback(handle)
        .with_state(state)
}

/// The clean-diff endpoint: the bare unified diff the client viewer fetches. A
/// modern route with no gitweb action (see [`crate::diff_text`]); it shares the
/// project store with the gitweb request loop and maps a failure to the same
/// `die_error` page.
async fn handle_diff_text(State(state): State<AppState>, uri: Uri) -> Response {
    let query: Vec<(String, String)> = decode_query(uri.query());
    match diff_text::serve(state.store.as_ref(), &query) {
        Ok(view) => view.into_response(),
        Err(error) => error_to_response(&error),
    }
}

/// The catch-all: gitweb's request loop for one request. Decode the URL, resolve
/// it into a validated request, dispatch it, and render the result — turning any
/// domain failure into gitweb's `die_error` page.
async fn handle(State(state): State<AppState>, uri: Uri) -> Response {
    let path: String = decode_path(uri.path());
    let query: Vec<(String, String)> = decode_query(uri.query());
    let view: Result<View, DomainError> = resolve(state.store.as_ref(), &query, &path)
        .and_then(|resolved| state.dispatcher.dispatch(&resolved.request));
    match view {
        Ok(view) => view.into_response(),
        Err(error) => error_to_response(&error),
    }
}

/// Percent-decodes the request path into gitweb's PATH_INFO (the CGI server
/// would have decoded it before gitweb saw it).
fn decode_path(raw: &str) -> String {
    percent_encoding::percent_decode_str(raw)
        .decode_utf8_lossy()
        .into_owned()
}

/// Splits and decodes the query string into gitweb's short-name parameter pairs
/// (`application/x-www-form-urlencoded`), preserving order and repeats.
fn decode_query(raw: Option<&str>) -> Vec<(String, String)> {
    raw.map(|query: &str| {
        form_urlencoded::parse(query.as_bytes())
            .map(|(key, value): (Cow<'_, str>, Cow<'_, str>)| {
                (key.into_owned(), value.into_owned())
            })
            .collect()
    })
    .unwrap_or_default()
}
