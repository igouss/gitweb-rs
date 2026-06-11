//! Static-asset serving (gitweb's `static/` directory).
//!
//! gitweb ships a stylesheet, a JavaScript file, a logo and a favicon as files
//! the web server hands out directly. We bake the render layer's modernized
//! stylesheet and favicon into the binary (see [`gitweb_render::assets`]) and
//! serve them here at stable URLs with the correct MIME type — no filesystem,
//! nothing to go missing at runtime.
//!
//! The JavaScript and logo assets are owned by the beads that produce them (the
//! client-side JS bead and the chrome logo); this module mounts what exists and
//! is the one place to add the rest as they land.

use axum::http::header;
use axum::response::{IntoResponse, Response};
use gitweb_render::assets::{
    ACTIONS_JS, BLAME_INCREMENTAL_JS, DIFF_VIEWER_JS, FAVICON_SVG, STYLESHEET, TIMEZONE_JS,
};

/// URL the stylesheet is served at — the value document heads link to.
pub const STYLESHEET_PATH: &str = "/static/style.css";
/// URL the favicon is served at.
pub const FAVICON_PATH: &str = "/static/favicon.svg";
/// URL the diff-viewer boot module is served at — the `src` the diff host page
/// boots (see the render `diff_host` module).
pub const DIFF_VIEWER_PATH: &str = "/static/diff-viewer.js";
/// URL the timezone client module is served at (gitweb's `javascript-timezone`).
pub const TIMEZONE_PATH: &str = "/static/gitweb-timezone.js";
/// URL the actions client module is served at (gitweb's `javascript-actions`).
pub const ACTIONS_PATH: &str = "/static/gitweb-actions.js";
/// URL the incremental-blame client module is served at (gitweb's
/// `blame_incremental.js`); wired in by the downstream `blame_data` bead.
pub const BLAME_INCREMENTAL_PATH: &str = "/static/blame-incremental.js";

/// `text/css; charset=utf-8`.
const CSS_MIME: &str = "text/css; charset=utf-8";
/// `image/svg+xml` — the scalable favicon.
const SVG_MIME: &str = "image/svg+xml";
/// `text/javascript; charset=utf-8` — the MIME type a browser requires before
/// it will execute an ES module.
const JS_MIME: &str = "text/javascript; charset=utf-8";

/// `GET /static/style.css` — the modernized stylesheet.
pub(crate) async fn stylesheet() -> Response {
    ([(header::CONTENT_TYPE, CSS_MIME)], STYLESHEET).into_response()
}

/// `GET /static/favicon.svg` — the site favicon.
pub(crate) async fn favicon() -> Response {
    ([(header::CONTENT_TYPE, SVG_MIME)], FAVICON_SVG).into_response()
}

/// `GET /static/diff-viewer.js` — the diff-viewer boot module.
pub(crate) async fn diff_viewer_js() -> Response {
    ([(header::CONTENT_TYPE, JS_MIME)], DIFF_VIEWER_JS).into_response()
}

/// `GET /static/gitweb-timezone.js` — the timezone client module.
pub(crate) async fn timezone_js() -> Response {
    ([(header::CONTENT_TYPE, JS_MIME)], TIMEZONE_JS).into_response()
}

/// `GET /static/gitweb-actions.js` — the JavaScript-actions client module.
pub(crate) async fn actions_js() -> Response {
    ([(header::CONTENT_TYPE, JS_MIME)], ACTIONS_JS).into_response()
}

/// `GET /static/blame-incremental.js` — the incremental-blame client module.
pub(crate) async fn blame_incremental_js() -> Response {
    ([(header::CONTENT_TYPE, JS_MIME)], BLAME_INCREMENTAL_JS).into_response()
}
