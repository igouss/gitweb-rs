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

/// The diff-viewer boot module: fetches a commit's clean unified diff and
/// renders it client-side with the vendored `@pierre/diffs` viewer (replaces
/// gitweb's server-side `format_diff_line` colouriser). It imports the viewer
/// from the git-ignored, built-on-demand bundle under `/static/vendor/pierre/`;
/// see the `diff_host` render module for the host page that boots it.
pub const DIFF_VIEWER_JS: &str = include_str!("../static/diff-viewer.js");
