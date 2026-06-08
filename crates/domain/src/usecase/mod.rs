//! Use cases: the application logic that drives the [ports](crate::port) and
//! assembles framework-free view-models for the boundaries to render.
//!
//! A use case is the Control in ECB terms: it orchestrates a request against the
//! ports (a [`ProjectStore`](crate::port::project_store::ProjectStore), a
//! [`Repository`](crate::port::repository::Repository)) and the pure entity
//! rules, and returns a view-model. It depends only inward — it knows nothing of
//! HTTP, gix, or HTML — so the web and render adapters consume its output
//! without the domain knowing they exist.

pub mod heads;
pub mod log;
pub mod log_generic;
pub mod project_list;
pub mod shortlog;
pub mod tag;
pub mod tags;
