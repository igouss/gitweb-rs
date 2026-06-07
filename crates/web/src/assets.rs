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
use gitweb_render::assets::{FAVICON_SVG, STYLESHEET};

/// URL the stylesheet is served at — the value document heads link to.
pub const STYLESHEET_PATH: &str = "/static/style.css";
/// URL the favicon is served at.
pub const FAVICON_PATH: &str = "/static/favicon.svg";

/// `text/css; charset=utf-8`.
const CSS_MIME: &str = "text/css; charset=utf-8";
/// `image/svg+xml` — the scalable favicon.
const SVG_MIME: &str = "image/svg+xml";

/// `GET /static/style.css` — the modernized stylesheet.
pub(crate) async fn stylesheet() -> Response {
    ([(header::CONTENT_TYPE, CSS_MIME)], STYLESHEET).into_response()
}

/// `GET /static/favicon.svg` — the site favicon.
pub(crate) async fn favicon() -> Response {
    ([(header::CONTENT_TYPE, SVG_MIME)], FAVICON_SVG).into_response()
}
