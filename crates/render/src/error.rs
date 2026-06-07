//! Modernized `die_error`: failures rendered as styled pages with the right
//! status.
//!
//! gitweb's `die_error` is invoked ~102 times and is really *two* concerns
//! welded together: deciding which HTTP status a failure deserves, and printing
//! a `status - message` page (optionally with a detail block) under that status.
//! This module keeps them apart. [`status_for`] classifies a [`DomainError`]
//! the way gitweb does — missing thing → 404, malformed input → 400, policy
//! denial → 403, internal failure → 500 — and [`error_page`] renders the page
//! body for *any* status, including the request-level 503 that never originates
//! from a port. [`error_response`] is the common path: classify a port failure
//! and render it in one step.
//!
//! Hexagonally this is a Boundary: render knows [`DomainError`] (it depends on
//! the domain) but owns no transport. [`HttpStatus`] is a plain value — a code
//! and its reason phrase — so the web boundary can adapt it to its framework's
//! status type without render ever importing one.

use crate::markup::{Markup, html};
use gitweb_domain::error::DomainError;

/// The HTTP status an error page is served with: a numeric code and its reason
/// phrase. These are the five statuses gitweb's `die_error` emits.
///
/// The code is shown in the page body (`404 - ...`); the reason composes the
/// page title (`404 Not Found`). The web boundary maps `code` onto its own
/// status type — render needs nothing from a web framework.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct HttpStatus {
    /// The numeric status code, e.g. `404`.
    pub code: u16,
    /// The reason phrase, e.g. `Not Found`.
    pub reason: &'static str,
}

impl HttpStatus {
    /// `400 Bad Request` — malformed or out-of-range request parameters.
    pub const BAD_REQUEST: Self = Self {
        code: 400,
        reason: "Bad Request",
    };
    /// `403 Forbidden` — access denied by policy (export rules, disabled
    /// feature, unmet precondition).
    pub const FORBIDDEN: Self = Self {
        code: 403,
        reason: "Forbidden",
    };
    /// `404 Not Found` — no such object, ref, or project.
    pub const NOT_FOUND: Self = Self {
        code: 404,
        reason: "Not Found",
    };
    /// `500 Internal Server Error` — an internal backend failure.
    pub const INTERNAL: Self = Self {
        code: 500,
        reason: "Internal Server Error",
    };
    /// `503 Service Unavailable` — a request-level gate (e.g. server load),
    /// never produced from a port failure.
    pub const UNAVAILABLE: Self = Self {
        code: 503,
        reason: "Service Unavailable",
    };

    /// gitweb's page title for this status, e.g. `404 Not Found`.
    #[must_use]
    pub fn title(self) -> String {
        format!("{} {}", self.code, self.reason)
    }
}

/// A classified, rendered error: the status to serve and the page body.
#[derive(Debug, Clone)]
pub struct ErrorResponse {
    /// The HTTP status to serve the page under.
    pub status: HttpStatus,
    /// The rendered error page body (a `<main>`).
    pub body: Markup,
}

/// Classifies a domain failure into the HTTP status gitweb's `die_error` would
/// use: a missing object/ref/project is 404, malformed input 400, a policy
/// denial 403, and an internal backend failure 500. (503 is a request-level
/// gate, not a port failure, so it never appears here.)
#[must_use]
pub fn status_for(error: &DomainError) -> HttpStatus {
    match error {
        DomainError::NotFound(_) => HttpStatus::NOT_FOUND,
        DomainError::Invalid(_) => HttpStatus::BAD_REQUEST,
        DomainError::Forbidden(_) => HttpStatus::FORBIDDEN,
        DomainError::Backend(_) => HttpStatus::INTERNAL,
    }
}

/// Renders the body of an error page: the `code - message` line and an optional
/// detail block, as a semantic `<main>`. The message is auto-escaped; `detail`
/// is already-safe markup (gitweb's raw `$extra`). The web boundary wraps this
/// in the document chrome with [`HttpStatus::title`] as the title and serves it
/// with [`HttpStatus::code`].
#[must_use]
pub fn error_page(status: HttpStatus, message: &str, detail: Option<Markup>) -> Markup {
    html! {
        main class="error" {
            p class="error-status" { (status.code) " - " (message) }
            @if let Some(detail) = detail {
                hr;
                div class="error-detail" { (detail) }
            }
        }
    }
}

/// The common path: classify a port failure with [`status_for`] and render it
/// with [`error_page`], using the failure's own description as the message.
/// Callers that have a better human message use the two functions directly.
#[must_use]
pub fn error_response(error: &DomainError) -> ErrorResponse {
    let status: HttpStatus = status_for(error);
    let message: String = error.to_string();
    ErrorResponse {
        status,
        body: error_page(status, &message, None),
    }
}
