//! Snapshot archive formats and the rules `git_snapshot` runs before streaming.
//!
//! gitweb's `%known_snapshot_formats` is the fixed table of archive formats it
//! understands ([`ArchiveFormat`]), each with a media type, a download suffix, a
//! display label, and — for `txz` — a static `disabled` flag (xz support shipped
//! experimental and off). `git_snapshot` then runs two pure rules over that table
//! and the site's configured formats:
//!
//!   - [`enabled_formats`] is gitweb's `filter_snapshot_fmts`: resolve the
//!     configured tokens through the legacy aliases, drop anything unknown or
//!     statically disabled — the effective `@snapshot_fmts`.
//!   - [`select_format`] is `git_snapshot`'s validation cascade: forbid when no
//!     format is enabled, default an unset request to the first enabled format,
//!     then reject a malformed / unknown / disabled / not-configured request with
//!     the same status gitweb's `die_error` would pick.
//!
//! [`snapshot_name`] is gitweb's `snapshot_name`: the `project-version` base name
//! shared by the download file name and the archive's top-level directory. The
//! version string needs an abbreviated id and the ref classification, which are
//! reads the use case performs; the rule takes those as inputs and stays pure.
//!
//! The bytes themselves come from the [`Repository::archive`] port over
//! [`ArchiveOptions`]; this module owns only the format table and the naming and
//! selection rules.
//!
//! [`Repository::archive`]: crate::port::repository::Repository::archive

use crate::error::DomainError;

/// A snapshot archive format — one entry of gitweb's `%known_snapshot_formats`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ArchiveFormat {
    /// gzip-compressed tar (`tgz`).
    TarGz,
    /// bzip2-compressed tar (`tbz2`).
    TarBz2,
    /// xz-compressed tar (`txz`) — statically disabled, as in gitweb.
    TarXz,
    /// zip (`zip`).
    Zip,
}

impl ArchiveFormat {
    /// Every known format, in gitweb's `%known_snapshot_formats` declaration order.
    pub const ALL: [ArchiveFormat; 4] = [
        ArchiveFormat::TarGz,
        ArchiveFormat::TarBz2,
        ArchiveFormat::TarXz,
        ArchiveFormat::Zip,
    ];

    /// The format token: the `sf` query value and the config name (gitweb's hash
    /// key).
    #[must_use]
    pub fn key(self) -> &'static str {
        match self {
            Self::TarGz => "tgz",
            Self::TarBz2 => "tbz2",
            Self::TarXz => "txz",
            Self::Zip => "zip",
        }
    }

    /// The download file name suffix (gitweb's `suffix`).
    #[must_use]
    pub fn suffix(self) -> &'static str {
        match self {
            Self::TarGz => ".tar.gz",
            Self::TarBz2 => ".tar.bz2",
            Self::TarXz => ".tar.xz",
            Self::Zip => ".zip",
        }
    }

    /// The `Content-Type` the archive is served under (gitweb's `type`).
    #[must_use]
    pub fn content_type(self) -> &'static str {
        match self {
            Self::TarGz => "application/x-gzip",
            Self::TarBz2 => "application/x-bzip2",
            Self::TarXz => "application/x-xz",
            Self::Zip => "application/x-zip",
        }
    }

    /// The human label shown in snapshot links (gitweb's `display`).
    #[must_use]
    pub fn display(self) -> &'static str {
        match self {
            Self::TarGz => "tar.gz",
            Self::TarBz2 => "tar.bz2",
            Self::TarXz => "tar.xz",
            Self::Zip => "zip",
        }
    }

    /// Whether gitweb ships this format disabled (`txz` only). It is a static
    /// property of the format, not a per-site toggle: `filter_snapshot_fmts` drops
    /// a disabled format from the configured list, and `git_snapshot` rejects a
    /// direct request for one.
    #[must_use]
    pub fn is_disabled(self) -> bool {
        matches!(self, Self::TarXz)
    }

    /// The known format whose token is `key`, or `None` for an unknown token
    /// (gitweb's `exists $known_snapshot_formats{$format}`). Legacy aliases are
    /// *not* resolved here — gitweb applies those only to the configured list, via
    /// [`enabled_formats`], never to the requested `sf`.
    #[must_use]
    pub fn from_key(key: &str) -> Option<Self> {
        Self::ALL
            .into_iter()
            .find(|format: &Self| format.key() == key)
    }
}

/// Resolves one configured token to a format, applying gitweb's
/// `%known_snapshot_format_aliases`: the long names map to their short form, and
/// the legacy junk names (`x-gzip`, `gz`, `bz2`, `x-zip`, the empty string, …)
/// map to nothing. An unknown token resolves to nothing too.
fn resolve_alias(token: &str) -> Option<ArchiveFormat> {
    match token {
        "gzip" => Some(ArchiveFormat::TarGz),
        "bzip2" => Some(ArchiveFormat::TarBz2),
        "xz" => Some(ArchiveFormat::TarXz),
        // x-gzip, gz, x-bzip2, bz2, x-zip and "" map to undef in gitweb.
        "x-gzip" | "gz" | "x-bzip2" | "bz2" | "x-zip" | "" => None,
        other => ArchiveFormat::from_key(other),
    }
}

/// gitweb's `filter_snapshot_fmts`: the site's `@snapshot_fmts`. Resolve each
/// configured token through the aliases, then keep only the known, not-disabled
/// formats, in configuration order.
#[must_use]
pub fn enabled_formats(configured: &[String]) -> Vec<ArchiveFormat> {
    configured
        .iter()
        .filter_map(|token: &String| resolve_alias(token))
        .filter(|format: &ArchiveFormat| !format.is_disabled())
        .collect()
}

/// gitweb's `git_snapshot` format-validation cascade: choose the archive format
/// for a request from the requested `sf` value and the site's `enabled` formats.
///
/// `requested` is the `sf` parameter (gitweb's `$input_params{snapshot_format}`);
/// an absent or empty value defaults to the first enabled format (gitweb's
/// `$format ||= $snapshot_fmts[0]`). The defaulted format is enabled by
/// construction, so it always passes.
///
/// # Errors
///
/// Mirrors `git_snapshot`'s `die_error` codes:
/// - [`Forbidden`](DomainError::Forbidden) `Snapshots not allowed` — no format is
///   enabled (the feature is off);
/// - [`Invalid`](DomainError::Invalid) `Invalid snapshot format parameter` — the
///   requested token is not `^[a-z0-9]+$`;
/// - [`Invalid`](DomainError::Invalid) `Unknown snapshot format` — the token is
///   well-formed but names no known format;
/// - [`Forbidden`](DomainError::Forbidden) `Snapshot format not allowed` — the
///   format is known but statically disabled;
/// - [`Forbidden`](DomainError::Forbidden) `Unsupported snapshot format` — the
///   format is known and enabled elsewhere, but not in this site's configured set.
pub fn select_format(
    requested: Option<&str>,
    enabled: &[ArchiveFormat],
) -> Result<ArchiveFormat, DomainError> {
    let first: ArchiveFormat = *enabled
        .first()
        .ok_or_else(|| DomainError::Forbidden("Snapshots not allowed".to_owned()))?;

    // `$format ||= $snapshot_fmts[0]`: undef and "" both default.
    let token: &str = match requested.filter(|value: &&str| !value.is_empty()) {
        Some(value) => value,
        None => return Ok(first),
    };

    if !is_format_token(token) {
        return Err(DomainError::Invalid(
            "Invalid snapshot format parameter".to_owned(),
        ));
    }
    let format: ArchiveFormat = ArchiveFormat::from_key(token)
        .ok_or_else(|| DomainError::Invalid("Unknown snapshot format".to_owned()))?;
    if format.is_disabled() {
        return Err(DomainError::Forbidden(
            "Snapshot format not allowed".to_owned(),
        ));
    }
    if !enabled.contains(&format) {
        return Err(DomainError::Forbidden(
            "Unsupported snapshot format".to_owned(),
        ));
    }
    Ok(format)
}

/// gitweb's `$format !~ m/^[a-z0-9]+$/` guard: a non-empty run of ASCII lowercase
/// letters and digits.
fn is_format_token(token: &str) -> bool {
    !token.is_empty()
        && token
            .bytes()
            .all(|byte: u8| byte.is_ascii_lowercase() || byte.is_ascii_digit())
}

/// gitweb's `snapshot_name`: the `project-version` base name shared by the
/// download file name (with the format suffix appended) and the archive's
/// top-level directory.
///
/// `project` is the project path; its basename, with a trailing `.git` stripped,
/// sanitized, is the leading component. The version string is derived from
/// `hash` — the raw `h` parameter — and the already-computed `short_hash`:
/// - an object id (all hex, longer than seven chars) abbreviates to `short_hash`;
///   a short id is kept verbatim;
/// - `refs/tags/<name>` is the bare tag name, with no id appended;
/// - a branch / remote ref strips its `refs/<dir>/` prefix (prepending the dir for
///   refs outside `heads`/`remotes`) and appends `-short_hash`;
/// - anything else keeps its text and appends `-short_hash`.
///
/// `branch_refs` are the directory names under `refs/` that count as branches
/// (gitweb's `get_branch_refs`: `heads`, plus any extra branch refs). Both the
/// name component and the version string are sanitized to gitweb's filename
/// alphabet, with the version's slashes turned to dots first.
#[must_use]
pub fn snapshot_name(project: &str, hash: &str, short_hash: &str, branch_refs: &[&str]) -> String {
    let name: String = sanitize_for_filename(basename(strip_dot_git(project)));
    let version: String = sanitize_version(&version_string(hash, short_hash, branch_refs));
    format!("{name}-{version}")
}

/// Strips a trailing `.git` from a project path (gitweb's
/// `s,([^/])/*\.git$,$1,`): `foo.git` and `foo/.git` both become `foo`.
fn strip_dot_git(project: &str) -> &str {
    let trimmed: &str = match project.strip_suffix(".git") {
        Some(head) => head.trim_end_matches('/'),
        None => return project,
    };
    // gitweb keeps the form `.git` (with nothing but slashes before it) intact:
    // the pattern requires a non-slash before the optional slashes.
    if trimmed.is_empty() { project } else { trimmed }
}

/// The last path component (gitweb's `basename`).
fn basename(path: &str) -> &str {
    path.rsplit('/').next().unwrap_or(path)
}

/// The version string before sanitization (gitweb's `$ver`).
fn version_string(hash: &str, short_hash: &str, branch_refs: &[&str]) -> String {
    if is_hex(hash) {
        // A full id abbreviates; a short id (<= 7 chars) is kept verbatim.
        return if hash.len() > 7 {
            short_hash.to_owned()
        } else {
            hash.to_owned()
        };
    }
    if let Some(tag) = hash.strip_prefix("refs/tags/") {
        return tag.to_owned();
    }
    let base: String = strip_branch_ref(hash, branch_refs).unwrap_or_else(|| hash.to_owned());
    format!("{base}-{short_hash}")
}

/// Strips a `refs/<dir>/<rest>` branch or remote ref to its display version,
/// prepending `<dir>-` for a ref directory outside `heads`/`remotes` — gitweb's
/// `m!^refs/($strip_refs|remotes)/(.*)$!`. Returns `None` when `hash` is not such
/// a ref (the caller keeps the text verbatim).
fn strip_branch_ref(hash: &str, branch_refs: &[&str]) -> Option<String> {
    let rest: &str = hash.strip_prefix("refs/")?;
    let (dir, name): (&str, &str) = rest.split_once('/')?;
    let is_branch_dir: bool = dir == "remotes" || branch_refs.contains(&dir);
    if !is_branch_dir {
        return None;
    }
    let dir: String = sanitize_for_filename(dir);
    if dir.is_empty() || dir == "heads" || dir == "remotes" {
        Some(name.to_owned())
    } else {
        Some(format!("{dir}-{name}"))
    }
}

/// Whether `hash` is all hexadecimal (gitweb's `/^[0-9a-fA-F]+$/`).
fn is_hex(hash: &str) -> bool {
    !hash.is_empty() && hash.bytes().all(|byte: u8| byte.is_ascii_hexdigit())
}

/// Turns the version's slashes to dots, then drops everything outside gitweb's
/// filename alphabet (gitweb's `s!/!.!g` then `s/[^[:alnum:]_.-]//g`).
fn sanitize_version(version: &str) -> String {
    let dotted: String = version.replace('/', ".");
    dotted
        .chars()
        .filter(|character: &char| is_filename_char(*character))
        .collect()
}

/// gitweb's `sanitize_for_filename`: slashes to dashes, then drop everything
/// outside the filename alphabet.
fn sanitize_for_filename(name: &str) -> String {
    let dashed: String = name.replace('/', "-");
    dashed
        .chars()
        .filter(|character: &char| is_filename_char(*character))
        .collect()
}

/// gitweb's `[:alnum:]_.-` filename alphabet.
fn is_filename_char(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
}

/// The non-format inputs the [`Repository::archive`] port needs to produce a
/// gitweb-faithful snapshot: the archive's top-level directory and the entry
/// modification time.
///
/// gitweb passes `git archive --prefix=<name>/`, wrapping every entry in the
/// `project-version` directory, and lets `git archive` stamp entries with the
/// commit time, which is also the snapshot's `Last-Modified`. The adapter takes
/// these as data so the same tree archives reproducibly.
///
/// [`Repository::archive`]: crate::port::repository::Repository::archive
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ArchiveOptions {
    /// The archive's top-level directory (gitweb's `--prefix`, without the
    /// trailing slash) — the snapshot base name.
    pub prefix: String,
    /// The modification time stamped on every entry, in seconds since the Unix
    /// epoch — the commit time, or `0` for a tree-ish with no commit.
    pub modification_time: i64,
}
