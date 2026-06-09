//! The dispatch table: gitweb's `%actions` map from action to handler.
//!
//! gitweb's `dispatch` sub defaults the action, gates on the project, then calls
//! `$actions{$action}->()`. The defaulting and the gate are the pure
//! [`route`](gitweb_domain::model::routing::route) rule; this is the table that
//! holds the handlers and invokes the chosen one. The per-action handlers are
//! supplied by the capability beads — each [`Handler`] holds its own port
//! references (wired by the composition root) and turns a validated [`Request`]
//! into a [`View`]. Until a capability lands its handler, that action takes
//! gitweb's `die_error(400, "Unknown action")` path.

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use gitweb_domain::error::DomainError;
use gitweb_domain::model::action::Action;
use gitweb_domain::model::request::Request;
use gitweb_domain::model::routing::{Dispatch, route};

use crate::response::View;

/// One gitweb action's view handler. Implemented by the capability beads, one
/// per action; the implementation captures whatever ports it needs.
pub trait Handler: Send + Sync {
    /// Serve this action for an already-validated, already-routed request.
    ///
    /// # Errors
    ///
    /// Returns the [`DomainError`] whose status the web boundary maps to
    /// gitweb's `die_error` page.
    fn handle(&self, request: &Request) -> Result<View, DomainError>;
}

/// gitweb's no-action default object resolution (the `dispatch` sub's
/// `git_get_type` branch). When a request names no action but carries a hash —
/// or a base ref and a file — gitweb resolves the referenced object's kind and
/// serves the matching view *inline* (not a redirect). That lookup needs the
/// repository, so it cannot be the pure [`route`] rule's job; this resolver does
/// the repository half — returning the action whose handler the dispatcher then
/// invokes with the request unchanged. The composition root wires the concrete
/// implementation (over the project store), mirroring how each [`Handler`] is.
pub trait ObjectKindResolver: Send + Sync {
    /// Resolves the no-action default to the action whose view serves the
    /// referenced object — gitweb's `dispatch` `$action = git_get_type(...)`.
    ///
    /// # Errors
    ///
    /// Returns the [`DomainError`] whose status the boundary maps to gitweb's
    /// `die_error` page — gitweb's `dispatch` 404s ("Object does not exist",
    /// "File or directory does not exist") for a lookup miss.
    fn resolve(&self, request: &Request) -> Result<Action, DomainError>;
}

/// gitweb's `%actions` dispatch table: the registered handler for each action,
/// plus the resolver for the no-action object-kind default.
#[derive(Default, Clone)]
pub struct Dispatcher {
    handlers: HashMap<Action, Arc<dyn Handler>>,
    object_resolver: Option<Arc<dyn ObjectKindResolver>>,
}

impl Dispatcher {
    /// An empty table. The composition root registers a handler per supported
    /// action before serving.
    #[must_use]
    pub fn new() -> Self {
        Self::default()
    }

    /// Registers the handler for one action, replacing any previous one.
    pub fn register(&mut self, action: Action, handler: Arc<dyn Handler>) -> &mut Self {
        self.handlers.insert(action, handler);
        self
    }

    /// Wires the resolver for gitweb's no-action object-kind default, replacing
    /// any previous one. Without it, a no-action request carrying a hash (or a
    /// base ref and a file) takes the `Unknown action` path.
    pub fn set_object_resolver(&mut self, resolver: Arc<dyn ObjectKindResolver>) -> &mut Self {
        self.object_resolver = Some(resolver);
        self
    }

    /// Routes and serves a validated request: apply the domain routing rule
    /// (default the action, gate on the project), then invoke the registered
    /// handler. A request that names no action but carries a hash (or a base ref
    /// and a file) has its object kind resolved (gitweb's `dispatch`
    /// `git_get_type`) and the matching view served inline. An action with no
    /// registered handler — or object resolution with no wired resolver — takes
    /// gitweb's `die_error(400, "Unknown action")` path.
    ///
    /// # Errors
    ///
    /// Returns the routing rule's [`DomainError`] (e.g. `Project needed`), the
    /// resolver's 404 for an object that does not exist, the `Unknown action`
    /// error for an unserved action, or whatever the invoked handler fails with.
    pub fn dispatch(&self, request: &Request) -> Result<View, DomainError> {
        match route(request)? {
            Dispatch::Action(action) => self.invoke(action, request),
            Dispatch::ResolveObjectKind => self.resolve_object_kind(request),
        }
    }

    /// gitweb's `dispatch` no-action default: resolve the referenced object's
    /// kind, then serve the matching view inline by invoking its registered
    /// handler with the request unchanged. With no resolver wired, this is the
    /// `Unknown action` path.
    fn resolve_object_kind(&self, request: &Request) -> Result<View, DomainError> {
        let action: Action = match &self.object_resolver {
            Some(resolver) => resolver.resolve(request)?,
            None => return Err(unknown_action()),
        };
        self.invoke(action, request)
    }

    /// Invokes the handler registered for `action`, or the unknown-action error
    /// when none is.
    fn invoke(&self, action: Action, request: &Request) -> Result<View, DomainError> {
        match self.handlers.get(&action) {
            Some(handler) => handler.handle(request),
            None => Err(unknown_action()),
        }
    }
}

impl fmt::Debug for Dispatcher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut actions: Vec<Action> = self.handlers.keys().copied().collect();
        actions.sort_by_key(|action: &Action| action.as_str());
        f.debug_struct("Dispatcher")
            .field("registered", &actions)
            .field("object_resolver", &self.object_resolver.is_some())
            .finish()
    }
}

/// gitweb's `dispatch` `die_error(400, "Unknown action")` — an action the server
/// does not (yet) serve.
fn unknown_action() -> DomainError {
    DomainError::Invalid("Unknown action".to_owned())
}
