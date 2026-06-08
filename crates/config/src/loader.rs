//! Reading config files into the domain's settings layers.
//!
//! The DTOs below mirror the on-disk TOML shape; [`to_layer`] maps them onto the
//! domain's [`SettingsLayer`] (validating feature names), and [`load`] reads an
//! ordered set of files and resolves them. The format stays sealed in this file.

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::Deserialize;

use gitweb_domain::model::settings::{FeatureLayer, FeatureName, Settings, SettingsLayer};

use crate::error::ConfigError;

/// The on-disk shape of one config file. Every setting is optional: a file
/// contributes only what it names, and the domain's precedence rule fills the
/// rest. Unknown keys are rejected so typos fail loudly.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigDoc {
    projectroot: Option<String>,
    site_name: Option<String>,
    logo: Option<String>,
    favicon: Option<String>,
    default_projects_order: Option<String>,
    projects_list_description_width: Option<usize>,
    omit_age_column: Option<bool>,
    omit_owner: Option<bool>,
    fallback_encoding: Option<String>,
    prevent_xss: Option<bool>,
    stylesheets: Option<Vec<String>>,
    git_base_url_list: Option<Vec<String>>,
    #[serde(default)]
    features: BTreeMap<String, FeatureDoc>,
}

/// The on-disk shape of one `[features.<name>]` table: gitweb's
/// `$feature{name}{default}` array and `$feature{name}{override}` flag, each
/// optional so a file may set either independently of the other.
#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FeatureDoc {
    default: Option<Vec<String>>,
    #[serde(rename = "override")]
    overridable: Option<bool>,
}

/// Maps a parsed document onto a domain layer, rejecting unknown feature names.
fn to_layer(path: &Path, doc: ConfigDoc) -> Result<SettingsLayer, ConfigError> {
    let mut features: BTreeMap<FeatureName, FeatureLayer> = BTreeMap::new();
    for (key, feature) in doc.features {
        let name: FeatureName =
            FeatureName::from_key(&key).ok_or_else(|| ConfigError::UnknownFeature {
                path: path.to_path_buf(),
                key,
            })?;
        features.insert(
            name,
            FeatureLayer {
                default: feature.default,
                overridable: feature.overridable,
            },
        );
    }
    Ok(SettingsLayer {
        projectroot: doc.projectroot,
        site_name: doc.site_name,
        logo: doc.logo,
        favicon: doc.favicon,
        default_projects_order: doc.default_projects_order,
        projects_list_description_width: doc.projects_list_description_width,
        omit_age_column: doc.omit_age_column,
        omit_owner: doc.omit_owner,
        fallback_encoding: doc.fallback_encoding,
        prevent_xss: doc.prevent_xss,
        stylesheets: doc.stylesheets,
        git_base_url_list: doc.git_base_url_list,
        features,
    })
}

/// Parses one config file's text into a partial layer. `path` is carried for
/// error messages only.
///
/// # Errors
/// [`ConfigError::Parse`] for invalid TOML or a mistyped value;
/// [`ConfigError::UnknownFeature`] for a `[features.<name>]` gitweb does not define.
pub fn parse_layer(path: &Path, text: &str) -> Result<SettingsLayer, ConfigError> {
    let doc: ConfigDoc =
        toml::from_str(text).map_err(|source: toml::de::Error| ConfigError::Parse {
            path: path.to_path_buf(),
            source,
        })?;
    to_layer(path, doc)
}

/// Loads the resolved settings from an ordered list of config files — weakest
/// first, as [`config_chain`] selects them. Each file is read and parsed into a
/// layer; the layers resolve over the built-in defaults. An empty list yields
/// the built-in defaults.
///
/// [`config_chain`]: gitweb_domain::model::config_chain
///
/// # Errors
/// [`ConfigError::Read`] if a file cannot be read, or the parse/validation
/// errors of [`parse_layer`].
pub fn load(paths: &[PathBuf]) -> Result<Settings, ConfigError> {
    let mut layers: Vec<SettingsLayer> = Vec::with_capacity(paths.len());
    for path in paths {
        let text: String =
            fs::read_to_string(path).map_err(|source: std::io::Error| ConfigError::Read {
                path: path.clone(),
                source,
            })?;
        layers.push(parse_layer(path, &text)?);
    }
    Ok(Settings::resolve(&layers))
}
