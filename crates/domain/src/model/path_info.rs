//! PATH_INFO decomposition: gitweb's `evaluate_path_info`, minus the project
//! walk.
//!
//! gitweb accepts requests written as a path — `/project/action/ref:path` or
//! `/project/oldref..newref` — instead of query parameters.
//! [`PathInfo::parse`] is the pure half of `evaluate_path_info`: it takes the
//! remainder *after* the project prefix and decomposes it into the long-named
//! request fields, applying gitweb's action-defaulting and snapshot-suffix
//! rules.
//!
//! Two impure concerns stay at the web boundary: resolving which path prefix is
//! the project (it walks the filesystem via `check_head_link`), and the rule
//! that PATH_INFO is ignored once the query string has pinned an action — the
//! caller decides whether to invoke this at all. The fields here are raw and
//! unvalidated; they pass through the same validation as query parameters.

use crate::model::action::Action;

/// The request fields a PATH_INFO remainder decomposes into. Ref and path
/// fields are raw strings, validated downstream exactly like query parameters.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct PathInfo {
    /// The peeled or defaulted action.
    pub action: Option<Action>,
    /// The primary ref/hash (`h`).
    pub hash: Option<String>,
    /// The base ref a file path resolves against (`hb`).
    pub hash_base: Option<String>,
    /// The parent ref for a diff (`hp`).
    pub hash_parent: Option<String>,
    /// The parent base ref for a diff (`hpb`).
    pub hash_parent_base: Option<String>,
    /// The current-side file path (`f`).
    pub file_name: Option<String>,
    /// The parent-side file path (`fp`).
    pub file_parent: Option<String>,
    /// The snapshot archive format stripped from the ref's extension (`sf`).
    pub snapshot_format: Option<String>,
}

/// The actions that take a ref as their base rather than as their hash, and so
/// can stand without a file path — gitweb's `@wants_base`.
fn wants_base(action: Action) -> bool {
    matches!(action, Action::Tree | Action::History)
}

/// The snapshot formats and the extensions that select them, longest first so
/// `.tar.gz` wins over `.gz` — gitweb's `%known_snapshot_formats`.
const SNAPSHOT_SUFFIXES: &[(&str, &[&str])] = &[
    ("tgz", &[".tar.gz", ".tgz"]),
    ("tbz2", &[".tar.bz2", ".tbz2"]),
    ("txz", &[".tar.xz", ".txz"]),
    ("zip", &[".zip"]),
];

impl PathInfo {
    /// Decomposes the PATH_INFO remainder that follows the project prefix.
    #[must_use]
    pub fn parse(remainder: &str) -> Self {
        let mut info: Self = Self::default();

        // gitweb strips the leading slashes left by removing the project prefix.
        let path: &str = remainder.trim_start_matches('/');
        if path.is_empty() {
            return info;
        }

        // Peel a leading action segment if the first component names one;
        // otherwise it is the start of the ref, not an action.
        let rest: &str = peel_action(&mut info, path);

        // Split the optional `oldref..newref` range off the current part at the
        // first `..` that has a non-empty ref before it.
        let (parent_raw, current_raw): (Option<&str>, &str) = split_parent(rest);
        let parent: Option<(&str, Option<&str>)> = parent_raw.map(split_parent_ref);

        // The current part is `ref:path`, where the ref stops at the first colon.
        let (refname, pathname): (Option<&str>, Option<&str>) = split_current(current_raw);

        apply_current(&mut info, parent.is_some(), refname, pathname);
        if let Some((parentref, parentpath)) = parent {
            apply_parent(&mut info, parentref, parentpath);
        }
        apply_snapshot_suffix(&mut info, refname);

        info
    }
}

/// Sets the action and returns the remaining path when the first component is a
/// known action (gitweb's `exists $actions{$action}`); otherwise leaves the
/// action unset and returns the path unchanged.
fn peel_action<'a>(info: &mut PathInfo, path: &'a str) -> &'a str {
    let segment: &str = path.split('/').next().unwrap_or("");
    match Action::parse(segment) {
        Some(action) => {
            info.action = Some(action);
            path[segment.len()..].trim_start_matches('/')
        }
        None => path,
    }
}

/// Splits the parent range off the current part at the first `..` with a
/// non-empty ref before it (gitweb's non-greedy `(.+?)..`).
fn split_parent(rest: &str) -> (Option<&str>, &str) {
    match rest
        .match_indices("..")
        .find(|(index, _): &(usize, &str)| *index >= 1)
    {
        Some((index, _)) => (Some(&rest[..index]), &rest[index + 2..]),
        None => (None, rest),
    }
}

/// Splits a `ref:path` current part: the ref runs up to the first colon (gitweb's
/// `[^:]+?`), and the rest is the path (`(.+)`). Either side may be absent.
fn split_current(part: &str) -> (Option<&str>, Option<&str>) {
    match part.split_once(':') {
        Some((reference, path)) => (non_empty(reference), Some(path)),
        None => (non_empty(part), None),
    }
}

/// Splits a `parentref:parentpath` part. The parent ref may itself contain a
/// colon (gitweb's `.+?`), so the split is at the first colon with a non-empty
/// ref before it; the part is never empty here.
fn split_parent_ref(part: &str) -> (&str, Option<&str>) {
    match part
        .char_indices()
        .skip(1)
        .find(|(_, character): &(usize, char)| *character == ':')
    {
        Some((index, _)) => (&part[..index], Some(&part[index + 1..])),
        None => (part, None),
    }
}

/// Applies the current part: the action default (tree for a directory, a blob
/// view for a file), the base ref, and the file path.
fn apply_current(
    info: &mut PathInfo,
    has_parent: bool,
    refname: Option<&str>,
    pathname: Option<&str>,
) {
    if let Some(raw_path) = pathname {
        let path: &str = raw_path.trim_start_matches('/');
        let file: String = if path.is_empty() || path.ends_with('/') {
            // "ref:dir/" or "ref:" — a directory listing.
            info.action.get_or_insert(Action::Tree);
            path.trim_end_matches('/').to_owned()
        } else {
            // "ref:file" — the raw blob, or a blob diff when there is a parent.
            let default: Action = if has_parent {
                Action::BlobdiffPlain
            } else {
                Action::BlobPlain
            };
            info.action.get_or_insert(default);
            path.to_owned()
        };
        if let Some(reference) = refname {
            info.hash_base.get_or_insert_with(|| reference.to_owned());
        }
        info.file_name.get_or_insert(file);
    } else if let Some(reference) = refname {
        // "ref" alone: a range defaults to shortlog; otherwise the ref is a hash
        // (or a base ref for the @wants_base actions).
        if has_parent {
            info.action.get_or_insert(Action::Shortlog);
        }
        if matches!(info.action, Some(action) if wants_base(action)) {
            info.hash_base.get_or_insert_with(|| reference.to_owned());
        } else {
            info.hash.get_or_insert_with(|| reference.to_owned());
        }
    }
}

/// Applies the parent range: the parent file path (defaulting to the current
/// file) and the parent ref (as a base when a path is present or the action
/// wants a base, otherwise as a plain hash).
fn apply_parent(info: &mut PathInfo, parentref: &str, parentpath: Option<&str>) {
    match parentpath {
        Some(path) if !path.is_empty() => {
            let trimmed: String = path
                .trim_start_matches('/')
                .trim_end_matches('/')
                .to_owned();
            info.file_parent.get_or_insert(trimmed);
        }
        // A missing parent path reuses the current file path.
        _ => {
            if let Some(file) = info.file_name.clone() {
                info.file_parent.get_or_insert(file);
            }
        }
    }
    if info.file_parent.is_some() || matches!(info.action, Some(action) if wants_base(action)) {
        info.hash_parent_base
            .get_or_insert_with(|| parentref.to_owned());
    } else {
        info.hash_parent.get_or_insert_with(|| parentref.to_owned());
    }
}

/// For a snapshot whose hash came straight from the ref, strips a known archive
/// extension off the ref to recover the real hash and the requested format.
fn apply_snapshot_suffix(info: &mut PathInfo, refname: Option<&str>) {
    if info.action != Some(Action::Snapshot) || info.snapshot_format.is_some() {
        return;
    }
    let refname: &str = match refname {
        Some(reference) if reference.contains('.') => reference,
        _ => return,
    };
    if info.hash.as_deref() != Some(refname) {
        return;
    }
    for (format, suffixes) in SNAPSHOT_SUFFIXES {
        for suffix in *suffixes {
            if let Some(stripped) = refname.strip_suffix(suffix) {
                info.snapshot_format = Some((*format).to_owned());
                info.hash = Some(stripped.to_owned());
                return;
            }
        }
    }
}

/// `Some` for a non-empty string, `None` for the empty string.
fn non_empty(value: &str) -> Option<&str> {
    if value.is_empty() { None } else { Some(value) }
}
