//! Inbound adapter: gitweb-rs's declarative config files into the domain's
//! resolved [`Settings`](gitweb_domain::model::settings::Settings).
//!
//! gitweb's global config files are executable Perl (`do $file`). We will not
//! run Perl, so gitweb-rs uses a declarative **TOML** format instead. This crate
//! reads the files [`config_chain`] selected (weakest first), parses each into a
//! domain [`SettingsLayer`], and resolves them over the built-in defaults. The
//! format lives ONLY here; the domain never learns it is TOML.
//!
//! [`config_chain`]: gitweb_domain::model::config_chain
//! [`SettingsLayer`]: gitweb_domain::model::settings::SettingsLayer
//!
//! # gitweb variable to TOML key
//!
//! Each file is a flat table of settings plus a `[features.<name>]` sub-table.
//! Any setting may be omitted; an omitted setting keeps what weaker sources (or
//! the built-in defaults) resolved. Unknown keys are rejected, so a typo fails
//! loudly rather than silently doing nothing.
//!
//! | gitweb (`gitweb.perl`)            | TOML key                          |
//! |-----------------------------------|-----------------------------------|
//! | `$projectroot`                    | `projectroot`                     |
//! | `$site_name`                      | `site_name`                       |
//! | `$default_projects_order`         | `default_projects_order`          |
//! | `$projects_list_description_width`| `projects_list_description_width` |
//! | `$omit_age_column`                | `omit_age_column`                 |
//! | `$omit_owner`                     | `omit_owner`                      |
//! | `$fallback_encoding`              | `fallback_encoding`               |
//! | `$prevent_xss`                    | `prevent_xss`                     |
//! | `@stylesheets`                    | `stylesheets` (array)             |
//! | `@git_base_url_list`              | `git_base_url_list` (array)       |
//! | `$feature{X}{default}` (array)    | `[features.X]` `default` (array)  |
//! | `$feature{X}{override}`           | `[features.X]` `override` (bool)  |
//!
//! A feature's `default` is an array of option strings, matching gitweb's array
//! model: a bool-ish feature is `default = ["1"]` or `["0"]`; a multi-option one
//! (e.g. snapshot formats) is `default = ["tgz", "zip"]`.
//!
//! ```toml
//! projectroot = "/srv/git"
//! site_name = "Example Git"
//! git_base_url_list = ["git://example.org/", "https://example.org/git/"]
//!
//! [features.blame]
//! default = ["1"]
//! override = true
//! ```

pub mod error;
pub mod loader;

pub use error::ConfigError;
pub use loader::{load, parse_layer};
