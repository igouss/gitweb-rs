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

/// gitweb's `%actions` dispatch table: the registered handler for each action.
#[derive(Default, Clone)]
pub struct Dispatcher {
    handlers: HashMap<Action, Arc<dyn Handler>>,
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

    /// Routes and serves a validated request: apply the domain routing rule
    /// (default the action, gate on the project), then invoke the registered
    /// handler. A request that defaults to object-kind resolution, or names an
    /// action with no registered handler, takes gitweb's
    /// `die_error(400, "Unknown action")` path until its capability lands.
    ///
    /// # Errors
    ///
    /// Returns the routing rule's [`DomainError`] (e.g. `Project needed`), the
    /// `Unknown action` error for an unserved action, or whatever the invoked
    /// handler fails with.
    pub fn dispatch(&self, request: &Request) -> Result<View, DomainError> {
        match route(request)? {
            Dispatch::Action(action) => self.invoke(action, request),
            Dispatch::ResolveObjectKind => Err(unknown_action()),
        }
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
            .finish()
    }
}

/// gitweb's `dispatch` `die_error(400, "Unknown action")` — an action the server
/// does not (yet) serve.
fn unknown_action() -> DomainError {
    DomainError::Invalid("Unknown action".to_owned())
}
