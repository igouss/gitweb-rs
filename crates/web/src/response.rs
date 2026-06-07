//! The outbound side of the boundary: rendered views and domain failures turned
//! into HTTP responses.
//!
//! A successful action produces a [`View`] — a content type and a body — that
//! this module serves with `200 OK`. A failure travels as a [`DomainError`] and
//! is mapped, by [`error_to_response`], to gitweb's `die_error` page: the status
//! gitweb would pick (404 / 400 / 403 / 500), the bare message under the status
//! line, wrapped in the document chrome and served as HTML.
//!
//! Hexagonally this is an adapter: it knows the domain ([`DomainError`]) and the
//! render layer (chrome + the error page), and owns the axum-specific mapping to
//! a [`Response`]. Handlers never touch a `Response`; they return a [`View`].

use axum::body::Body;
use axum::http::{HeaderValue, StatusCode, header};
use axum::response::{IntoResponse, Response};
use gitweb_domain::error::DomainError;
use gitweb_render::chrome::{DocumentHead, document};
use gitweb_render::error::{HttpStatus, error_page, status_for};
use gitweb_render::markup::Markup;

use crate::assets::{FAVICON_PATH, STYLESHEET_PATH};

/// `text/html; charset=utf-8` — every gitweb HTML page.
const HTML_MIME: &str = "text/html; charset=utf-8";
/// `text/plain; charset=utf-8` — gitweb's plain endpoints (commitdiff_plain,
/// blob_plain, patch, patches, blame_data, project_index).
const TEXT_MIME: &str = "text/plain; charset=utf-8";

/// The body of a successful response: decoded text (an HTML page or a plain feed
/// / patch) or raw bytes (a snapshot archive, a binary blob).
enum ViewBody {
    /// UTF-8 text.
    Text(String),
    /// Opaque bytes.
    Bytes(Vec<u8>),
}

/// A successful view a handler produces: a content type and a body, served with
/// `200 OK`. Handler failures travel as [`DomainError`] instead and are mapped
/// by [`error_to_response`].
pub struct View {
    content_type: &'static str,
    body: ViewBody,
}

impl View {
    /// An HTML page (`text/html; charset=utf-8`). The handler supplies the
    /// already-assembled document; chrome is the render layer's job, not the
    /// adapter's.
    #[must_use]
    pub fn html(markup: Markup) -> Self {
        Self {
            content_type: HTML_MIME,
            body: ViewBody::Text(markup.into_string()),
        }
    }

    /// A plain-text body (`text/plain; charset=utf-8`).
    #[must_use]
    pub fn plain_text(text: impl Into<String>) -> Self {
        Self {
            content_type: TEXT_MIME,
            body: ViewBody::Text(text.into()),
        }
    }

    /// A text body under an explicit content type — for endpoints whose media
    /// type is neither plain HTML nor plain text (RSS/Atom/OPML XML feeds).
    #[must_use]
    pub fn text(content_type: &'static str, body: String) -> Self {
        Self {
            content_type,
            body: ViewBody::Text(body),
        }
    }

    /// A raw-bytes body under an explicit content type — gitweb's snapshot
    /// archives and binary blobs.
    #[must_use]
    pub fn bytes(content_type: &'static str, bytes: Vec<u8>) -> Self {
        Self {
            content_type,
            body: ViewBody::Bytes(bytes),
        }
    }
}

impl IntoResponse for View {
    fn into_response(self) -> Response {
        let body: Body = match self.body {
            ViewBody::Text(text) => Body::from(text),
            ViewBody::Bytes(bytes) => Body::from(bytes),
        };
        (
            [(
                header::CONTENT_TYPE,
                HeaderValue::from_static(self.content_type),
            )],
            body,
        )
            .into_response()
    }
}

/// Maps a domain failure to gitweb's `die_error` response: the status gitweb's
/// `die_error` would serve, the bare message under the status line, wrapped in
/// the document chrome and served as an HTML page.
#[must_use]
pub fn error_to_response(error: &DomainError) -> Response {
    let status: HttpStatus = status_for(error);
    let main: Markup = error_page(status, error.message(), None);
    error_page_response(status, main)
}

/// Wraps an error page `main` in the full document and serves it under `status`.
fn error_page_response(status: HttpStatus, main: Markup) -> Response {
    let head: DocumentHead = DocumentHead {
        title: status.title(),
        stylesheet_href: STYLESHEET_PATH.to_owned(),
        favicon_href: Some(FAVICON_PATH.to_owned()),
        feeds: Vec::new(),
    };
    let page: Markup = document(&head, main);
    let code: StatusCode =
        StatusCode::from_u16(status.code).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
    (
        code,
        [(header::CONTENT_TYPE, HeaderValue::from_static(HTML_MIME))],
        page.into_string(),
    )
        .into_response()
}
