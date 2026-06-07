//! Static assets owned by the render layer.
//!
//! The fresh stylesheet that replaces gitweb.css and the favicon are baked into
//! the binary at compile time. The web boundary serves these bytes; it does not
//! need to know where on disk they live, and they can never go missing at
//! runtime.

/// The modernized stylesheet (replaces gitweb's `static/gitweb.css`).
pub const STYLESHEET: &str = include_str!("../static/style.css");

/// The site favicon, a scalable SVG (replaces gitweb's `git-favicon.png`).
pub const FAVICON_SVG: &str = include_str!("../static/favicon.svg");
