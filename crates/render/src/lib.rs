//! Modernized, framework-free HTML rendering for gitweb-rs.
//!
//! This is a Boundary (adapter): it turns framework-free domain view-models
//! into semantic HTML5. It must never reach for a git library or a web
//! framework — it depends only on the domain crate, so all dependencies point
//! inward.
//!
//! The first and most pervasive concern here is *escaping*: no git-derived
//! text is ever emitted into HTML or a URL raw. The [`escape`] module mirrors
//! gitweb's `esc_html` / `esc_path` / `esc_url` / `esc_param` family so that
//! metacharacters cannot break out of their context and control characters are
//! made visible instead of corrupting the page.

pub mod age;
pub mod assets;
pub mod blob;
pub mod chrome;
pub mod error;
pub mod escape;
pub mod heads;
pub mod history;
pub mod log;
pub mod markup;
pub mod project_list;
pub mod remotes;
pub mod shortlog;
pub mod summary;
pub mod tag;
pub mod tags;
pub mod timestamp;
pub mod tree;
