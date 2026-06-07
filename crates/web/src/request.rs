//! Inbound request resolution: raw HTTP request -> validated [`Request`].
//!
//! gitweb turns a request into its `%input_params` in three steps:
//! `evaluate_query_params` (read the query string under short CGI names),
//! `evaluate_path_info` (decompose a path-style URL, resolving its project
//! prefix against the filesystem), and `evaluate_and_validate_params` (validate
//! every field). The *pure* halves are already domain rules:
//! [`Request::from_query`] is the query read + validation, and [`PathInfo::parse`]
//! is the path decomposition. What lives here is the *impure* glue gitweb does
//! between them:
//!
//! - resolve which leading PATH_INFO component(s) name an existing project by
//!   walking the project root (gitweb's `check_head_link`), since a project name
//!   can itself contain slashes — done through the [`ProjectStore`] port;
//! - run [`PathInfo::parse`] on the remainder, but only when the query string
//!   did not already pin an action (gitweb's `return if $input_params{'action'}`);
//! - merge the PATH_INFO-derived fields with the query string under gitweb's
//!   precedence (the query wins, gitweb's `||=`), then validate the whole into a
//!   [`Request`];
//! - carry the three view-preference params gitweb reads only from the query and
//!   never validates: `ds` (diff style), `by_tag` (project-list content tag),
//!   and `js` (the JavaScript-actions marker).

use std::collections::BTreeMap;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::path_info::PathInfo;
use gitweb_domain::model::request::Request;
use gitweb_domain::port::project_store::ProjectStore;

/// The view-preference parameters gitweb reads only from the query string (never
/// from PATH_INFO) and never validates — they steer presentation, not domain
/// logic, so they sit at the boundary rather than in the domain [`Request`].
///
/// Each is gitweb's raw, untouched value; consumers apply their own defaults
/// (e.g. `diff_style || 'inline'`) at the point of use, exactly as gitweb does.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ViewPrefs {
    /// `ds` — the diff display style (gitweb defaults this to `inline`).
    pub diff_style: Option<String>,
    /// `by_tag` — the content-tag filter for the project list.
    pub content_tag: Option<String>,
    /// `js` — the marker that a request came from JavaScript-rewritten links.
    pub javascript: Option<String>,
}

/// The fully resolved inbound request: the validated domain [`Request`] plus the
/// boundary-only [`ViewPrefs`].
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct ResolvedRequest {
    /// The validated domain request, ready to dispatch.
    pub request: Request,
    /// The view preferences read from the query string.
    pub view: ViewPrefs,
}

/// Resolves a raw inbound request into a validated [`ResolvedRequest`].
///
/// `query` is the already-decoded query string as gitweb's short CGI names;
/// `path_info` is the raw PATH_INFO. `store` is consulted only to decide which
/// PATH_INFO prefix names an existing project. The returned [`DomainError`]
/// carries the same status gitweb's `die_error` would on the first invalid
/// field.
pub fn resolve(
    store: &dyn ProjectStore,
    query: &[(String, String)],
    path_info: &str,
) -> Result<ResolvedRequest, DomainError> {
    // The view preferences come straight off the query string, untouched.
    let view: ViewPrefs = ViewPrefs {
        diff_style: first(query, "ds").map(str::to_owned),
        content_tag: first(query, "by_tag").map(str::to_owned),
        javascript: first(query, "js").map(str::to_owned),
    };

    // Seed the single-valued long fields from the query under gitweb's short CGI
    // names (gitweb's `evaluate_query_params`).
    let mut fields: BTreeMap<&'static str, String> = BTreeMap::new();
    for &key in QUERY_SINGLE_KEYS {
        if let Some(value) = first(query, key) {
            fields.insert(key, value.to_owned());
        }
    }

    // gitweb's `evaluate_path_info`. It runs only when the query did not already
    // pin the project (`return if defined $input_params{'project'}`): an explicit
    // `p=` — even empty — suppresses PATH_INFO entirely.
    if !query_defined(query, "p") {
        // `s,^/+,,`: PATH_INFO with its leading slashes removed.
        let trimmed: &str = path_info.trim_start_matches('/');
        if !trimmed.is_empty()
            && let Some(project) = resolve_project_prefix(store, trimmed)
        {
            fields.insert("p", project.clone());
            // `return if $input_params{'action'}`: a query action (truthy)
            // suppresses the rest of the PATH_INFO; the project still stands.
            if !is_truthy(first(query, "a")) {
                let info: PathInfo = PathInfo::parse(strip_project(trimmed, &project));
                for (key, value) in path_info_fields(&info) {
                    fill(&mut fields, key, value);
                }
            }
        }
    }

    // Rebuild the merged short-name params: every single-valued field, plus the
    // query's multi-valued `opt` entries (PATH_INFO never produces `opt`).
    let mut merged: Vec<(String, String)> = fields
        .into_iter()
        .map(|(key, value): (&'static str, String)| (key.to_owned(), value))
        .collect();
    for (key, value) in query {
        if key == "opt" {
            merged.push((key.clone(), value.clone()));
        }
    }

    // The frozen domain rule validates the whole, failing with gitweb's status.
    let request: Request = Request::from_query(&merged)?;
    Ok(ResolvedRequest { request, view })
}

/// The single-valued short CGI names the boundary merges (gitweb's
/// `%cgi_param_mapping`, minus the multi-valued `opt` and the view-preference
/// `ds` / `by_tag` / `js`, which are handled separately).
const QUERY_SINGLE_KEYS: &[&str] = &[
    "p", "a", "f", "fp", "h", "hp", "hb", "hpb", "pg", "o", "s", "st", "sf", "sr", "pf",
];

/// The first value supplied for a short CGI name, mirroring CGI's `param()`.
fn first<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(name, _): &&(String, String)| name == key)
        .map(|(_, value): &(String, String)| value.as_str())
}

/// Whether a short CGI name appears in the query at all, value or not — gitweb's
/// `defined`, the test that decides project-from-PATH_INFO suppression.
fn query_defined(params: &[(String, String)], key: &str) -> bool {
    params
        .iter()
        .any(|(name, _): &(String, String)| name == key)
}

/// CGI/Perl truthiness: a present, non-empty value other than `"0"` is true.
fn is_truthy(value: Option<&str>) -> bool {
    matches!(value, Some(text) if !text.is_empty() && text != "0")
}

/// gitweb's `||=`: set `key` to `value` only when the current value is not
/// truthy, so a meaningful query value always wins over the PATH_INFO one.
fn fill(fields: &mut BTreeMap<&'static str, String>, key: &'static str, value: String) {
    let current_truthy: bool = fields
        .get(key)
        .is_some_and(|current: &String| !current.is_empty() && current != "0");
    if !current_truthy {
        fields.insert(key, value);
    }
}

/// The PATH_INFO-derived fields paired with the short CGI name each merges into.
fn path_info_fields(info: &PathInfo) -> Vec<(&'static str, String)> {
    let mut fields: Vec<(&'static str, String)> = Vec::new();
    if let Some(action) = info.action {
        fields.push(("a", action.as_str().to_owned()));
    }
    push_some(&mut fields, "h", info.hash.as_deref());
    push_some(&mut fields, "hb", info.hash_base.as_deref());
    push_some(&mut fields, "hp", info.hash_parent.as_deref());
    push_some(&mut fields, "hpb", info.hash_parent_base.as_deref());
    push_some(&mut fields, "f", info.file_name.as_deref());
    push_some(&mut fields, "fp", info.file_parent.as_deref());
    push_some(&mut fields, "sf", info.snapshot_format.as_deref());
    fields
}

/// Appends `(key, value)` when `value` is present.
fn push_some(fields: &mut Vec<(&'static str, String)>, key: &'static str, value: Option<&str>) {
    if let Some(value) = value {
        fields.push((key, value.to_owned()));
    }
}

/// gitweb's project-prefix walk: the longest leading prefix of `path` that names
/// a repository, dropping one trailing path component at a time. Existence is
/// gitweb's `check_head_link`, which the `ProjectStore` answers by opening — an
/// unsafe or non-repository prefix simply fails and the walk continues.
fn resolve_project_prefix(store: &dyn ProjectStore, path: &str) -> Option<String> {
    let mut candidate: &str = path.trim_end_matches('/');
    while !candidate.is_empty() {
        if store.open(candidate).is_ok() {
            return Some(candidate.to_owned());
        }
        candidate = drop_last_component(candidate);
    }
    None
}

/// gitweb's `s,/*[^/]*$,,`: drop the last `/component`, leaving no trailing slash.
fn drop_last_component(path: &str) -> &str {
    match path.rfind('/') {
        Some(slash) => path[..slash].trim_end_matches('/'),
        None => "",
    }
}

/// gitweb's `s,^\Q$project\E/*,,`: the PATH_INFO remainder after the project
/// prefix, with the slashes that followed it removed.
fn strip_project<'a>(path: &'a str, project: &str) -> &'a str {
    path.strip_prefix(project)
        .unwrap_or("")
        .trim_start_matches('/')
}
