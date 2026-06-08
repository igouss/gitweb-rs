//! The clean-diff endpoint: a commit's diff as git's bare unified patch text.
//!
//! This is a *modern* route with no gitweb equivalent. gitweb colourises a
//! commit's diff in the `git_commitdiff` page itself; we modernize that by
//! shipping a host page whose `#diff-root` the vendored `@pierre/diffs` client
//! viewer fills — and the viewer fetches the diff as a clean unified patch from
//! here. It must not be `commitdiff_plain`, whose `From:` / `Subject:` mail
//! header breaks the viewer's `parsePatchFiles`; this serves the bare bytes
//! `git diff-tree -p` emits, starting at `diff --git`.
//!
//! It is therefore mounted as its own router route (a dynamic sibling of the
//! static-asset routes), not a gitweb `%actions` entry — the domain
//! [`Action`](gitweb_domain::model::action::Action) table stays exactly gitweb's
//! `is_valid_action`. The route reuses the [`assemble_commit_diff`] use case over
//! the wired project store, validating every untrusted parameter before touching
//! the repository.

use gitweb_domain::error::DomainError;
use gitweb_domain::model::safety::{SafePath, SafeRef};
use gitweb_domain::port::project_store::ProjectStore;
use gitweb_domain::usecase::commitdiff::assemble_commit_diff;

use crate::response::View;

/// The URL the clean-diff endpoint is served at — the path the diff host page's
/// `data-diff-url` points at.
pub const DIFF_TEXT_PATH: &str = "/diff";

/// Builds the clean-diff endpoint URL the viewer fetches, from the short CGI
/// names the commitdiff host page supplies (`p`, `h`, optionally `hp`),
/// percent-encoded the way the inbound decoder parses them.
#[must_use]
pub fn diff_text_href(params: &[(&str, &str)]) -> String {
    let query: String = form_urlencoded::Serializer::new(String::new())
        .extend_pairs(params.iter().copied())
        .finish();
    format!("{DIFF_TEXT_PATH}?{query}")
}

/// Serves the clean unified diff of the requested commit: validate the project
/// and revisions, open the project through the wired `store`, render the patch.
///
/// `query` is the decoded query string as gitweb's short CGI names. The commit
/// is `h` (defaulting to `HEAD`); an optional `hp` selects the parent to diff
/// against (otherwise the use case picks the base); an optional `f` scopes the
/// diff to a single file — the html `blobdiff` viewer's fetch, where `h` / `hp`
/// are the base / parent-base commits and `f` the file's new-side path.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] for a missing project or an unsafe path /
/// ref parameter (gitweb's `die_error(400, …)`), and propagates the store's and
/// use case's failures (a missing project, a non-commit revision, a `f` the diff
/// does not touch).
pub fn serve(store: &dyn ProjectStore, query: &[(String, String)]) -> Result<View, DomainError> {
    let project: &str =
        first(query, "p").ok_or_else(|| DomainError::Invalid("Project needed".to_owned()))?;
    if SafePath::parse(project).is_none() {
        return Err(DomainError::Invalid("Invalid project parameter".to_owned()));
    }
    let hash: Option<&str> = validate_ref(first(query, "h"), "Invalid hash parameter")?;
    let parent: Option<&str> = validate_ref(first(query, "hp"), "Invalid hash parent parameter")?;
    let file: Option<&str> = validate_path(first(query, "f"))?;
    let repository = store.open(project)?;
    let text: String = assemble_commit_diff(repository.as_ref(), hash, parent, file)?;
    Ok(View::plain_text(text))
}

/// Validates an optional file path (gitweb's `is_valid_pathname`), failing 400
/// when present but unsafe — run before the repository is touched.
fn validate_path(value: Option<&str>) -> Result<Option<&str>, DomainError> {
    match value {
        Some(raw) if SafePath::parse(raw).is_none() => {
            Err(DomainError::Invalid("Invalid file parameter".to_owned()))
        }
        other => Ok(other),
    }
}

/// Validates an optional revision selector (gitweb's `is_valid_refname`), failing
/// with `message` when it is present but unsafe — the front line against
/// malformed-ref attacks, run before the repository is touched.
fn validate_ref<'a>(value: Option<&'a str>, message: &str) -> Result<Option<&'a str>, DomainError> {
    match value {
        Some(raw) if SafeRef::parse(raw).is_none() => Err(DomainError::Invalid(message.to_owned())),
        other => Ok(other),
    }
}

/// The first value supplied for a short CGI name, mirroring CGI's `param()`.
fn first<'a>(params: &'a [(String, String)], key: &str) -> Option<&'a str> {
    params
        .iter()
        .find(|(name, _): &&(String, String)| name == key)
        .map(|(_, value): &(String, String)| value.as_str())
}
