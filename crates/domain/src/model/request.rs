//! The parsed, validated request: gitweb's `%input_params` as a domain entity.
//!
//! gitweb reads short CGI names into long-named parameters
//! (`evaluate_query_params`) and then validates each one before any repository
//! access (`evaluate_and_validate_params`). [`Request::from_query`] is that
//! rule as a pure function: it maps the short names, validates every field, and
//! either yields a `Request` or the [`DomainError`] whose status matches
//! gitweb's `die_error` for that failure (bad action/hash/file/page/searchtype
//! → 400, bad project → 404, too-short search → 403).
//!
//! Project *existence* and export rules are the store's job; here we reject
//! only what is unsafe on its face (path traversal) or malformed.

use crate::error::DomainError;
use crate::model::action::Action;
use crate::model::safety::{SafePath, SafeRef};

/// A fully validated request, ready to dispatch. Every field is already in
/// gitweb's long-named, validated form; the typed fields ([`SafeRef`],
/// [`SafePath`], [`Action`]) carry their own construction invariants.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Request {
    /// `p` — the project path under the root (pathname-safe; existence is the
    /// store's concern).
    pub project: Option<String>,
    /// `a` — the requested action (gitweb's `%actions` key).
    pub action: Option<Action>,
    /// `h` — the primary object id or ref.
    pub hash: Option<SafeRef>,
    /// `hp` — the parent object id or ref for a diff.
    pub hash_parent: Option<SafeRef>,
    /// `hb` — the base ref a file path is resolved against.
    pub hash_base: Option<SafeRef>,
    /// `hpb` — the parent base ref for a diff.
    pub hash_parent_base: Option<SafeRef>,
    /// `f` — the file path within the tree.
    pub file_name: Option<SafePath>,
    /// `fp` — the parent-side file path for a rename diff.
    pub file_parent: Option<SafePath>,
    /// `pg` — the zero-based page of a paginated list.
    pub page: Option<u32>,
    /// `o` — a list's sort order.
    pub order: Option<String>,
    /// `s` — the search term (at least two characters).
    pub search_text: Option<String>,
    /// `st` — the search kind (lowercase letters only).
    pub search_type: Option<String>,
    /// `sr` — whether the search term is a regular expression.
    pub search_use_regexp: bool,
    /// `sf` — the snapshot archive format.
    pub snapshot_format: Option<String>,
    /// `pf` — the project-list filter (pathname-safe).
    pub project_filter: Option<String>,
    /// `opt` — extra git options, validated against the per-action allow-list.
    pub extra_options: Vec<String>,
}

impl Request {
    /// Validates a request given its raw, already-decoded query parameters,
    /// keyed by gitweb's short CGI names. Returns the [`DomainError`] whose
    /// status mirrors gitweb's `die_error` on the first invalid field.
    pub fn from_query(params: &[(String, String)]) -> Result<Self, DomainError> {
        let action: Option<Action> = parse_action(first(params, "a"))?;
        Ok(Self {
            project: parse_project(first(params, "p"))?,
            action,
            hash: parse_ref(first(params, "h"), "hash parameter")?,
            hash_parent: parse_ref(first(params, "hp"), "hash parent parameter")?,
            hash_base: parse_ref(first(params, "hb"), "hash base parameter")?,
            hash_parent_base: parse_ref(first(params, "hpb"), "hash parent base parameter")?,
            file_name: parse_path(first(params, "f"), "file parameter")?,
            file_parent: parse_path(first(params, "fp"), "file parent parameter")?,
            page: parse_page(first(params, "pg"))?,
            order: first(params, "o").map(str::to_owned),
            search_text: parse_search_text(first(params, "s"))?,
            search_type: parse_search_type(first(params, "st"))?,
            search_use_regexp: is_truthy(first(params, "sr")),
            snapshot_format: first(params, "sf").map(str::to_owned),
            project_filter: parse_project_filter(first(params, "pf"))?,
            extra_options: parse_extra_options(params, action)?,
        })
    }
}

/// The first value supplied for a short CGI name, mirroring CGI's `param()`
/// (which returns the first of any repeated values).
fn first<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(name, _): &&(String, String)| name == key)
        .map(|(_, value): &(String, String)| value.as_str())
}

/// Resolves the action, rejecting a present-but-unknown name as gitweb's
/// `is_valid_action` does (`die_error(400, ...)`). An absent action is `None`.
fn parse_action(value: Option<&str>) -> Result<Option<Action>, DomainError> {
    match value {
        None => Ok(None),
        Some(name) => Action::parse(name)
            .map(Some)
            .ok_or_else(|| DomainError::Invalid("action parameter".to_owned())),
    }
}

/// A project path must be safe on its face (no traversal); existence and export
/// rules are the store's job. gitweb dies 404 on an unsafe project.
fn parse_project(value: Option<&str>) -> Result<Option<String>, DomainError> {
    match value {
        None => Ok(None),
        Some(path) => SafePath::parse(path)
            .map(|safe: SafePath| Some(safe.as_str().to_owned()))
            .ok_or_else(|| DomainError::NotFound("No such project".to_owned())),
    }
}

/// The project-list filter is validated as a safe pathname; gitweb dies 404
/// on a bad filter.
fn parse_project_filter(value: Option<&str>) -> Result<Option<String>, DomainError> {
    match value {
        None => Ok(None),
        Some(path) => SafePath::parse(path)
            .map(|safe: SafePath| Some(safe.as_str().to_owned()))
            .ok_or_else(|| DomainError::NotFound("Invalid project_filter parameter".to_owned())),
    }
}

/// A file path within a tree; gitweb dies 400 on a path that escapes the tree.
fn parse_path(value: Option<&str>, what: &str) -> Result<Option<SafePath>, DomainError> {
    match value {
        None => Ok(None),
        Some(path) => SafePath::parse(path)
            .map(Some)
            .ok_or_else(|| DomainError::Invalid(format!("Invalid {what}"))),
    }
}

/// An object id or ref name; gitweb dies 400 on a malformed one.
fn parse_ref(value: Option<&str>, what: &str) -> Result<Option<SafeRef>, DomainError> {
    match value {
        None => Ok(None),
        Some(text) => SafeRef::parse(text)
            .map(Some)
            .ok_or_else(|| DomainError::Invalid(format!("Invalid {what}"))),
    }
}

/// The page number must be digits only (gitweb's `m/[^0-9]/` check). An empty
/// value is gitweb's falsy "no page"; any non-digit (or an overflow) is 400.
fn parse_page(value: Option<&str>) -> Result<Option<u32>, DomainError> {
    match value {
        None => Ok(None),
        Some("") => Ok(None),
        Some(text) if text.bytes().all(|byte: u8| byte.is_ascii_digit()) => text
            .parse::<u32>()
            .map(Some)
            .map_err(|_| DomainError::Invalid("page parameter".to_owned())),
        Some(_) => Err(DomainError::Invalid("page parameter".to_owned())),
    }
}

/// The search kind must be lowercase letters only (gitweb's `m/[^a-z]/`). An
/// empty value is treated as no kind.
fn parse_search_type(value: Option<&str>) -> Result<Option<String>, DomainError> {
    match value {
        None => Ok(None),
        Some("") => Ok(None),
        Some(text) if text.bytes().all(|byte: u8| byte.is_ascii_lowercase()) => {
            Ok(Some(text.to_owned()))
        }
        Some(_) => Err(DomainError::Invalid("searchtype parameter".to_owned())),
    }
}

/// The search term needs at least two characters; gitweb dies 403 otherwise.
/// (Regexp-syntax validation lands with the search capability, where the
/// pattern is actually compiled.)
fn parse_search_text(value: Option<&str>) -> Result<Option<String>, DomainError> {
    match value {
        None => Ok(None),
        Some(text) if text.chars().count() < 2 => Err(DomainError::Forbidden(
            "At least two characters are required for search parameter".to_owned(),
        )),
        Some(text) => Ok(Some(text.to_owned())),
    }
}

/// CGI truthiness: a present, non-empty value other than `"0"` is true.
fn is_truthy(value: Option<&str>) -> bool {
    matches!(value, Some(text) if !text.is_empty() && text != "0")
}

/// The actions an extra git option may be combined with, or `None` if the
/// option itself is not allowed (gitweb's `%allowed_options`).
fn allowed_actions_for_option(option: &str) -> Option<&'static [&'static str]> {
    match option {
        "--no-merges" => Some(&["rss", "atom", "log", "shortlog", "history"]),
        _ => None,
    }
}

/// Validates every `opt` value against the per-action allow-list: an unknown
/// option is 400, and an option not allowed for the current action is 400.
fn parse_extra_options(
    params: &[(String, String)],
    action: Option<Action>,
) -> Result<Vec<String>, DomainError> {
    let current: &str = action.map_or("", Action::as_str);
    let mut options: Vec<String> = Vec::new();
    for (key, value) in params {
        if key != "opt" {
            continue;
        }
        let allowed: &[&str] = allowed_actions_for_option(value)
            .ok_or_else(|| DomainError::Invalid("option parameter".to_owned()))?;
        if !allowed.contains(&current) {
            return Err(DomainError::Invalid(
                "option parameter for this action".to_owned(),
            ));
        }
        options.push(value.clone());
    }
    Ok(options)
}
