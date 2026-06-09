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
use gitweb_domain::model::accept::prefer_text_xml_feed;
use gitweb_domain::model::conditional::{Freshness, freshness};
use gitweb_domain::model::expiry::Expiry;
use gitweb_domain::model::timestamp::Timestamp;

use crate::clock::now_epoch;
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
    /// The resource's `Last-Modified` time, formatted into the header on the way
    /// out and compared against the client's `If-Modified-Since` for the `304`
    /// path. `None` for bodies gitweb does not date.
    last_modified: Option<Timestamp>,
    /// Whether this body is a feed whose media type may be renegotiated down to
    /// `text/xml` when the reader's `Accept` prefers it (gitweb's `git_feed`).
    feed_negotiable: bool,
    /// The cache freshness gitweb's by-oid sites carry (`$expires = "+1d"`): the
    /// boundary turns [`Expiry::OneDay`] into an absolute `Expires` date one day
    /// past the request clock. [`Expiry::None`] for content gitweb does not cache.
    expiry: Expiry,
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
    /// Wraps a `200 OK` body and its headers. Not feed-negotiable; feeds use
    /// [`View::feed`].
    fn body(
        content_type: Cow<'static, str>,
        content_disposition: Option<String>,
        last_modified: Option<Timestamp>,
        body: ViewBody,
    ) -> Self {
        Self {
            kind: ViewKind::Body(BodyView {
                content_type,
                content_disposition,
                last_modified,
                feed_negotiable: false,
                expiry: Expiry::None,
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
    /// media type, carrying the `Last-Modified` time gitweb derives from the
    /// newest commit (`None` for an empty feed, which has no commit to date it).
    /// Feed-negotiable: the boundary downgrades the media type to `text/xml` when
    /// the reader's `Accept` prefers it, and honours `If-Modified-Since` against
    /// the carried time — see [`View::into_response_with`].
    #[must_use]
    pub fn feed(
        content_type: &'static str,
        body: String,
        last_modified: Option<Timestamp>,
    ) -> Self {
        Self {
            kind: ViewKind::Body(BodyView {
                content_type: Cow::Borrowed(content_type),
                content_disposition: None,
                last_modified,
                feed_negotiable: true,
                expiry: Expiry::None,
                body: ViewBody::Text(body),
            }),
        }
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

    /// A snapshot archive (gitweb's `snapshot`): the bytes under the format's
    /// media type, offered inline under the download file name, and dated with the
    /// commit's `Last-Modified` (absent for a tree-ish with no commit). The media
    /// type is one of the fixed format strings, so it is borrowed; the disposition
    /// and date are per-request, so they are owned.
    #[must_use]
    pub fn snapshot(
        content_type: &'static str,
        content_disposition: String,
        last_modified: Option<Timestamp>,
        bytes: Vec<u8>,
    ) -> Self {
        Self::body(
            Cow::Borrowed(content_type),
            Some(content_disposition),
            last_modified,
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

    /// Stamps the body with the cache freshness gitweb's by-oid sites carry: a
    /// view addressed by a literal object id may be held by a cache for a day
    /// (`$expires = "+1d"`), so the boundary emits an `Expires` header for it.
    /// A no-op for [`Expiry::None`] and for a redirect, so a handler can hand the
    /// by-oid rule's result straight through without branching.
    #[must_use]
    pub fn with_expiry(mut self, expiry: Expiry) -> Self {
        if let ViewKind::Body(ref mut view) = self.kind {
            view.expiry = expiry;
        }
        self
    }
}

/// The transport-level conditions a request carries that shape its response:
/// the cache validator, the reader's media preference, and whether only headers
/// are wanted. These live on the HTTP request, not in gitweb's `%input_params`,
/// so the boundary extracts them and the handlers stay ignorant of them.
#[derive(Debug, Default, Clone)]
pub struct RequestConditions {
    /// The client's `If-Modified-Since`, if any — drives the `304` path.
    pub if_modified_since: Option<String>,
    /// The client's `Accept`, if any — drives feed media-type negotiation.
    pub accept: Option<String>,
    /// Whether the request was a `HEAD`: emit the headers, drop the body.
    pub is_head: bool,
}

impl IntoResponse for View {
    /// Serves the view with no request conditions — the default for endpoints
    /// outside gitweb's conditional-GET surface.
    fn into_response(self) -> Response {
        self.into_response_with(&RequestConditions::default())
    }
}

impl View {
    /// Serves the view, applying the request's transport conditions: a current
    /// cache short-circuits to a bare `304`; a feed renegotiates to `text/xml`
    /// when the reader prefers it; a `HEAD` keeps the headers but drops the body.
    #[must_use]
    pub fn into_response_with(self, conditions: &RequestConditions) -> Response {
        match self.kind {
            ViewKind::Body(view) => body_into_response(view, conditions),
            ViewKind::Redirect { location } => redirect_into_response(&location),
        }
    }
}

/// Serves a body, honouring the conditional-GET, `Accept`, and `HEAD` behaviours
/// gitweb applies at its feed/snapshot sites.
fn body_into_response(view: BodyView, conditions: &RequestConditions) -> Response {
    // gitweb's exit_if_unmodified_since runs first: a resource no newer than the
    // client's cache earns a bare 304 (Last-Modified only, no body).
    if let Some(stamp) = view.last_modified.as_ref() {
        let validator: Option<&str> = conditions.if_modified_since.as_deref();
        if freshness(stamp.epoch(), validator) == Freshness::NotModified {
            return not_modified_response(stamp);
        }
    }
    // gitweb downgrades a feed to text/xml when the reader's Accept prefers it.
    let content_type: Cow<'static, str> =
        negotiate_content_type(&view, conditions.accept.as_deref());

    let body: Body = if conditions.is_head {
        // gitweb's HEAD optimisation: the same headers, no body.
        Body::empty()
    } else {
        match view.body {
            ViewBody::Text(text) => Body::from(text),
            ViewBody::Bytes(bytes) => Body::from(bytes),
        }
    };
    let mut response: Response = body.into_response();
    // `from_bytes` permits the obs-text range (0x80–0xFF), so a non-ASCII
    // file name in the disposition (a latin1 blob name) is preserved rather
    // than dropped; a value with control bytes is simply omitted.
    if let Ok(value) = HeaderValue::from_bytes(content_type.as_bytes()) {
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
        .and_then(|stamp: Timestamp| HeaderValue::from_str(&stamp.rfc2822()).ok())
    {
        response.headers_mut().insert(header::LAST_MODIFIED, value);
    }
    // gitweb's `$expires = "+1d"`: by-oid content earns an absolute Expires date
    // one freshness window past the request clock — CGI.pm's resolution of "+1d".
    if let Some(value) = expires_header(view.expiry) {
        response.headers_mut().insert(header::EXPIRES, value);
    }
    response
}

/// The absolute `Expires` header value for a body's freshness: the request clock
/// plus the window, in the same HTTP-date form the boundary writes its other
/// dates. `None` when the body carries no freshness window ([`Expiry::None`]).
fn expires_header(expiry: Expiry) -> Option<HeaderValue> {
    let window: i64 = expiry.seconds()?;
    let stamp: Timestamp = Timestamp::from_epoch(now_epoch() + window);
    HeaderValue::from_str(&stamp.rfc2822()).ok()
}

/// gitweb's `Accept`-driven content-type: a feed-negotiable body downgrades to
/// `text/xml` when the reader's `Accept` prefers it over the feed's own bare
/// media type; everything else keeps its declared type.
fn negotiate_content_type(view: &BodyView, accept: Option<&str>) -> Cow<'static, str> {
    if view.feed_negotiable {
        let bare_type: &str = view
            .content_type
            .split(';')
            .next()
            .unwrap_or_default()
            .trim();
        if prefer_text_xml_feed(accept, bare_type) {
            return Cow::Borrowed("text/xml; charset=utf-8");
        }
    }
    view.content_type.clone()
}

/// gitweb's `304 Not Modified`: the status and the resource's `Last-Modified`,
/// with no body.
fn not_modified_response(stamp: &Timestamp) -> Response {
    let mut response: Response = StatusCode::NOT_MODIFIED.into_response();
    if let Ok(value) = HeaderValue::from_str(&stamp.rfc2822()) {
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
