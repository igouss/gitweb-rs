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

/// The timezone client module (gitweb's `javascript-timezone` feature): rewrites
/// `<span class="datetime">` dates to the viewer's chosen timezone, persisting
/// the choice in the `gitweb_tz` cookie (ports `adjust-timezone.js`).
pub const TIMEZONE_JS: &str = include_str!("../static/gitweb-timezone.js");

/// The actions client module (gitweb's `javascript-actions` feature): rewrites
/// in-page links to carry `js=1` so the server serves their JavaScript-driven
/// variant, e.g. `blame_incremental` for `blame` (ports `javascript-detection.js`).
pub const ACTIONS_JS: &str = include_str!("../static/gitweb-actions.js");

/// The incremental-blame client module (gitweb's `blame_incremental.js`):
/// progressively fills a blame view from the `blame_data` endpoint. Served now so
/// the downstream `blame_data` bead can wire it in; it does not require the
/// endpoint to exist.
pub const BLAME_INCREMENTAL_JS: &str = include_str!("../static/blame-incremental.js");

/// The diff-viewer boot module: fetches a commit's clean unified diff and
/// renders it client-side with the vendored `@pierre/diffs` viewer (replaces
/// gitweb's server-side `format_diff_line` colouriser). It imports the viewer
/// from the git-ignored, built-on-demand bundle under `/static/vendor/pierre/`;
/// see the `diff_host` render module for the host page that boots it.
pub const DIFF_VIEWER_JS: &str = include_str!("../static/diff-viewer.js");
