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
use std::borrow::Cow;

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
/// patch, patches, blame_data, project_index).
const TEXT_MIME: &str = "text/plain; charset=utf-8";

/// The body of a successful response: decoded text (an HTML page or a plain feed
/// / patch) or raw bytes (a snapshot archive, a binary blob).
enum ViewBody {
    /// UTF-8 text.
    Text(String),
    /// Opaque bytes.
    Bytes(Vec<u8>),
}

/// A `200 OK` body and the headers it carries.
struct BodyView {
    content_type: Cow<'static, str>,
    content_disposition: Option<String>,
    last_modified: Option<String>,
    body: ViewBody,
}

/// What a handler produces: a `200 OK` body, or a `302 Found` redirect. Handler
/// failures travel as [`DomainError`] instead and are mapped by
/// [`error_to_response`].
enum ViewKind {
    /// A body served with `200 OK`.
    Body(BodyView),
    /// A redirect served with `302 Found` and a `Location` header — gitweb's
    /// `git_object` `$cgi->redirect(-status => '302 Found')`.
    Redirect { location: String },
}

/// A successful view a handler produces. Most actions render a body; gitweb's
/// `object` action instead redirects to the view for the resolved object kind.
pub struct View {
    kind: ViewKind,
}

impl View {
    /// Wraps a `200 OK` body and its headers.
    fn body(
        content_type: Cow<'static, str>,
        content_disposition: Option<String>,
        last_modified: Option<String>,
        body: ViewBody,
    ) -> Self {
        Self {
            kind: ViewKind::Body(BodyView {
                content_type,
                content_disposition,
                last_modified,
                body,
            }),
        }
    }

    /// An HTML page (`text/html; charset=utf-8`). The handler supplies the
    /// already-assembled document; chrome is the render layer's job, not the
    /// adapter's.
    #[must_use]
    pub fn html(markup: Markup) -> Self {
        Self::body(
            Cow::Borrowed(HTML_MIME),
            None,
            None,
            ViewBody::Text(markup.into_string()),
        )
    }

    /// A plain-text body (`text/plain; charset=utf-8`).
    #[must_use]
    pub fn plain_text(text: impl Into<String>) -> Self {
        Self::body(
            Cow::Borrowed(TEXT_MIME),
            None,
            None,
            ViewBody::Text(text.into()),
        )
    }

    /// A text body under an explicit content type — for endpoints whose media
    /// type is neither plain HTML nor plain text (RSS/Atom/OPML XML feeds).
    #[must_use]
    pub fn text(content_type: &'static str, body: String) -> Self {
        Self::body(
            Cow::Borrowed(content_type),
            None,
            None,
            ViewBody::Text(body),
        )
    }

    /// A syndication feed (gitweb's `rss`/`atom`): an XML body under its feed
    /// media type, carrying the `Last-Modified` value gitweb derives from the
    /// newest commit (`None` for an empty feed, which has no commit to date it).
    /// The conditional-GET `304` path that consumes this header lives with the
    /// caching cross-cut, not here.
    #[must_use]
    pub fn feed(content_type: &'static str, body: String, last_modified: Option<String>) -> Self {
        Self::body(
            Cow::Borrowed(content_type),
            None,
            last_modified,
            ViewBody::Text(body),
        )
    }

    /// A text body served under an explicit content type and offered inline
    /// under a fixed filename — gitweb's machine-readable listings
    /// (`project_index`'s `index.aux`, `opml`'s `opml.xml`), which set both a
    /// `-type`/`-charset` and a `-content_disposition`. Both header values are
    /// fixed per endpoint, so they are `'static`.
    #[must_use]
    pub fn text_attachment(
        content_type: &'static str,
        content_disposition: &'static str,
        body: String,
    ) -> Self {
        Self::body(
            Cow::Borrowed(content_type),
            Some(content_disposition.to_owned()),
            None,
            ViewBody::Text(body),
        )
    }

    /// A plain-text body (`text/plain; charset=utf-8`) offered inline under a
    /// per-request filename — gitweb's `commitdiff_plain` / `patch`, which set a
    /// fixed `-type`/`-charset` but a `-content_disposition` whose filename
    /// carries the project and hash, so the disposition is owned.
    #[must_use]
    pub fn plain_attachment(content_disposition: String, body: String) -> Self {
        Self::body(
            Cow::Borrowed(TEXT_MIME),
            Some(content_disposition),
            None,
            ViewBody::Text(body),
        )
    }

    /// A raw-bytes body under an explicit content type — gitweb's snapshot
    /// archives.
    #[must_use]
    pub fn bytes(content_type: &'static str, bytes: Vec<u8>) -> Self {
        Self::body(
            Cow::Borrowed(content_type),
            None,
            None,
            ViewBody::Bytes(bytes),
        )
    }

    /// A raw blob streamed verbatim (gitweb's `blob_plain`): the bytes under a
    /// content type and content disposition the use case derived from the blob's
    /// content and file name. Both header values are dynamic, so they are owned.
    #[must_use]
    pub fn raw_blob(content_type: String, content_disposition: String, bytes: Vec<u8>) -> Self {
        Self::body(
            Cow::Owned(content_type),
            Some(content_disposition),
            None,
            ViewBody::Bytes(bytes),
        )
    }

    /// A `302 Found` redirect to `location` — gitweb's `git_object`, which
    /// resolves an object's kind and redirects to the view for it. The body is
    /// empty; the `Location` carries the absolute target URL the boundary built.
    #[must_use]
    pub fn redirect(location: String) -> Self {
        Self {
            kind: ViewKind::Redirect { location },
        }
    }
}

impl IntoResponse for View {
    fn into_response(self) -> Response {
        match self.kind {
            ViewKind::Body(view) => body_into_response(view),
            ViewKind::Redirect { location } => redirect_into_response(&location),
        }
    }
}

/// Serves a `200 OK` body with its content type and any optional headers.
fn body_into_response(view: BodyView) -> Response {
    let body: Body = match view.body {
        ViewBody::Text(text) => Body::from(text),
        ViewBody::Bytes(bytes) => Body::from(bytes),
    };
    let mut response: Response = body.into_response();
    // `from_bytes` permits the obs-text range (0x80–0xFF), so a non-ASCII
    // file name in the disposition (a latin1 blob name) is preserved rather
    // than dropped; a value with control bytes is simply omitted.
    if let Ok(value) = HeaderValue::from_bytes(view.content_type.as_bytes()) {
        response.headers_mut().insert(header::CONTENT_TYPE, value);
    }
    if let Some(value) = view
        .content_disposition
        .and_then(|disposition: String| HeaderValue::from_bytes(disposition.as_bytes()).ok())
    {
        response
            .headers_mut()
            .insert(header::CONTENT_DISPOSITION, value);
    }
    if let Some(value) = view
        .last_modified
        .and_then(|stamp: String| HeaderValue::from_str(&stamp).ok())
    {
        response.headers_mut().insert(header::LAST_MODIFIED, value);
    }
    response
}

/// Serves a `302 Found` with a `Location` header (gitweb's `git_object`
/// redirect). The target URL is `esc_param`-encoded ASCII, so a control-byte
/// rejection cannot happen for a well-formed URL.
fn redirect_into_response(location: &str) -> Response {
    let mut response: Response = StatusCode::FOUND.into_response();
    if let Ok(value) = HeaderValue::from_str(location) {
        response.headers_mut().insert(header::LOCATION, value);
    }
    response
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
