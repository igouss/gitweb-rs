//! The failure modes of loading a gitweb-rs config file.

use std::fmt;
use std::io;
use std::path::PathBuf;

/// A failure while reading or parsing a global config file.
#[derive(Debug)]
pub enum ConfigError {
    /// The file could not be read from disk.
    Read {
        /// The file that could not be read.
        path: PathBuf,
        /// The underlying I/O failure.
        source: io::Error,
    },
    /// The file's contents are not valid TOML, or a value has the wrong type.
    Parse {
        /// The file that failed to parse.
        path: PathBuf,
        /// The underlying TOML deserialization failure.
        source: toml::de::Error,
    },
    /// A `[features.<name>]` table named a feature gitweb does not define.
    UnknownFeature {
        /// The file naming the unknown feature.
        path: PathBuf,
        /// The unrecognized feature key.
        key: String,
    },
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Read { path, .. } => {
                write!(f, "cannot read config file {}", path.display())
            }
            Self::Parse { path, source } => {
                write!(f, "cannot parse config file {}: {source}", path.display())
            }
            Self::UnknownFeature { path, key } => {
                write!(
                    f,
                    "unknown feature \"{key}\" in config file {}",
                    path.display()
                )
            }
        }
    }
}

impl std::error::Error for ConfigError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Read { source, .. } => Some(source),
            Self::Parse { source, .. } => Some(source),
            Self::UnknownFeature { .. } => None,
        }
    }
}
