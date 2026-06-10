//! gitweb's branch-ref directory set: which directories under `refs/` count as
//! branches.
//!
//! gitweb's `get_branch_refs` answers "which ref directories are branches": always
//! `heads`, plus whatever the `extra-branch-refs` feature configures. The feature's
//! configured entries pass through `filter_and_validate_refs` first — a malformed
//! entry takes the whole site down (gitweb's `die_error(500, ...)`), a literal
//! `heads` is dropped (it is added back implicitly), and the survivors are sorted
//! and de-duplicated. The result is `heads` followed by that set.
//!
//! Wiring this is what lets a snapshot of a ref under an extra branch directory be
//! named with the directory prefix (gitweb's `snapshot_name`), and — for the
//! consumers still on the bare default — what would let the heads listing and last
//! activity count refs outside `refs/heads/`.
//!
//! Divergence in *when* validation fires: gitweb validates once at config load
//! (`evaluate_gitweb_config`), so a malformed entry breaks every page. We resolve
//! the set lazily at each consumer and surface the same 500 there, keeping this a
//! pure rule with no global state.

use std::collections::BTreeSet;

use crate::error::DomainError;

/// gitweb's `get_branch_refs`: the ref directories under `refs/` that count as
/// branches — `heads`, followed by the validated `extra-branch-refs` entries.
///
/// `extra` is the `extra-branch-refs` feature's option list (gitweb's
/// `gitweb_get_feature('extra-branch-refs')`, already split into one entry per
/// ref). It runs through [`filter_and_validate_refs`]; the result is `heads`
/// prepended to that sorted, de-duplicated set.
///
/// # Errors
/// [`DomainError::Backend`] (gitweb's `die_error(500, ...)`) for the first entry
/// that fails [`is_valid_ref_format`], with the message
/// `Invalid ref '<ref>' in 'extra-branch-refs' feature`.
pub fn get_branch_refs(extra: &[String]) -> Result<Vec<String>, DomainError> {
    let mut refs: Vec<String> = vec!["heads".to_owned()];
    refs.extend(filter_and_validate_refs(extra)?);
    Ok(refs)
}

/// gitweb's `filter_and_validate_refs`: validate every entry (a malformed one
/// dies), drop a literal `heads` (it is added implicitly by [`get_branch_refs`]),
/// then return the survivors sorted and de-duplicated (gitweb's
/// `sort keys %unique_refs`).
fn filter_and_validate_refs(refs: &[String]) -> Result<Vec<String>, DomainError> {
    let mut unique: BTreeSet<String> = BTreeSet::new();
    for reference in refs {
        if !is_valid_ref_format(reference) {
            return Err(DomainError::Backend(format!(
                "Invalid ref '{reference}' in 'extra-branch-refs' feature"
            )));
        }
        if reference != "heads" {
            unique.insert(reference.clone());
        }
    }
    Ok(unique.into_iter().collect())
}

/// gitweb's `is_valid_ref_format`: the `git-check-ref-format` subset gitweb
/// enforces (`m!(/\.|\.\.|[\000-\040\177 ~^:?*\[]|/$)!`). A fragment is rejected
/// if it contains `/.`, `..`, or any control character (`0x00`–`0x20`, including
/// space), `0x7f`, or one of `~ ^ : ? * [`, or if it ends with `/`.
fn is_valid_ref_format(reference: &str) -> bool {
    if reference.contains("/.") || reference.contains("..") || reference.ends_with('/') {
        return false;
    }
    !reference.bytes().any(|byte: u8| {
        matches!(
            byte,
            0x00..=0x20 | 0x7f | b'~' | b'^' | b':' | b'?' | b'*' | b'['
        )
    })
}
