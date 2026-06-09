//! The ref-markers use case: read every reference once, then badge any commit
//! row with the refs whose tip is that commit.
//!
//! gitweb's commit-list bodies (shortlog, log, history) call `git_get_references`
//! once before walking the log, then `format_ref_marker` per row. This use case is
//! that first read, indexed: a body builds a [`RefMarkerIndex`] for its
//! [`MarkerView`] and asks it for each commit's markers, so the refs are read a
//! single time per page rather than per row.

use crate::error::DomainError;
use crate::model::object_id::ObjectId;
use crate::model::ref_marker::{MarkerView, RefMarker, markers_for};
use crate::model::reference::DereferencedRef;
use crate::port::repository::Repository;

/// Every reference read once, ready to badge any commit row for one view.
#[derive(Debug, Clone)]
pub struct RefMarkerIndex {
    refs: Vec<DereferencedRef>,
    view: MarkerView,
}

impl RefMarkerIndex {
    /// The badges decorating `commit` in this index's view, in ref-name order —
    /// `format_ref_marker($refs, $commit)` for one commit id.
    #[must_use]
    pub fn for_commit(&self, commit: &ObjectId) -> Vec<RefMarker> {
        markers_for(&self.refs, commit, self.view)
    }
}

/// Reads every reference from the repository and indexes it for `view`'s commit
/// rows — gitweb reading `git_get_references` once before walking the log.
///
/// # Errors
///
/// Propagates the repository's error if listing the references fails.
pub fn assemble_ref_markers(
    repo: &dyn Repository,
    view: MarkerView,
) -> Result<RefMarkerIndex, DomainError> {
    let refs: Vec<DereferencedRef> = repo.dereferenced_references()?;
    Ok(RefMarkerIndex { refs, view })
}
