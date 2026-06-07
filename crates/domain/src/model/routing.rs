//! The dispatch routing rule: gitweb's `dispatch()` defaulting + project gate.
//!
//! Once a request has been validated into a [`Request`], gitweb's `dispatch`
//! sub decides which `%actions` handler runs. That decision is two pure steps
//! welded to a third impure one:
//!
//! 1. **Default the action** when the request names none (gitweb's
//!    `if (!defined $action) { ... }`): a bare hash — or a base ref plus a file
//!    — means "resolve the object's kind and show the matching view"; a project
//!    with nothing else means [`summary`](Action::Summary); nothing at all means
//!    the [`project list`](Action::ProjectList).
//! 2. **Gate on the project** (gitweb's `Project needed` check): every action
//!    except `opml` / `project_list` / `project_index` needs a project.
//! 3. Resolve the object kind for the bare-hash case (gitweb's `git_get_type`).
//!
//! Steps 1 and 2 are pure — they read only the already-validated request — so
//! they live here as a domain rule. Step 3 needs the repository, so this rule
//! does not perform it: it only *flags* the case as [`Dispatch::ResolveObjectKind`],
//! and the object-resolution use case (a Control over the `Repository` port)
//! carries it out. The web boundary's dispatcher consumes the [`Dispatch`] this
//! returns and invokes the registered handler.

use crate::error::DomainError;
use crate::model::action::Action;
use crate::model::request::Request;

/// What gitweb's `dispatch` decides to do with a validated request: serve a
/// concrete action, or resolve an object's kind first because none was named.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Dispatch {
    /// Serve this action directly — either the request named it, or it is the
    /// default gitweb chose ([`summary`](Action::Summary) for a bare project,
    /// [`project_list`](Action::ProjectList) for a bare request).
    Action(Action),
    /// The request named no action but carried a hash (or a base ref and a
    /// file): gitweb resolves the referenced object's kind with `git_get_type`
    /// and serves the matching view. That lookup needs the repository, so it is
    /// the object-resolution use case's job, not this rule's.
    ResolveObjectKind,
}

/// Applies gitweb's `dispatch` routing rule to a validated [`Request`]: default
/// the action, then gate on the project. Returns the [`Dispatch`] to carry out,
/// or [`DomainError::Invalid`] with gitweb's `Project needed` message (a 400)
/// when the resolved action requires a project the request did not supply.
///
/// # Errors
///
/// Returns [`DomainError::Invalid`] when the routed action needs a project and
/// none was given.
pub fn route(request: &Request) -> Result<Dispatch, DomainError> {
    let dispatch: Dispatch = default_action(request);
    if needs_project(dispatch) && request.project.is_none() {
        return Err(DomainError::Invalid("Project needed".to_owned()));
    }
    Ok(dispatch)
}

/// gitweb's action-defaulting ladder (`if (!defined $action) { ... }`): an
/// explicit action stands; otherwise a hash — or a base ref plus a file —
/// defers to object-kind resolution, a project alone is the summary, and a bare
/// request is the project list. The order matters: a hash outranks a project,
/// exactly as gitweb tests `defined $hash` before `defined $project`.
fn default_action(request: &Request) -> Dispatch {
    if let Some(action) = request.action {
        return Dispatch::Action(action);
    }
    if request.hash.is_some() || (request.hash_base.is_some() && request.file_name.is_some()) {
        return Dispatch::ResolveObjectKind;
    }
    if request.project.is_some() {
        return Dispatch::Action(Action::Summary);
    }
    Dispatch::Action(Action::ProjectList)
}

/// Whether the routed dispatch requires a project (gitweb's gate). A concrete
/// action defers to [`Action::needs_project`]; object-kind resolution always
/// yields a project-scoped view (blob / tree / commit / tag), so it needs one.
fn needs_project(dispatch: Dispatch) -> bool {
    match dispatch {
        Dispatch::Action(action) => action.needs_project(),
        Dispatch::ResolveObjectKind => true,
    }
}
